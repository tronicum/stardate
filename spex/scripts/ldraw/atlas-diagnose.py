#!/usr/bin/env python3
"""Which STEP is failing, not which placement.

    python3 scripts/ldraw/atlas-diagnose.py recipes/heritage/*.json

`spex build` reports illegalities by placement index, which is the right thing
for the engine and useless for an author: "placement 372 is floating" in a
960-placement model says nothing about which of four massing steps is wrong.
This maps every reported index back to its step by counting `0 STEP` markers in
the emitted `.ldr`, and prints a table.

Forty sites times four steps is a hundred and sixty things that can be wrong.
Reading them one placement at a time is not a plan.
"""
import argparse
import collections
import pathlib
import re
import subprocess
import sys

REPO = pathlib.Path(__file__).resolve().parents[2]
SPEX = REPO / "target" / "release" / "spex"

PLACEMENT_RE = re.compile(r"placements? (\d+)(?: and (\d+))?: (.*)")


def step_of(ldr_path):
    """Placement index -> build-stage index, by counting the real STEP markers."""
    out, stage = [], 0
    for line in ldr_path.read_text().splitlines():
        if line.startswith("0 STEP"):
            stage += 1
        elif line.startswith("1 "):
            out.append(stage)
    return out


def diagnose(recipe):
    ldr = pathlib.Path("/tmp") / f"diag-{recipe.stem}.ldr"
    proc = subprocess.run([str(SPEX), "build", str(recipe), "-o", str(ldr)],
                          capture_output=True, text=True, timeout=900)
    text = proc.stdout + proc.stderr
    total = 0
    m = re.search(r"(\d+) placement\(s\)", text)
    if m:
        total = int(m.group(1))
    if "zero Illegality" in text:
        return total, 0, {}, {}
    stages = step_of(ldr) if ldr.exists() else []
    by_stage = collections.Counter()
    by_kind = collections.Counter()
    problems = 0
    for line in text.splitlines():
        hit = PLACEMENT_RE.search(line)
        if not hit:
            continue
        problems += 1
        kind = "overlap" if "overlap" in hit.group(3) else ("floating" if "floating" in hit.group(3) else hit.group(3)[:24])
        by_kind[kind] += 1
        for idx in (hit.group(1), hit.group(2)):
            if idx is None:
                continue
            i = int(idx)
            by_stage[stages[i] if i < len(stages) else -1] += 1
    return total, problems, by_kind, by_stage


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("recipes", nargs="+")
    args = ap.parse_args()
    if not SPEX.exists():
        sys.exit(f"{SPEX} is not built")

    clean = 0
    for r in sorted(pathlib.Path(p) for p in args.recipes):
        total, problems, kinds, stages = diagnose(r)
        name = r.stem
        if problems == 0:
            clean += 1
            print(f"{name:<16} {total:>6} Steine   sauber")
            continue
        kind_s = ", ".join(f"{k} {n}" for k, n in kinds.most_common())
        # Only the first few stages are worth naming; a step with one problem
        # is a corner, a step with hundreds is the wrong step.
        stage_s = ", ".join(f"Stufe {s}: {n}" for s, n in stages.most_common(4))
        print(f"{name:<16} {total:>6} Steine   {problems:>5} Problem(e)   {kind_s}")
        print(f"{'':<16} {'':>6}          {stage_s}")
    print(f"\n{clean}/{len(args.recipes)} sauber")


if __name__ == "__main__":
    main()
