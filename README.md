# Volant extensions

Extensions for [Volant](https://usevolant.com), the macOS launcher. Each folder is one extension: a
`manifest.json` and a WebAssembly module, ready to install.

| Extension | Launcher keyword | Reads | What it shows |
|---|---|---|---|
| [Hacker News](hackernews) | `hn`, `hn new`, `hn ask`, `hn show`, `hn <search>` | `hn.algolia.com` | Front page, newest, Ask HN, Show HN or search results |
| [Lobsters](lobsters) | `lobsters`, `lobsters new`, `lobsters <tag>` | `lobste.rs` | Hottest, newest or one tag |

Return opens a story; Command-Return opens its discussion in your browser.

## Install

In Volant versions with the extension catalog, open Settings → Extensions, turn on **Allow Community
Extensions**, choose **Check Catalog**, then **Install**. Type the extension's keyword in the launcher;
the first run lists the hosts it needs, and **Allow and Run** enables it.

To install by hand instead:

1. In Volant, open Settings → Extensions, turn on **Allow Community Extensions**, and choose **Open Folder**.
2. Copy an extension's folder (for example `hackernews/`, with its `manifest.json` and `.wasm`) into that folder, then choose **Refresh**.
3. Type its keyword in the launcher. The first run lists the hosts it needs; **Allow and Run** enables it.

Allowed hosts are listed under Settings → Extensions → Network Access, where you can remove them.
Uninstalling an extension there also removes hosts no other extension needs.

## How these extensions work

They are list extensions for Volant's ABI 1 and never touch the network themselves. Volant runs the
module twice:

1. **Plan:** the module receives `{"phase":"plan","query":"…","now":…}` and returns
   `{"fetch":["https://…"]}`. Volant fetches each URL only if it is https and its host is listed in the
   manifest's `hosts` and allowed by the owner.
2. **Render:** the module receives the same request plus `"responses":[{"url","status","body"}]`
   and returns `{"items":[{"title","subtitle","url","secondaryURL","secondaryTitle"}]}`.

A plan may return `items` instead, such as one row explaining an unusable query. Requests and
replies are limited to 64 KiB, responses to 48 KB, and a module to 2 MiB with at most 16 MiB of
memory. See Volant's [network extensions](https://github.com/mysticcoders/volant/blob/main/docs/network-extensions.md)
documentation for the full contract.

[`kit/`](kit) is a small Rust crate with the ABI glue (`volant_extension!`), the request and reply
types, and helpers for ages, domains, query encoding and RFC 3339 timestamps.

## Build

```sh
./build.sh          # cargo test, build every module, pin each sha256 in its manifest
./build.sh --check  # test and build, and fail if a manifest's sha256 does not match its module (CI)
```

The toolchain is pinned in `rust-toolchain.toml` (with the `wasm32-unknown-unknown` target), so
rebuilds on the same machine produce the same bytes; Linux and macOS builds can differ in function
order. Build paths are remapped, so no machine path or username ends up in a module. The script runs cargo through rustup, so a Homebrew cargo earlier
on your `PATH` does not get in the way.

## Catalog

Every push to `main` runs `.github/workflows/catalog.yml`: it tests and checks the modules, builds the
site with `tools/catalog.py` (every folder with a `listing.json` is listed), signs the exact bytes of
`catalog.json` with Ed25519 using the `CATALOG_SIGNING_KEY` secret, and publishes the site to
<https://mysticcoders.github.io/volant-extensions/>. Volant holds the public key, refuses a catalog
whose signature fails or whose `sequence` (the commit count of `main`) is older than one it accepted,
and checks every downloaded file against the sizes and SHA-256 hashes in the signed index.

Merging to `main` publishes, so review an extension's source, hosts and license before merging it.
To add one, give its folder a `listing.json`:

```json
{ "summary": "One sentence.", "publisher": "Your name", "license": "MIT", "notes": "Optional detail." }
```

## License

MIT. See [LICENSE](LICENSE).
