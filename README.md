# Volant extensions

Extensions for [Volant](https://usevolant.com), the macOS launcher. Each folder is one extension: a
`manifest.json` and a WebAssembly module, ready to install.

| Extension | Language | Launcher keyword | Reads | What it shows |
|---|---|---|---|---|
| [Hacker News](hackernews) | Rust | `hn`, `hn new`, `hn ask`, `hn show`, `hn <search>` | `hn.algolia.com` | Front page, newest, Ask HN, Show HN or search results |
| [Lobsters](lobsters) | Rust | `lobsters`, `lobsters new`, `lobsters <tag>` | `lobste.rs` | Hottest, newest or one tag |
| [npm](npm) | TypeScript | `npm <search>` | `registry.npmjs.org` | Packages matching a name or keyword |
| [Wikipedia](wikipedia) | Python | `wiki <search>` | `en.wikipedia.org` | English Wikipedia articles matching a topic |
| [Meteo Weather](meteo) | Rust | `meteo`, `meteo <place>` | `api.open-meteo.com`, `geocoding-api.open-meteo.com` | Open-Meteo's current conditions, hourly strip and 10-day forecast, for your location or a place |
| [GitHub](github) | Rust | `github`, `github <search>` | `api.github.com` | Pull requests to review, your pull requests, and issues assigned to or involving you |
| [Homebrew](homebrew) | Rust | `homebrew <name>` | `formulae.brew.sh` | A formula or cask by exact name, with its install command |
| [Todoist](todoist) | Rust | `todo`, `todo upcoming`, `todo inbox`, `todo p1`, `todo /<filter>`, `todo <new task>` | `api.todoist.com` | Your tasks; adds, completes and reschedules them |
| [Service Status](status) | Rust | `status`, `status <name>` | `status.claude.com`, `status.openai.com`, `www.githubstatus.com`, `www.cloudflarestatus.com` | Whether Claude, OpenAI, GitHub and Cloudflare are up, with open incidents and maintenance |
| [Maps](maps) | Rust | `maps <place>` | `nominatim.openstreetmap.org` | Places and addresses, opened in Apple Maps or on OpenStreetMap |
| [Public Holidays](holidays) | Rust | `holidays`, `holidays <name>`, `holidays <country code>` | `date.nager.at` | Upcoming public holidays for your country or another |
| [Stack Overflow](stackoverflow) | Rust | `stackoverflow <search>` | `api.stackexchange.com` | Questions matching a search, with score, answers and answered state |
| [crates.io](crates) | Rust | `crates <search>` | `crates.io` | Rust crates matching a name or keyword, with downloads |
| [PyPI](pypi) | Rust | `pypi <name>` | `pypi.org` | A Python package by exact name, with its `uv add` and `pip install` commands |
| [Docker Hub](docker) | Rust | `docker <search>` | `hub.docker.com` | Container images matching a name or keyword, with stars, pulls and the `docker pull` command |
| [MDN](mdn) | Rust | `mdn <search>` | `developer.mozilla.org` | MDN Web Docs pages matching a topic |
| [Word Count](wordcount) | Rust | `word count`, `word count <text>` | nothing (offline) | Words, characters, sentences, paragraphs, lines, and reading and speaking times |
| [Case Converter](case) | Rust | `case`, `case <text>` | nothing (offline) | The text in camelCase, PascalCase, snake_case, SCREAMING_SNAKE_CASE, kebab-case, Title Case, Sentence case, lower case, UPPER CASE and dot.case |
| [Encode/Decode](encode) | Rust | `encode`, `encode <text>` | nothing (offline) | Base64, Base64URL, URL and HTML encodings, and decodings when the text already is one |
| [Hash](hash) | Rust | `hash`, `hash <text>` | nothing (offline) | SHA-256, SHA-1, MD5 and SHA-512 digests in lowercase hex |
| [Characters](characters) | Rust | `characters`, `characters <name>` | nothing (offline) | Typographic and special characters by name: dashes, curly quotes, arrows, math, currency, legal marks, fractions and keyboard symbols |
| [Fancy Text](fancytext) | Rust | `fancy text`, `fancy text <text>` | nothing (offline) | The text in bold, italic, script, fraktur, double-struck, monospace, small caps, circled, upside down and other Unicode styles |
| [SF Symbols](sfsymbols) | Rust | `sf symbols <name>` | nothing (offline) | SF Symbol names matching a search, with categories and the iOS and macOS versions that introduced them |
| [Apple HIG](hig) | Rust | `apple hig`, `apple hig <topic>` | `developer.apple.com` | Human Interface Guidelines topics matching a search, or one section's topics |
| [Google Fonts](fonts) | Rust | `google fonts`, `google fonts <name>` | `api.fontsource.org` | Font families matching a name, or one category's, with weights, styles and the Fontsource install command |
| [Svgl](svgl) | Rust | `svgl <brand>` | `api.svgl.app`, `svgl.app`, `cdn.jsdelivr.net` | Brand logos from svgl and Simple Icons matching a name, with their SVGs ready to copy |
| [Get Favicon](favicon) | Rust | `get favicon <domain>` | `t1.gstatic.com`, `icons.duckduckgo.com` | A site's icon as a PNG or ICO, ready to copy |

Return opens a story or package; Command-Return opens its discussion, homepage or repository in your browser.

In Volant 0.3.7 or later, Return on an extension enters it: the search field clears and
belongs to the extension, with a Back chip at its start, and what you type runs it again once you pause.
Escape clears the text, then goes back. GitHub (Volant 0.3.9 or later for its dropdown) asks once for a
fine-grained personal access token with read access to pull requests and issues, kept in your Keychain
and added only to requests to api.github.com; its dropdown picks Review Requests, My Pull Requests,
Assigned to Me or Involving Me, and typed text narrows the list with GitHub's search syntax, such as
`repo:owner/name`. Homebrew looks up the exact formula or cask name you type, since Homebrew has no
search API; Return on its install row copies `brew install …`. PyPI likewise looks up the exact package
name, reading the package's releases feed first and then only its latest release, since a package's full
JSON can pass Volant's 512 KB limit; Return on its `uv add` or `pip install` row copies it, and Docker
Hub's `docker pull` row works the same way. Hacker News adds a dropdown at the end of the field (Top, New,
Ask HN, Show HN; Command-P picks the next one) and searches within the chosen page, and shows each
story's points, comments and site as badges. Wikipedia, npm, Homebrew, crates.io, PyPI, Docker Hub, MDN, Maps, Stack Overflow, SF Symbols, Svgl and Get Favicon declare `"mode": "search"`, so they wait for
you to type; the others show their list at once. Older Volant versions ignore `mode` and `placeholder`.

