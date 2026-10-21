# @anyproto/spelling-dictionaries

The Hunspell dictionaries Any uses for on-device spell checking, kept in their
own repository so the app carries a dependency line instead of tens of megabytes
of word lists. It is not published to a registry: the app installs it straight
from git, pinned to a commit.

The package holds data only. The checking engine, the language picker and the
personal dictionary live in the app.

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
`npm run verify:upstream` downloads each npm tarball and proves it byte for byte.

- `dictionary-pt` is the Brazilian VERO dictionary despite its generic name, so
  it is exposed as `pt-BR`; Portugal has its own pack.
- The French pack is Grammalecte's recommended **Classique** variant.
- The Spanish pack is the **Spain** dictionary.
- The Polish npm release declares GPL 3.0 / LGPL 3.0 / MPL 2.0 alternatives while
  its original 2008 notice lists license families without versions. Any uses the
  GPL 3.0 option and keeps the original notice unchanged.

## Usage

Add the dependency pinned to a full commit SHA; the lockfile records it, so every
install gets the same bytes:

```json
"@anyproto/spelling-dictionaries": "github:anyproto/spelling-dictionaries#<commit-sha>"
```

```js
import { dictionaries } from '@anyproto/spelling-dictionaries'
import aff from '@anyproto/spelling-dictionaries/dictionaries/en-US/index.aff?url'
import dic from '@anyproto/spelling-dictionaries/dictionaries/en-US/index.dic?url'
```

`dictionaries` lists each id with the npm release and license it came from. The
`?url` imports are Vite's: the files are emitted as hashed assets and fetched
only when a language is selected.

## Layout

| Path                     | Installed | What it is                                                   |
| ------------------------ | --------- | ------------------------------------------------------------ |
| `dictionaries/<id>/`     | yes       | `index.aff`, `index.dic` and the original `license` notice   |
| `licenses/`              | yes       | Full GPL, LGPL and MPL texts the notices refer to            |
| `manifest.json`          | yes       | Upstream release, size and SHA-256 of every file and archive |
| `index.js`, `index.d.ts` | yes       | The dictionary list and its `DictionaryId` type              |
| `sources/`               | no        | Preferred-form sources for the GPL / LGPL / MPL dictionaries |
| `scripts/verify.js`      | no        | The hash and upstream check                                  |

`sources/` and `scripts/` are `export-ignore` in `.gitattributes`, so the commit
archive a package manager downloads leaves them out; `files` in `package.json`
keeps them out of an install from a full clone. Both stay in git history.

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

## Licensing

The code in this repository (`index.js`, `index.d.ts`, `scripts/`) is MIT; see
[LICENSE](LICENSE).

Each dictionary keeps its own license, recorded in its `license` file and in the
table above. The dictionaries are separate data files and are not relicensed by
this package or by the app that loads them. That is why `package.json` points at
this section instead of declaring `MIT`: a license scanner would otherwise read
the GPL dictionaries as MIT.

## Updating a dictionary

1. Replace the files under `dictionaries/<id>/` with the new npm release.
2. Update that entry in `manifest.json` (version, tarball, sizes, hashes), and
   its source archive if the upstream source changed.
3. Run `npm run verify` and `npm run verify:upstream`, bump `version` in
   `package.json`, and commit.
4. Point the app's dependency at the new commit SHA.
