# Fresh-macOS build — October 30, 2026

Every extension was built from scratch on a Mac with no developer tools, to show that the pinned
toolchains install and build from nothing. The evidence asked for in mysticcoders/volant#64.

## Machine

- A disposable Tart clone of vanilla macOS 27.0 (26A428), arm64, with no Command Line Tools, Xcode, Rust or Node; `git` and `python3` were only the system's install stubs.
- Source: `git archive` of this repository at `6c92fcf` (main after #19), copied into the guest.

## Steps and times (UTC)

| Step | Started | What ran |
| --- | --- | --- |
| Command Line Tools | 23:44:28 | `softwareupdate -i "Command Line Tools for Xcode 27.2-27.2"`, headless: Apple clang 21.0.0, git 2.54.0, Python 3.9.6 |
| Rust | 23:46:18 | rustup with no default toolchain, then the pinned `rustc 1.89.0` with `wasm32-unknown-unknown` (minimal profile) |
| Node | 23:46:26 | `node-v24.22.1-darwin-arm64.tar.gz` from nodejs.org, checked against its `SHASUMS256.txt` (npm 11.19.1) |
| `./build.sh` | 23:46:29 | Cargo tests and release builds of the seven Rust extensions; `npm/build.sh` downloaded and checksum-checked Javy 9.1.0; `wikipedia/build.sh` downloaded and checksum-checked wasi-sdk 34 and MicroPython 1.29.0 and compiled them |
| Checks | 23:47:21 | `./build.sh --check`, `tools/new-extension.py --self-test`, `tools/check-licenses.py` |

From a bare machine to every module built took about three minutes, most of it the Command Line Tools.

## Modules against the committed ones

| Module | Fresh build |
| --- | --- |
| hackernews, homebrew (Rust) | byte-identical |
| github, lobsters, todoist, weather (Rust) | differ in bytes |
| npm (TypeScript, Javy) | differs in bytes |
| wikipedia (Python, MicroPython on wasi-sdk) | byte-identical |

Every freshly built module passed `tools/check-module.py`: its ABI, exports, imports, single memory with a 16 MiB maximum, and size. The differences are expected and documented. Javy builds aren't byte-identical, and Rust builds match only on the same machine, since codegen order can differ between hosts. The committed modules stay the published ones; CI checks them against their manifest hashes.

## What it found

`tools/check-licenses.py` imported `tomllib`, which needs Python 3.11, so on macOS's own Python 3.9.6 it failed. That also failed the template self-test, which runs it. It now reads `Cargo.lock` with the standard library. With that fix, the license check and `tools/new-extension.py --self-test` pass on Python 3.9.6, and every tool in `tools/` parses as Python 3.9. A Mac with a newer Python on its `PATH`, such as the owner's or CI's, never saw the problem.

## Not covered

- Intel Macs, and macOS 15 or 26 hosts.
- A clean Linux build: CI covers that on Ubuntu.
- `tools/catalog.py`, which numbers the catalog by git commit count and so needs the real repository history. CI runs it.
