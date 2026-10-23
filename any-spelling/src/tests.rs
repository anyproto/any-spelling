use std::path::PathBuf;
use std::sync::atomic::AtomicBool;
use std::sync::{Arc, LazyLock};

use super::engine::{Issue, SpellingEngine, SpellingError};
use super::hunspell::{library_version, Dictionary};
use super::languages::LANGUAGES;
use super::registry::{open_dictionary, Registry};
use super::tokenize::{spelling_words, utf16_len};

fn dictionaries() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../dictionaries")
}

static REGISTRY: LazyLock<Registry> = LazyLock::new(|| Registry::new(dictionaries()));

fn engine(languages: &[&str], words: &[&str]) -> Arc<SpellingEngine> {
    let languages: Vec<String> = languages.iter().map(|id| (*id).to_owned()).collect();
    let words: Vec<String> = words.iter().map(|word| (*word).to_owned()).collect();
    let session = REGISTRY
        .prepare(&format!("test-{}", rand_label()), &languages, &words)
        .expect("the dictionaries load");
    REGISTRY.engine(session).expect("the session exists")
}

fn rand_label() -> u64 {
    use std::sync::atomic::{AtomicU64, Ordering};
    static NEXT: AtomicU64 = AtomicU64::new(0);
    NEXT.fetch_add(1, Ordering::Relaxed)
}

fn check(engine: &SpellingEngine, text: &str) -> Vec<Issue> {
    engine
        .check(text, &AtomicBool::new(false))
        .expect("checked")
}

fn words(issues: &[Issue]) -> Vec<&str> {
    issues.iter().map(|issue| issue.word.as_str()).collect()
}

fn suggest(engine: &SpellingEngine, word: &str) -> Vec<String> {
    engine.suggest(word).expect("suggested")
}

fn issue_at(text: &str, word: &str) -> Issue {
    let byte = text.find(word).expect("the word is in the text");
    let start = utf16_len(&text[..byte]);
    Issue {
        start,
        end: start + utf16_len(word),
        word: word.to_owned(),
    }
}

fn whole(typo: &str) -> Vec<Issue> {
    vec![Issue {
        start: 0,
        end: utf16_len(typo),
        word: typo.to_owned(),
    }]
}

#[test]
fn compiles_hunspell_1_7_5() {
    assert_eq!(library_version(), "1.7.5");
}

#[test]
fn keeps_all_reachable_for_al_instead_of_spending_slots_on_case_variants() {
    for languages in [&["en-US"][..], &["en-GB"], &["en-US", "en-GB"]] {
        let checker = engine(languages, &[]);
        assert_eq!(check(&checker, "all"), []);
        assert_eq!(check(&checker, "al"), whole("al"));
        let suggestions = suggest(&checker, "al");
        assert!(
            suggestions.iter().any(|word| word == "all"),
            "{languages:?}: {suggestions:?}"
        );
        assert!(suggestions.len() <= 8);
        let mut keys: Vec<String> = suggestions
            .iter()
            .map(|word| super::tokenize::personal_key(word))
            .collect();
        keys.sort();
        keys.dedup();
        assert_eq!(keys.len(), suggestions.len(), "{suggestions:?}");
        assert_eq!(suggest(&checker, "al"), suggestions);
    }
}

#[test]
fn preserves_dictionary_capitalization_corrections_and_multilingual_suggestions() {
    let german = engine(&["de-DE"], &[]);
    assert_eq!(check(&german, "Vater"), []);
    assert_eq!(
        suggest(&german, "vater").first().map(String::as_str),
        Some("Vater")
    );
    assert_eq!(
        suggest(&german, "berlin").first().map(String::as_str),
        Some("Berlin")
    );
    let mixed = engine(&["en-US", "de-DE"], &[]);
    assert!(suggest(&mixed, "al").contains(&"all".to_owned()));
    assert!(suggest(&mixed, "Huas").contains(&"Haus".to_owned()));
}

struct LanguageCase {
    language: &'static str,
    text: &'static str,
    typo: &'static str,
    correction: &'static str,
}

