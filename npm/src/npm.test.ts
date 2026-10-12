import { test } from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { handle, manifestPermits, domain, age } from "./kit.ts";
import { handler, urlFor, count, browsableRepository } from "./npm.ts";

const manifest = JSON.parse(readFileSync(new URL("../manifest.json", import.meta.url), "utf8"));

/** Runs the handler on a request object the way Volant would, and parses the reply. */
function run(request: object) {
  return JSON.parse(handle(JSON.stringify(request), handler));
}

test("queries become one registry search", () => {
  assert.equal(urlFor("date picker"), "https://registry.npmjs.org/-/v1/search?text=date%20picker&size=15");
  assert.equal(urlFor("@scope/x & y"), "https://registry.npmjs.org/-/v1/search?text=%40scope%2Fx%20%26%20y&size=15");
  assert.equal(urlFor("   "), undefined);
});

test("every planned URL is on a manifest host", () => {
  for (const query of ["react", "date picker", "@types/node", "a#b?c"]) {
    const url = urlFor(query)!;
    assert.ok(manifestPermits(manifest, url), url);
  }
  assert.ok(!manifestPermits(manifest, "https://www.npmjs.com/package/react"));
});

test("an empty query explains itself instead of fetching", () => {
  assert.deepEqual(run({ phase: "plan", query: " ", now: 0 }), { items: [{ title: "Type npm and a package name or keyword, such as npm date picker." }] });
  assert.deepEqual(run({ phase: "plan", query: "react", now: 0 }), { fetch: ["https://registry.npmjs.org/-/v1/search?text=react&size=15"] });
});

test("render builds rows with package and homepage or repository links", () => {
  const body = JSON.stringify({ objects: [
    { downloads: { weekly: 1520 }, package: { name: "fictional-picker", version: "2.1.0", description: "Picks fictional dates.", date: "1970-01-01T01:00:00.000Z",
      links: { homepage: "https://example.com/picker", repository: "git+https://example.com/picker.git" } } },
    { package: { name: "@fictional/scoped", version: "0.0.1", links: { repository: "git+https://example.org/scoped.git" } } },
    { package: { name: "../../evil", version: "1.0.0" } },
    { package: { name: "no-links", links: { homepage: "javascript:alert(1)", repository: "git+ssh://git@example.org/x.git" } } },
  ] });
  const reply = run({ phase: "render", query: "picker", now: 7200, responses: [{ url: "u", status: 200, body }] });
  assert.equal(reply.items.length, 3);
  assert.deepEqual(reply.items[0], { title: "fictional-picker", subtitle: "2.1.0 · Picks fictional dates. · 1.5K weekly downloads · 1h ago",
    url: "https://www.npmjs.com/package/fictional-picker", secondaryURL: "https://example.com/picker", secondaryTitle: "Open Homepage" });
  assert.equal(reply.items[1].url, "https://www.npmjs.com/package/@fictional/scoped");
  assert.equal(reply.items[1].secondaryURL, "https://example.org/scoped");
  assert.equal(reply.items[1].secondaryTitle, "Open Repository");
  assert.equal(reply.items[2].secondaryURL, undefined);
});

test("failures and empty results become one explaining row", () => {
  assert.deepEqual(run({ phase: "render", query: "x", now: 0, responses: [{ url: "u", status: 0, body: "" }] }).items, [{ title: "The npm registry is unavailable (HTTP 0). Try again." }]);
  assert.deepEqual(run({ phase: "render", query: "x", now: 0, responses: [{ url: "u", status: 200, body: "<html>" }] }).items, [{ title: "The npm registry sent a page Volant could not read." }]);
  assert.deepEqual(run({ phase: "render", query: "zzz", now: 0, responses: [{ url: "u", status: 200, body: "{\"objects\":[]}" }] }).items, [{ title: "No npm packages match zzz." }]);
  assert.match(handle("not json", handler), /does not understand/);
});

test("helpers format counts, repositories, domains and ages", () => {
  assert.equal(count(188_481_362), "188.5M");
  assert.equal(count(2_000_000_000), "2B");
  assert.equal(count(999), "999");
  assert.equal(browsableRepository("git+https://github.com/a/b.git"), "https://github.com/a/b");
  assert.equal(browsableRepository("git://github.com/a/b.git"), undefined);
  assert.equal(domain("https://www.Example.com:8080/a"), "example.com");
  assert.equal(age(100_000, 100_000 - 7_200), "2h ago");
  assert.equal(age(1_000_000_000, 1_000_000_000 - 1_942 * 86_400), "5y ago");
});
