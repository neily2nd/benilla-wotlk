#!/usr/bin/env python3
"""Summarize per-step timings from preserved POC CSV and extracted replay summary."""

import argparse
import csv
import statistics
from pathlib import Path


def read_csv(path, delimiter=","):
    with path.open(newline="", encoding="utf-8-sig") as stream:
        return list(csv.DictReader(stream, delimiter=delimiter))


def quantile(values, fraction):
    return sorted(values)[round((len(values) - 1) * fraction)]


def summarize(label, rows, rate_field, mean_field, ticks_field):
    result = {}
    for rate in (30, 60, 120):
        current = [r for r in rows if int(r[rate_field]) == rate and r["scenario"] != "goldshire_jump_and_land"]
        means = [float(r[mean_field]) for r in current]
        ticks = [int(r[ticks_field]) for r in current]
        total_steps = sum(ticks)
        weighted = sum(mean * count for mean, count in zip(means, ticks)) / total_steps
        result[rate] = {
            "label": label,
            "pairs": len(current),
            "steps": total_steps,
            "weighted_mean_us": weighted,
            "scenario_mean_median_us": statistics.median(means),
            "scenario_mean_p95_us": quantile(means, 0.95),
            "scenario_mean_max_us": max(means),
        }
    return result


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--poc", type=Path, required=True)
    parser.add_argument("--extracted", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    poc = read_csv(args.poc)
    extracted = read_csv(args.extracted, "\t")
    # POC stores the step duration in seconds, while the extracted harness stores Hz.
    old_rows = [
        {**r, "rate_hz": round(1 / float(r["dt"]))}
        for r in poc
    ]
    old = summarize("preserved POC optimized-dev", old_rows, "rate_hz", "mover_avg_us", "ticks")
    new_rows = [
        {**r, "rate_hz": int(r["rate_hz"])}
        for r in extracted
    ]
    new = summarize("extracted optimized-dev", new_rows, "rate_hz", "mean_step_us", "ticks")
    args.output.parent.mkdir(parents=True, exist_ok=True)
    with args.output.open("w", newline="", encoding="utf-8") as stream:
        writer = csv.writer(stream, delimiter="\t")
        writer.writerow(
            [
                "rate_hz",
                "reference_steps",
                "reference_weighted_mean_us",
                "reference_scenario_mean_median_us",
                "reference_scenario_mean_p95_us",
                "extracted_steps",
                "extracted_weighted_mean_us",
                "extracted_scenario_mean_median_us",
                "extracted_scenario_mean_p95_us",
                "weighted_mean_ratio",
            ]
        )
        for rate in (30, 60, 120):
            writer.writerow(
                [
                    rate,
                    old[rate]["steps"],
                    f"{old[rate]['weighted_mean_us']:.3f}",
                    f"{old[rate]['scenario_mean_median_us']:.3f}",
                    f"{old[rate]['scenario_mean_p95_us']:.3f}",
                    new[rate]["steps"],
                    f"{new[rate]['weighted_mean_us']:.3f}",
                    f"{new[rate]['scenario_mean_median_us']:.3f}",
                    f"{new[rate]['scenario_mean_p95_us']:.3f}",
                    f"{new[rate]['weighted_mean_us'] / old[rate]['weighted_mean_us']:.3f}",
                ]
            )
    for rate in (30, 60, 120):
        print(
            f"{rate}Hz steps={old[rate]['steps']}/{new[rate]['steps']} "
            f"weighted mean us={old[rate]['weighted_mean_us']:.3f}/{new[rate]['weighted_mean_us']:.3f} "
            f"scenario median us={old[rate]['scenario_mean_median_us']:.3f}/{new[rate]['scenario_mean_median_us']:.3f} "
            f"scenario p95 us={old[rate]['scenario_mean_p95_us']:.3f}/{new[rate]['scenario_mean_p95_us']:.3f} "
            f"ratio={new[rate]['weighted_mean_us'] / old[rate]['weighted_mean_us']:.3f}"
        )
    print(f"Performance TSV: {args.output}")


if __name__ == "__main__":
    main()
