#!/bin/bash
# Tests and builds the Wikipedia extension: src/wikipedia.py on MicroPython, compiled with wasi-sdk to a
# Volant ABI 2 WASI command (wikipedia.wasm), then pins the module's SHA-256 in manifest.json.
#
# The compiler and MicroPython are downloaded once into .toolchain/ and checked against the pinned
# SHA-256 below before use. `./build.sh --check` runs the Python tests under CPython and confirms the
# committed module matches its manifest hash, without the WASI toolchain.
#
# Usage: ./build.sh [--check]
set -euo pipefail
cd "$(dirname "$0")"

WASI_SDK_TAG=wasi-sdk-34
WASI_SDK_VERSION=34.0
MICROPYTHON_VERSION=1.29.0
MICROPYTHON_SHA256=d925a7c664e79a2bdf3dfcb285ba5e2237041cc35a0bd4ee573b6c5711efeca0
SOURCE_DATE_EPOCH=1767225600
MEMORY_MAXIMUM=16777216
STACK_SIZE=1048576
MODULE=wikipedia.wasm
MANIFEST=manifest.json
TOOLCHAIN=${WIKIPEDIA_TOOLCHAIN:-$PWD/.toolchain}
BUILD=$PWD/.build

python3 tests/test_wikipedia.py

pin_hash() {
  python3 - "$MANIFEST" "$MODULE" "$1" <<'PY'
import hashlib, json, pathlib, sys
manifest, module, mode = pathlib.Path(sys.argv[1]), pathlib.Path(sys.argv[2]), sys.argv[3]
data = json.loads(manifest.read_text())
digest = hashlib.sha256(module.read_bytes()).hexdigest()
if mode == "--check":
    sys.exit(0 if data["sha256"] == digest else f"{manifest} has a stale sha256; run ./build.sh and commit it")
data["sha256"] = digest
manifest.write_text(json.dumps(data, indent=2) + "\n")
PY
}

if [ "${1:-}" = --check ]; then
  pin_hash --check
  exit 0
fi

case "$(uname -s):$(uname -m)" in
  Darwin:arm64) platform=arm64-macos; wasi_sha256=9c59398106b417f8f14913380fdf0097a8cc0ff4af9eb3ce0065a859e88d49e9 ;;
  Darwin:x86_64) platform=x86_64-macos; wasi_sha256=87d27fa8adc68dee59bfbf2e22a6d34ef717c34d6bf1d8af2a56fc929d9ce0eb ;;
  Linux:x86_64) platform=x86_64-linux; wasi_sha256=b761e3a0721dbae9c09a0059e5fdb2bf917d1b4a8a7b430fb3b5aafb0984b2c4 ;;
  Linux:aarch64) platform=arm64-linux; wasi_sha256=f7e243dff54d60bcc576e94d6166b69f410f2500ae4a9ceef34315be10e77971 ;;
  *) echo "No pinned wasi-sdk for $(uname -s) $(uname -m)." >&2; exit 1 ;;
esac

# Downloads a pinned archive into the toolchain folder, checks its SHA-256 and unpacks it once.
fetch() {
  local url=$1 sha256=$2 archive=$3 folder=$4
  [ -d "$TOOLCHAIN/$folder" ] && return
  mkdir -p "$TOOLCHAIN"
  curl -fsSL --retry 3 --connect-timeout 15 --max-time 300 "$url" -o "$TOOLCHAIN/$archive"
  printf '%s  %s\n' "$sha256" "$TOOLCHAIN/$archive" | shasum -a 256 -c - >/dev/null
  tar -xf "$TOOLCHAIN/$archive" -C "$TOOLCHAIN"
  rm "$TOOLCHAIN/$archive"
}

fetch "https://github.com/WebAssembly/wasi-sdk/releases/download/$WASI_SDK_TAG/wasi-sdk-$WASI_SDK_VERSION-$platform.tar.gz" \
  "$wasi_sha256" wasi-sdk.tar.gz "wasi-sdk-$WASI_SDK_VERSION-$platform"
fetch "https://github.com/micropython/micropython/releases/download/v$MICROPYTHON_VERSION/micropython-$MICROPYTHON_VERSION.tar.xz" \
  "$MICROPYTHON_SHA256" micropython.tar.xz "micropython-$MICROPYTHON_VERSION"

WASI_SDK=$TOOLCHAIN/wasi-sdk-$WASI_SDK_VERSION-$platform
MICROPYTHON=$TOOLCHAIN/micropython-$MICROPYTHON_VERSION
EH_FLAGS="-mllvm -wasm-enable-sjlj -mllvm -wasm-use-legacy-eh=true"
CC="$WASI_SDK/bin/clang --target=wasm32-wasip1 --sysroot=$WASI_SDK/share/wasi-sysroot $EH_FLAGS"
PREFIX_MAPS="-ffile-prefix-map=$TOOLCHAIN=/toolchain -ffile-prefix-map=$BUILD=/build -ffile-prefix-map=$PWD=/src"

rm -rf "$BUILD"
mkdir -p "$BUILD"
cp runtime/mpconfigport.h "$BUILD/"
export SOURCE_DATE_EPOCH
make -s -C "$BUILD" -f "$PWD/runtime/embed.mk" MICROPYTHON_TOP="$MICROPYTHON" PACKAGE_DIR=micropython_embed \
  BUILD=build-embed CC="$CC" >/dev/null

python3 - src/wikipedia.py "$BUILD/wikipedia_source.h" <<'PY'
import pathlib, sys
source = pathlib.Path(sys.argv[1]).read_bytes()
data = ",".join(str(byte) for byte in source + b"\0")
pathlib.Path(sys.argv[2]).write_text(f"static const char wikipedia_source[] = {{{data}}};\n")
PY

EMBED=$BUILD/micropython_embed
sources=(runtime/main.c "$EMBED"/py/*.c "$EMBED"/port/mphalport.c "$MICROPYTHON/extmod/modjson.c")
$CC -Os -std=c99 -Wall -Wno-unused-function $PREFIX_MAPS \
  -I"$BUILD" -I"$EMBED" -I"$EMBED/port" -I"$MICROPYTHON" \
  "${sources[@]}" -lsetjmp \
  -Wl,--max-memory=$MEMORY_MAXIMUM -Wl,-z,stack-size=$STACK_SIZE -Wl,--strip-all -Wl,--gc-sections \
  -o "$MODULE"

pin_hash write
echo "$MODULE: $(wc -c < "$MODULE" | tr -d ' ') bytes, sha256 $(python3 -c "import json,sys; print(json.load(open(sys.argv[1]))['sha256'][:12])" "$MANIFEST")…"
