#!/usr/bin/env python3
"""Integrate a normalized power CSV over a selected time interval."""

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
            except (KeyError, TypeError, ValueError) as error:
                raise ValueError(f"invalid CSV row {line}: {error}") from error
            if not all(math.isfinite(value) for value in (time_s, current_a, voltage_v)):
                raise ValueError(f"non-finite value on CSV row {line}")
            if voltage_v <= 0:
                raise ValueError(f"invalid voltage on CSV row {line}")
            yield time_s, current_a * voltage_v


def integrate(samples, start_s, end_s):
    samples = list(samples)
    if len(samples) < 2:
        raise ValueError("capture requires at least two samples")
    if start_s >= end_s:
        raise ValueError("start time must precede end time")
    if start_s < samples[0][0] or end_s > samples[-1][0]:
        raise ValueError("selected interval lies outside the capture")

    energy_j = 0.0
    duration_s = 0.0
    for left, right in zip(samples, samples[1:]):
        if right[0] <= left[0]:
            raise ValueError("timestamps must be strictly increasing")
        interval_start = max(left[0], start_s)
        interval_end = min(right[0], end_s)
        if interval_start < interval_end:
            sample_span = right[0] - left[0]
            start_fraction = (interval_start - left[0]) / sample_span
            end_fraction = (interval_end - left[0]) / sample_span
            start_power = left[1] + start_fraction * (right[1] - left[1])
            end_power = left[1] + end_fraction * (right[1] - left[1])
            delta_t = interval_end - interval_start
            energy_j += delta_t * (start_power + end_power) / 2.0
            duration_s += delta_t

    return energy_j, duration_s


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("capture", type=Path)
    parser.add_argument("--start-s", type=float, required=True)
    parser.add_argument("--end-s", type=float, required=True)
    parser.add_argument("--temperature-reads", type=int, default=50)
    args = parser.parse_args()
    if args.temperature_reads <= 0:
        parser.error("--temperature-reads must be positive")

    energy_j, duration_s = integrate(read_samples(args.capture), args.start_s, args.end_s)
    print(json.dumps({
        "temperature_reads": args.temperature_reads,
        "total_energy_j": energy_j,
        "total_duration_s": duration_s,
        "energy_per_temperature_read_j": energy_j / args.temperature_reads,
        "average_power_w": energy_j / duration_s,
    }, indent=2))


if __name__ == "__main__":
    main()
