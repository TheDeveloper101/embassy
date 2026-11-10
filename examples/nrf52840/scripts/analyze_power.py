#!/usr/bin/env python3
"""Integrate an instrument CSV over periods where the measurement gate is high."""

import argparse
import csv
import json
import math
from pathlib import Path


def read_samples(path: Path):
    with path.open(newline="") as source:
        for line, row in enumerate(csv.DictReader(source), start=2):
            try:
                time_s = float(row["time_s"])
                current_a = float(row["current_a"])
                voltage_v = float(row["voltage_v"])
                gate = int(row["gate"])
            except (KeyError, TypeError, ValueError) as error:
                raise ValueError(f"invalid CSV row {line}: {error}") from error
            if not all(math.isfinite(value) for value in (time_s, current_a, voltage_v)):
                raise ValueError(f"non-finite value on CSV row {line}")
            if voltage_v <= 0 or gate not in (0, 1):
                raise ValueError(f"invalid voltage or gate on CSV row {line}")
            yield time_s, current_a * voltage_v, gate


def integrate(samples):
    samples = list(samples)
    if len(samples) < 2:
        raise ValueError("capture requires at least two samples")
    if samples[0][2] or samples[-1][2]:
        raise ValueError("capture must begin and end with gate low")

    energy_j = 0.0
    duration_s = 0.0
    rising_edges = 0
    falling_edges = 0
    previous_gate = samples[0][2]

    for left, right in zip(samples, samples[1:]):
        if right[0] <= left[0]:
            raise ValueError("timestamps must be strictly increasing")
        if not previous_gate and right[2]:
            rising_edges += 1
        if previous_gate and not right[2]:
            falling_edges += 1
        previous_gate = right[2]

        # Treat the gate as held from the left sample until the right sample.
        if left[2]:
            delta_t = right[0] - left[0]
            energy_j += delta_t * (left[1] + right[1]) / 2.0
            duration_s += delta_t

    if rising_edges != 1 or falling_edges != 1:
        raise ValueError("expected exactly one complete measurement-gate pulse")
    return energy_j, duration_s


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("capture", type=Path)
    parser.add_argument("--iterations", type=int, default=1_000)
    args = parser.parse_args()
    if args.iterations <= 0:
        parser.error("--iterations must be positive")

    energy_j, duration_s = integrate(read_samples(args.capture))
    print(json.dumps({
        "iterations": args.iterations,
        "total_energy_j": energy_j,
        "total_duration_s": duration_s,
        "energy_per_iteration_j": energy_j / args.iterations,
        "duration_per_iteration_s": duration_s / args.iterations,
        "average_power_w": energy_j / duration_s,
    }, indent=2))


if __name__ == "__main__":
    main()
