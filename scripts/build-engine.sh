#!/usr/bin/env bash
# Compiles sources/hunspell-<version>.tar.gz to engine/hunspell.{js,wasm}.
# Run inside scripts/engine.Dockerfile, or with an activated emsdk of exactly
# EMSDK_VERSION; any other compiler is refused so the output stays reproducible.
set -euo pipefail

HUNSPELL_VERSION=1.7.5
HUNSPELL_SHA256=2e559f0c2a592ba48e421477f58e9bf7e01062277a4a7821afb52fc9401010ae
EMSDK_VERSION=6.0.13

root=$(cd "$(dirname "$0")/.." && pwd)
archive="$root/sources/hunspell-$HUNSPELL_VERSION.tar.gz"
out="$root/engine"

actual=$(sha256sum "$archive" 2>/dev/null || shasum -a 256 "$archive")
if [ "${actual%% *}" != "$HUNSPELL_SHA256" ]; then
  echo "error: $archive is not Hunspell $HUNSPELL_VERSION ($HUNSPELL_SHA256)" >&2
  exit 1
fi

compiler=$(em++ --version | head -n 1)
case "$compiler" in
  *" $EMSDK_VERSION "*) ;;
  *)
    echo "error: need em++ $EMSDK_VERSION, found: $compiler" >&2
    exit 1
    ;;
esac

work=$(mktemp -d -t hunspell-wasm.XXXXXX)
trap 'rm -rf "$work"' EXIT
tar -xzf "$archive" -C "$work"

exports=(
  _Hunspell_create _Hunspell_destroy _Hunspell_spell _Hunspell_suggest
  _Hunspell_free_list _Hunspell_add _Hunspell_remove _Hunspell_get_dic_encoding
  _malloc _free
)

# Relative paths and a fixed working directory keep the absolute build path out
# of the output.
cd "$work/hunspell-$HUNSPELL_VERSION/src/hunspell"
em++ -O3 -DHUNSPELL_STATIC \
  affentry.cxx affixmgr.cxx csutil.cxx filemgr.cxx hashmgr.cxx hunspell.cxx \
  hunspelltrace.cxx hunzip.cxx phonet.cxx replist.cxx suggestmgr.cxx \
  -sMODULARIZE=1 -sEXPORT_ES6=1 -sEXPORT_NAME=createHunspellModule \
  -sENVIRONMENT=web,worker,node \
  -sDYNAMIC_EXECUTION=0 \
  -sALLOW_MEMORY_GROWTH=1 -sSTACK_SIZE=1MB \
  -sFORCE_FILESYSTEM=1 \
  -sINCOMING_MODULE_JS_API=locateFile,wasmBinary \
  -sEXPORTED_FUNCTIONS="$(IFS=,; echo "${exports[*]}")" \
  -sEXPORTED_RUNTIME_METHODS=FS,HEAPU8,HEAPU32,UTF8ToString,stringToNewUTF8 \
  -o "$work/hunspell.js"

mkdir -p "$out"
cp "$work/hunspell.js" "$work/hunspell.wasm" "$out/"
chmod 644 "$out/hunspell.js" "$out/hunspell.wasm"
(cd "$out" && { sha256sum hunspell.js hunspell.wasm 2>/dev/null || shasum -a 256 hunspell.js hunspell.wasm; })
