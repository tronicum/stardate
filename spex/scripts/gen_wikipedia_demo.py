#!/usr/bin/env python3
"""Generates the wikipedia-crawl demo's graph.json from a committed real data
snapshot (scripts/wikipedia-crawl-data/frankfurt-depth3-fanout3.json — a real
breadth-first crawl of English Wikipedia's outbound links, captured once by
gen_wikipedia_crawl.py) rather than live-crawling on every run: Wikipedia's
API sustained-rate-limited even a well-paced curl-based crawl (see
gen_wikipedia_crawl.py's docstring), so re-crawling on every walkthrough.sh
run would be slow and flaky, especially in CI. Same pattern as
gen_stock_demo.py's committed Alpha Vantage snapshots.
"""
import json
import os
import shutil
import sys

SNAPSHOT = os.path.join(os.path.dirname(__file__), "wikipedia-crawl-data", "frankfurt-depth3-fanout3.json")


def main():
    out_path = sys.argv[1] if len(sys.argv) > 1 else "demos/wikipedia-crawl/graph.json"
    os.makedirs(os.path.dirname(out_path), exist_ok=True)
    shutil.copyfile(SNAPSHOT, out_path)
    with open(SNAPSHOT) as f:
        nodes = json.load(f)["nodes"]
    print(f"wrote {len(nodes)} real Wikipedia pages (from committed snapshot) to {out_path}")


if __name__ == "__main__":
    main()
