"""Wikipedia for Volant: English Wikipedia article search, read through the public REST search API on
en.wikipedia.org. Return opens the article in the browser.

Volant runs this twice. The plan phase turns the query into one search URL; the render phase turns the
fetched search results into rows. The same source runs under CPython for tests and under MicroPython,
compiled into the WebAssembly module, inside Volant.
"""
import json

SITE = "https://en.wikipedia.org"
SEARCH = SITE + "/w/rest.php/v1/search/page?q="
SEARCH_LIMIT = "&limit=10"
ARTICLE = SITE + "/wiki/"

PLAN = "plan"
OK = 200

USAGE = "Type wiki and a topic, such as wiki ada lovelace."
NOTHING = "Wikipedia returned nothing."
UNAVAILABLE = "Wikipedia search is unavailable (HTTP {}). Try again."
UNREADABLE = "Wikipedia sent a page Volant could not read."
NO_MATCHES = "No Wikipedia articles match {}."
FAILED = "The Wikipedia extension could not finish: {}"

SEPARATOR = " · "
EMPTY = ""
SPACE = " "
TAG_OPEN = "<"
TAG_CLOSE = ">"
ENTITY_OPEN = "&"
ENTITY_CLOSE = ";"
NUMERIC_ENTITY = "#"
HEX_ENTITY = "x"
MAX_ENTITY_LENGTH = 10
HEX_BASE = 16
DECIMAL_BASE = 10
PERCENT = "%{:02X}"
ENTITIES = {"amp": "&", "lt": "<", "gt": ">", "quot": '"', "apos": "'", "nbsp": " "}

UNRESERVED = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_.~"
PATH_SAFE = UNRESERVED + b"()_,:;!*'@$/"

PHASE = "phase"
QUERY = "query"
RESPONSES = "responses"
STATUS = "status"
BODY = "body"
PAGES = "pages"
KEY = "key"
PAGE_TITLE = "title"
DESCRIPTION = "description"
EXCERPT = "excerpt"
FETCH = "fetch"
ITEMS = "items"
ROW_TITLE = "title"
ROW_SUBTITLE = "subtitle"
ROW_URL = "url"


def percent_encode(text, safe):
    """Percent-encodes the UTF-8 bytes of text, passing through the bytes in safe."""
    return EMPTY.join(chr(byte) if byte in safe else PERCENT.format(byte) for byte in text.encode())


def url_for(query):
    """The search URL for a query, or None when there is nothing to search for."""
    query = query.strip()
    if not query:
        return None
    return SEARCH + percent_encode(query, UNRESERVED) + SEARCH_LIMIT


def article_url(key):
    """The article link for a page key such as "Ada_Lovelace"."""
    return ARTICLE + percent_encode(key, PATH_SAFE)


def decode_entity(name):
    """The text for an HTML entity name such as "amp" or "#39", or None if it is not one."""
    if name.startswith(NUMERIC_ENTITY):
        digits = name[1:]
        base = DECIMAL_BASE
        if digits.lower().startswith(HEX_ENTITY):
            digits = digits[1:]
            base = HEX_BASE
        try:
            return chr(int(digits, base))
        except (ValueError, OverflowError):
            return None
    return ENTITIES.get(name)


def plain_text(html):
    """Search-excerpt HTML as plain text: tags removed, entities decoded and whitespace collapsed."""
    parts = []
    position = 0
    while position < len(html):
        if html[position] == TAG_OPEN:
            end = html.find(TAG_CLOSE, position)
            if end < 0:
                break
            position = end + 1
            continue
        if html[position] == ENTITY_OPEN:
            end = html.find(ENTITY_CLOSE, position, position + MAX_ENTITY_LENGTH)
            if end > 0:
                text = decode_entity(html[position + 1:end])
                if text is not None:
                    parts.append(text)
                    position = end + 1
                    continue
        parts.append(html[position])
        position += 1
    return SPACE.join(EMPTY.join(parts).split())


def row(title, subtitle=None, url=None):
    """One launcher row, leaving out the fields it does not have."""
    item = {ROW_TITLE: title}
    if subtitle:
        item[ROW_SUBTITLE] = subtitle
    if url:
        item[ROW_URL] = url
    return item


def item(page):
    """The row for one search result, or None when it has no title or key."""
    title = page.get(PAGE_TITLE)
    key = page.get(KEY)
    if not isinstance(title, str) or not title.strip() or not isinstance(key, str) or not key:
        return None
    details = []
    for value in (page.get(DESCRIPTION), page.get(EXCERPT)):
        if isinstance(value, str):
            text = plain_text(value)
            if text:
                details.append(text)
    return row(title, SEPARATOR.join(details), article_url(key))


def plan(query):
    """The plan reply: the search URL to fetch, or a row explaining an empty query."""
    url = url_for(query)
    if url is None:
        return {ITEMS: [row(USAGE)]}
    return {FETCH: [url]}


def render(query, responses):
    """The render reply: one row per article, or one row explaining why there are none."""
    if not responses:
        return {ITEMS: [row(NOTHING)]}
    response = responses[0]
    status = response.get(STATUS)
    if status != OK:
        return {ITEMS: [row(UNAVAILABLE.format(status))]}
    try:
        pages = json.loads(response.get(BODY) or EMPTY).get(PAGES)
    except (ValueError, AttributeError):
        pages = None
    if not isinstance(pages, list):
        return {ITEMS: [row(UNREADABLE)]}
    items = [entry for entry in (item(page) for page in pages if isinstance(page, dict)) if entry]
    if not items:
        return {ITEMS: [row(NO_MATCHES.format(query.strip()))]}
    return {ITEMS: items}


def respond(text):
    """Volant's request JSON in, the reply JSON out; any failure becomes one explaining row."""
    try:
        request = json.loads(text)
        query = request.get(QUERY) or EMPTY
        if request.get(PHASE) == PLAN:
            reply = plan(query)
        else:
            reply = render(query, request.get(RESPONSES) or [])
    except Exception as error:
        reply = {ITEMS: [row(FAILED.format(error))]}
    return json.dumps(reply)
