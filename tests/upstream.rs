//! Proves every dictionary file byte for byte against the upstream release
//! `manifest.json` names: an npm `tarball`, a zip `archive`, or files under a
//! `base` URL, with `names` mapping a file to its upstream name. It downloads
//! every release, so it runs only on request:
//! `cargo test --test upstream -- --ignored`. Needs `curl`, `tar` and `unzip`.

use std::path::PathBuf;
use std::process::Command;

use serde_json::Value;
use sha2::{Digest, Sha256};

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn run(command: &mut Command) -> Result<(), String> {
    let output = command.output().map_err(|error| error.to_string())?;
    if output.status.success() {
        Ok(())
    } else {
        Err(String::from_utf8_lossy(&output.stderr).trim().to_owned())
    }
}

fn fetch(url: &str, to: &std::path::Path) -> Result<(), String> {
    run(Command::new("curl").args(["-fsSL", "-o"]).arg(to).arg(url))
}

/// Downloads a dictionary's release into `folder`; returns where its files sit.
fn download(upstream: &Value, folder: &std::path::Path) -> Result<PathBuf, String> {
    if let Some(tarball) = upstream["tarball"].as_str() {
        let archive = folder.join("package.tgz");
        fetch(tarball, &archive)?;
        run(Command::new("tar")
            .arg("-xzf")
            .arg(&archive)
            .arg("-C")
            .arg(folder))?;
        return Ok(folder.join("package"));
    }
    if let Some(url) = upstream["archive"].as_str() {
        let archive = folder.join("release.zip");
        fetch(url, &archive)?;
        run(Command::new("unzip")
            .arg("-q")
            .arg(&archive)
            .arg("-d")
            .arg(folder))?;
        return Ok(folder.to_owned());
    }
    let base = upstream["base"]
        .as_str()
        .ok_or("no tarball, archive or base")?;
    let names = upstream["names"].as_object().ok_or("base without names")?;
    for name in names.values() {
        let name = name.as_str().ok_or("a name")?;
        fetch(&format!("{base}{name}"), &folder.join(name))?;
    }
    Ok(folder.to_owned())
}

#[test]
#[ignore = "downloads the upstream releases"]
fn every_dictionary_file_matches_its_upstream_release() {
    let manifest: Value = serde_json::from_str(
        &std::fs::read_to_string(root().join("manifest.json")).expect("manifest.json"),
    )
    .expect("valid JSON");
    let scratch = std::env::temp_dir().join(format!("spelling-upstream-{}", std::process::id()));
    let mut problems = Vec::new();
    for dictionary in manifest["dictionaries"].as_array().expect("dictionaries") {
        let id = dictionary["id"].as_str().expect("id");
        let upstream = &dictionary["upstream"];
        let folder = scratch.join(id);
        std::fs::create_dir_all(&folder).expect("scratch folder");
        let unpacked = match download(upstream, &folder) {
            Ok(unpacked) => unpacked,
            Err(error) => {
                problems.push(format!("{id}: {error}"));
                continue;
            }
        };
        for entry in dictionary["files"].as_array().expect("files") {
            let file = entry["file"].as_str().expect("file");
            let name = file.rsplit('/').next().expect("a file name");
            let upstream_name = upstream["names"][name].as_str().unwrap_or(name);
            let Ok(bytes) = std::fs::read(unpacked.join(upstream_name)) else {
                problems.push(format!("{id}: {upstream_name} is absent upstream"));
                continue;
            };
            let hash: String = Sha256::digest(&bytes)
                .iter()
                .map(|byte| format!("{byte:02x}"))
                .collect();
            if Some(hash.as_str()) != entry["sha256"].as_str() {
                problems.push(format!(
                    "{id}: {name} differs from {upstream_name} upstream"
                ));
            }
        }
    }
    let _ = std::fs::remove_dir_all(&scratch);
    assert_eq!(problems, Vec::<String>::new());
}
