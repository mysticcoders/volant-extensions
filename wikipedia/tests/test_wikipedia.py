"""Tests for wikipedia.py with fictional data. Plain asserts and a small runner, so the file needs only
the standard library: `python3 wikipedia/tests/test_wikipedia.py`."""
import json
import os
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, os.path.join(HERE, "..", "src"))

import wikipedia  # noqa: E402

MANIFEST = os.path.join(HERE, "..", "manifest.json")
HTTPS = "https://"


def reply(request):
    """Runs a request dictionary through respond and decodes the reply."""
    return json.loads(wikipedia.respond(json.dumps(request)))


def render(body, status=200, query="lovelace"):
    """The render reply for one fetched response."""
    return reply({"phase": "render", "query": query, "now": 0,
                  "responses": [{"url": "u", "status": status, "body": body}]})


def manifest_permits(url):
    """Whether Volant would fetch url for this manifest: https on the default port, no credentials or
    fragment, and a declared host."""
    with open(MANIFEST) as handle:
        hosts = json.load(handle)["hosts"]
    if not url.startswith(HTTPS) or "#" in url:
        return False
    authority = url[len(HTTPS):].split("/")[0].split("?")[0]
    return "@" not in authority and ":" not in authority and authority in hosts


def test_queries_become_one_encoded_search():
    plan = reply({"phase": "plan", "query": "  Ada Lovelace & C++ ", "now": 0})
    assert plan == {"fetch": ["https://en.wikipedia.org/w/rest.php/v1/search/page?q=Ada%20Lovelace%20%26%20C%2B%2B&limit=10"]}, plan
    assert wikipedia.url_for("Zürich") == "https://en.wikipedia.org/w/rest.php/v1/search/page?q=Z%C3%BCrich&limit=10"


def test_every_planned_url_is_on_a_manifest_host():
    for query in ["a", "ada lovelace", "C++ & you?", "Zürich", "#fragment", "user@host:8080"]:
        url = wikipedia.url_for(query)
        assert manifest_permits(url), url


def test_an_empty_query_explains_itself_instead_of_fetching():
    assert reply({"phase": "plan", "query": "   ", "now": 0}) == {"items": [{"title": wikipedia.USAGE}]}


def test_render_builds_rows_with_plain_subtitles():
    body = json.dumps({"pages": [
        {"id": 1, "key": "Fictional_Engine_(machine)", "title": "Fictional Engine (machine)",
         "excerpt": "The <span class=\"searchmatch\">Fictional</span> Engine &amp; its &quot;cards&quot;&#39; &#x263A;",
         "description": "Imagined computing machine"},
        {"id": 2, "key": "Café", "title": "Café", "excerpt": "", "description": None},
        {"id": 3, "key": "", "title": "No key"},
        {"id": 4, "key": "Blank", "title": "  "}]})
    items = render(body)["items"]
    assert len(items) == 2, items
    assert items[0] == {
        "title": "Fictional Engine (machine)",
        "subtitle": "Imagined computing machine · The Fictional Engine & its \"cards\"' ☺",
        "url": "https://en.wikipedia.org/wiki/Fictional_Engine_(machine)",
    }, items[0]
    assert items[1] == {"title": "Café", "url": "https://en.wikipedia.org/wiki/Caf%C3%A9"}, items[1]


def test_unknown_entities_and_unclosed_tags_stay_readable():
    assert wikipedia.plain_text("a &bogus; b &#zz; c") == "a &bogus; b &#zz; c"
    assert wikipedia.plain_text("tail <span") == "tail"


def test_failures_become_a_single_explaining_row():
    assert render("", status=0) == {"items": [{"title": "Wikipedia search is unavailable (HTTP 0). Try again."}]}
    assert render("not json") == {"items": [{"title": wikipedia.UNREADABLE}]}
    assert render(json.dumps({"pages": []}), query=" nothing here ") == {"items": [{"title": "No Wikipedia articles match nothing here."}]}
    assert reply({"phase": "render", "query": "x", "now": 0}) == {"items": [{"title": wikipedia.NOTHING}]}
    failed = json.loads(wikipedia.respond("not a request"))["items"]
    assert len(failed) == 1 and failed[0]["title"].startswith("The Wikipedia extension could not finish"), failed


def main():
    """Runs every test_ function and exits nonzero if any fail."""
    tests = [(name, value) for name, value in sorted(globals().items()) if name.startswith("test_")]
    failures = 0
    for name, test in tests:
        try:
            test()
        except Exception as error:
            failures += 1
            print("FAIL", name, repr(error))
    print("{} passed, {} failed".format(len(tests) - failures, failures))
    sys.exit(1 if failures else 0)


if __name__ == "__main__":
    main()
