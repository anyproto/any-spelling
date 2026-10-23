//! `manifest.json` pins every dictionary file and source archive this
//! repository ships; `scripts/verify.js --upstream` checks them against their
//! upstream releases.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use serde_json::Value;
use sha2::{Digest, Sha256};

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("..")
}

fn manifest() -> Value {
    let text = std::fs::read_to_string(root().join("manifest.json")).expect("manifest.json");
    serde_json::from_str(&text).expect("valid JSON")
}

fn entries(value: &Value) -> &Vec<Value> {
    value.as_array().expect("an array")
}

fn text<'a>(value: &'a Value, key: &str) -> &'a str {
    value[key]
        .as_str()
        .unwrap_or_else(|| panic!("{key} is a string"))
}

fn files_under(dir: &str) -> Vec<String> {
    fn walk(dir: &Path, root: &Path, out: &mut Vec<String>) {
        for entry in std::fs::read_dir(dir).expect("readable") {
            let path = entry.expect("entry").path();
            if path.is_dir() {
                walk(&path, root, out);
            } else {
                let relative = path.strip_prefix(root).expect("inside the root");
                out.push(relative.to_string_lossy().replace('\\', "/"));
            }
        }
    }
    let mut out = Vec::new();
    walk(&root().join(dir), &root(), &mut out);
    out
}

#[test]
fn every_shipped_file_matches_its_pinned_size_and_hash() {
    let manifest = manifest();
    let pinned: Vec<&Value> = entries(&manifest["dictionaries"])
        .iter()
        .flat_map(|dictionary| entries(&dictionary["files"]))
        .chain(entries(&manifest["sources"]))
        .collect();
    let mut problems = Vec::new();
    for entry in &pinned {
        let file = text(entry, "file");
        let Ok(bytes) = std::fs::read(root().join(file)) else {
            problems.push(format!("{file}: missing"));
            continue;
        };
        if Some(bytes.len() as u64) != entry["bytes"].as_u64() {
            problems.push(format!("{file}: {} bytes", bytes.len()));
        }
        let hash: String = Sha256::digest(&bytes)
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect();
        if hash != text(entry, "sha256") {
            problems.push(format!("{file}: sha256 {hash}"));
        }
    }
    let listed: BTreeSet<&str> = pinned.iter().map(|entry| text(entry, "file")).collect();
    for file in files_under("dictionaries")
        .into_iter()
        .chain(files_under("sources"))
    {
        if !listed.contains(file.as_str()) {
            problems.push(format!("{file}: not in manifest.json"));
        }
    }
    assert_eq!(problems, Vec::<String>::new());
}

#[test]
fn every_dictionary_has_its_files_and_is_selectable() {
    let manifest = manifest();
    let ids: Vec<&str> = entries(&manifest["dictionaries"])
        .iter()
        .map(|dictionary| text(dictionary, "id"))
        .collect();
    for dictionary in entries(&manifest["dictionaries"]) {
        let id = text(dictionary, "id");
        let files: Vec<&str> = entries(&dictionary["files"])
            .iter()
            .map(|file| text(file, "file"))
            .collect();
        for name in ["index.aff", "index.dic", "license"] {
            let expected = format!("dictionaries/{id}/{name}");
            assert!(files.contains(&expected.as_str()), "{id}: no {name}");
        }
    }
    for source in entries(&manifest["sources"]) {
        for language in entries(&source["languages"]) {
            let language = language.as_str().expect("an id");
            assert!(
                ids.contains(&language),
                "{}: unknown language {language}",
                text(source, "file")
            );
        }
    }
    let engine: Vec<&str> = any_spelling::LANGUAGES
        .iter()
        .map(|language| language.id)
        .collect();
    assert_eq!(engine, ids, "the engine's catalog follows manifest.json");
}
