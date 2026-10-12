#!/usr/bin/env python3
"""Checks an extension's module against its manifest and the rules Volant enforces before running it.

The module must match the manifest's sha256 and be at most 2 MiB, with one non-shared wasm32 memory
whose explicit maximum is at most 16 MiB. ABI 1 modules export `memory`, `alloc` and `run` and may
import only the `vey` functions their capabilities grant; ABI 2 modules export `memory` and `_start`
and may import only Volant's restricted set of WASI preview 1 functions. Volant checks all of this
again at run time; this script catches a bad module before it is committed or published.

Usage:
  tools/check-module.py MANIFEST              check the module and print a summary
  tools/check-module.py --pin MANIFEST        write the module's sha256 into the manifest
  tools/check-module.py --module-name MANIFEST print the manifest's module file name
"""
import hashlib
import json
import pathlib
import sys

MAX_MODULE_BYTES = 2 * 1024 * 1024
MAX_MEMORY_PAGES = 256
WASM_MAGIC = b"\0asm\1\0\0\0"
IMPORT_SECTION, MEMORY_SECTION, EXPORT_SECTION = 2, 5, 7
FUNCTION_KIND, TABLE_KIND, MEMORY_KIND, GLOBAL_KIND, TAG_KIND = 0, 1, 2, 3, 4
ABI1_NAMESPACE = "vey"
ABI1_CAPABILITY_IMPORTS = {"log": "log", "clipboard_write": "clipboard.write"}
ABI1_EXPORTS = {"memory", "alloc", "run"}
WASI_NAMESPACE = "wasi_snapshot_preview1"
WASI_IMPORTS = {
    "fd_read", "fd_write", "fd_close", "fd_fdstat_get", "fd_seek",
    "environ_sizes_get", "environ_get", "clock_time_get", "proc_exit",
}
ABI2_EXPORTS = {"memory", "_start"}


class Reader:
    """Reads the LEB128 integers, names and limits a WASM binary is made of."""

    def __init__(self, data, position=0, end=None):
        self.data, self.position, self.end = data, position, len(data) if end is None else end

    def byte(self):
        if self.position >= self.end:
            sys.exit("truncated module")
        value = self.data[self.position]
        self.position += 1
        return value

    def unsigned(self):
        value, shift = 0, 0
        while True:
            byte = self.byte()
            value |= (byte & 0x7F) << shift
            if byte < 0x80:
                return value
            shift += 7

    def name(self):
        length = self.unsigned()
        start = self.position
        self.position += length
        return self.data[start:self.position].decode("utf-8")

    def limits(self):
        flags = self.byte()
        initial = self.unsigned()
        maximum = self.unsigned() if flags & 1 else None
        return flags, initial, maximum


def sections(data):
    """Yields (section id, reader over its body) for every section of a module."""
    if data[:8] != WASM_MAGIC:
        sys.exit("not a WebAssembly 1.0 module")
    reader = Reader(data, 8)
    while reader.position < len(data):
        kind = reader.byte()
        length = reader.unsigned()
        yield kind, Reader(data, reader.position, reader.position + length)
        reader.position += length


def inspect(data):
    """The module's imports as (namespace, name, kind), its exported names, and its memory limits."""
    imports, exports, memories = [], set(), []
    for kind, body in sections(data):
        if kind == IMPORT_SECTION:
            for _ in range(body.unsigned()):
                namespace, name, import_kind = body.name(), body.name(), body.byte()
                imports.append((namespace, name, import_kind))
                if import_kind == MEMORY_KIND:
                    memories.append(body.limits())
                elif import_kind == TABLE_KIND:
                    body.byte()
                    body.limits()
                elif import_kind == GLOBAL_KIND:
                    body.byte()
                    body.byte()
                elif import_kind == TAG_KIND:
                    body.byte()
                    body.unsigned()
                else:
                    body.unsigned()
        elif kind == MEMORY_SECTION:
            memories.extend(body.limits() for _ in range(body.unsigned()))
        elif kind == EXPORT_SECTION:
            for _ in range(body.unsigned()):
                exports.add(body.name())
                body.byte()
                body.unsigned()
    return imports, exports, memories


def problems(manifest, data):
    """Every way the module breaks the manifest or Volant's rules, as sentences."""
    found = []
    abi = manifest.get("abiVersion", 1)
    if len(data) > MAX_MODULE_BYTES:
        found.append(f"module is {len(data)} bytes, over the {MAX_MODULE_BYTES}-byte limit")
    if hashlib.sha256(data).hexdigest() != manifest.get("sha256"):
        found.append("manifest sha256 does not match the module; run ./build.sh and commit it")
    imports, exports, memories = inspect(data)
    if len(memories) != 1:
        found.append(f"module has {len(memories)} memories instead of one")
    for flags, initial, maximum in memories:
        if flags & 2:
            found.append("memory is shared")
        if maximum is None or maximum > MAX_MEMORY_PAGES:
            found.append(f"memory maximum is {maximum} pages; Volant requires at most {MAX_MEMORY_PAGES}")
    if abi == 1:
        granted = {name for name, capability in ABI1_CAPABILITY_IMPORTS.items() if capability in manifest.get("capabilities", [])}
        allowed, required = {(ABI1_NAMESPACE, name) for name in granted}, ABI1_EXPORTS
    elif abi == 2:
        if manifest.get("capabilities"):
            found.append("ABI 2 modules cannot declare capabilities")
        allowed, required = {(WASI_NAMESPACE, name) for name in WASI_IMPORTS}, ABI2_EXPORTS
    else:
        return found + [f"unsupported abiVersion {abi}"]
    for namespace, name, kind in imports:
        if kind != FUNCTION_KIND or (namespace, name) not in allowed:
            found.append(f"import {namespace}.{name} is not allowed for ABI {abi}")
    for name in sorted(required - exports):
        found.append(f"module does not export {name}")
    return found


def main(arguments):
    flag = arguments[0] if arguments and arguments[0].startswith("--") else None
    manifest_path = pathlib.Path(arguments[-1])
    manifest = json.loads(manifest_path.read_text())
    if flag == "--module-name":
        print(manifest["module"])
        return
    module_path = manifest_path.parent / manifest["module"]
    data = module_path.read_bytes()
    if flag == "--pin":
        manifest["sha256"] = hashlib.sha256(data).hexdigest()
        manifest_path.write_text(json.dumps(manifest, indent=2) + "\n")
        return
    found = problems(manifest, data)
    if found:
        sys.exit("\n".join(f"{module_path}: {problem}" for problem in found))
    print(f"{module_path}: ABI {manifest.get('abiVersion', 1)}, {len(data)} bytes, sha256 {manifest['sha256'][:12]}…")


if __name__ == "__main__":
    main(sys.argv[1:])
