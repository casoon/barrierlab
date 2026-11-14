#!/usr/bin/env python3
"""Download the W3C ACT Rules test cases into a local cache (not committed).

Usage: python3 xtask/act/fetch.py [cache-dir]   (default: target/act)

Writes <cache>/testcases.json and every test case page plus the stylesheets it
links to under <cache>/files/<host>/<path>. The test cases are published by the
ACT Rules Community Group (https://act-rules.github.io/pages/license/); they are
fetched for a run, never vendored into this repository.
"""

import json
import re
import sys
import urllib.parse
import urllib.request
from pathlib import Path

INDEX = "https://act-rules.github.io/testcases.json"
LINK = re.compile(r"<link\b[^>]*>", re.I)
ATTR = re.compile(r"""(\w[\w-]*)\s*=\s*("[^"]*"|'[^']*'|[^\s>]+)""")


def local(cache: Path, url: str) -> Path:
    parts = urllib.parse.urlsplit(url)
    return cache / "files" / parts.netloc / parts.path.lstrip("/")


def get(url: str) -> bytes:
    req = urllib.request.Request(url, headers={"User-Agent": "barrierlab-act-runner"})
    with urllib.request.urlopen(req, timeout=30) as r:
        return r.read()


def save(cache: Path, url: str) -> bytes | None:
    path = local(cache, url)
    if path.exists():
        return path.read_bytes()
    try:
        body = get(url)
    except Exception as e:  # noqa: BLE001 — a missing asset is noted, not fatal
        print(f"  missing {url}: {e}", file=sys.stderr)
        return None
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_bytes(body)
    return body


def stylesheets(html: str, base: str):
    for tag in LINK.findall(html):
        attrs = {k.lower(): v.strip("\"'") for k, v in ATTR.findall(tag)}
        rel = attrs.get("rel", "").lower().split()
        if "stylesheet" in rel and attrs.get("href"):
            yield urllib.parse.urljoin(base, attrs["href"])


def main() -> None:
    cache = Path(sys.argv[1] if len(sys.argv) > 1 else "target/act")
    cache.mkdir(parents=True, exist_ok=True)
    index = get(INDEX)
    (cache / "testcases.json").write_bytes(index)
    cases = json.loads(index)["testcases"]
    for i, case in enumerate(cases, 1):
        url = case["url"]
        body = save(cache, url)
        if body is None or not url.endswith(".html"):
            continue
        for sheet in stylesheets(body.decode("utf-8", "replace"), url):
            save(cache, sheet)
        if i % 100 == 0:
            print(f"{i}/{len(cases)}", file=sys.stderr)
    print(f"{len(cases)} test cases in {cache}", file=sys.stderr)


if __name__ == "__main__":
    main()
