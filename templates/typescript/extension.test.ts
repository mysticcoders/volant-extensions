import { test } from "node:test";
import assert from "node:assert/strict";
import { handle } from "./kit.ts";
import { handler, PROMPT } from "./__FOLDER__.ts";

/** Runs the handler on a request object the way Volant would, and parses the reply. */
function run(request: object) {
  return JSON.parse(handle(JSON.stringify(request), handler));
}

test("counts the typed text", () => {
  const reply = run({ phase: "render", query: " hello wide world ", now: 0 });
  assert.equal(reply.items[0].title, "hello wide world");
  assert.equal(reply.items[0].subtitle, "16 characters · 3 words");
});

test("asks for text when nothing is typed", () => {
  assert.equal(run({ phase: "render", query: "", now: 0 }).items[0].title, PROMPT);
});
