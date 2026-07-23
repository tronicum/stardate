#!/usr/bin/env python3
"""Generates a stock-price demo graph.json from a committed real data
snapshot (scripts/stock-data/<key>.json — raw Yahoo Finance chart API
responses, fetched once and checked in) rather than live-fetching on every
run: Yahoo's endpoint is TLS-fingerprint-sensitive (blocks plain Python
urllib even with a spoofed User-Agent) and aggressively rate-limits repeat
requests from the same IP, which would make a CI job that regenerates
demos on every push unreliable. A committed snapshot is also literally what
was asked for — a copied real dump, not fabricated numbers, and not a live
dependency either.
"""
import datetime
import json
import os
import sys

STOCKS = {
    "tsla": "TSLA",
    "vow3": "VOW3.DE",
    "byd": "1211.HK",
}


def build_nodes(chart_result):
    meta = chart_result["meta"]
    timestamps = chart_result["timestamp"]
    closes = chart_result["indicators"]["quote"][0]["close"]

    points = [(ts, c) for ts, c in zip(timestamps, closes) if c is not None]
    points.sort(key=lambda p: p[0])

    nodes = []
    prev_id = None
    for ts, close in points:
        date = datetime.datetime.utcfromtimestamp(ts).date().isoformat()
        node_id = f"d{date}"
        nodes.append({
            "id": node_id,
            "label": date,
            "parent": prev_id,
            "metric": None if prev_id is None else round(close, 2),
            "metadata": {
                "date": date,
                "close": round(close, 2),
                "currency": meta["currency"],
                "note": "real weekly close, Yahoo Finance chart API (fetched once, checked in as a data snapshot)",
            },
        })
        prev_id = node_id
    return nodes, meta


def main():
    out_path = sys.argv[1] if len(sys.argv) > 1 else "demos/stock/graph.json"
    key = sys.argv[2] if len(sys.argv) > 2 else "tsla"
    if key not in STOCKS:
        raise SystemExit(f"unknown stock key {key!r}, expected one of {list(STOCKS)}")

    script_dir = os.path.dirname(os.path.abspath(__file__))
    snapshot_path = os.path.join(script_dir, "stock-data", f"{key}.json")
    with open(snapshot_path) as f:
        raw = json.load(f)
    chart_result = raw["chart"]["result"][0]

    nodes, meta = build_nodes(chart_result)
    name = meta.get("longName") or meta.get("shortName") or STOCKS[key]
    graph = {
        "title": f"{name} ({STOCKS[key]}) weekly close, real data via Yahoo Finance: {nodes[0]['label']} - {nodes[-1]['label']}",
        "metric_label": f"weekly close ({meta['currency']})",
        "nodes": nodes,
    }
    os.makedirs(os.path.dirname(out_path), exist_ok=True)
    with open(out_path, "w") as f:
        json.dump(graph, f, indent=2)
    print(f"wrote {len(nodes)} real data points ({nodes[0]['label']} to {nodes[-1]['label']}) to {out_path}")


if __name__ == "__main__":
    main()
