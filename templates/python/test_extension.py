"""Tests for __FOLDER__.py. Plain asserts and a small runner, so the file needs only the standard
library: `python3 __FOLDER__/tests/test___FOLDER__.py`."""
import json
import os
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, os.path.join(HERE, "..", "src"))

import __FOLDER__  # noqa: E402


def rows(request):
    """Runs a request dictionary through respond and returns the reply's rows."""
    return json.loads(__FOLDER__.respond(json.dumps(request)))["items"]


def test_counts_the_typed_text():
    first = rows({"phase": "render", "query": " hello wide world ", "now": 0})[0]
    assert first["title"] == "hello wide world"
    assert first["subtitle"] == "16 characters · 3 words"


def test_asks_for_text_when_nothing_is_typed():
    assert rows({"phase": "render", "query": "", "now": 0})[0]["title"] == __FOLDER__.PROMPT


if __name__ == "__main__":
    for name, function in sorted(globals().items()):
        if name.startswith("test_"):
            function()
    print("__FOLDER__: tests passed")
