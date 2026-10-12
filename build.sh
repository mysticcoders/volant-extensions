#!/bin/bash
# Tests and builds every extension and pins each module's SHA-256 in its manifest. Rust extensions
# are members of the Cargo workspace, built for ABI 1 (wasm32-unknown-unknown, 16 MiB memory
# maximum). Any other folder with a manifest builds itself with its own build.sh, which takes the same
# --check flag. `./build.sh --check` (run by CI) tests and builds without touching the committed
# modules, then fails if a manifest's sha256 does not match its module or a module breaks Volant's
# import, memory or size rules.
# The pinned Rust toolchain gives identical bytes on the same machine; Linux and macOS builds can
# differ in function order, so CI does not compare bytes across hosts.
#
# Usage: ./build.sh [--check]
set -euo pipefail
cd "$(dirname "$0")"
MODE=${1:-}
TOOLCHAIN=$(sed -n 's/^channel = "\(.*\)"/\1/p' rust-toolchain.toml)
RUST_TARGET=target/wasm32-unknown-unknown/release
# rustup's own cargo honors the pinned toolchain; a Homebrew or mise cargo earlier on PATH would not.
cargo() { rustup run "$TOOLCHAIN" cargo "$@"; }
cargo test --quiet --workspace
# Remapping keeps machine paths (and usernames) out of panic messages, so modules are identical on any host.
CARGO_DIR=${CARGO_HOME:-$HOME/.cargo}
RUSTFLAGS="-C link-arg=--max-memory=16777216 --remap-path-prefix=$CARGO_DIR=/cargo --remap-path-prefix=$PWD=/src" cargo build --quiet --release --target wasm32-unknown-unknown --workspace
for manifest in */manifest.json; do
  folder=$(dirname "$manifest")
  if [ -f "$folder/Cargo.toml" ]; then
    if [ "$MODE" != --check ]; then
      cp "$RUST_TARGET/volant_$folder.wasm" "$folder/$(python3 tools/check-module.py --module-name "$manifest")"
      python3 tools/check-module.py --pin "$manifest"
    fi
  elif [ -x "$folder/build.sh" ]; then
    "$folder/build.sh" $MODE
  else
    echo "$folder has a manifest but neither a Cargo.toml nor a build.sh" >&2
    exit 1
  fi
  python3 tools/check-module.py "$manifest"
done
