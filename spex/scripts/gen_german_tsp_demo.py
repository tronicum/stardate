#!/usr/bin/env python3
"""Generates demos/german-tsp/graph.json: Frankfurt am Main as a fixed root,
plus 5 randomly picked German cities (population > 100k, fixed seed for
reproducibility), with an *actually solved* shortest Hamiltonian path
(brute-force exact search over all orderings — trivial at N=6) instead of a
fixed city-to-city sequence like the existing traveling-salesman/
deutsche-bahn demos (which are honestly just a fixed order, not a solved
TSP). Real haversine distances between real city coordinates throughout;
no illustrative/simulated latency this time, since there's no network
metaphor here — the metric is real distance.
"""
import itertools
import json
import os
import random
import sys
from math import asin, cos, radians, sin, sqrt

EARTH_RADIUS_KM = 6371.0
SEED = 7  # fixed, so the 5 random cities are reproducible across runs

# Real German cities, population > 100k (2024/2025 official estimates),
# with real approximate lat/lon. Frankfurt am Main is the fixed root and
# excluded from the random pool below.
FRANKFURT = ("frankfurt", "Frankfurt am Main", 50.1109, 8.6821)

CITY_POOL = [
    ("berlin", "Berlin", 52.5200, 13.4050),
    ("hamburg", "Hamburg", 53.5511, 9.9937),
    ("munich", "Munich", 48.1351, 11.5820),
    ("cologne", "Cologne", 50.9375, 6.9603),
    ("stuttgart", "Stuttgart", 48.7758, 9.1829),
    ("dusseldorf", "Dusseldorf", 51.2277, 6.7735),
    ("leipzig", "Leipzig", 51.3397, 12.3731),
    ("dortmund", "Dortmund", 51.5136, 7.4653),
    ("essen", "Essen", 51.4556, 7.0116),
    ("bremen", "Bremen", 53.0793, 8.8017),
    ("dresden", "Dresden", 51.0504, 13.7373),
    ("hannover", "Hannover", 52.3759, 9.7320),
    ("nuremberg", "Nuremberg", 49.4521, 11.0767),
    ("duisburg", "Duisburg", 51.4344, 6.7623),
    ("bochum", "Bochum", 51.4818, 7.2162),
    ("wuppertal", "Wuppertal", 51.2562, 7.1508),
    ("bielefeld", "Bielefeld", 52.0302, 8.5325),
    ("bonn", "Bonn", 50.7374, 7.0982),
    ("mannheim", "Mannheim", 49.4875, 8.4660),
    ("karlsruhe", "Karlsruhe", 49.0069, 8.4037),
    ("munster", "Munster", 51.9607, 7.6261),
    ("wiesbaden", "Wiesbaden", 50.0782, 8.2398),
    ("augsburg", "Augsburg", 48.3705, 10.8978),
    ("gelsenkirchen", "Gelsenkirchen", 51.5177, 7.0857),
    ("monchengladbach", "Monchengladbach", 51.1805, 6.4428),
    ("braunschweig", "Braunschweig", 52.2689, 10.5268),
    ("chemnitz", "Chemnitz", 50.8278, 12.9214),
    ("kiel", "Kiel", 54.3233, 10.1228),
    ("aachen", "Aachen", 50.7753, 6.0839),
    ("halle", "Halle", 51.4964, 11.9688),
    ("magdeburg", "Magdeburg", 52.1205, 11.6276),
    ("freiburg", "Freiburg", 47.9990, 7.8421),
    ("krefeld", "Krefeld", 51.3388, 6.5853),
    ("lubeck", "Lubeck", 53.8655, 10.6866),
    ("oberhausen", "Oberhausen", 51.4963, 6.8638),
    ("erfurt", "Erfurt", 50.9848, 11.0299),
    ("mainz", "Mainz", 49.9929, 8.2473),
    ("rostock", "Rostock", 54.0887, 12.1400),
    ("kassel", "Kassel", 51.3127, 9.4797),
    ("hagen", "Hagen", 51.3670, 7.4633),
    ("saarbrucken", "Saarbrucken", 49.2401, 6.9969),
    ("potsdam", "Potsdam", 52.3906, 13.0645),
    ("ludwigshafen", "Ludwigshafen", 49.4811, 8.4353),
]


def haversine_km(a, b):
    lat1, lon1 = radians(a[0]), radians(a[1])
    lat2, lon2 = radians(b[0]), radians(b[1])
    dlat, dlon = lat2 - lat1, lon2 - lon1
    h = sin(dlat / 2) ** 2 + cos(lat1) * cos(lat2) * sin(dlon / 2) ** 2
    return 2 * EARTH_RADIUS_KM * asin(sqrt(h))


def solve_shortest_path(cities):
    """Brute-force exact: cities[0] is the fixed start; try every ordering
    of the rest and keep the one with the lowest total path distance (no
    return leg to the start - this is a journey, not a closed tour)."""
    start, *rest = cities
    best_order = None
    best_distance = float("inf")
    for perm in itertools.permutations(rest):
        route = [start, *perm]
        total = sum(haversine_km(route[i][2:4], route[i + 1][2:4]) for i in range(len(route) - 1))
        if total < best_distance:
            best_distance = total
            best_order = route
    return best_order, best_distance


def build_nodes(route):
    nodes = []
    prev_id = None
    for i, (city_id, label, lat, lon) in enumerate(route):
        metric = None
        metadata = {"lat": lat, "lon": lon}
        if i == 0:
            metadata["note"] = "fixed root of the search, real coordinates"
        else:
            prev = route[i - 1]
            dist = haversine_km(prev[2:4], (lat, lon))
            metric = round(dist, 1)
            metadata["distanceKm"] = round(dist, 1)
            metadata["note"] = "real haversine distance; this is an actually-solved shortest path (brute-force exact), not a fixed sequence"
        nodes.append({"id": city_id, "label": label, "parent": prev_id, "metric": metric, "metadata": metadata})
        prev_id = city_id
    return nodes


def main():
    out_path = sys.argv[1] if len(sys.argv) > 1 else "demos/german-tsp/graph.json"
    rng = random.Random(SEED)
    chosen = rng.sample(CITY_POOL, 5)
    cities = [FRANKFURT] + chosen

    route, total_km = solve_shortest_path(cities)
    nodes = build_nodes(route)

    labels = " -> ".join(c[1] for c in route)
    graph = {
        "title": f"German cities real-TSP (brute-force exact): {labels} ({total_km:.0f} km total)",
        "metric_label": "real haversine distance (km)",
        "nodes": nodes,
    }
    os.makedirs(os.path.dirname(out_path), exist_ok=True)
    with open(out_path, "w") as f:
        json.dump(graph, f, indent=2)
    print(f"wrote {len(nodes)} nodes ({total_km:.1f} km total, exact shortest path) to {out_path}")


if __name__ == "__main__":
    main()
