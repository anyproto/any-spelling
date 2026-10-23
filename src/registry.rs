//! Sessions for every window, sharing each loaded dictionary between them.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, PoisonError, Weak};

use super::engine::{LanguageDictionary, SpellingEngine, SpellingError};
use super::hunspell::Dictionary;
use super::languages::language;

struct Session {
    window: String,
    engine: Arc<SpellingEngine>,
}

#[derive(Default)]
struct Inner {
    next_session: u64,
    sessions: HashMap<u64, Session>,
    dictionaries: HashMap<&'static str, Weak<Dictionary>>,
    requests: HashMap<(u64, u64), Arc<AtomicBool>>,
}

pub struct Registry {
    directory: PathBuf,
    inner: Mutex<Inner>,
}

/// Unregisters a request's cancellation flag when the request settles.
pub struct Request<'a> {
    registry: &'a Registry,
    key: (u64, u64),
    pub cancelled: Arc<AtomicBool>,
}

impl Drop for Request<'_> {
    fn drop(&mut self) {
        self.registry.inner().requests.remove(&self.key);
    }
}

impl Registry {
    pub fn new(directory: PathBuf) -> Self {
        Self {
            directory,
            inner: Mutex::new(Inner::default()),
        }
    }

    fn inner(&self) -> std::sync::MutexGuard<'_, Inner> {
        self.inner.lock().unwrap_or_else(PoisonError::into_inner)
    }

    fn dictionary(&self, id: &str) -> Result<LanguageDictionary, SpellingError> {
        let entry = language(id).ok_or("This spelling language is unavailable.")?;
        let shared = self
            .inner()
            .dictionaries
            .get(entry.id)
            .and_then(Weak::upgrade);
        let dictionary = match shared {
            Some(dictionary) => dictionary,
            None => {
                // Loading takes up to a few hundred milliseconds; other windows
                // keep checking meanwhile. A concurrent load of the same
                // language costs one redundant parse, never a wrong result.
                let loaded = Arc::new(open_dictionary(&self.directory, entry.id, entry.label)?);
                let mut inner = self.inner();
                match inner.dictionaries.get(entry.id).and_then(Weak::upgrade) {
                    Some(raced) => raced,
                    None => {
                        inner.dictionaries.insert(entry.id, Arc::downgrade(&loaded));
                        loaded
                    }
                }
            }
        };
        Ok(LanguageDictionary {
            dictionary,
            script: entry.script,
        })
    }

    pub fn prepare(
        &self,
        window: &str,
        languages: &[String],
        personal_words: &[String],
    ) -> Result<u64, SpellingError> {
        if languages.is_empty() {
            return Err("Choose a language to check spelling.".into());
        }
        let mut unique: Vec<&String> = Vec::new();
        for id in languages {
            if !unique.contains(&id) {
                unique.push(id);
            }
        }
        // Each dictionary parses on its own thread; Ukrainian alone takes
        // about as long as the other eleven together.
        let dictionaries = std::thread::scope(|scope| {
            let loads: Vec<_> = unique
                .into_iter()
                .map(|id| scope.spawn(move || self.dictionary(id)))
                .collect();
            loads
                .into_iter()
                .map(|load| {
                    load.join()
                        .unwrap_or_else(|_| Err("The spelling checker could not load.".into()))
                })
                .collect::<Result<Vec<_>, _>>()
        })?;
        let engine = Arc::new(SpellingEngine::new(dictionaries, personal_words)?);
        let mut inner = self.inner();
        inner.next_session += 1;
        let id = inner.next_session;
        inner.sessions.insert(
            id,
            Session {
                window: window.to_owned(),
                engine,
            },
        );
        Ok(id)
    }

    pub fn engine(&self, session: u64) -> Result<Arc<SpellingEngine>, SpellingError> {
        self.inner()
            .sessions
            .get(&session)
            .map(|session| Arc::clone(&session.engine))
            .ok_or_else(|| "The spelling checker was closed.".into())
    }

    pub fn begin(&self, session: u64, request: u64) -> Request<'_> {
        let cancelled = Arc::new(AtomicBool::new(false));
        self.inner()
            .requests
            .insert((session, request), Arc::clone(&cancelled));
        Request {
            registry: self,
            key: (session, request),
            cancelled,
        }
    }

    pub fn cancel(&self, session: u64, request: u64) {
        if let Some(flag) = self.inner().requests.get(&(session, request)) {
            flag.store(true, Ordering::Relaxed);
        }
    }

    pub fn dispose(&self, session: u64) {
        let removed = self.inner().sessions.remove(&session);
        if let Some(session) = removed {
            session.engine.dispose();
        }
    }

    pub fn forget_window(&self, window: &str) {
        let mut inner = self.inner();
        inner.sessions.retain(|_, session| {
            let keep = session.window != window;
            if !keep {
                session.engine.dispose();
            }
            keep
        });
    }
}

pub fn open_dictionary(directory: &Path, id: &str, label: &str) -> Result<Dictionary, String> {
    let folder = directory.join(id);
    Dictionary::open(&folder.join("index.aff"), &folder.join("index.dic"), label)
}
