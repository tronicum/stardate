#!/usr/bin/env python3
"""Generates a real "N-degrees of Wikipedia" demo: breadth-first crawl of a
starting page's real outbound links (English Wikipedia's MediaWiki API, no
key needed), up to a fixed depth, with a hard per-page fan-out cap and a
visited-page dedup set. Both are load-bearing, not just tidiness: Wikipedia's
link graph is enormously cyclic (A links to B links back to A within a
couple hops) and a real page can have hundreds of outbound links, so an
uncapped/undeduped crawl is both unbounded and would violate Graph's
tree-only model (a node can't have two parents here). Second half of the
German-cities-TSP + 5-degrees-of-Wikipedia meta idea; part 1 is
gen_german_tsp_demo.py.

This is the live-crawl tool used to (re)generate the committed snapshot at
scripts/wikipedia-crawl-data/frankfurt-depth3-fanout3.json — it is NOT run
by walkthrough.sh on every regeneration (see gen_wikipedia_demo.py, which
copies that snapshot instead). Wikipedia's API sustained-rate-limited even
a well-paced curl-based crawl during development, so re-running this live
on every walkthrough would be slow and flaky, especially in CI.
"""
import json
import os
import subprocess
import sys
import time
import urllib.parse

API = "https://en.wikipedia.org/w/api.php"
DEFAULT_MAX_DEPTH = 3
DEFAULT_FANOUT = 3
REQUEST_DELAY_SECONDS = 3.0
USER_AGENT = "spex-demo/1.0 (educational project, github.com/tronicum/stardate)"


def fetch_links(title, retries=8):
    """Shells out to curl rather than using Python's urllib: Wikipedia's API
    rate-limited/stalled urllib specifically (same pattern hit earlier this
    session with Yahoo Finance — likely a TLS-fingerprint or client-library
    based throttle), while curl reached it cleanly on the first try."""
    params = {
        "action": "query",
        "titles": title,
        "prop": "links",
        "pllimit": "max",
        "plnamespace": "0",  # real articles only, not Talk:/File:/Category:/...
        "format": "json",
    }
    url = API + "?" + urllib.parse.urlencode(params)
    for attempt in range(retries):
        result = subprocess.run(
            ["curl", "-sL", "--max-time", "15", "-w", "\n%{http_code}", "-A", USER_AGENT, url],
            capture_output=True,
            text=True,
        )
        *body_lines, status_code = result.stdout.rsplit("\n", 1)
        body = "\n".join(body_lines) if body_lines else ""
        retryable = status_code == "429" or status_code == "000" or not body
        if retryable and attempt < retries - 1:
            wait = 2 ** attempt
            print(f"  {title!r}: HTTP {status_code or '(none)'}, retrying in {wait}s...", file=sys.stderr)
            time.sleep(wait)
            continue
        if status_code != "200" or not body:
            raise RuntimeError(f"curl failed for {title!r}: HTTP {status_code}, stderr={result.stderr!r}")
        data = json.loads(body)
        for page in data.get("query", {}).get("pages", {}).values():
            return [link["title"] for link in page.get("links", [])]
        return []
    return []


def crawl(start_title, max_depth, fanout):
    visited = {start_title}
    nodes = [{
        "id": "n0",
        "label": start_title,
        "parent": None,
        "metric": 0,
        "metadata": {"degree": 0, "note": "start page"},
    }]
    frontier = [("n0", start_title, 0)]
    counter = 1
    while frontier:
        parent_id, title, depth = frontier.pop(0)
        if depth >= max_depth:
            continue
        time.sleep(REQUEST_DELAY_SECONDS)
        links = fetch_links(title)
        picked = [link for link in links if link not in visited][:fanout]
        for link_title in picked:
            visited.add(link_title)
            node_id = f"n{counter}"
            counter += 1
            nodes.append({
                "id": node_id,
                "label": link_title,
                "parent": parent_id,
                "metric": depth + 1,
                "metadata": {
                    "degree": depth + 1,
                    "note": "real Wikipedia outbound link (English Wikipedia); fan-out capped and deduped against already-visited pages to keep the crawl bounded and tree-shaped",
                },
            })
            frontier.append((node_id, link_title, depth + 1))
    return nodes


def main():
    out_path = sys.argv[1] if len(sys.argv) > 1 else "demos/wikipedia-crawl/graph.json"
    start_title = sys.argv[2] if len(sys.argv) > 2 else "Frankfurt"
    max_depth = int(sys.argv[3]) if len(sys.argv) > 3 else DEFAULT_MAX_DEPTH
    fanout = int(sys.argv[4]) if len(sys.argv) > 4 else DEFAULT_FANOUT

    nodes = crawl(start_title, max_depth, fanout)
    graph = {
        "title": f"{max_depth} degrees of Wikipedia from \"{start_title}\" (real links, fan-out capped at {fanout})",
        "metric_label": "degrees of separation",
        "nodes": nodes,
    }
    os.makedirs(os.path.dirname(out_path), exist_ok=True)
    with open(out_path, "w") as f:
        json.dump(graph, f, indent=2)
    print(f"wrote {len(nodes)} real Wikipedia pages (depth {max_depth}, fan-out {fanout}) to {out_path}")


if __name__ == "__main__":
    main()
