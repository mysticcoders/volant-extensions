# Volant extensions

Extensions for [Volant](https://usevolant.com), the macOS launcher. Each folder is one extension: a
`manifest.json` and a WebAssembly module, ready to install.

| Extension | Language | Launcher keyword | Reads | What it shows |
|---|---|---|---|---|
| [Hacker News](hackernews) | Rust | `hn`, `hn new`, `hn ask`, `hn show`, `hn <search>` | `hn.algolia.com` | Front page, newest, Ask HN, Show HN or search results |
| [Lobsters](lobsters) | Rust | `lobsters`, `lobsters new`, `lobsters <tag>` | `lobste.rs` | Hottest, newest or one tag |
| [npm](npm) | TypeScript | `npm <search>` | `registry.npmjs.org` | Packages matching a name or keyword |
| [Wikipedia](wikipedia) | Python | `wiki <search>` | `en.wikipedia.org` | English Wikipedia articles matching a topic |
| [Weather](weather) | Rust | `weather`, `weather <place>` | `api.open-meteo.com`, `geocoding-api.open-meteo.com` | Current conditions and a 10-day forecast, for your location or a place |
| [GitHub](github) | Rust | `github`, `github <search>` | `api.github.com` | Pull requests to review, your pull requests, and issues assigned to or involving you |
| [Homebrew](homebrew) | Rust | `homebrew <name>` | `formulae.brew.sh` | A formula or cask by exact name, with its install command |
| [Todoist](todoist) | Rust | `todo`, `todo upcoming`, `todo inbox`, `todo p1`, `todo /<filter>`, `todo <new task>` | `api.todoist.com` | Your tasks; adds, completes and reschedules them |

Return opens a story or package; Command-Return opens its discussion, homepage or repository in your browser.

In Volant 0.3.7 or later, Return on an extension enters it: the search field clears and
belongs to the extension, with a Back chip at its start, and what you type runs it again once you pause.
Escape clears the text, then goes back. GitHub (Volant 0.3.9 or later for its dropdown) asks once for a
fine-grained personal access token with read access to pull requests and issues, kept in your Keychain
and added only to requests to api.github.com; its dropdown picks Review Requests, My Pull Requests,
Assigned to Me or Involving Me, and typed text narrows the list with GitHub's search syntax, such as
`repo:owner/name`. Homebrew looks up the exact formula or cask name you type, since Homebrew has no
search API; Return on its install row copies `brew install …`. Hacker News adds a dropdown at the end of the field (Top, New,
Ask HN, Show HN; Command-P picks the next one) and searches within the chosen page, and shows each
story's points, comments and site as badges. Wikipedia and npm declare `"mode": "search"`, so they wait for
you to type; the others show their list at once. Older Volant versions ignore `mode` and `placeholder`.

Weather and Todoist need Volant 0.3.4 or later. Weather asks once to share your approximate location
(about 11 km) with it and Open-Meteo; `weather <place>` works without it. Weather data is by
[Open-Meteo.com](https://open-meteo.com) (CC BY 4.0). Todoist asks once for your Todoist API token,
which Volant keeps in your Keychain and adds only to requests to api.todoist.com; the extension never
sees it. Type `todo` and a task, such as `todo buy milk tomorrow p1`, and Return adds it with Todoist's
Quick Add; a task's Actions complete it or move it to tomorrow or next week, and Return opens it in
Todoist.app, or on the web without the app.

In Hacker News and Lobsters, a story's Actions (Command-K) can pin it, hide it, or hide every story
from its domain; Show Hidden Stories brings them back, and stories you open show as read. Pinned
stories lead the list. This needs a Volant version with extension storage; the list is kept only on
your Mac, deleted when you uninstall the extension, and cleared with Clear Data in Settings →
Extensions.

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

They are list extensions and never touch the network themselves. The Rust ones use Volant's ABI 1
(`alloc` and `run` exports); the others use ABI 2, a restricted WASI command that reads the request
on stdin and writes its reply to stdout. Either way, Volant runs the module twice:

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
types, and helpers for ages, domains, query encoding, RFC 3339 timestamps and checking planned URLs
against the manifest's hosts. [`npm/src/kit.ts`](npm/src/kit.ts) is its TypeScript counterpart.
Every extension has a test that each URL it plans is https on a host its manifest declares, so the
source cannot drift from what the owner approves.

## Storage and row actions

A manifest with `"storage": true` gets the JSON object it last saved as `"storage"` in every request,
and any reply may include `"storage"` to replace it (at most 16 KB, an object nested at most 8 levels).
Rows may carry `"actions": [{"id","title"}]`, shown in their Actions menu, and `"onOpen": true`.
Choosing an action runs the module with `{"phase":"action","action":…,"item":{…the row…},"storage":…}`,
and opening a row with `onOpen` sends `{"phase":"opened","item":…}`; neither fetches. A reply of only
`{"storage":…}` saves it and Volant runs the list again. `kit::reader` implements the pins, hidden
stories and domains, and read state both readers use, with every list capped well under the limit.

## Build

```sh
./build.sh          # test and build every module, pin each sha256 in its manifest
./build.sh --check  # test and build Rust, test the rest, and check every committed module (CI)
```

Rust extensions are members of the Cargo workspace. Any other folder with a `manifest.json` builds
itself with its own `build.sh`, which takes the same `--check` flag. After building,
`tools/check-module.py` checks every module the way Volant will: its sha256, at most 2 MiB, one memory
with a maximum of at most 16 MiB, and only the imports and exports its ABI allows.
`tools/volant-host.sh MODULE ABI < request.json` runs a module once in Volant's production
JavaScriptCore host, from a Volant checkout beside this repository, and prints its reply.

The TypeScript extension is bundled with esbuild and compiled with [Javy](https://github.com/bytecodealliance/javy)
9.1.0, which embeds the QuickJS-ng 0.15.0 engine through rquickjs 0.12.0. The module carries code from
all three, so `npm/` includes their licenses: [`JAVY-LICENSE`](npm/JAVY-LICENSE) (Apache-2.0 WITH
LLVM-exception), [`RQUICKJS-LICENSE`](npm/RQUICKJS-LICENSE) and [`QUICKJS-LICENSE`](npm/QUICKJS-LICENSE) (both MIT).
`npm/build.sh` downloads Javy and checks its SHA-256 first.
It needs Node 24. Javy does not produce identical bytes on every build, so CI checks only the
committed module and its hash, not a rebuild.

The Python extension runs on [MicroPython](https://micropython.org) 1.29.0, compiled with
[wasi-sdk](https://github.com/WebAssembly/wasi-sdk) 34; `wikipedia/build.sh` downloads both and checks
their SHA-256 first. Its tests run under plain CPython, so `--check` needs only `python3`. See
[`wikipedia/README.md`](wikipedia/README.md) for how the runtime is built and its limits.

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

## Icons and trademarks

An extension can show an icon in the launcher and Settings: a square WebP of at most 32 KB beside its
manifest, named by `icon` and pinned by `iconSha256` (`./build.sh` writes the hash). GitHub, Hacker News,
Homebrew, Lobsters, Todoist and Wikipedia use their sites' own touch icons, and npm uses its logo from the npm
organization's GitHub avatar, each converted to a 128-pixel WebP. Weather's icon is original art
(`weather/icon.svg`). Hacker News and the Y Combinator logo are trademarks of Y Combinator; the
Lobsters name and logo belong to Lobsters; Todoist is a trademark of Doist; GitHub and the Invertocat logo are trademarks of GitHub, Inc.; the
Homebrew name and logo belong to the Homebrew project; npm is a trademark of
npm, Inc., part of GitHub; Wikipedia and its logo are trademarks of the Wikimedia Foundation. These
extensions are unofficial and not endorsed by any of them, and the MIT license below covers the code,
not these marks.

## License

MIT. See [LICENSE](LICENSE).
