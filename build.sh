#!/bin/bash
# Tests and builds every extension for Volant's ABI 1 (wasm32-unknown-unknown, 16 MiB memory maximum)
# and pins each module's SHA-256 in its manifest. The toolchain is pinned in rust-toolchain.toml so a
# rebuild produces the same bytes; `./build.sh --check` fails if a committed module or hash is stale.
#
# Usage: ./build.sh [--check]
set -euo pipefail
cd "$(dirname "$0")"
TOOLCHAIN=$(sed -n 's/^channel = "\(.*\)"/\1/p' rust-toolchain.toml)
# rustup's own cargo honors the pinned toolchain; a Homebrew or mise cargo earlier on PATH would not.
cargo() { rustup run "$TOOLCHAIN" cargo "$@"; }
cargo test --quiet --workspace
# Remapping keeps machine paths (and usernames) out of panic messages, so modules are identical on any host.
CARGO_DIR=${CARGO_HOME:-$HOME/.cargo}
RUSTFLAGS="-C link-arg=--max-memory=16777216 --remap-path-prefix=$CARGO_DIR=/cargo --remap-path-prefix=$PWD=/src" cargo build --quiet --release --target wasm32-unknown-unknown --workspace
for manifest in */manifest.json; do
  folder=$(dirname "$manifest")
  module=$(python3 -c "import json,sys; print(json.load(open(sys.argv[1]))['module'])" "$manifest")
  built="target/wasm32-unknown-unknown/release/volant_$folder.wasm"
  if [ "${1:-}" = --check ]; then
    cmp -s "$built" "$folder/$module" || { echo "$folder/$module is stale; run ./build.sh and commit it"; exit 1; }
  else
    cp "$built" "$folder/$module"
  fi
  python3 - "$manifest" "$folder/$module" "${1:-}" <<'PY'
import hashlib, json, pathlib, sys
manifest, module, mode = pathlib.Path(sys.argv[1]), pathlib.Path(sys.argv[2]), sys.argv[3]
data = json.loads(manifest.read_text())
digest = hashlib.sha256(module.read_bytes()).hexdigest()
if mode == "--check":
    sys.exit(0 if data["sha256"] == digest else f"{manifest} has a stale sha256; run ./build.sh and commit it")
data["sha256"] = digest
manifest.write_text(json.dumps(data, indent=2) + "\n")
PY
  echo "$folder/$module: $(wc -c < "$folder/$module" | tr -d ' ') bytes, sha256 $(python3 -c "import json,sys; print(json.load(open(sys.argv[1]))['sha256'][:12])" "$manifest")…"
done
