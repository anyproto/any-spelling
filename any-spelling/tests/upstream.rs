//! Proves every dictionary file byte for byte against the npm release
//! `manifest.json` names. It downloads twelve tarballs, so it runs only on
//! request: `cargo test --test upstream -- --ignored`. Needs `curl` and `tar`.

use std::path::{Path, PathBuf};
use std::process::Command;

use serde_json::Value;
use sha2::{Digest, Sha256};

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("..")
}

fn run(command: &mut Command) -> Result<(), String> {
    let output = command.output().map_err(|error| error.to_string())?;
    if output.status.success() {
        Ok(())
    } else {
        Err(String::from_utf8_lossy(&output.stderr).trim().to_owned())
    }
}

#[test]
#[ignore = "downloads the npm releases"]
fn every_dictionary_file_matches_its_npm_release() {
    let manifest: Value = serde_json::from_str(
        &std::fs::read_to_string(root().join("manifest.json")).expect("manifest.json"),
    )
    .expect("valid JSON");
    let scratch = std::env::temp_dir().join(format!("spelling-upstream-{}", std::process::id()));
    let mut problems = Vec::new();
    for dictionary in manifest["dictionaries"].as_array().expect("dictionaries") {
        let id = dictionary["id"].as_str().expect("id");
        let tarball = dictionary["upstream"]["tarball"].as_str().expect("tarball");
        let folder = scratch.join(id);
        std::fs::create_dir_all(&folder).expect("scratch folder");
        let archive = folder.join("package.tgz");
        let fetched = run(Command::new("curl")
            .args(["-fsSL", "-o"])
            .arg(&archive)
            .arg(tarball))
        .and_then(|()| {
            run(Command::new("tar")
                .arg("-xzf")
                .arg(&archive)
                .arg("-C")
                .arg(&folder))
        });
        if let Err(error) = fetched {
            problems.push(format!("{id}: {tarball}: {error}"));
            continue;
        }
        for entry in dictionary["files"].as_array().expect("files") {
            let file = entry["file"].as_str().expect("file");
            let name = file.rsplit('/').next().expect("a file name");
            let Ok(bytes) = std::fs::read(folder.join("package").join(name)) else {
                problems.push(format!("{id}: {name} is absent from the npm tarball"));
                continue;
            };
            let hash: String = Sha256::digest(&bytes)
                .iter()
                .map(|byte| format!("{byte:02x}"))
                .collect();
            if Some(hash.as_str()) != entry["sha256"].as_str() {
                problems.push(format!("{id}: {name} differs from the npm tarball"));
            }
        }
    }
    let _ = std::fs::remove_dir_all(&scratch);
    assert_eq!(problems, Vec::<String>::new());
}
