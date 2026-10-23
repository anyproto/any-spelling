//! Spell checking over the Hunspell dictionaries in this repository: one
//! session per selected language set, sharing each loaded dictionary, with
//! issues reported in UTF-16 offsets for a JavaScript caller.

mod engine;
mod ffi;
mod hunspell;
mod languages;
mod registry;
#[cfg(test)]
mod tests;
mod tokenize;

pub use engine::{Issue, SpellingEngine, SpellingError};
pub use languages::{Language, Script, LANGUAGES};
pub use registry::{Registry, Request};
