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

pub fn language(id: &str) -> Option<&'static Language> {
    LANGUAGES.iter().find(|language| language.id == id)
}
