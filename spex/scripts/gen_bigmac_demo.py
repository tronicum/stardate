#!/usr/bin/env python3
"""Generates a Big Mac Index demo graph.json: one country's real Big Mac
price history (2000-present) as a chain, one node per publication date.
Real data throughout — downloads The Economist's own public dataset
(github.com/TheEconomist/big-mac-data, real prices they've published twice a
year since 2000) rather than inventing numbers. Safe to re-run; caches the
downloaded CSV next to the output file so repeat runs don't re-fetch.
"""
import csv
import json
import os
import sys
import urllib.request

CSV_URL = "https://raw.githubusercontent.com/TheEconomist/big-mac-data/master/output-data/big-mac-adjusted-index.csv"


def download(csv_path):
    if os.path.exists(csv_path):
        return
    os.makedirs(os.path.dirname(csv_path), exist_ok=True)
    with urllib.request.urlopen(CSV_URL, timeout=30) as resp:
        data = resp.read()
    with open(csv_path, "wb") as f:
        f.write(data)


def build_nodes(rows, country):
    rows = [r for r in rows if r["name"] == country]
    rows.sort(key=lambda r: r["date"])
    if not rows:
        raise SystemExit(f"no rows found for country {country!r} in the dataset")

    nodes = []
    prev_id = None
    for r in rows:
        node_id = f"d{r['date']}"
        price = float(r["dollar_price"])
        nodes.append({
            "id": node_id,
            "label": r["date"],
            "parent": prev_id,
            "metric": None if prev_id is None else round(price, 2),
            "metadata": {
                "date": r["date"],
                "localPrice": float(r["local_price"]),
                "currency": r["currency_code"],
                "dollarExchangeRate": float(r["dollar_ex"]),
                "dollarPrice": round(price, 2),
                "gdpPerCapita": float(r["GDP_bigmac"]) if r["GDP_bigmac"] else None,
                "note": "real Big Mac price published by The Economist (github.com/TheEconomist/big-mac-data)",
            },
        })
        prev_id = node_id
    return nodes


def main():
    out_path = sys.argv[1] if len(sys.argv) > 1 else "demos/bigmac/graph.json"
    country = sys.argv[2] if len(sys.argv) > 2 else "United States"
    csv_path = os.path.join(os.path.dirname(out_path) or ".", "big-mac-data.csv")

    download(csv_path)
    with open(csv_path, newline="", encoding="utf-8") as f:
        rows = list(csv.DictReader(f))

    nodes = build_nodes(rows, country)
    graph = {
        "title": f"Big Mac Index: {country} ({nodes[0]['label']} - {nodes[-1]['label']}), real data via The Economist",
        "metric_label": "Big Mac price (USD)",
        "nodes": nodes,
    }
    os.makedirs(os.path.dirname(out_path), exist_ok=True)
    with open(out_path, "w") as f:
        json.dump(graph, f, indent=2)
    print(f"wrote {len(nodes)} real data points ({nodes[0]['label']} to {nodes[-1]['label']}) to {out_path}")


if __name__ == "__main__":
    main()
