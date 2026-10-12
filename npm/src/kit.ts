/**
 * Types and helpers for Volant list extensions written in TypeScript, the counterpart of the Rust
 * `kit/` crate. Volant runs the module twice: `plan` returns URLs to fetch, then `render` receives the
 * responses and returns rows. The module never touches the network itself.
 */

export const PLAN = "plan";
export const HTTPS = "https://";
export const HTTP = "http://";
export const SEPARATOR = " · ";
const WWW = "www.";
const JUST_NOW = "just now";
const MINUTE = 60;
const HOUR = 60 * MINUTE;
const DAY = 24 * HOUR;
const YEAR = 365 * DAY;
const AGE_UNITS: [number, string][] = [[YEAR, "y"], [DAY, "d"], [HOUR, "h"], [MINUTE, "m"]];

export interface Response {
  url: string;
  status: number;
  body: string;
}

export interface Request {
  phase: string;
  query: string;
  now: number;
  responses?: Response[];
}

export interface Item {
  title: string;
  subtitle?: string;
  url?: string;
  secondaryURL?: string;
  secondaryTitle?: string;
}

export type Reply = { fetch: string[] } | { items: Item[] };

export type Handler = (request: Request) => Reply;

/** A reply with one row whose title explains why there is nothing else to show. */
export function explain(message: string): Reply {
  return { items: [{ title: message }] };
}

/**
 * Decodes the request, calls the handler and encodes its reply. A malformed request or a thrown
 * error becomes a single explaining row, so the launcher always has something to show.
 */
export function handle(input: string, handler: Handler): string {
  let request: Request;
  try {
    request = JSON.parse(input) as Request;
  } catch (error) {
    return JSON.stringify(explain(`Volant sent a request this extension does not understand: ${String(error)}`));
  }
  try {
    return JSON.stringify(handler(request));
  } catch (error) {
    return JSON.stringify(explain(error instanceof Error ? error.message : String(error)));
  }
}

/** "5m ago", "3h ago", "2d ago", "4y ago" between two Unix times in seconds. */
export function age(now: number, then: number): string {
  const seconds = Math.max(0, Math.floor(now - then));
  const unit = AGE_UNITS.find(([size]) => seconds >= size);
  return unit ? `${Math.floor(seconds / unit[0])}${unit[1]} ago` : JUST_NOW;
}

/** Whether a link is http or https, the only kinds Volant lets a row open. */
export function isWebLink(url: unknown): url is string {
  return typeof url === "string" && (url.startsWith(HTTPS) || url.startsWith(HTTP));
}

/** The host of an http or https URL without a leading "www.", for showing where a link goes. */
export function domain(url: string): string | undefined {
  if (!isWebLink(url)) return undefined;
  const rest = url.slice(url.startsWith(HTTPS) ? HTTPS.length : HTTP.length);
  const host = rest.split(/[/?#]/)[0].split("@").pop()?.split(":")[0].toLowerCase();
  if (!host) return undefined;
  return host.startsWith(WWW) ? host.slice(WWW.length) : host;
}

/**
 * Whether Volant would fetch a planned URL for this manifest: https on the default port, no
 * credentials or fragment, and a host the manifest's `hosts` declares. Tests call it so a URL in the
 * source cannot drift from the hosts the owner approves.
 */
export function manifestPermits(manifest: { hosts?: string[] }, url: string): boolean {
  if (!url.startsWith(HTTPS) || url.includes("#")) return false;
  const authority = url.slice(HTTPS.length).split(/[/?#]/)[0];
  if (!authority || authority.includes("@") || authority.includes(":")) return false;
  return (manifest.hosts ?? []).includes(authority);
}
