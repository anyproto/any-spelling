# Changes to the vendored Hunspell

`hunspell/` is Hunspell 1.7.5 from the release tarball
(`https://github.com/hunspell/hunspell/releases/download/v1.7.5/hunspell-1.7.5.tar.gz`, SHA-256
`2e559f0c2a592ba48e421477f58e9bf7e01062277a4a7821afb52fc9401010ae`), used
under its MPL 1.1 option, with the one change below. Each change is marked in
the file it touches and kept as a patch in `patches/`.

## Compound typo test compiled out — 2026-10-22

File: `hunspell/hunspell.cxx`, `HunspellImpl::checkword`. Patch:
[`patches/compound-typo-test.patch`](patches/compound-typo-test.patch).

Hunspell 1.7.2 added a check (LibreOffice tdf#136306): a word accepted as a
compound of three or more parts is rejected again if the suggester finds a
"simpler" spelling of it. The suggester runs for every such compound, so each
German one costs 5–25 ms instead of 0.1–0.5 ms, and a cold check of a German
paragraph took 169 ms instead of 5 ms. The crate's `build.rs` defines
`ANY_HUNSPELL_SKIP_COMPOUND_TYPO_TEST`, which wraps that block in `#ifndef`;
nothing else changes, and suggestions are unaffected.

On 40 German compounds and their 1,866 single-letter deletions, doublings and
transpositions, and 15 Dutch compounds and their 711, the patched and upstream
engines accept exactly the same words. `any-spelling`'s tests pin both the result
and the speed.

To update Hunspell, replace `hunspell/` with the new release's `src/hunspell/`
and its license files, then re-apply the patch (`patch -p3 < patches/…` from
`hunspell/`), or drop it if upstream has made the check cheap.
