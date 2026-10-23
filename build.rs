#![allow(
    clippy::print_stdout,
    reason = "cargo reads build-script directives from stdout"
)]

const HUNSPELL_SOURCES: &[&str] = &[
    "affentry.cxx",
    "affixmgr.cxx",
    "csutil.cxx",
    "filemgr.cxx",
    "hashmgr.cxx",
    "hunspell.cxx",
    "hunspelltrace.cxx",
    "hunzip.cxx",
    "phonet.cxx",
    "replist.cxx",
    "suggestmgr.cxx",
];

fn main() {
    compile_hunspell();

    let manifest = std::env::var("CARGO_MANIFEST_DIR").expect("set by cargo");
    let dictionaries = std::path::Path::new(&manifest)
        .join("dictionaries")
        .canonicalize()
        .expect("dictionaries/ sits beside this crate");
    // The app bundles these files; it reads the path as DEP_ANY_SPELLING_DICTIONARIES.
    println!("cargo::metadata=dictionaries={}", dictionaries.display());
    println!("cargo::rerun-if-changed=dictionaries");
}

fn compile_hunspell() {
    let mut build = cc::Build::new();
    build
        .cpp(true)
        .std("c++14")
        .define("HUNSPELL_STATIC", None)
        .define("BUILDING_LIBHUNSPELL", None)
        // vendor/PATCHES.md: the one change to the vendored source.
        .define("ANY_HUNSPELL_SKIP_COMPOUND_TYPO_TEST", None)
        .warnings(false);
    // Hunspell is the one hot loop of every check; a debug build of it makes
    // the test suite and the app's debug build measure the wrong engine.
    build.opt_level(2);
    if build.get_compiler().is_like_msvc() {
        build.flag("/EHsc").flag("/utf-8");
    }
    for source in HUNSPELL_SOURCES {
        build.file(format!("vendor/hunspell/{source}"));
    }
    println!("cargo::rerun-if-changed=vendor/hunspell");
    build.compile("hunspell");
}
