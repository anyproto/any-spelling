//! One checker per selected language set: the union of its Hunspell
//! dictionaries, a personal vocabulary Hunspell never sees, and bounded caches.

use std::collections::{HashMap, HashSet, VecDeque};
use std::hash::Hash;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, LazyLock, Mutex, PoisonError};

use regex::Regex;
use serde::Serialize;

use super::hunspell::Dictionary;
use super::languages::Script;
use super::tokenize::{
    js_trim, normalize_spelling_word, personal_key, spelling_words, utf16_len, Utf16Offsets,
    MAX_SPELLING_TEXT, MAX_SPELLING_WORD,
};

const MAX_CACHE: usize = 8_000;
const MAX_SUGGESTION_CACHE: usize = 128;
const MAX_SUGGESTIONS: usize = 8;
const MAX_PERSONAL_WORDS: usize = 10_000;

static LATIN: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^(?:\p{Script=Latin}|\p{M}|['’ʼ-])+$").expect("valid"));
static CYRILLIC: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^(?:\p{Script=Cyrillic}|\p{M}|['’ʼ-])+$").expect("valid"));
static PERSONAL_WORD: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"[\p{L}\p{M}\p{N}]+(?:['’.-][\p{L}\p{M}\p{N}]+)*").expect("valid")
});

/// A misspelling; offsets address the checked string in UTF-16 code units.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Issue {
    pub start: usize,
    pub end: usize,
    pub word: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SpellingError {
    Cancelled,
    Message(String),
}

impl From<String> for SpellingError {
    fn from(message: String) -> Self {
        Self::Message(message)
    }
}

impl From<&str> for SpellingError {
    fn from(message: &str) -> Self {
        Self::Message(message.to_owned())
    }
}

pub struct LanguageDictionary {
    pub dictionary: Arc<Dictionary>,
    pub script: Script,
}

impl LanguageDictionary {
    fn accepts(&self, word: &str) -> bool {
        match self.script {
            Script::Latin => LATIN.is_match(word),
            Script::Cyrillic => CYRILLIC.is_match(word),
        }
    }
}

/// Insertion-ordered, evicting the oldest entry when full.
struct FifoCache<K, V> {
    max: usize,
    order: VecDeque<K>,
    values: HashMap<K, V>,
}

impl<K: Clone + Eq + Hash, V: Clone> FifoCache<K, V> {
    fn new(max: usize) -> Self {
        Self {
            max,
            order: VecDeque::new(),
            values: HashMap::new(),
        }
    }

    fn get(&self, key: &K) -> Option<V> {
        self.values.get(key).cloned()
    }

    fn insert(&mut self, key: K, value: V) {
        if let Some(existing) = self.values.get_mut(&key) {
            *existing = value;
            return;
        }
        if self.values.len() >= self.max {
            if let Some(oldest) = self.order.pop_front() {
                self.values.remove(&oldest);
            }
        }
        self.order.push_back(key.clone());
        self.values.insert(key, value);
    }

    fn remove(&mut self, key: &K) {
        if self.values.remove(key).is_some() {
            self.order.retain(|item| item != key);
        }
    }

    fn clear(&mut self) {
        self.order.clear();
        self.values.clear();
    }
}

struct State {
    words: FifoCache<String, bool>,
    suggestions: FifoCache<String, Vec<String>>,
    personal: HashSet<String>,
}

pub struct SpellingEngine {
    dictionaries: Vec<LanguageDictionary>,
    state: Mutex<State>,
    disposed: AtomicBool,
}

impl SpellingEngine {
    pub fn new(
        dictionaries: Vec<LanguageDictionary>,
        personal_words: &[String],
    ) -> Result<Self, SpellingError> {
        if dictionaries.is_empty() {
            return Err("Choose a language to check spelling.".into());
        }
        let engine = Self {
            dictionaries,
            state: Mutex::new(State {
                words: FifoCache::new(MAX_CACHE),
                suggestions: FifoCache::new(MAX_SUGGESTION_CACHE),
                personal: HashSet::new(),
            }),
            disposed: AtomicBool::new(false),
        };
        engine.set_personal_words(personal_words)?;
        Ok(engine)
    }

    fn state(&self) -> std::sync::MutexGuard<'_, State> {
        self.state.lock().unwrap_or_else(PoisonError::into_inner)
    }

    fn assert_active(&self) -> Result<(), SpellingError> {
        if self.disposed.load(Ordering::Acquire) {
            return Err("The spelling checker was closed.".into());
        }
        Ok(())
    }

    fn correct(&self, word: &str) -> bool {
        let normalized = normalize_spelling_word(word);
        {
            let state = self.state();
            if state.personal.contains(&normalized.to_lowercase()) {
                return true;
            }
            if let Some(known) = state.words.get(&normalized) {
                return known;
            }
        }
        // Some dictionaries (notably Ukrainian) discard foreign letters via
        // ICONV, accepting arbitrary Latin typos. Only a dictionary for the
        // word's script can validate it; personal vocabulary spans all scripts.
        let correct = self
            .dictionaries
            .iter()
            .any(|entry| entry.accepts(&normalized) && entry.dictionary.spell(&normalized));
        self.state().words.insert(normalized, correct);
        correct
    }

    pub fn check(&self, text: &str, cancelled: &AtomicBool) -> Result<Vec<Issue>, SpellingError> {
        self.assert_active()?;
        if utf16_len(text) > MAX_SPELLING_TEXT {
            return Err("This text is too large to check spelling at once.".into());
        }
        // Segmenters split hyphenated vocabulary. Accept the complete saved
        // word before checking its fragments, without accepting those
        // fragments elsewhere.
        let accepted: Vec<(usize, usize)> = {
            let state = self.state();
            if state.personal.is_empty() {
                Vec::new()
            } else {
                PERSONAL_WORD
                    .find_iter(text)
                    .filter(|found| state.personal.contains(&personal_key(found.as_str())))
                    .map(|found| (found.start(), found.end()))
                    .collect()
            }
        };
        let mut accepted_index = 0;
        let mut offsets = Utf16Offsets::new(text);
        let mut issues = Vec::new();
        for word in spelling_words(text) {
            if cancelled.load(Ordering::Relaxed) {
                return Err(SpellingError::Cancelled);
            }
            self.assert_active()?;
            while accepted
                .get(accepted_index)
                .is_some_and(|span| span.1 <= word.start)
            {
                accepted_index += 1;
            }
            let personal = accepted
                .get(accepted_index)
                .is_some_and(|span| span.0 <= word.start && span.1 >= word.end);
            if !personal && !self.correct(word.word) {
                issues.push(Issue {
                    start: offsets.at(word.start),
                    end: offsets.at(word.end),
                    word: word.word.to_owned(),
                });
            }
        }
        if cancelled.load(Ordering::Relaxed) {
            return Err(SpellingError::Cancelled);
        }
        Ok(issues)
    }

    pub fn suggest(&self, word: &str) -> Result<Vec<String>, SpellingError> {
        self.assert_active()?;
        if word.is_empty() || utf16_len(word) > MAX_SPELLING_WORD || self.correct(word) {
            return Ok(Vec::new());
        }
        let normalized = normalize_spelling_word(word);
        if let Some(previous) = self.state().suggestions.get(&normalized) {
            return Ok(previous);
        }
        let candidates: Vec<&LanguageDictionary> = self
            .dictionaries
            .iter()
            .filter(|entry| entry.accepts(&normalized))
            .collect();
        // Suggesting is the slow call; each dictionary has its own lock.
        let choices: Vec<Vec<String>> = std::thread::scope(|scope| {
            let workers: Vec<_> = candidates
                .iter()
                .map(|entry| scope.spawn(|| entry.dictionary.suggest(&normalized)))
                .collect();
            workers
                .into_iter()
                .map(|worker| worker.join().unwrap_or_default())
                .collect()
        });
        let ranks = choices.iter().map(Vec::len).max().unwrap_or(0);
        let mut result = Vec::new();
        let mut seen = HashSet::new();
        // Interleave each dictionary's ranking; the first selected language
        // must not hide every correction from the second one. Case variants
        // share one slot, retaining Hunspell's first spelling. Limit unique
        // results, not raw ranks: duplicates near the top must not hide later
        // useful corrections.
        'ranks: for index in 0..ranks {
            for list in &choices {
                let Some(suggestion) = list.get(index) else {
                    continue;
                };
                if *suggestion == normalized || !seen.insert(personal_key(suggestion)) {
                    continue;
                }
                result.push(suggestion.clone());
                if result.len() == MAX_SUGGESTIONS {
                    break 'ranks;
                }
            }
        }
        self.state().suggestions.insert(normalized, result.clone());
        Ok(result)
    }

    pub fn add_word(&self, word: &str) -> Result<(), SpellingError> {
        self.assert_active()?;
        let normalized = personal_key(js_trim(word));
        let length = utf16_len(&normalized);
        if length == 0 || length > MAX_SPELLING_WORD {
            return Err("Personal words must contain between 1 and 128 characters.".into());
        }
        let mut state = self.state();
        if state.personal.contains(&normalized) {
            return Ok(());
        }
        if state.personal.len() >= MAX_PERSONAL_WORDS {
            return Err("The personal dictionary can contain at most 10,000 words.".into());
        }
        state.words.remove(&normalized);
        state.personal.insert(normalized);
        state.suggestions.clear();
        Ok(())
    }

    /// Replaces the vocabulary; Hunspell itself never sees personal words.
    pub fn set_personal_words(&self, words: &[String]) -> Result<(), SpellingError> {
        self.assert_active()?;
        {
            let mut state = self.state();
            state.personal.clear();
            state.words.clear();
            state.suggestions.clear();
        }
        words.iter().try_for_each(|word| self.add_word(word))
    }

    pub fn dispose(&self) {
        if self.disposed.swap(true, Ordering::AcqRel) {
            return;
        }
        let mut state = self.state();
        state.words.clear();
        state.suggestions.clear();
        state.personal.clear();
    }
}