const LANGUAGE_CASES: &[LanguageCase] = &[
    LanguageCase {
        language: "ru-RU",
        text: "привет мир книга книги книгами ошибки",
        typo: "превет",
        correction: "привет",
    },
    LanguageCase {
        language: "uk-UA",
        text: "привіт світ книга книги книжками п'ять п’ять пʼять",
        typo: "привт",
        correction: "привіт",
    },
    LanguageCase {
        language: "fr-FR",
        text: "bonjour école écoles mangent l'école aujourd’hui e\u{301}cole",
        typo: "bonjor",
        correction: "bonjour",
    },
    LanguageCase {
        language: "es-ES",
        text: "hola niño niños canción canciones caminando",
        typo: "cancióm",
        correction: "canción",
    },
    LanguageCase {
        language: "it-IT",
        text: "buongiorno ragazzo ragazzi città mangiando l'amico",
        typo: "buongirno",
        correction: "buongiorno",
    },
    LanguageCase {
        language: "pt-BR",
        text: "olá coração corações caminhando ônibus ações recepção",
        typo: "coraçãp",
        correction: "coração",
    },
    LanguageCase {
        language: "pt-PT",
        text: "olá coração corações caminhando autocarro ações receção",
        typo: "coraçãp",
        correction: "coração",
    },
    LanguageCase {
        language: "nl-NL",
        text: "hallo boek boeken huizen vriendelijk vriendelijke",
        typo: "boeek",
        correction: "boek",
    },
    LanguageCase {
        language: "pl-PL",
        text: "cześć książka książki książkami przyjaciele błędy",
        typo: "ksiązka",
        correction: "książka",
    },
];

#[test]
fn checks_each_language_inflections_and_suggests_a_real_correction() {
    for case in LANGUAGE_CASES {
        let checker = engine(&[case.language], &[]);
        assert_eq!(check(&checker, case.text), [], "{}", case.language);
        assert_eq!(
            check(&checker, case.typo),
            whole(case.typo),
            "{}",
            case.language
        );
        let suggestions = suggest(&checker, case.typo);
        assert!(
            suggestions.iter().any(|word| word == case.correction),
            "{}: {suggestions:?}",
            case.language
        );
    }
}

#[test]
fn every_catalog_language_has_a_language_case() {
    let mut covered: Vec<&str> = LANGUAGE_CASES.iter().map(|case| case.language).collect();
    covered.extend(["en-US", "en-GB", "de-DE"]);
    covered.sort_unstable();
    let mut catalog: Vec<&str> = LANGUAGES.iter().map(|language| language.id).collect();
    catalog.sort_unstable();
    assert_eq!(covered, catalog);
}

#[test]
fn loads_the_complete_catalog_together_without_hiding_latin_or_cyrillic_mistakes() {
    let all: Vec<&str> = LANGUAGES.iter().map(|language| language.id).collect();
    let checker = engine(&all, &[]);
    let text = "hello colour Krankenhaus привет привіт école canción buongiorno recepção receção vriendelijk książkami zzzzqxx щщжжффцц";
    assert_eq!(words(&check(&checker, text)), ["zzzzqxx", "щщжжффцц"]);
    assert!(suggest(&checker, "превет").contains(&"привет".to_owned()));
    let five = engine(&["en-US", "de-DE", "ru-RU", "uk-UA", "fr-FR"], &[]);
    assert_eq!(check(&five, "hello Krankenhaus привет привіт école"), []);
    assert!(suggest(&five, "превет").contains(&"привет".to_owned()));
}

#[test]
fn does_not_let_ukrainian_iconv_hide_english_typos_in_a_mixed_selection() {
    let checker = engine(&["en-US", "uk-UA"], &[]);
    let text = "hello привіт пʼять helo привт zzzqxx";
    assert_eq!(words(&check(&checker, text)), ["helo", "привт", "zzzqxx"]);
    assert!(suggest(&checker, "helo").contains(&"hello".to_owned()));
    assert!(suggest(&checker, "привт").contains(&"привіт".to_owned()));
    let text = "hello привіт helo";
    assert_eq!(check(&checker, text), [issue_at(text, "helo")]);
}

