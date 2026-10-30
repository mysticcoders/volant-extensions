# Dependency audit, 2026-10-30

This closes the dependency-audit item of [mysticcoders/volant#64](https://github.com/mysticcoders/volant/issues/64):
cargo-deny and cargo-audit for the Rust workspace, `npm audit` for the TypeScript extension, and the
license notices for MicroPython, Javy and everything else compiled into a module. It is an advisory and
license review of locked inputs, not a code review of the dependencies or proof that none has an
unknown flaw.

## What ships

Each module is self-contained WebAssembly, so a dependency matters if its code is compiled in:

| Module | Compiled in | Build-only |
|---|---|---|
| Rust (github, hackernews, homebrew, lobsters, todoist, weather) | `kit`, serde, serde_core, serde_json, itoa, memchr, zmij, Rust `core`/`alloc` | serde_derive, proc-macro2, quote, syn, unicode-ident (proc macros) |
| `npm/npm.wasm` | the extension's own TypeScript, Javy 9.1.0's plugin: QuickJS-ng 0.15.0 via rquickjs 0.12.0, Rust crates from Javy's lockfile, Rust `std`, wasi-libc | esbuild 0.25.10, TypeScript 5.9.3 (dev dependencies, nothing bundled from npm) |
| `wikipedia/wikipedia.wasm` | MicroPython 1.29.0 (`py/`, `extmod/modjson.c`, `shared/runtime`), wasi-libc and compiler-rt from wasi-sdk 34 | CPython for the tests (standard library only) |

## Results

| Check | Tool and version | Result |
|---|---|---|
| Rust advisories (RustSec) | cargo-deny 0.20.2, `check advisories` | ok: no advisories, nothing yanked |
| Rust advisories (RustSec) | cargo-audit 0.22.2 | 0 vulnerabilities in 18 crates, 1395 advisories loaded |
| Rust licenses | cargo-deny 0.20.2, `check licenses` | ok: MIT, Apache-2.0, Unlicense, Unicode-3.0 only |
| Duplicate crates, wildcards | cargo-deny 0.20.2, `check bans` | ok |
| Crate sources | cargo-deny 0.20.2, `check sources` | ok: crates.io only, no git dependencies |
| npm advisories | npm 11.19.1 (Node 24.22.1), `npm audit` in `npm/` | found 0 vulnerabilities (28 locked dev packages, 26 of them optional esbuild platform binaries) |
| npm advisories, new TypeScript extension | the same, in a folder made by `tools/new-extension.py typescript` | found 0 vulnerabilities (same lockfile, renamed) |
| Python | none needed | no package-manager dependencies: MicroPython and wasi-sdk are pinned downloads checked by SHA-256 in `wikipedia/build.sh` |

The machine's existing cargo-audit 0.21.2 could not run: `error loading advisory database: ... RUSTSEC-2026-0073.md: ...
unsupported CVSS version: 4.0`. RustSec now publishes CVSS 4.0 scores, which that release cannot parse;
cargo-audit 0.22.2 (installed with `cargo install cargo-audit --locked`) reads them. CI uses cargo-deny,
which covers the same advisory database plus licenses, bans and sources.

Nothing needed an upgrade, override or exception. `deny.toml` has an empty `ignore` list.

The workspace crates now say `publish = false` (also in `templates/rust/Cargo.toml`), which is accurate
and lets cargo-deny skip license checks for our own unpublished crates (`[licenses.private]`) and accept
path dependencies without a version. It changes no module: the Rust modules built before and after are
byte-identical.

## License notices

| Notice | Covers | Finding |
|---|---|---|
| `npm/JAVY-LICENSE` | Javy 9.1.0 (Apache-2.0 WITH LLVM-exception) | Byte-identical to `LICENSE.md` at tag v9.1.0 |
| `npm/QUICKJS-LICENSE` | QuickJS-ng 0.15.0 (MIT) | Byte-identical to `LICENSE` at tag v0.15.0 |
| `npm/RQUICKJS-LICENSE` | rquickjs 0.12.0 (MIT) | Byte-identical to `LICENSE` at tag v0.12.0 |
| `npm/JAVY-CRATES-LICENSES` | Rust crates in Javy's plugin | **Added.** `npm.wasm` is built with `dynamic=n`, so Javy's plugin is compiled in, and it carries anyhow, serde_json, simd-json, rquickjs-serde and others (their source paths are visible in the module). Generated with cargo-about from the plugin's locked graph for `wasm32-wasip1` |
| `npm/WASI-LIBC-LICENSE`, `wikipedia/WASI-LIBC-LICENSE` | wasi-libc (MIT, plus musl MIT, cloudlibc BSD-2-Clause, musl-fts BSD-3-Clause) | **Added.** Both modules link wasi-libc statically, and the wasi-sdk archive ships no license files. Texts are from wasi-libc commit `2e6fb9d8`, the submodule pinned by wasi-sdk-34; dlmalloc is CC0 |
| `wikipedia/MICROPYTHON-LICENSE` | MicroPython 1.29.0 (MIT) | Byte-identical to the release's `LICENSE`. Every compiled file is under MicroPython's own MIT terms; none of the differently licensed `lib/` or `drivers/` code is built |
| `RUST-LICENSES` | Crates in the Rust modules | **Added.** The modules compile in serde, serde_json, itoa, memchr and zmij, whose MIT or Apache-2.0 terms ask for the license with copies. Generated with cargo-about; lists proc macros too |

compiler-rt builtins, which wasi-sdk links into `wikipedia.wasm`, are Apache-2.0 WITH LLVM-exception,
whose exception waives the notice requirement for code compiled into a program. Rust's `core`, `alloc`
and `std` (MIT OR Apache-2.0) are in every Rust module and in Javy's plugin; they are named here but have
no notice file (see Limitations).

`tools/new-extension.py` now copies `JAVY-CRATES-LICENSES` and `WASI-LIBC-LICENSE` into a new TypeScript
extension and `WASI-LIBC-LICENSE` into a new Python one, beside the Javy, QuickJS, rquickjs and
MicroPython licenses it already copied. A new Rust extension is covered by `RUST-LICENSES` at the root.

`tools/check-licenses.py` fails when a Javy or wasi-sdk extension lacks one of its notices, when
`tools/new-extension.py` would not copy one, or when `Cargo.lock` holds a crates.io package that
`RUST-LICENSES` does not list at that version. `tools/new-extension.py --self-test` runs it on its
scratch copy with one extension of each language.

## CI

The `audit` job in `.github/workflows/ci.yml` runs on every pull request, every push to `main` and every
Monday. It has read-only repository permissions, downloads cargo-deny 0.20.2 and checks it against the
release's SHA-256, then runs:

```sh
cargo-deny --locked check advisories licenses bans sources
(cd npm && npm audit --audit-level=low)
tools/check-licenses.py
```

Any advisory, unaccepted license, duplicate crate, unknown source, npm advisory at any severity, or
stale notice fails it. The weekly run catches advisories published against unchanged lockfiles.

## Rerun it

```sh
cargo install cargo-deny --locked     # or the release binary CI uses
cargo deny --locked check advisories licenses bans sources
cargo install cargo-audit --locked    # 0.22 or later reads CVSS 4.0
cargo audit
(cd npm && npm audit)
tools/check-licenses.py
```

After a Rust dependency change, regenerate `RUST-LICENSES` with
[cargo-about](https://github.com/EmbarkStudios/cargo-about) (0.9.3 here; `cargo install cargo-about --locked --features cli`):

```sh
cargo about generate tools/licenses.hbs --target wasm32-unknown-unknown --locked -o RUST-LICENSES
```

After a Javy upgrade, regenerate `npm/JAVY-CRATES-LICENSES` from Javy's source at the new tag, and
recheck the Javy, QuickJS-ng and rquickjs licenses against their tags:

```sh
git clone --depth 1 --branch v9.1.0 https://github.com/bytecodealliance/javy.git
cd javy
cargo about generate -c ../volant-extensions/about.toml ../volant-extensions/tools/licenses.hbs \
  -m crates/plugin/Cargo.toml --target wasm32-wasip1 --locked -o ../volant-extensions/npm/JAVY-CRATES-LICENSES
```

After a wasi-sdk upgrade, take the wasi-libc submodule commit from `src/wasi-libc` at the new tag and
refresh `WASI-LIBC-LICENSE` from that commit's `LICENSE`, `LICENSE-MIT`, `libc-top-half/musl/COPYRIGHT`,
`libc-bottom-half/cloudlibc/LICENSE` and `fts/musl-fts/COPYING`. After a MicroPython upgrade, compare
`MICROPYTHON-LICENSE` with the release's `LICENSE` and check that no newly compiled file has other terms.

## Limitations

- The catalog publishes only each manifest, module and icon, so notices travel with the source folder
  (linked from each listing's `source`) and a hand-copied folder, not with a catalog download. Changing
  that would change the catalog format Volant reads.
- `JAVY-CRATES-LICENSES` describes Javy's lockfile at v9.1.0, which is what the release binary was built
  from; it is not reconstructed from the module. Some listed crates (such as quickcheck, regex and
  env_logger, normal dependencies of Javy's crate) may be dropped by the linker; listing them is the
  safe side.
- `npm audit` and cargo-deny cover npm and crates.io packages. Javy, QuickJS-ng, MicroPython and wasi-sdk
  are pinned downloads with no lockfile those tools read; their advisories need watching upstream when
  a pin moves.
- Rust's standard library has no notice file of its own, a common practice that this audit did not
  change; add one if the owner wants every compiled-in component listed.
- An advisory match or its absence says nothing about reachability in a sandboxed module that has no
  network or file access of its own.
