//! Word extraction for spell checking, matching a JavaScript caller's tokenizer:
//! `Intl.Segmenter` word segments (ICU rules), with code spans, URLs, emails,
//! paths and dotted identifiers excluded before segmentation.

use std::sync::LazyLock;

use fancy_regex::Regex as FancyRegex;
use icu_normalizer::ComposingNormalizerBorrowed;
use icu_segmenter::options::WordBreakInvariantOptions;
use icu_segmenter::WordSegmenter;
use regex::Regex;

/// Both limits count UTF-16 code units, as JavaScript measures strings.
pub const MAX_SPELLING_TEXT: usize = 200_000;
pub const MAX_SPELLING_WORD: usize = 128;

/// ECMAScript `\s`, which differs from Unicode `White_Space` at U+0085 and U+FEFF.
const JS_SPACE: &str = r"\t\n\x0B\x0C\r \u{A0}\u{1680}\u{2000}-\u{200A}\u{2028}\u{2029}\u{202F}\u{205F}\u{3000}\u{FEFF}";

// JavaScript's `\b` under the `u` flag is ASCII; it precedes `[a-z]` or `w`
// here, so it is exactly "not after an ASCII word character". Under `(?i)` that
// class also matches U+017F and U+212A, as `\b` does in JavaScript with `iu`.
static EXCLUDED: LazyLock<FancyRegex> = LazyLock::new(|| {
    FancyRegex::new(&format!(
        r"(?i)```(?s:.)*?(?:```|$)|`[^`\n]*`|(?<![A-Za-z0-9_])(?:[a-z][a-z0-9+.-]*://|www\.)[^{JS_SPACE}<>]+|(?<![\p{{L}}\p{{N}}.!#$%&'*+/=?^_`{{|}}~-])[\p{{L}}\p{{N}}.!#$%&'*+/=?^_`{{|}}~-]+@[\p{{L}}\p{{N}}.-]+\.\p{{L}}{{2,}}|(?:[A-Za-z]:\\|/)[\p{{L}}\p{{N}}_.\\/-]+|(?<![\p{{L}}\p{{N}}_])[\p{{L}}\p{{N}}_]+(?:\.[\p{{L}}\p{{N}}_]+)+"
    ))
    .expect("the exclusion pattern compiles")
});
static LETTER: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\p{L}").expect("valid"));
static NUMBER_OR_UNDERSCORE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"[\p{N}_]").expect("valid"));
static CAMEL_CASE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\p{Ll}\p{Lu}").expect("valid"));

/// A word to check; offsets are UTF-8 byte offsets into the checked text.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Word<'a> {
    pub start: usize,
    pub end: usize,
    pub word: &'a str,
}

pub fn utf16_len(text: &str) -> usize {
    text.chars().map(char::len_utf16).sum()
}

fn word_segments(text: &str) -> Vec<Word<'_>> {
    let mut breaks =
        WordSegmenter::new_dictionary(WordBreakInvariantOptions::default()).segment_str(text);
    let mut words = Vec::new();
    let mut start = 0;
    while let Some(end) = breaks.next() {
        if end > start && breaks.is_word_like() {
            words.push(Word {
                start,
                end,
                word: &text[start..end],
            });
        }
        start = end;
    }
    words
}

pub fn spelling_words(text: &str) -> Vec<Word<'_>> {
    // A runaway match is a pattern failure, not text to check: skip the rest
    // of the exclusions rather than reporting fragments of an address.
    let ignored: Vec<(usize, usize)> = EXCLUDED
        .find_iter(text)
        .map_while(Result::ok)
        .map(|found| (found.start(), found.end()))
        .collect();
    let mut index = 0;
    word_segments(text)
        .into_iter()
        .filter(|word| {
            while ignored.get(index).is_some_and(|span| span.1 <= word.start) {
                index += 1;
            }
            if ignored
                .get(index)
                .is_some_and(|span| span.0 < word.end && span.1 > word.start)
            {
                return false;
            }
            utf16_len(word.word) <= MAX_SPELLING_WORD
                && LETTER.is_match(word.word)
                && !NUMBER_OR_UNDERSCORE.is_match(word.word)
                && !CAMEL_CASE.is_match(word.word)
        })
        .collect()
}

/// NFC with typographic apostrophes folded to ASCII, as Hunspell expects.
pub fn normalize_spelling_word(word: &str) -> String {
    ComposingNormalizerBorrowed::new_nfc()
        .normalize(word)
        .replace('\u{2019}', "'")
}

/// The personal-vocabulary key: `toLocaleLowerCase('und')` of the normalized word.
pub fn personal_key(word: &str) -> String {
    normalize_spelling_word(word).to_lowercase()
}

/// ECMAScript `String.prototype.trim`.
pub fn js_trim(text: &str) -> &str {
    let space = |c: char| {
        matches!(
            c,
            '\t' | '\n' | '\u{B}' | '\u{C}' | '\r' | ' ' | '\u{A0}' | '\u{1680}' | '\u{2000}'
                ..='\u{200A}'
                    | '\u{2028}'
                    | '\u{2029}'
                    | '\u{202F}'
                    | '\u{205F}'
                    | '\u{3000}'
                    | '\u{FEFF}'
        )
    };
    text.trim_matches(space)
}

/// Converts ascending byte offsets into UTF-16 offsets in one pass.
pub struct Utf16Offsets<'a> {
    text: &'a str,
    byte: usize,
    unit: usize,
}

impl<'a> Utf16Offsets<'a> {
    pub fn new(text: &'a str) -> Self {
        Self {
            text,
            byte: 0,
            unit: 0,
        }
    }

    pub fn at(&mut self, byte: usize) -> usize {
        if byte < self.byte {
            self.byte = 0;
            self.unit = 0;
        }
        self.unit += utf16_len(&self.text[self.byte..byte]);
        self.byte = byte;
        self.unit
    }
}