#[test]
fn requires_a_selected_dictionary_for_foreign_words_while_personal_words_override_scripts() {
    let checker = engine(&["uk-UA"], &[]);
    assert_eq!(words(&check(&checker, "hello привіт")), ["hello"]);
    checker.add_word("hello").expect("added");
    checker.add_word("漢字").expect("added");
    assert_eq!(check(&checker, "hello 漢字"), []);
    // The Latin a must not disappear through the Ukrainian conversion table.
    assert_eq!(words(&check(&checker, "aпривіт")), ["aпривіт"]);
}

#[test]
fn unions_cyrillic_and_accented_latin_dictionaries_without_changing_utf16_offsets() {
    let checker = engine(&["ru-RU", "fr-FR"], &[]);
    let text = "😀 привет e\u{301}cole bonjor превет";
    let issues = check(&checker, text);
    assert_eq!(issues, [issue_at(text, "bonjor"), issue_at(text, "превет")]);
    assert_eq!(issues[1].end, utf16_len(text));
    assert_eq!(issues[0].start, 17);
}

#[test]
fn keeps_brazilian_and_european_portuguese_orthography_separate_unless_both_are_selected() {
    let text = "recepção receção";
    assert_eq!(words(&check(&engine(&["pt-BR"], &[]), text)), ["receção"]);
    assert_eq!(words(&check(&engine(&["pt-PT"], &[]), text)), ["recepção"]);
    assert_eq!(check(&engine(&["pt-BR", "pt-PT"], &[]), text), []);
}

#[test]
fn accepts_english_and_productive_german_compounds_in_the_same_text() {
    let checker = engine(&["en-US", "de-DE"], &[]);
    let text = "hello Krankenhaus Krankenhäuser Schreibtischlampe Rechtschreibprüfung Datenverarbeitung Fußballspieler Straße STRASSE zzzqxx";
    assert_eq!(check(&checker, text), [issue_at(text, "zzzqxx")]);
    let text = "hello Krankenhaus Rechtschreibprüfung anytwo ANYTWO Huas";
    let personal = engine(&["en-US", "de-DE"], &["Anytwo"]);
    assert_eq!(check(&personal, text), [issue_at(text, "Huas")]);
    assert_eq!(issue_at(text, "Huas").start, 52);
}

const GERMAN_COMPOUNDS: &[&str] = &[
    "Krankenhaus",
    "Krankenhäuser",
    "Schreibtischlampe",
    "Rechtschreibprüfung",
    "Datenverarbeitung",
    "Fußballspieler",
    "Bundesverfassungsgericht",
    "Donaudampfschifffahrt",
    "Kinderbuchautorin",
    "Arbeitszeitverkürzung",
    "Haustürschlüssel",
    "Wohnungsbaugesellschaft",
    "Sonnenblumenkerne",
    "Gartenstuhl",
    "Weihnachtsbaumschmuck",
    "Autobahnraststätte",
    "Lebensversicherung",
    "Kühlschranktür",
    "Sprachlernprogramm",
    "Bahnhofsvorplatz",
    "Straßenbahnhaltestelle",
    "Fahrradreparaturwerkstatt",
    "Bürgermeisterwahl",
    "Geburtstagsfeier",
    "Wasserflaschenhalter",
    "Kaffeemaschinenreinigung",
    "Softwareentwicklungsabteilung",
    "Hochschulabschluss",
    "Universitätsbibliothek",
    "Steuererklärungsformular",
    "Zahnarztpraxis",
    "Handyladekabel",
    "Computerbildschirm",
    "Fensterputzmittel",
    "Feuerwehrauto",
    "Schokoladenkuchen",
    "Regenschirmständer",
    "Kinderwagenrad",
    "Tischtennisplatte",
    "Eisenbahnbrücke",
];

fn german() -> Dictionary {
    open_dictionary(&dictionaries(), "de-DE", "Deutsch").expect("de-DE loads")
}

