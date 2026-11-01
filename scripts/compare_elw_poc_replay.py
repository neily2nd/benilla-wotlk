#!/usr/bin/env python3
"""Confirm a fresh isolated POC replay reproduces the preserved upstream-capsule CSV."""

import argparse
import csv
import math
import sys
from pathlib import Path

from compare_elw_collision_014b1a import (
    POSITION_TOLERANCE,
    SAMPLE_RE,
    TIME_TOLERANCE,
    VELOCITY_TOLERANCE,
)


def sample(row):
    match = SAMPLE_RE.fullmatch(row.strip())
    if not match:
        raise ValueError(f"unrecognized sample: {row!r}")
    values = match.groupdict()
    for key in ("t", "x", "y", "z", "ix", "iz", "vx", "vy", "vz"):
        values[key] = float(values[key])
    values["grounded"] = values["grounded"] == "true"
    return values


def read_csv(path):
    with path.open(newline="", encoding="utf-8-sig") as stream:
        return list(csv.DictReader(stream))


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--preserved", type=Path, required=True)
    parser.add_argument("--replay", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    old_rows = read_csv(args.preserved)
    new_rows = read_csv(args.replay)
    new_by_key = {(r["scenario"], round(1 / float(r["dt"]))): r for r in new_rows}
    records = []
    failures = []
    for old in old_rows:
        key = (old["scenario"], round(1 / float(old["dt"])))
        new = new_by_key.get(key)
        if new is None:
            failures.append(f"missing fresh POC pair {key}")
            continue
        old_samples = [sample(x) for x in old["samples"].split(" | ") if x]
        new_samples = [sample(x) for x in new["samples"].split(" | ") if x]
        max_pos = max_vel = 0.0
        mismatches = 0
        for a, b in zip(old_samples, new_samples):
            pos = math.dist([a["x"], a["y"], a["z"]], [b["x"], b["y"], b["z"]])
            vel = max(abs(a[k] - b[k]) for k in ("vx", "vy", "vz"))
            max_pos, max_vel = max(max_pos, pos), max(max_vel, vel)
            ok = (
                abs(a["t"] - b["t"]) <= TIME_TOLERANCE
                and all(abs(a[k] - b[k]) <= POSITION_TOLERANCE for k in ("x", "y", "z"))
                and all(abs(a[k] - b[k]) <= VELOCITY_TOLERANCE for k in ("vx", "vy", "vz"))
                and abs(a["ix"] - b["ix"]) <= 0.011
                and abs(a["iz"] - b["iz"]) <= 0.011
                and a["grounded"] == b["grounded"]
            )
            mismatches += not ok
        exact_fields = (
            "ticks",
            "grounded",
            "airborne",
            "airborne_episodes",
            "contacts",
            "obstacle_contacts",
            "final_grounded",
            "classification",
        )
        row_failures = [
            f"{key_name}={old[key_name]}/{new[key_name]}"
            for key_name in exact_fields
            if old[key_name] != new[key_name]
        ]
        if len(old_samples) != len(new_samples):
            row_failures.append(f"sample_count={len(old_samples)}/{len(new_samples)}")
        if mismatches:
            row_failures.append(f"sample_mismatches={mismatches}")
        if row_failures:
            failures.append(f"{key}: " + "; ".join(row_failures))
        records.append(
            {
                "scenario": key[0],
                "rate_hz": key[1],
                "samples": min(len(old_samples), len(new_samples)),
                "max_sample_position_error_yd": max_pos,
                "max_sample_velocity_error_yd_s": max_vel,
                "sample_mismatches": mismatches,
                "tick_delta": int(new["ticks"]) - int(old["ticks"]),
                "grounded_delta": int(new["grounded"]) - int(old["grounded"]),
                "contact_delta": int(new["contacts"]) - int(old["contacts"]),
                "result": "PASS" if not row_failures else "FAIL",
            }
        )
    if len(old_rows) != 63 or len(new_rows) != 63:
        failures.append(f"expected 63 rows in each CSV, found {len(old_rows)}/{len(new_rows)}")
    args.output.parent.mkdir(parents=True, exist_ok=True)
    with args.output.open("w", newline="", encoding="utf-8") as stream:
        writer = csv.DictWriter(stream, fieldnames=list(records[0]), delimiter="\t")
        writer.writeheader()
        writer.writerows(records)
    print(f"Fresh POC versus preserved pairs: {len(records)}")
    print(f"Fresh POC versus preserved samples: {sum(r['samples'] for r in records)}")
    print(f"Maximum sample position delta: {max(r['max_sample_position_error_yd'] for r in records):.6f} yd")
    print(f"Maximum sample velocity delta: {max(r['max_sample_velocity_error_yd_s'] for r in records):.6f} yd/s")
    print(f"Failures: {len(failures)}")
    for failure in failures[:100]:
        print(f"FAIL: {failure}")
    print(f"Comparison TSV: {args.output}")
    return 1 if failures else 0


if __name__ == "__main__":
    sys.exit(main())
