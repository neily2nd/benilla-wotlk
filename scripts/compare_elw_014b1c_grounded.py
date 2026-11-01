#!/usr/bin/env python3
"""Strictly compare paired current-POC/extracted movement traces from ELW-014B.1C."""

import argparse
import csv
import math
from collections import defaultdict
from pathlib import Path

POSITION_TOLERANCE = 0.001
VELOCITY_TOLERANCE = 0.001


def vector(row, indexes):
    return tuple(float(row[i]) for i in indexes)


def distance(a, b):
    return math.sqrt(sum((x - y) ** 2 for x, y in zip(a, b)))


def compare(input_dir: Path) -> list[str]:
    trace_path = input_dir / "paired-per-tick.tsv"
    grouped = defaultdict(list)
    with trace_path.open(newline="", encoding="utf-8") as stream:
        rows = csv.reader(stream, delimiter="\t")
        header = next(rows)
        if len(header) != 28:
            raise ValueError(f"expected 28 trace columns, found {len(header)}")
        for row in rows:
            if not row:
                continue
            grouped[(row[0], int(row[1]))].append(row)

    output = [
        "scenario\thz\tticks\tmax_position_delta_yd\tmax_velocity_delta_yd_s\tfirst_divergent_tick\tposition_failures\tvelocity_failures\tground_state_mismatches\tairborne_time_mismatches\tcontact_probe_mismatches\tlegacy_contact_ticks\textracted_contact_ticks\tlegacy_obstacle_ticks\textracted_obstacle_ticks\tverdict"
    ]
    for (scenario, hz), rows in sorted(grouped.items(), key=lambda item: (item[0][1], item[0][0])):
        rows.sort(key=lambda row: int(row[2]))
        if [int(row[2]) for row in rows] != list(range(1, len(rows) + 1)):
            raise ValueError(f"non-contiguous ticks for {scenario} at {hz} Hz")
        max_pos = max_vel = 0.0
        first_divergence = "none"
        pos_fail = vel_fail = ground_mismatch = air_time_mismatch = contact_mismatch = 0
        legacy_contacts = extracted_contacts = legacy_obstacles = extracted_obstacles = 0
        for row in rows:
            tick = int(row[2])
            p_delta = distance(vector(row, (6, 7, 8)), vector(row, (17, 18, 19)))
            v_delta = distance(vector(row, (9, 10, 11)), vector(row, (20, 21, 22)))
            max_pos = max(max_pos, p_delta)
            max_vel = max(max_vel, v_delta)
            pos_bad = p_delta > POSITION_TOLERANCE
            vel_bad = v_delta > VELOCITY_TOLERANCE
            ground_bad = row[12] != row[23] or row[13] != row[24]
            air_bad = row[14] != row[25]
            contact_bad = row[15] != row[26] or row[16] != row[27]
            pos_fail += int(pos_bad)
            vel_fail += int(vel_bad)
            ground_mismatch += int(ground_bad)
            air_time_mismatch += int(air_bad)
            contact_mismatch += int(contact_bad)
            legacy_contacts += int(row[15] == "true")
            extracted_contacts += int(row[26] == "true")
            legacy_obstacles += int(row[16] == "true")
            extracted_obstacles += int(row[27] == "true")
            if first_divergence == "none" and (pos_bad or vel_bad or ground_bad or air_bad or contact_bad):
                first_divergence = str(tick)
        passed = not any((pos_fail, vel_fail, ground_mismatch, air_time_mismatch, contact_mismatch))
        output.append(
            f"{scenario}\t{hz}\t{len(rows)}\t{max_pos:.9f}\t{max_vel:.9f}\t{first_divergence}\t"
            f"{pos_fail}\t{vel_fail}\t{ground_mismatch}\t{air_time_mismatch}\t{contact_mismatch}\t"
            f"{legacy_contacts}\t{extracted_contacts}\t{legacy_obstacles}\t{extracted_obstacles}\t"
            f"{'PASS' if passed else 'FAIL'}"
        )
    return output


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("input_dir", type=Path)
    args = parser.parse_args()
    lines = compare(args.input_dir)
    output_path = args.input_dir / "paired-analysis.tsv"
    output_path.write_text("\n".join(lines) + "\n", encoding="utf-8")
    print("\n".join(lines))
    return 0 if all(line.endswith("PASS") for line in lines[1:]) else 1


if __name__ == "__main__":
    raise SystemExit(main())