/// Every single-letter deletion, doubling and transposition inside a word.
fn single_edits(word: &str) -> Vec<String> {
    let letters: Vec<char> = word.chars().collect();
    let mut edits = Vec::new();
    for index in 1..letters.len().saturating_sub(1) {
        let mut deleted = letters.clone();
        deleted.remove(index);
        let mut doubled = letters.clone();
        doubled.insert(index, letters[index]);
        let mut swapped = letters.clone();
        swapped.swap(index, index + 1);
        edits.extend([deleted, doubled, swapped].map(|edit| edit.into_iter().collect()));
    }
    edits
}

#[test]
fn accepts_german_compounds_and_rejects_their_typos() {
    let german = german();
    for word in GERMAN_COMPOUNDS {
        assert!(german.spell(word), "{word}");
    }
    let mut typos: Vec<String> = GERMAN_COMPOUNDS
        .iter()
        .flat_map(|word| single_edits(word))
        .filter(|typo| !GERMAN_COMPOUNDS.contains(&typo.as_str()))
        .collect();
    typos.sort();
    typos.dedup();
    assert_eq!(typos.len(), 1_866);
    let accepted: Vec<&str> = typos
        .iter()
        .filter(|typo| german.spell(typo))
        .map(String::as_str)
        .collect();
    // Upstream 1.7.5 accepts exactly these too: each splits into real words.
    assert_eq!(
        accepted,
        [
            "Autobahnaststätte",
            "Autobahnratsstätte",
            "Bahnhofsvorlatz",
            "Bahnhofvorplatz",
            "Bundeserfassungsgericht",
            "Bürgermeisterwal",
            "Datenerarbeitung",
            "Fahrradreparaturwerkstat",
            "Geburtstagseier",
            "Handladekabel",
            "Haustürschüssel",
            "Hochschulabschuss",
            "Kaffeemaschineneinigung",
            "Krankennhaus",
            "Krankennhäuser",
            "Rechtsschreibprüfung",
            "Tischtennislatte",
            "Wasserflaschenalter",
            "Weihnachtbaumschmuck",
            "Weihnachtsbauschmuck",
            "Zahnartpraxis",
        ]
    );
}

#[test]
fn checks_long_german_compounds_without_running_the_suggester() {
    let german = german();
    let long: Vec<&str> = GERMAN_COMPOUNDS
        .iter()
        .copied()
        .filter(|word| word.chars().count() >= 20)
        .collect();
    assert!(long.len() >= 10);
    // Upstream 1.7.5 spends 5-25 ms on each (hunspell-sys/PATCHES.md).
    let fastest = (0..3)
        .map(|_| {
            let started = std::time::Instant::now();
            for word in &long {
                assert!(german.spell(word), "{word}");
            }
            started.elapsed()
        })
        .min()
        .unwrap_or_default();
    assert!(
        fastest < std::time::Duration::from_millis(20),
        "{} long compounds took {fastest:?}",
        long.len()
    );
}

#[test]
fn distinguishes_british_and_american_spelling_and_unions_both_selections() {
    let text = "color colour";
    assert_eq!(words(&check(&engine(&["en-US"], &[]), text)), ["colour"]);
    assert_eq!(words(&check(&engine(&["en-GB"], &[]), text)), ["color"]);
    assert_eq!(check(&engine(&["en-US", "en-GB"], &[]), text), []);
}

#[test]
fn checks_nfc_but_returns_original_positions_and_spelling() {
    let checker = engine(&["de-DE"], &[]);
    let text = "😀 Ha\u{308}user zzzqxx";
    assert_eq!(
        check(&checker, text),
        [Issue {
            start: 11,
            end: 17,
            word: "zzzqxx".to_owned()
        }]
    );
}

#[test]
fn provides_real_corrections_only_on_request() {
    let checker = engine(&["en-US", "de-DE"], &[]);
    assert!(suggest(&checker, "Huas").contains(&"Haus".to_owned()));
    assert!(suggest(&checker, "Rechtschreibprüfunng").contains(&"Rechtschreibprüfung".to_owned()));
    assert_eq!(suggest(&checker, "hello"), Vec::<String>::new());
    assert!(suggest(&checker, "Huas").len() <= 8);
    assert_eq!(suggest(&checker, ""), Vec::<String>::new());
    assert_eq!(suggest(&checker, &"z".repeat(129)), Vec::<String>::new());
}

