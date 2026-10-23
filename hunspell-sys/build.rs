const SOURCES: &[&str] = &[
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
    let mut build = cc::Build::new();
    build
        .cpp(true)
        .std("c++14")
        .define("HUNSPELL_STATIC", None)
        .define("BUILDING_LIBHUNSPELL", None)
        // PATCHES.md: the one change to the vendored source.
        .define("ANY_HUNSPELL_SKIP_COMPOUND_TYPO_TEST", None)
        .warnings(false);
    // Hunspell is the one hot loop of every check; a debug build of it makes
    // the test suite and `tauri build --debug` measure the wrong engine.
    build.opt_level(2);
    if build.get_compiler().is_like_msvc() {
        build.flag("/EHsc").flag("/utf-8");
    }
    for source in SOURCES {
        build.file(format!("hunspell/{source}"));
    }
    println!("cargo::rerun-if-changed=hunspell");
    build.compile("hunspell");
}
