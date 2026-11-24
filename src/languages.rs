//! The dictionaries in `dictionaries/`, with the script each one can validate.
//! The app's language picker lists the same ids and keeps a test that they agree.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Script {
    Latin,
    Cyrillic,
}

pub struct Language {
    pub id: &'static str,
    pub label: &'static str,
    pub script: Script,
}

pub const LANGUAGES: &[Language] = &[
    Language {
        id: "en-US",
        label: "English (US)",
        script: Script::Latin,
    },
    Language {
        id: "en-GB",
        label: "English (UK)",
        script: Script::Latin,
    },
    Language {
        id: "de-DE",
        label: "Deutsch",
        script: Script::Latin,
    },
    Language {
        id: "ru-RU",
        label: "Русский",
        script: Script::Cyrillic,
    },
    Language {
        id: "uk-UA",
        label: "Українська",
        script: Script::Cyrillic,
    },
    Language {
        id: "fr-FR",
        label: "Français",
        script: Script::Latin,
    },
    Language {
        id: "es-ES",
        label: "Español (España)",
        script: Script::Latin,
    },
    Language {
        id: "it-IT",
        label: "Italiano",
        script: Script::Latin,
    },
    Language {
        id: "pt-BR",
        label: "Português (Brasil)",
        script: Script::Latin,
    },
    Language {
        id: "pt-PT",
        label: "Português (Portugal)",
        script: Script::Latin,
    },
    Language {
        id: "nl-NL",
        label: "Nederlands",
        script: Script::Latin,
    },
    Language {
        id: "pl-PL",
        label: "Polski",
        script: Script::Latin,
    },
];

const ENGLISH_VOCABULARY: &str = include_str!("../vocabulary/en.txt");

/// Words a language's dictionary lacks, added to it when it loads.
pub fn vocabulary(id: &str) -> impl Iterator<Item = &'static str> {
    let text = if id.starts_with("en-") {
        ENGLISH_VOCABULARY
    } else {
        ""
    };
    text.lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
}

pub fn language(id: &str) -> Option<&'static Language> {
    LANGUAGES.iter().find(|language| language.id == id)
}
