# anyproto/spelling-dictionaries

The Hunspell dictionaries Any uses for on-device spell checking, and the Rust
crates that check spelling with them. The desktop app depends on this
repository through Cargo, pinned to a commit; nothing is published to a
registry.

- `any-spelling` — the checker: sessions over any set of the dictionaries
  below, a personal vocabulary Hunspell never sees, script gating, suggestion
  ranking, and issues reported in UTF-16 offsets for a JavaScript caller.
- `hunspell-sys` — Hunspell 1.7.5 compiled from vendored source, with one
  marked change ([`hunspell-sys/PATCHES.md`](hunspell-sys/PATCHES.md)).

## Dictionaries

| Id      | Language              | Taken from               | Dictionary license                                    |
| ------- | --------------------- | ------------------------ | ----------------------------------------------------- |
| `en-US` | English (US)          | `dictionary-en@4.0.0`    | MIT and BSD                                           |
| `en-GB` | English (UK)          | `dictionary-en-gb@3.0.0` | MIT and BSD                                           |
| `de-DE` | German                | `dictionary-de@3.0.0`    | GPL 2.0 or GPL 3.0                                    |
| `ru-RU` | Russian               | `dictionary-ru@3.0.0`    | BSD 3-Clause                                          |
| `uk-UA` | Ukrainian             | `dictionary-uk@4.0.0`    | GPL 3.0                                               |
| `fr-FR` | French                | `dictionary-fr@3.0.0`    | MPL 2.0                                               |
| `es-ES` | Spanish (Spain)       | `dictionary-es@4.0.0`    | GPL 3.0, LGPL 3.0, or MPL 1.1                         |
| `it-IT` | Italian               | `dictionary-it@2.0.0`    | GPL 3.0                                               |
| `pt-BR` | Portuguese (Brazil)   | `dictionary-pt@4.0.0`    | LGPL 3.0 or MPL 2.0                                   |
| `pt-PT` | Portuguese (Portugal) | `dictionary-pt-pt@2.0.0` | GPL 2.0, LGPL 2.1, or MPL 1.1                         |
| `nl-NL` | Dutch                 | `dictionary-nl@2.0.0`    | BSD 3-Clause or CC BY 3.0                             |
| `pl-PL` | Polish                | `dictionary-pl@2.0.0`    | GPL, LGPL, MPL, or CC ShareAlike; see original notice |

