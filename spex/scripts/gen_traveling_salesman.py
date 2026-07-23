#!/usr/bin/env python3
"""Generates demos/traveling-salesman/graph.json: a simulated packet journey
Neuss -> Hamburg -> Kiel -> Berlin -> Sonneberg -> Bayreuth -> Tegernsee, with
2 synthetic intermediate router hops between each city pair (instead of one
hop per city) so the chain reads more like a real traceroute. Real haversine
distances between real city coordinates; per-hop latency is illustrative
(split proportionally across the synthetic sub-hops), not measured. Safe to
re-run any time — fully regenerates the file from scratch.
"""
import json
import os
import random
import sys
from math import asin, cos, radians, sin, sqrt

EARTH_RADIUS_KM = 6371.0
HOPS_PER_EDGE = 2

# "Deutsche Bahn mode" (--deutsche-bahn): purely for fun, layered on top of
# the same per-hop structure above — simulated delays/cancellations, *not*
# real train data (there's no real German rail API being hit here, unlike
# the real coordinates/haversine distances everywhere else in this file).
# Fixed seed so a given run is reproducible, not different every time.
DB_SEED = 42
DB_ON_TIME_CHANCE = 0.70
DB_DELAYED_CHANCE = 0.20
# remaining probability mass (0.10) is "cancelled"


def db_status(rng):
    """Returns (metric_multiplier, status_label, note) for one hop, purely
    for fun — not real train data."""
    roll = rng.random()
    if roll < DB_ON_TIME_CHANCE:
        return 1.0, "on time", "Deutsche Bahn mode: simulated, not real train data"
    if roll < DB_ON_TIME_CHANCE + DB_DELAYED_CHANCE:
        delay_min = rng.randint(3, 45)
        return (
            1.0 + delay_min / 15.0,
            f"delayed +{delay_min} min",
            "Deutsche Bahn mode: simulated delay, not a real train report",
        )
    return (
        rng.uniform(6.0, 12.0),
        "CANCELLED - replacement bus dispatched",
        "Deutsche Bahn mode: simulated cancellation, not a real train report",
    )


def haversine_km(a, b):
    lat1, lon1 = radians(a[0]), radians(a[1])
    lat2, lon2 = radians(b[0]), radians(b[1])
    dlat, dlon = lat2 - lat1, lon2 - lon1
    h = sin(dlat / 2) ** 2 + cos(lat1) * cos(lat2) * sin(dlon / 2) ** 2
    return 2 * EARTH_RADIUS_KM * asin(sqrt(h))


def lerp(a, b, t):
    return a + (b - a) * t


# (id, label, lat, lon, one-way latency in ms for the hop arriving *into* this
# city from the previous one — None for the root)
CITIES = [
    ("neuss", "Neuss", 51.1985, 6.6956, None),
    ("hamburg", "Hamburg", 53.5511, 9.9937, 8.8),
    ("kiel", "Kiel", 54.3233, 10.1228, 5.2),
    ("berlin", "Berlin", 52.5200, 13.4050, 8.1),
    ("sonneberg", "Sonneberg", 50.3572, 11.1720, 8.0),
    ("bayreuth", "Bayreuth", 49.9456, 11.5713, 4.8),
    ("tegernsee", "Tegernsee", 47.7167, 11.7500, 7.5),
]


def build_nodes(rng=None):
    nodes = []
    prev_id = None
    prev = CITIES[0]
    nodes.append({
        "id": prev[0],
        "label": prev[1],
        "parent": None,
        "metric": None,
        "metadata": {"lat": prev[2], "lon": prev[3], "note": "start of the journey"},
    })
    prev_id = prev[0]

    for city in CITIES[1:]:
        city_id, label, lat, lon, latency_ms = city
        a_lat, a_lon = prev[2], prev[3]

        # Synthetic intermediate router hops, evenly spaced along a straight
        # lat/lon interpolation between the two cities (a simplification of
        # the real great-circle path, close enough to be illustrative).
        points = [(a_lat, a_lon)]
        for i in range(1, HOPS_PER_EDGE + 1):
            t = i / (HOPS_PER_EDGE + 1)
            points.append((lerp(a_lat, lat, t), lerp(a_lon, lon, t)))
        points.append((lat, lon))

        leg_km = [haversine_km(points[i], points[i + 1]) for i in range(len(points) - 1)]
        total_km = sum(leg_km)

        last_id = prev_id
        for i in range(HOPS_PER_EDGE):
            hop_id = f"{prev_id}-{city_id}-hop{i + 1}"
            hop_lat, hop_lon = points[i + 1]
            hop_latency = latency_ms * (leg_km[i] / total_km) if total_km > 0 else 0.0
            metadata = {
                "lat": round(hop_lat, 4),
                "lon": round(hop_lon, 4),
                "distanceKm": round(leg_km[i], 1),
                "hostname": f"rtr-{prev_id}-{city_id}-{i + 1}.example.net",
                "ip": f"10.{(hash(hop_id) % 200) + 10}.{(hash(hop_id + 'b') % 200) + 10}.{i + 1}",
                "note": "fabricated router hostname/IP for demo purposes; position interpolated along a straight lat/lon path (not the true great circle); latency is illustrative, not measured",
            }
            if rng is not None:
                multiplier, status, db_note = db_status(rng)
                hop_latency *= multiplier
                metadata["status"] = status
                metadata["note"] = db_note
            nodes.append({
                "id": hop_id,
                "label": f"hop {i + 1} ({prev[1]} -> {label})",
                "parent": last_id,
                "metric": round(hop_latency, 2),
                "metadata": metadata,
            })
            last_id = hop_id

        final_leg_km = leg_km[-1]
        final_latency = latency_ms * (final_leg_km / total_km) if total_km > 0 else latency_ms
        final_metadata = {
            "lat": lat,
            "lon": lon,
            "distanceKm": round(final_leg_km, 1),
            "note": "real haversine distance for this final leg; latency is illustrative, not measured",
        }
        if rng is not None:
            multiplier, status, db_note = db_status(rng)
            final_latency *= multiplier
            final_metadata["status"] = status
            final_metadata["note"] = db_note
        nodes.append({
            "id": city_id,
            "label": label,
            "parent": last_id,
            "metric": round(final_latency, 2),
            "metadata": final_metadata,
        })
        prev = (city_id, label, lat, lon, latency_ms)
        prev_id = city_id

    return nodes


def main():
    args = sys.argv[1:]
    deutsche_bahn = "--deutsche-bahn" in args
    args = [a for a in args if a != "--deutsche-bahn"]
    out_path = args[0] if args else "demos/traveling-salesman/graph.json"

    rng = random.Random(DB_SEED) if deutsche_bahn else None
    title = "traveling-salesman demo: Neuss -> Hamburg -> Kiel -> Berlin -> Sonneberg -> Bayreuth -> Tegernsee (with simulated intermediate hops)"
    if deutsche_bahn:
        title += " -- Deutsche Bahn mode: simulated delays & cancellations, for fun, not real train data"
    graph = {
        "title": title,
        "metric_label": "simulated one-way latency (ms) - illustrative, not measured",
        "nodes": build_nodes(rng),
    }
    os.makedirs(os.path.dirname(out_path), exist_ok=True)
    with open(out_path, "w") as f:
        json.dump(graph, f, indent=2)
    print(f"wrote {len(graph['nodes'])} nodes to {out_path}")


if __name__ == "__main__":
    main()