#[test]
fn accepts_personal_words_after_a_cached_miss_and_isolates_sessions() {
    let checker = engine(&["en-US"], &["Anytwo"]);
    assert_eq!(check(&checker, "Anytwo anytwo ANYTWO zzzqxx").len(), 1);
    checker.add_word("zzzqxx").expect("added");
    assert_eq!(check(&checker, "Anytwo zzzqxx"), []);
    assert_eq!(suggest(&checker, "zzzqxx"), Vec::<String>::new());
    checker.add_word("Zzzz’qxx").expect("added");
    assert_eq!(check(&checker, "Zzzz’qxx ZZZZ'QXX"), []);
    let fresh = engine(&["en-US"], &[]);
    assert_eq!(check(&fresh, "zzzqxx").len(), 1);
}

#[test]
fn replaces_personal_words_without_reloading_dictionaries() {
    let checker = engine(&["en-US"], &["zzzqxx"]);
    assert_eq!(check(&checker, "zzzqxx qqqzzx").len(), 1);
    checker
        .set_personal_words(&["qqqzzx".to_owned()])
        .expect("replaced");
    assert_eq!(words(&check(&checker, "zzzqxx qqqzzx")), ["zzzqxx"]);
}

#[test]
fn validates_personal_words() {
    let checker = engine(&["en-US"], &[]);
    let length = Err(SpellingError::Message(
        "Personal words must contain between 1 and 128 characters.".to_owned(),
    ));
    assert_eq!(checker.add_word(" \u{FEFF} "), length);
    assert_eq!(checker.add_word(&"z".repeat(129)), length);
    checker
        .add_word(&format!("\u{FEFF}{}\u{3000}", "z".repeat(128)))
        .expect("trimmed");
    let many: Vec<String> = (0..10_001).map(|index| format!("w{index}")).collect();
    assert_eq!(
        checker.set_personal_words(&many),
        Err(SpellingError::Message(
            "The personal dictionary can contain at most 10,000 words.".to_owned()
        ))
    );
}

#[test]
fn cancels_long_checks_and_closes_disposed_sessions() {
    let checker = engine(&["en-US"], &[]);
    let cancelled = AtomicBool::new(true);
    assert_eq!(
        checker.check(&"hello ".repeat(1_000), &cancelled),
        Err(SpellingError::Cancelled)
    );
    checker.dispose();
    checker.dispose();
    let closed = SpellingError::Message("The spelling checker was closed.".to_owned());
    assert_eq!(
        checker.check("hello", &AtomicBool::new(false)),
        Err(closed.clone())
    );
    assert_eq!(checker.suggest("helo"), Err(closed));
}

#[test]
fn cancels_a_registered_request_and_unregisters_it_when_settled() {
    let registry = Registry::new(dictionaries());
    let session = registry
        .prepare("window", &["en-US".to_owned()], &[])
        .expect("prepared");
    let request = registry.begin(session, 7);
    registry.cancel(session, 7);
    let checker = registry.engine(session).expect("open");
    assert_eq!(
        checker.check("hello", &request.cancelled),
        Err(SpellingError::Cancelled)
    );
    drop(request);
    let next = registry.begin(session, 7);
    assert_eq!(checker.check("hello", &next.cancelled), Ok(Vec::new()));
}

#[test]
fn shares_dictionaries_between_sessions_and_releases_closed_or_reloaded_windows() {
    let registry = Registry::new(dictionaries());
    let languages = ["en-US".to_owned()];
    let first = registry.prepare("a", &languages, &[]).expect("prepared");
    let second = registry.prepare("b", &languages, &[]).expect("prepared");
    registry.forget_window("a");
    assert!(registry.engine(first).is_err());
    assert!(registry.engine(second).is_ok());
    let sessions: Vec<u64> = (0..6)
        .map(|_| registry.prepare("b", &languages, &[]).expect("prepared"))
        .collect();
    assert!(
        registry.engine(second).is_ok(),
        "a window's live sessions are never evicted"
    );
    registry.dispose(sessions[5]);
    assert!(registry.engine(sessions[5]).is_err());
    assert!(registry.engine(sessions[4]).is_ok());
    registry.forget_window("b");
    assert!(registry.engine(second).is_err());
    assert!(registry.engine(sessions[4]).is_err());
}

