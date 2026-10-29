/**
 * __NAME__ for Volant, started from `tools/new-extension.py`. It shows the text you type with its
 * character and word counts, without reading any website.
 *
 * To read a website, list its host in manifest.json's `hosts`. Volant then runs the module twice:
 * `plan`, where you return `{ fetch: [urls] }`, and `render`, which receives their responses in
 * `request.responses`. See `src/kit.ts` and Volant's network extensions docs.
 */
import { SEPARATOR, explain, type Reply, type Request } from "./kit.ts";

export const PROMPT = "Type some text to count it.";

/** One row for the typed text with its counts, or a prompt when nothing is typed. */
export function handler(request: Request): Reply {
  const text = request.query.trim();
  if (!text) return explain(PROMPT);
  const counts = [`${[...text].length} characters`, `${text.split(/\s+/).length} words`];
  return { items: [{ title: text, subtitle: counts.join(SEPARATOR) }] };
}
