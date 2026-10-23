#![allow(
    clippy::print_stdout,
    reason = "cargo reads build-script directives from stdout"
)]

fn main() {
    let manifest = std::env::var("CARGO_MANIFEST_DIR").expect("set by cargo");
    let dictionaries = std::path::Path::new(&manifest)
        .join("../dictionaries")
        .canonicalize()
        .expect("dictionaries/ sits beside this crate");
    // The app bundles these files; it reads the path as DEP_ANY_SPELLING_DICTIONARIES.
    println!("cargo::metadata=dictionaries={}", dictionaries.display());
    println!("cargo::rerun-if-changed=../dictionaries");
}