#[test]
fn accepts_saved_hyphenated_words_without_accepting_their_fragments_elsewhere() {
    let checker = engine(&["en-US"], &["Zzqxx-Üüqxx"]);
    let text = "zzqxx-U\u{308}u\u{308}qxx Zzqxx zzzqxx-Üüqxx";
    assert_eq!(words(&check(&checker, text)), ["Zzqxx", "zzzqxx", "Üüqxx"]);
}

#[test]
fn reports_a_missing_or_invalid_dictionary_instead_of_pretending_spelling_is_correct() {
    let missing = Registry::new(PathBuf::from("/nonexistent"));
    assert_eq!(
        missing.prepare("w", &["de-DE".to_owned()], &[]).err(),
        Some(SpellingError::Message(
            "The Deutsch dictionary could not load.".to_owned()
        ))
    );
    assert_eq!(
        missing.prepare("w", &["xx-XX".to_owned()], &[]).err(),
        Some(SpellingError::Message(
            "This spelling language is unavailable.".to_owned()
        ))
    );
    assert_eq!(
        missing.prepare("w", &[], &[]).err(),
        Some(SpellingError::Message(
            "Choose a language to check spelling.".to_owned()
        ))
    );
    let folder = std::env::temp_dir().join(format!("any-spelling-invalid-{}", std::process::id()));
    std::fs::create_dir_all(folder.join("en-US")).expect("temp dir");
    let html = "<!doctype html><html><body>App fallback</body></html>";
    std::fs::write(folder.join("en-US/index.aff"), html).expect("write");
    std::fs::write(folder.join("en-US/index.dic"), html).expect("write");
    assert_eq!(
        open_dictionary(&folder, "en-US", "English (US)").err(),
        Some("The English (US) dictionary is invalid. Try again.".to_owned())
    );
    std::fs::write(folder.join("en-US/index.aff"), "SET UTF-8\n").expect("write");
    assert!(open_dictionary(&folder, "en-US", "English (US)").is_err());
    std::fs::write(folder.join("en-US/index.dic"), "1\nhello\n").expect("write");
    assert!(open_dictionary(&folder, "en-US", "English (US)").is_ok());
    std::fs::remove_dir_all(&folder).expect("cleanup");
}

#[test]
fn excludes_code_addresses_paths_and_identifiers() {
    let text = "see `codee` https://exampel.com/pathh www.siet.org mail jhon@exampel.com /usr/lcoal C:\\Usres\\x foo.barr helloWorld abc123 snake_case ```\nfenced codde\n``` endd";
    let found: Vec<&str> = spelling_words(text).iter().map(|word| word.word).collect();
    assert_eq!(found, ["see", "mail", "endd"]);
}

#[test]
fn matches_intl_segmenter_offsets() {
    let text = "😀 Ha\u{308}user zzzqxx don’t e.g. U.S.A. well-known";
    let mut offsets = super::tokenize::Utf16Offsets::new(text);
    let found: Vec<(usize, usize, &str)> = spelling_words(text)
        .iter()
        .map(|word| (offsets.at(word.start), offsets.at(word.end), word.word))
        .collect();
    assert_eq!(
        found,
        [
            (3, 10, "Ha\u{308}user"),
            (11, 17, "zzzqxx"),
            (18, 23, "don’t"),
            (36, 40, "well"),
            (41, 46, "known"),
        ]
    );
}

#[test]
fn handles_a_pasted_long_identifier_without_quadratic_address_matching() {
    let started = std::time::Instant::now();
    assert_eq!(spelling_words(&"a".repeat(200_000)), []);
    assert_eq!(spelling_words(&"a.".repeat(100_000)), []);
    assert!(started.elapsed() < std::time::Duration::from_secs(5));
}