Service Status reads all four status pages at once and shows each service's overall status, with
badges for open incidents, maintenance under way and affected components, then a row per open incident
or maintenance; typed text finds a service, component or incident by name, and a page that fails shows
only that service as unavailable. Maps searches OpenStreetMap's Nominatim once per pause in typing, as
its usage policy asks; Return opens the place in Apple Maps and Command-Return on openstreetmap.org
(search data © OpenStreetMap contributors, ODbL). Public Holidays lists holidays from today on, into
next year once it is within 90 days; its preference picks the country (United States by default), and
a two-letter code typed first, such as `holidays de`, shows another. Dates count from today in UTC,
since modules have no time zone. Stack Overflow uses the Stack Exchange API without a key, which allows
300 searches a day from one network, and says when that runs out.

Meteo Weather and Todoist need Volant 0.3.4 or later. Meteo Weather asks once to share your approximate
location (about 11 km) with it and Open-Meteo; `meteo <place>` (or `meteo in <place>`) works without it.
In Volant 0.4.1 and later, the current conditions show as a card with an hourly strip and the week, and
Tab switches between them. Volant 0.4.1 also has its own `weather` command from Apple Weather; Meteo
Weather, which replaced the Weather extension, uses its own `meteo` trigger beside it. Weather data is by
[Open-Meteo.com](https://open-meteo.com) (CC BY 4.0). Todoist asks once for your Todoist API token,
which Volant keeps in your Keychain and adds only to requests to api.todoist.com; the extension never
sees it. Type `todo` and a task, such as `todo buy milk tomorrow p1`, and Return adds it with Todoist's
Quick Add; a task's Actions complete it or move it to tomorrow or next week, and Return opens it in
Todoist.app, or on the web without the app.

Word Count, Case Converter, Encode/Decode and Hash work offline and list no hosts. Each uses the text
you type, or, with the field empty, the clipboard's text, and each row says which. They ask once to read
your clipboard's text when you run them (Volant 0.3.24 or later); Volant reads it only as a run starts,
and the extensions never store it. Return on a row copies its result. Volant copies at most 300
characters from a row, so a longer result shows its length instead of a cut-off copy. Fancy Text works
the same way, showing the text in sixteen Unicode styles from the Mathematical Alphanumeric Symbols and
other blocks, with the letters Unicode keeps elsewhere (script B is ℬ, italic h is ℎ) in place.

Characters and SF Symbols also work offline. Characters finds a character by name, alias, HTML entity or
code point, and lists the common ones with the field empty; Return copies it. Volant trims spaces from
a row's title, so invisible characters such as the no-break space and zero-width joiner show and copy
their HTML entity instead. SF Symbols searches the names of Apple's SF Symbols, from a list
`sfsymbols/generate.py` writes from the symbol metadata macOS ships (names, categories, keywords and
releases only, never the glyphs); the committed list comes from macOS 27. Return copies a name, and an
exact name adds its SwiftUI, UIKit and AppKit code. Volant draws only its own row icons, so rows show
names, not the symbols themselves.

Apple HIG reads the Human Interface Guidelines' own data from developer.apple.com on every run. Its
dropdown searches All, which covers Foundations, Patterns, Components, Inputs and Technologies in three
rounds of fetches, or browses one section, Getting Started included. Google Fonts reads font data from
[Fontsource](https://fontsource.org) (MIT), since Google's Fonts API needs a key; its dropdown picks All
or one category, Return opens a font's specimen on Google Fonts (or its Fontsource page for fonts Google
does not host), Command-Return opens it on Fontsource, and the best match adds a row whose Return copies
`npm install @fontsource/<id>`.

Svgl and Get Favicon need Volant 0.3.35 or later, the release with row copying. Their rows carry
`"copy": {"url","type","title"}` and their manifests ask for the `clipboard-write` permission; Volant
fetches the copy URL, on a host the manifest declares, only when you choose Copy, checks the bytes are
the declared type, and puts the image on the clipboard. Earlier Volant versions refuse the unknown
permission.

Svgl searches [svgl](https://svgl.app)'s logo library (about 680 logos, mostly software) and the
[Simple Icons](https://simpleicons.org) set (about 3,500 brands, CC0) in the same round. svgl's rows
come first, copying a logo's SVG from svgl.app, with rows for its dark version and wordmarks; Return
opens the brand's website and Command-Return its brand guidelines. Simple Icons' rows follow, up to 20,
exact matches first, each with its brand color as a badge; they copy the single-color SVG from
cdn.jsdelivr.net (`simple-icons@16`), and Return opens it. Each search reads Simple Icons' whole index,
about 450 KB, which Svgl searches itself; if one source fails the other's rows still show. From 0.3.0 (Volant 0.3.37 or
later) each row also shows the logo it copies, once you allow "Show images from its sites in its rows";
Volant checks each SVG and draws it in its extension helper, which has no network access, and logos
that use filters or masks keep the extension's icon.

Get Favicon turns a domain or URL into its host and copies the site's icon from Google's and
DuckDuckGo's favicon services. Rows for 32, 64, 128 and 256 pixels copy a PNG from Google, which sends
the largest icon it has up to that size and never enlarges one, so a site with only a 32-pixel icon
gets 32 pixels in every row; the last row copies DuckDuckGo's icon, which Volant identifies itself
since DuckDuckGo answers with an ICO or a PNG. From 0.3.0 (Volant 0.3.36 or later) each row also shows
the icon it copies, once you allow "Show images from its sites in its rows"; Volant's sandboxed fetch
helper downloads and shrinks it. A site neither service knows gets a placeholder or an empty reply that
Volant refuses to copy. A site's own SVG
or touch icon is out of reach, because Volant lets an extension read only the fixed hosts its manifest
lists. Return opens the icon and Command-Return the site.

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

## Preferences

A list extension can declare settings the owner chooses in Volant's Settings › Extensions ›
**Preferences…** (Volant 0.3.22 and later): up to 12 of `toggle`, `text`, `dropdown` and `number`.

```json
"preferences": [
  {"id": "limit", "type": "number", "title": "Stories to show", "default": 30, "min": 10, "max": 50},
  {"id": "showBadges", "type": "toggle", "title": "Show points and comments", "default": true}
]
```

Every list request then carries `"preferences"` with each declared id: the owner's choice, or the
default until they pick one. Volant checks values against the declaration (type, range, choices,
length) before sending them. Older Volant versions send no `preferences`, so always fall back to the
same defaults the manifest declares; then an extension behaves the same everywhere until the owner
changes something.

In Rust, read them with `kit::Preferences`, kept apart from `Request` so extensions without
preferences compile unchanged:

```rust
let preferences = volant_extension_kit::Preferences::from_input(input);
let limit = preferences.number("limit", 30.0) as usize;
let show_badges = preferences.bool("showBadges", true);
```

In TypeScript, read `request.preferences?.limit ?? 30`. Hacker News is the worked example: story count,
a minimum score and whether rows show badges.

Preferences are the owner's settings, not access. They can't add hosts, and they must never hold a
password or token: declare a `secret` for that, which Volant keeps in the Keychain and never passes to
the module. Values are stored in Volant's config.json, included in Volant backups, never synced through
iCloud, and removed when the extension is uninstalled. Declaring or changing preferences doesn't switch
an enabled extension off on update. The full rules are in Volant's
[docs/extension-preferences.md](https://github.com/mysticcoders/volant/blob/main/docs/extension-preferences.md).

## Start a new extension

```sh
tools/new-extension.py rust wordcount --name "Word Count" --id com.example.wordcount
tools/new-extension.py typescript wordcount --name "Word Count"
tools/new-extension.py python wordcount --name "Word Count"
```

Each creates a folder holding the same small list extension, which shows the typed text with its
character and word counts, plus a test and a manifest that `./build.sh` completes with the module's
hash. Rust uses the `kit` crate (ABI 1) and joins the Cargo workspace. TypeScript (ABI 2, through
Javy) and Python (ABI 2, through MicroPython) take their pinned build scripts from `npm/` and
`wikipedia/`, renamed for the new folder. The templates live in [`templates/`](templates); the
comments in each say where to go next, such as declaring `hosts` and returning URLs from `plan`.

Run `./build.sh`, then copy the folder into Volant's Extensions folder to try it. The new folder has
no `listing.json`, so it stays out of the published catalog until you add one. `tools/new-extension.py
--self-test`, which CI runs, starts one extension per language in a scratch copy and runs its tests.

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
Javy's plugin also links Rust crates and wasi-libc, listed in [`JAVY-CRATES-LICENSES`](npm/JAVY-CRATES-LICENSES)
and [`WASI-LIBC-LICENSE`](npm/WASI-LIBC-LICENSE).
`npm/build.sh` downloads Javy and checks its SHA-256 first.
It needs Node 24. Javy does not produce identical bytes on every build, so CI checks only the
committed module and its hash, not a rebuild.

The Python extension runs on [MicroPython](https://micropython.org) 1.29.0, compiled with
[wasi-sdk](https://github.com/WebAssembly/wasi-sdk) 34; `wikipedia/build.sh` downloads both and checks
their SHA-256 first. The module statically links wasi-libc, so `wikipedia/` carries
[`MICROPYTHON-LICENSE`](wikipedia/MICROPYTHON-LICENSE) and [`WASI-LIBC-LICENSE`](wikipedia/WASI-LIBC-LICENSE). Its tests run under plain CPython, so `--check` needs only `python3`. See
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

To withdraw an extension, add it to `revoked.json` at the repository root:

```json
[{ "id": "community.volant.hn", "sha256": "<module sha256>", "reason": "One sentence the owner sees." }]
```

Leave out `sha256` to revoke every version of the ID. The catalog then publishes the list and stops
listing matching versions. Volant versions that read it turn off installed copies the next time their
owner checks the catalog, show the reason, and refuse to enable or reinstall them; updating past the
revoked module clears it.

Merging to `main` publishes, so review an extension's source, hosts and license before merging it.
To add one, give its folder a `listing.json`:

```json
{ "summary": "One sentence.", "publisher": "Your name", "license": "MIT", "notes": "Optional detail." }
```

## Icons and trademarks

An extension can show an icon in the launcher and Settings: a square WebP of at most 32 KB beside its
manifest, named by `icon` and pinned by `iconSha256` (`./build.sh` writes the hash). GitHub, Hacker News,
Homebrew, Lobsters, Maps (OpenStreetMap's), MDN, Stack Overflow, Todoist and Wikipedia use their sites' own
touch icons; npm, PyPI and Docker Hub use their logos from the npm, PyPI and Docker organizations' GitHub
avatars; crates.io uses the Cargo logo from the crates.io repository; each is converted to a 128-pixel
WebP. Apple HIG uses developer.apple.com's touch icon and Google Fonts the touch icon fonts.google.com
links, each converted the same way. Svgl uses the app icon from svgl.app's web manifest.
Meteo Weather, Service Status, Public Holidays, Characters, Fancy Text, SF Symbols and Get Favicon use original
art (`meteo/icon.svg`, `status/icon.svg`, `holidays/icon.svg`, `characters/icon.svg`,
`fancytext/icon.svg`, `sfsymbols/icon.svg`, `favicon/icon.svg`); SF Symbols does not use Apple's app icon. Hacker News and the Y Combinator logo are trademarks of Y Combinator; the
Lobsters name and logo belong to Lobsters; Todoist is a trademark of Doist; GitHub and the Invertocat logo are trademarks of GitHub, Inc.; the
Homebrew name and logo belong to the Homebrew project; npm is a trademark of
npm, Inc., part of GitHub; Wikipedia and its logo are trademarks of the Wikimedia Foundation; Rust,
Cargo and crates.io and their logos are trademarks of the Rust Foundation; Python, PyPI and their logos
are trademarks of the Python Software Foundation; Docker and the Docker logo are trademarks of Docker,
Inc.; MDN, Mozilla and the MDN logo are trademarks of the Mozilla Foundation; OpenStreetMap and its logo
are trademarks of the OpenStreetMap Foundation; Stack Overflow and its logo are trademarks of Stack
Exchange, Inc.; Claude and Anthropic are trademarks of Anthropic, PBC; OpenAI is a trademark of OpenAI;
Cloudflare is a trademark of Cloudflare, Inc.; Apple, the Apple logo, Apple Maps, SF Symbols and Human
Interface Guidelines are trademarks of Apple Inc.; Google Fonts is a trademark of Google LLC; Fontsource
belongs to the Fontsource project; the svgl name and logo belong to the svgl project, and each logo in its
library to its owner; Simple Icons' icons are CC0, and each brand mark remains its owner's trademark;
DuckDuckGo is a trademark of Duck Duck Go, Inc.; Google is a trademark of Google LLC. These extensions
are unofficial and not endorsed by any of them, and the MIT license below covers the code, not these marks.

## Dependency audit

CI runs cargo-deny (advisories, licenses, duplicate crates and sources, per [`deny.toml`](deny.toml)),
`npm audit` on `npm/package-lock.json`, and `tools/check-licenses.py`, on every pull request, every push to
`main` and weekly. [`docs/dependency-audit.md`](docs/dependency-audit.md) records the latest results, what
each notice covers and how to regenerate them.

## License

MIT. See [LICENSE](LICENSE). The Rust modules compile in crates listed with their licenses in
[`RUST-LICENSES`](RUST-LICENSES); the TypeScript and Python extensions carry their runtimes' notices in their
folders.
