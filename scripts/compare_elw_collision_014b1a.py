#!/usr/bin/env python3
"""Compare extracted Goldshire replay against preserved POC samples and summaries."""

import argparse
import csv
import math
import re
import sys
from collections import defaultdict
from pathlib import Path


# Fixed before the formal comparison. POC sample coordinates and velocities are rounded to 0.001;
# 0.01 yd / yd/s allows serialization and ordinary float variation without masking visible drift.
POSITION_TOLERANCE = 0.01
VELOCITY_TOLERANCE = 0.01
TIME_TOLERANCE = 0.011  # POC sample times are written to 0.01 seconds.

SAMPLE_RE = re.compile(
    r"t=(?P<t>-?[\d.]+)s pos=(?P<x>-?[\d.]+),(?P<y>-?[\d.]+),(?P<z>-?[\d.]+) "
    r"intent=(?P<ix>-?[\d.]+),(?P<iz>-?[\d.]+) grounded=(?P<grounded>true|false) "
    r"vxz=(?P<vx>-?[\d.]+),(?P<vz>-?[\d.]+) vy=(?P<vy>-?[\d.]+)(?: ground=.*)?"
)


def read_tsv(path):
    with path.open(newline="", encoding="utf-8-sig") as stream:
        return list(csv.DictReader(stream, delimiter="\t"))


def parse_poc_sample(sample):
    match = SAMPLE_RE.fullmatch(sample.strip())
    if match is None:
        raise ValueError(f"unrecognized preserved POC sample: {sample!r}")
    row = match.groupdict()
    for key in ("t", "x", "y", "z", "ix", "iz", "vx", "vy", "vz"):
        row[key] = float(row[key])
    row["grounded"] = row["grounded"] == "true"
    return row


