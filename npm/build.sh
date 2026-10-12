#!/bin/bash
# Tests the npm extension and, without --check, builds npm.wasm with a pinned, checksum-verified
# Javy 9.1.0 and pins its SHA-256 in manifest.json. Javy does not produce identical bytes on every
# build, so `--check` (CI) only runs the tests; the top-level build.sh compares the committed module
# with its manifest hash. Set JAVY to use your own Javy 9.1.0 executable.
#
# Usage: npm/build.sh [--check]
set -euo pipefail
cd "$(dirname "$0")"
JAVY_VERSION=9.1.0
npm ci --ignore-scripts --no-audit --no-fund --silent
npm test --silent
[ "${1:-}" = --check ] && exit 0
if [[ -z "${JAVY:-}" ]]; then
  case "$(uname -s):$(uname -m)" in
    Darwin:arm64) platform=arm-macos; checksum=99e9ec6a8e8c98e119d137c08a921d2443d3b873c675a5571e1800f4451e6294 ;;
    Darwin:x86_64) platform=x86_64-macos; checksum=6eed2927575dc2b3fb5a1563eee1ce0874e6da91c9cec39a728c9377a8cc7b5a ;;
    *) echo "Set JAVY to a Javy $JAVY_VERSION executable on this platform." >&2; exit 1 ;;
  esac
  cache="${XDG_CACHE_HOME:-$HOME/.cache}/volant-extensions/javy-$JAVY_VERSION"
  if [[ ! -x "$cache/javy" ]]; then
    mkdir -p "$cache"
    curl -fsSL --retry 3 --connect-timeout 15 --max-time 120 "https://github.com/bytecodealliance/javy/releases/download/v$JAVY_VERSION/javy-$platform-v$JAVY_VERSION.gz" -o "$cache/javy.gz"
    printf '%s  %s\n' "$checksum" "$cache/javy.gz" | shasum -a 256 -c - >/dev/null
    gzip -dc "$cache/javy.gz" > "$cache/javy"
    chmod +x "$cache/javy"
  fi
  export JAVY="$cache/javy"
fi
node build.mjs
