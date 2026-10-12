# Wikipedia

Searches English Wikipedia from the launcher: `wiki ada lovelace`. Return opens the article. Reads
`en.wikipedia.org`'s public REST search API, ten results at a time.

The extension is written in Python ([`src/wikipedia.py`](src/wikipedia.py)) and runs on
[MicroPython](https://micropython.org) compiled to WebAssembly. It is a Volant **ABI 2** module: a
restricted WASI command that reads the request JSON on stdin and writes the reply JSON on stdout. The
plan and render phases are the same as the Rust extensions'.

## Build

```sh
./build.sh          # test, build wikipedia.wasm, pin its sha256 in manifest.json
./build.sh --check  # run the Python tests and check the committed module against its hash (no toolchain needed)
```

`./build.sh` downloads two pinned archives into `.toolchain/` (or `$WIKIPEDIA_TOOLCHAIN`) and checks
their SHA-256 before use:

| Input | Version | Notes |
|---|---|---|
| [wasi-sdk](https://github.com/WebAssembly/wasi-sdk) | 34.0 (clang 23) | Hashes for macOS arm64/x86_64 and Linux x86_64/arm64 |
| [MicroPython](https://github.com/micropython/micropython) | 1.29.0 | Release tarball, built through its `embed` port |

The build generates MicroPython's embed package for [`runtime/mpconfigport.h`](runtime/mpconfigport.h),
adds the `json` module, embeds `src/wikipedia.py` as a C string, and links
[`runtime/main.c`](runtime/main.c) with a 16 MiB memory maximum and a 1 MiB stack. Path prefixes are
remapped and `SOURCE_DATE_EPOCH` is fixed, so rebuilding on the same machine gives the same bytes.

The tests in [`tests/test_wikipedia.py`](tests/test_wikipedia.py) run the same `wikipedia.py` under
CPython with fictional data, including a check that every planned URL is https on a host the
manifest declares.

## Runtime

- **Imports:** only `fd_read`, `fd_write`, `fd_close`, `fd_fdstat_get`, `fd_seek`, `clock_time_get` and
  `proc_exit` from `wasi_snapshot_preview1`. Exports `memory` (18 pages initial, 256 maximum) and `_start`.
- **Exceptions:** MicroPython unwinds Python exceptions with setjmp/longjmp, which wasi-sdk lowers to
  the **legacy** WebAssembly exception-handling instructions (`try`/`catch`/`delegate`/`throw`), not
  `exnref`. JavaScriptCore has supported legacy exception handling since Safari 15.2.
- **Memory:** the garbage collector is off. Each request runs in a fresh instance, so objects are taken
  from wasi-libc's `malloc` and never freed; a 48 KB response uses well under the 16 MiB cap. Leaving
  the collector out also avoids conservative stack scanning, which cannot see values held in
  WebAssembly locals.
- **No filesystem:** `import` resolves only built-in modules, and `open()` raises `OSError`.
- **Failures:** `respond()` turns any Python error into one explaining row; an error that escapes it
  becomes a fixed fallback row from `main.c`.

The module is about 254 KiB. The Python source is compiled to bytecode on every run, which takes a few
milliseconds.

## License

The extension is MIT. The module includes MicroPython (MIT, see
[`MICROPYTHON-LICENSE`](MICROPYTHON-LICENSE)) and wasi-libc from wasi-sdk (Apache-2.0 WITH
LLVM-exception, MIT and others; see the wasi-sdk release).
