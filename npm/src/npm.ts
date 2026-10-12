/**
 * npm for Volant: searches the public npm registry. Return opens the package on npmjs.com;
 * Command-Return opens its homepage, or its repository when it has no homepage.
 */
import { age, explain, isWebLink, PLAN, SEPARATOR, HTTPS } from "./kit.ts";
import type { Item, Reply, Request } from "./kit.ts";

export const SEARCH = "https://registry.npmjs.org/-/v1/search?text=";
export const SIZE = "&size=15";
export const PACKAGE_PAGE = "https://www.npmjs.com/package/";
export const USAGE = "Type npm and a package name or keyword, such as npm date picker.";
export const NOTHING = "The npm registry returned nothing.";
export const UNREADABLE = "The npm registry sent a page Volant could not read.";
export const HOMEPAGE_TITLE = "Open Homepage";
export const REPOSITORY_TITLE = "Open Repository";
const GIT_PREFIX = "git+";
const GIT_SUFFIX = ".git";
const PACKAGE_NAME = /^(@[a-z0-9-~][a-z0-9-._~]*\/)?[a-z0-9-~][a-z0-9-._~]*$/;
const COUNT_UNITS: [number, string][] = [[1e9, "B"], [1e6, "M"], [1e3, "K"]];

interface SearchPage {
  objects?: SearchObject[];
}

interface SearchObject {
  downloads?: { weekly?: number };
  updated?: string;
  package?: {
    name?: string;
    version?: string;
    description?: string;
    date?: string;
    links?: { homepage?: string; repository?: string };
  };
}

/** The registry search URL for a query, or undefined when there is nothing to search for. */
export function urlFor(query: string): string | undefined {
  const text = query.trim();
  return text ? `${SEARCH}${encodeURIComponent(text)}${SIZE}` : undefined;
}

/** "No npm packages match …" for a query with no results. */
function noMatches(query: string): string {
  return `No npm packages match ${query.trim()}.`;
}

/** "The npm registry is unavailable (HTTP n). Try again." for a failed fetch. */
function unavailable(status: number): string {
  return `The npm registry is unavailable (HTTP ${status}). Try again.`;
}

/** 188481362 as "188.5M", 1520 as "1.5K". */
export function count(value: number): string {
  for (const [size, unit] of COUNT_UNITS) {
    if (value >= size) return `${(value / size).toFixed(1).replace(/\.0$/, "")}${unit}`;
  }
  return String(value);
}

/** A repository link as a browsable https URL: "git+https://…/x.git" becomes "https://…/x". */
export function browsableRepository(url: string | undefined): string | undefined {
  if (!url) return undefined;
  let link = url.startsWith(GIT_PREFIX) ? url.slice(GIT_PREFIX.length) : url;
  if (link.endsWith(GIT_SUFFIX)) link = link.slice(0, -GIT_SUFFIX.length);
  return link.startsWith(HTTPS) ? link : undefined;
}

/** One row for a search result; results without a valid package name are skipped. */
function item(result: SearchObject, now: number): Item | undefined {
  const pkg = result.package;
  const name = pkg?.name;
  if (!name || !PACKAGE_NAME.test(name)) return undefined;
  const details: string[] = [];
  if (pkg.version) details.push(pkg.version);
  if (pkg.description?.trim()) details.push(pkg.description.trim());
  if (typeof result.downloads?.weekly === "number") details.push(`${count(result.downloads.weekly)} weekly downloads`);
  const published = Date.parse(pkg.date ?? result.updated ?? "");
  if (!Number.isNaN(published)) details.push(age(now, published / 1000));
  const homepage = isWebLink(pkg.links?.homepage) ? pkg.links.homepage : undefined;
  const repository = browsableRepository(pkg.links?.repository);
  const row: Item = { title: name, subtitle: details.join(SEPARATOR), url: `${PACKAGE_PAGE}${name}` };
  if (homepage) {
    row.secondaryURL = homepage;
    row.secondaryTitle = HOMEPAGE_TITLE;
  } else if (repository) {
    row.secondaryURL = repository;
    row.secondaryTitle = REPOSITORY_TITLE;
  }
  return row;
}

/** Plans one registry search, or renders its results as rows. */
export function handler(request: Request): Reply {
  if (request.phase === PLAN) {
    const url = urlFor(request.query);
    return url ? { fetch: [url] } : explain(USAGE);
  }
  const response = request.responses?.[0];
  if (!response) return explain(NOTHING);
  if (response.status !== 200) return explain(unavailable(response.status));
  let page: SearchPage;
  try {
    page = JSON.parse(response.body) as SearchPage;
  } catch {
    return explain(UNREADABLE);
  }
  const items = (page.objects ?? []).map((result) => item(result, request.now)).filter((row) => row !== undefined);
  return items.length ? { items } : explain(noMatches(request.query));
}
