#!/usr/bin/env python3
"""Generates a stock-price demo graph.json from a committed real data
snapshot (scripts/stock-data/<key>.json — raw Alpha Vantage
TIME_SERIES_WEEKLY responses, fetched once and checked in) rather than
live-fetching on every run: this keeps `walkthrough.sh` reliable in CI
(no API key needed there, no rate limit to hit) and is literally what was
asked for — a copied real dump, not fabricated numbers, and not a live
dependency either. (Earlier attempt used Yahoo Finance's chart API, which
blocks plain Python urllib outright and hit a sustained rate-limit from
this machine even via curl — Alpha Vantage, with a real free-tier API key,
worked cleanly on the first request.)
"""
import json
import os
import sys

STOCKS = {
    "tsla": {"symbol": "TSLA", "currency": "USD"},
    "vow3": {"symbol": "VOW3.DE", "currency": "EUR"},
    "byd": {"symbol": "BYDDY", "currency": "USD"},  # US OTC ADR — Alpha Vantage doesn't serve the Hong Kong listing (1211.HK)
}

# The real snapshots have 800-1100+ weekly points (full history back to each
# stock's earliest listing) — real data, but too many nodes for the radial
# layout to stay legible or fast (verified: the full Tesla history rendered
# as an unreadable tangle at 11fps). Window to the most recent N weeks
# instead of the full history — still real, unmodified numbers, just fewer
# of them, the same kind of honest choice `bigmac`/`traveling-salesman`
# already make about how much real history to include.
RECENT_WEEKS = 104  # ~2 years


def build_nodes(data, currency):
    weekly = data["Weekly Time Series"]
    dates = sorted(weekly.keys())[-RECENT_WEEKS:]

    nodes = []
    prev_id = None
    for date in dates:
        close = float(weekly[date]["4. close"])
        node_id = f"d{date}"
        nodes.append({
            "id": node_id,
            "label": date,
            "parent": prev_id,
            "metric": None if prev_id is None else round(close, 2),
            "metadata": {
                "date": date,
                "close": round(close, 2),
                "currency": currency,
                "note": "real weekly close, Alpha Vantage TIME_SERIES_WEEKLY (fetched once, checked in as a data snapshot)",
            },
        })
        prev_id = node_id
    return nodes


def main():
    out_path = sys.argv[1] if len(sys.argv) > 1 else "demos/stock/graph.json"
    key = sys.argv[2] if len(sys.argv) > 2 else "tsla"
    if key not in STOCKS:
        raise SystemExit(f"unknown stock key {key!r}, expected one of {list(STOCKS)}")
    info = STOCKS[key]

    script_dir = os.path.dirname(os.path.abspath(__file__))
    snapshot_path = os.path.join(script_dir, "stock-data", f"{key}.json")
    with open(snapshot_path) as f:
        data = json.load(f)

    nodes = build_nodes(data, info["currency"])
    symbol = data.get("Meta Data", {}).get("2. Symbol", info["symbol"])
    graph = {
        "title": f"{symbol} weekly close, real data via Alpha Vantage: {nodes[0]['label']} - {nodes[-1]['label']}",
        "metric_label": f"weekly close ({info['currency']})",
        "nodes": nodes,
    }
    os.makedirs(os.path.dirname(out_path), exist_ok=True)
    with open(out_path, "w") as f:
        json.dump(graph, f, indent=2)
    print(f"wrote {len(nodes)} real data points ({nodes[0]['label']} to {nodes[-1]['label']}) to {out_path}")


if __name__ == "__main__":
    main()
