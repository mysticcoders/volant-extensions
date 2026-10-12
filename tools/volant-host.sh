#!/bin/bash
# Runs a module once in Volant's production JavaScriptCore extension host, with a Volant checkout
# beside this repository (or VOLANT_DIR), and prints its output. It compiles the host sources into a
# small runner, ad-hoc signed with the JIT entitlement Volant's helper has, so ABI 2 modules that use
# SIMD run as they would on macOS 15. Nothing is written to the Volant checkout.
#
# Usage: tools/volant-host.sh MODULE.wasm ABI_VERSION < request.json
set -euo pipefail
module=$(cd "$(dirname "$1")" && pwd)/$(basename "$1")
abi=$2
volant=${VOLANT_DIR:-$(cd "$(dirname "$0")/../.." && pwd)/volant}
cache="${XDG_CACHE_HOME:-$HOME/.cache}/volant-extensions/host"
mkdir -p "$cache"
swift build --quiet --package-path "$volant/Core" --scratch-path "$cache/core" >&2
bin=$(swift build --package-path "$volant/Core" --scratch-path "$cache/core" --show-bin-path)
cat > "$cache/main.swift" <<'SWIFT'
import Foundation
let module = try Data(contentsOf: URL(fileURLWithPath: CommandLine.arguments[1]))
let input = String(decoding: FileHandle.standardInput.readDataToEndOfFile(), as: UTF8.self)
let reply: (String?, String?) -> Void = { output, error in
    if let error { FileHandle.standardError.write(Data("error: \(error)\n".utf8)); exit(1) }
    print(output ?? "")
}
if CommandLine.arguments[2] == "2" {
    ExtensionHost().runCommand(module: module, input: input, timeout: 10, reply: reply)
} else {
    ExtensionHost().run(module: module, capabilities: [], input: input, timeout: 10, reply: reply)
}
SWIFT
swiftc -O -I "$bin" -I "$bin/Modules" -L "$bin" -lVolantCore "$volant/VolantExtensionHost/ExtensionHost.swift" \
  "$volant/VolantExtensionHost/WASICommand.swift" "$cache/main.swift" -o "$cache/run" 2>&1 | rg -v "warning" >&2 || true
printf '%s' '<?xml version="1.0" encoding="UTF-8"?><plist version="1.0"><dict><key>com.apple.security.cs.allow-jit</key><true/></dict></plist>' > "$cache/jit.plist"
codesign --force --sign - --options runtime --entitlements "$cache/jit.plist" "$cache/run" 2>/dev/null
"$cache/run" "$module" "$abi"