def close(a, b, tolerance):
    return abs(a - b) <= tolerance


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--reference", type=Path, required=True)
    parser.add_argument("--extracted", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()

    with args.reference.open(newline="", encoding="utf-8-sig") as stream:
        references = list(csv.DictReader(stream))
    summaries = read_tsv(args.extracted / "extracted-summary.tsv")
    samples = read_tsv(args.extracted / "extracted-samples.tsv")

    summary_by_key = {(r["scenario"], int(r["rate_hz"])): r for r in summaries}
    samples_by_key = defaultdict(list)
    for row in samples:
        samples_by_key[(row["scenario"], int(row["rate_hz"]))].append(row)

    report = []
    failures = []
    expected_keys = set()
    for ref in references:
        scenario = ref["scenario"]
        rate = round(1.0 / float(ref["dt"]))
        key = (scenario, rate)
        expected_keys.add(key)
        if key not in summary_by_key:
            failures.append(f"missing extracted summary: {scenario} {rate}Hz")
            continue

        extracted_summary = summary_by_key[key]
        extracted_samples = samples_by_key[key]
        reference_samples = [parse_poc_sample(s) for s in ref["samples"].split(" | ") if s]
        compared = min(len(reference_samples), len(extracted_samples))
        max_position_error = 0.0
        max_velocity_error = 0.0
        grounded_mismatches = 0
        sample_failures = []

        if len(reference_samples) != len(extracted_samples):
            sample_failures.append(
                f"sample count POC={len(reference_samples)} extracted={len(extracted_samples)}"
            )

        for index, (old, new) in enumerate(zip(reference_samples, extracted_samples)):
            new_time = float(new["elapsed_s"])
            new_pos = [float(new["wow_x"]), float(new["wow_y"]), float(new["wow_z"])]
            new_vel = [float(new["vel_x"]), float(new["vel_y"]), float(new["vel_z"])]
            old_pos = [old["x"], old["y"], old["z"]]
            old_vel = [old["vx"], old["vy"], old["vz"]]
            pos_error = math.dist(old_pos, new_pos)
            vel_error = max(abs(a - b) for a, b in zip(old_vel, new_vel))
            max_position_error = max(max_position_error, pos_error)
            max_velocity_error = max(max_velocity_error, vel_error)
            time_ok = close(old["t"], new_time, TIME_TOLERANCE)
            position_ok = all(
                close(a, b, POSITION_TOLERANCE) for a, b in zip(old_pos, new_pos)
            )
            velocity_ok = all(
                close(a, b, VELOCITY_TOLERANCE) for a, b in zip(old_vel, new_vel)
            )
            intent_ok = close(old["ix"], float(new["intent_x"]), 0.011) and close(
                old["iz"], float(new["intent_z"]), 0.011
            )
            grounded_ok = old["grounded"] == (new["grounded"] == "true")
            if not grounded_ok:
                grounded_mismatches += 1
            if not (time_ok and position_ok and velocity_ok and intent_ok and grounded_ok):
                sample_failures.append(
                    f"sample {index} t={old['t']:.2f}s position_error={pos_error:.6f} "
                    f"velocity_error={vel_error:.6f} grounded={old['grounded']}/{new['grounded']}"
                )

        old_final = [
            float(ref["end_wow_x"]),
            float(ref["end_wow_y"]),
            float(ref["end_wow_z"]),
        ]
        new_final = [
            float(extracted_summary["final_wow_x"]),
            float(extracted_summary["final_wow_y"]),
            float(extracted_summary["final_wow_z"]),
        ]
        final_error = math.dist(old_final, new_final)
        summary_differences = []
        for old_key, new_key in (
            ("ticks", "ticks"),
            ("grounded", "grounded_ticks"),
            ("airborne", "airborne_ticks"),
            ("airborne_episodes", "airborne_episodes"),
            ("contacts", "contact_ticks"),
            ("obstacle_contacts", "obstacle_contact_ticks"),
        ):
            if int(ref[old_key]) != int(extracted_summary[new_key]):
                summary_differences.append(
                    f"{old_key}={ref[old_key]}/{extracted_summary[new_key]}"
                )
        if not close(final_error, 0.0, POSITION_TOLERANCE):
            summary_differences.append(f"final_position_error={final_error:.6f} yd")
        if (ref["final_grounded"] == "true") != (
            extracted_summary["final_grounded"] == "true"
        ):
            summary_differences.append(
                f"final_grounded={ref['final_grounded']}/{extracted_summary['final_grounded']}"
            )

        issues = sample_failures + summary_differences
        if issues:
            failures.extend(f"{scenario} {rate}Hz: {issue}" for issue in issues)
        report.append(
            {
                "scenario": scenario,
                "rate_hz": rate,
                "ticks_poc": int(ref["ticks"]),
                "samples_compared": compared,
                "max_sample_position_error_yd": max_position_error,
                "final_position_error_yd": final_error,
                "max_velocity_error_yd_s": max_velocity_error,
                "grounded_mismatches": grounded_mismatches,
                "contacts_delta": int(extracted_summary["contact_ticks"])
                - int(ref["contacts"]),
                "obstacle_contacts_delta": int(extracted_summary["obstacle_contact_ticks"])
                - int(ref["obstacle_contacts"]),
                "result": "PASS" if not issues else "FAIL",
                "details": "; ".join(issues),
            }
        )

    if set(summary_by_key) & expected_keys != expected_keys:
        failures.append("extracted summary does not contain every POC scenario/rate pair")
    if len(references) != 21 * 3:
        failures.append(f"expected 63 preserved POC rows, found {len(references)}")

    args.output.parent.mkdir(parents=True, exist_ok=True)
    with args.output.open("w", newline="", encoding="utf-8") as stream:
        writer = csv.DictWriter(stream, fieldnames=list(report[0]), delimiter="\t")
        writer.writeheader()
        writer.writerows(report)

    print(f"POC scenario-rate pairs: {len(report)}")
    print(f"Compared quarter-second samples: {sum(r['samples_compared'] for r in report)}")
    print(f"Maximum sampled position error: {max(r['max_sample_position_error_yd'] for r in report):.6f} yd")
    print(f"Maximum final position error: {max(r['final_position_error_yd'] for r in report):.6f} yd")
    print(f"Maximum sampled velocity error: {max(r['max_velocity_error_yd_s'] for r in report):.6f} yd/s")
    print(f"Failures: {len(failures)}")
    for failure in failures[:100]:
        print(f"FAIL: {failure}")
    if len(failures) > 100:
        print(f"... and {len(failures) - 100} additional failures")
    print(f"Comparison TSV: {args.output}")
    return 1 if failures else 0


if __name__ == "__main__":
    sys.exit(main())
