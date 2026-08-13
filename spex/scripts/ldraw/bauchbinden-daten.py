#!/usr/bin/env python3
"""Build the lower-third data for every Atlas sheet from the committed snapshot.

`bauchbinde.py` needs a name/year/criteria record per slug. This makes it, so
the pipeline is reproducible from the repository rather than from a /tmp file
that happened to exist on one machine.

    python3 scripts/ldraw/bauchbinden-daten.py > scripts/heritage-data/bauchbinden.json
"""
import glob, json, pathlib, sys

snaps = sorted(glob.glob("scripts/heritage-data/wikidata-whs-*.json"))
if not snaps:
    sys.exit("kein Wikidata-Snapshot unter scripts/heritage-data/")
snap = json.loads(pathlib.Path(snaps[-1]).read_text())
rows = {r["id"]: r for r in (snap["sites"] if isinstance(snap, dict) else snap)}

out, missing = {}, []
for f in sorted(pathlib.Path("heritage").glob("*.json")):
    d = json.loads(f.read_text())
    r = rows.get(d.get("qid"))
    if r is None:
        # the sheet's QID is not a World Heritage Site record. Ten sheets carried
        # the QID of the BUILDING, which is a different entity from the SITE.
        missing.append((d["slug"], d.get("qid")))
    out[d["slug"]] = {
        "name": d["name"], "qid": d.get("qid"),
        "year": (r or {}).get("inscribedYear"),
        "crit": (r or {}).get("criteria"),
        "states": (r or {}).get("stateParties"),
    }
for slug, qid in missing:
    print(f"WARNUNG: {slug} — {qid} ist kein Welterbe-Datensatz im Snapshot", file=sys.stderr)
print(json.dumps(out, ensure_ascii=False, indent=1))