The `index.aff`, `index.dic` and `license` files are the unmodified npm
distribution files from [wooorm/dictionaries](https://github.com/wooorm/dictionaries).
`cargo test --test upstream -- --ignored` downloads each npm tarball and proves
them byte for byte; plain `cargo test` skips it.

- `dictionary-pt` is the Brazilian VERO dictionary despite its generic name, so
  it is exposed as `pt-BR`; Portugal has its own pack.
- The French pack is Grammalecte's recommended **Classique** variant.
- The Spanish pack is the **Spain** dictionary.
- The Polish npm release declares GPL 3.0 / LGPL 3.0 / MPL 2.0 alternatives while
  its original 2008 notice lists license families without versions. Any uses the
  GPL 3.0 option and keeps the original notice unchanged.

## Usage

```toml
any-spelling = { git = "https://github.com/anyproto/spelling-dictionaries", rev = "<commit-sha>" }
```

```rust
let registry = any_spelling::Registry::new(dictionaries_dir);
let session = registry.prepare("window", &["en-US".into(), "de-DE".into()], &[])?;
let issues = registry.engine(session)?.check(text, &cancelled)?;
```

The crate does not embed the dictionaries: a consumer ships them as files.
`any-spelling` declares `links = "any_spelling"`, so a dependent's build script
reads this checkout's `dictionaries/` path from `DEP_ANY_SPELLING_DICTIONARIES`
and copies the files into its bundle. Only selected languages are loaded, each
once, however many sessions use it.

## Layout

| Path                   | What it is                                                    |
| ---------------------- | ------------------------------------------------------------- |
| `dictionaries/<id>/`   | `index.aff`, `index.dic` and the original `license` notice    |
| `any-spelling/`        | The checker crate and its tests against the real dictionaries |
| `hunspell-sys/`        | Hunspell 1.7.5 source, its licenses, `PATCHES.md`, `patches/` |
| `licenses/`            | Full GPL, LGPL and MPL texts the dictionary licenses name     |
| `manifest.json`        | Upstream release, size and SHA-256 of every file and archive  |
| `sources/`             | Preferred-form sources for the dictionaries                   |

`cargo test` checks every file against `manifest.json`. `sources/` is
`export-ignore` in `.gitattributes`, so a commit archive leaves it out; Cargo
clones the repository, so it is present there, and it stays in git history.

## Sources

Several dictionaries are generated from other sources, which their licenses
require us to make available. `sources/` keeps them in this repository: the
commit an app pins carries the exact sources of the dictionaries it ships.
`manifest.json` pins each archive's URL, size and SHA-256 and the dictionaries it
covers. GitHub's "Download ZIP" omits them because of `export-ignore`; clone the
repository to get them.

- German: Björn Jacke's [igerman98 20161207](https://www.j3e.de/ispell/igerman98/).
- Ukrainian: complete `brown-uk/dict_uk` **v6.1.0** word lists, affix sources,
  Gradle files and build tools. The npm version is 4.0.0; its underlying
  dictionary release is 6.1.0.
- Spanish: complete `sbosio/rla-es` **v2.8** sources and build tools.
- Italian: the original **4.2** extension with editable `.aff` / `.dic` files,
  notices and historical sources.
- Polish: the original **2008-12-06** extension with editable ISO-8859-2 sources;
  the npm files are normalized UTF-8 copies of them.
- Portuguese (Portugal): the original **20220621** source distribution and makefile.
- French: the unmodified Classique **7.5** source files and notice extracted from
  the Grammalecte **2.1.2** extension; the manifest pins the extension's hash too.
- Portuguese (Brazil) and Russian: unmodified `.aff` / `.dic` sources and notices
  from LibreOffice commit `5cf23af85b47be6e398add826da1a02175be4238`; the npm
  payload differs only by BOM and whitespace normalization.
- The nine npm releases above share wooorm/dictionaries revision
  `a7c018ca64fcb125cabbd5e4ab120b146b2fa0e8`; its crawl, normalization and
  package-generation scripts are kept as
  `sources/wooorm-dictionary-tools-a7c018c.tar.gz`.

Hunspell's own source is `hunspell-sys/hunspell/`: the `src/hunspell/` of the
1.7.5 release with the change `hunspell-sys/PATCHES.md` records.

## Licensing

The code written for this repository (`any-spelling/`, `scripts/`, the build
script and bindings in `hunspell-sys/`) is MIT; see [LICENSE](LICENSE).

Each dictionary keeps its own license, recorded in its `license` file and in the
table above. The dictionaries are separate data files and are not relicensed by
this repository or by the app that loads them.

### Hunspell

`hunspell-sys/hunspell/` is **Hunspell 1.7.5**, offered under MPL 1.1, GPL 2.0 or
LGPL 2.1; Any uses the **MPL 1.1** option. Its license texts sit beside it
(`COPYING`, `COPYING.LESSER`, `COPYING.MPL`, `license.hunspell`,
`license.myspell`), and every change to it is marked in the file and recorded
in `hunspell-sys/PATCHES.md`.

## Updating a dictionary

1. Replace the files under `dictionaries/<id>/` with the new npm release.
2. Update that entry in `manifest.json` (version, tarball, sizes, hashes), and
   its source archive if the upstream source changed.
3. Run `cargo test` and `cargo test --test upstream -- --ignored`, and commit.
4. Point the app's `rev` at the new commit SHA.

## Updating Hunspell

Replace `hunspell-sys/hunspell/` with the new release's `src/hunspell/` and license files, re-apply
the patches `hunspell-sys/PATCHES.md` lists (or drop them), bump the version in
`hunspell-sys/Cargo.toml`, and run `cargo test`.
