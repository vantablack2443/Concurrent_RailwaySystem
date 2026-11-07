#!/usr/bin/env python3
"""Benchmark Slug vs Rocket and check they produce the same output.

Writes site/data/bench.json for the compare panel of the visualization page.
Run from the repository root after `cargo build --release -p train-sim`
(without the `viz` feature, so Rocket is timed without instrumentation):

    python3 scripts/viz_bench.py
"""

import json
import os
import re
import statistics
import subprocess
import sys
import tempfile
from pathlib import Path

BINARY = Path("target/release/train-sim")
RUNS = 5
SCENARIOS = [
    ("Island", "scenarios/viz/island.json"),
    ("Europe (500 trains)", "scenarios/public/europe_500.json"),
    ("Europe (10 000 trains)", "scenarios/public/europe.json"),
]


def run(scenario, out, slug):
    args = [str(BINARY), "--scenario", scenario, "--out", out]
    if slug:
        args.append("--slug")
    stdout = subprocess.run(args, check=True, capture_output=True, text=True).stdout
    # The binary prints the simulation time (excluding loading/saving).
    match = re.search(r"Time: (\d+)ms", stdout)
    return int(match.group(1))


def normalized(path):
    """Trace as a sorted list (order within a tick may differ) plus stats."""
    with open(path) as f:
        output = json.load(f)
    trace = sorted(
        (t["tick"], t["train"], t["origin"], t["destination"] or "") for t in output["trace"]
    )
    return trace, output["stats"]


def main():
    if not BINARY.exists():
        sys.exit(f"{BINARY} not found, run `cargo build --release -p train-sim` first")

    results = []
    with tempfile.TemporaryDirectory() as tmp:
        for name, scenario in SCENARIOS:
            slug_out = os.path.join(tmp, "slug.json")
            rocket_out = os.path.join(tmp, "rocket.json")
            slug_ms = [run(scenario, slug_out, slug=True) for _ in range(RUNS)]
            rocket_ms = [run(scenario, rocket_out, slug=False) for _ in range(RUNS)]
            slug_trace, slug_stats = normalized(slug_out)
            rocket_trace, rocket_stats = normalized(rocket_out)
            with open(scenario) as f:
                zones = len(json.load(f)["zones"])
            results.append({
                "name": name,
                "zones": zones,
                "traced": len(slug_trace) > 0,
                "traceEqual": slug_trace == rocket_trace,
                "statsEqual": slug_stats == rocket_stats,
                "slugMs": statistics.median(slug_ms),
                "rocketMs": statistics.median(rocket_ms),
            })
            print(f"{name}: slug {results[-1]['slugMs']} ms, rocket {results[-1]['rocketMs']} ms, "
                  f"equal={results[-1]['traceEqual'] and results[-1]['statsEqual']}")

    bench = {"cpus": os.cpu_count(), "runs": RUNS, "scenarios": results}
    Path("site/data").mkdir(parents=True, exist_ok=True)
    with open("site/data/bench.json", "w") as f:
        json.dump(bench, f, indent=2)


if __name__ == "__main__":
    main()
