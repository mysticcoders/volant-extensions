"""__NAME__ for Volant, started from `tools/new-extension.py`. It shows the text you type with its
character and word counts, without reading any website.

The module runs on MicroPython: keep to the standard `json` module and plain Python. To read a
website, list its host in manifest.json's `hosts`. Volant then calls `respond` twice: with phase
`plan`, where you return {"fetch": [urls]}, and with `render`, whose request carries the responses.
See Volant's network extensions docs."""
import json

PROMPT = "Type some text to count it."
SEPARATOR = " · "
FAILED = "__NAME__ could not finish: {}"


def row(title, subtitle=None):
    """One launcher row; Volant ignores fields that are absent."""
    entry = {"title": title}
    if subtitle:
        entry["subtitle"] = subtitle
    return entry


def render(query):
    """One row for the typed text with its counts, or a prompt when nothing is typed."""
    text = query.strip()
    if not text:
        return {"items": [row(PROMPT)]}
    counts = ["{} characters".format(len(text)), "{} words".format(len(text.split()))]
    return {"items": [row(text, SEPARATOR.join(counts))]}


def respond(text):
    """Volant's request JSON in, the reply JSON out; any failure becomes one explaining row."""
    try:
        reply = render(json.loads(text).get("query") or "")
    except Exception as error:
        reply = {"items": [row(FAILED.format(error))]}
    return json.dumps(reply)
