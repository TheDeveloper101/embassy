# Temperature/UART power benchmark

`temp_uart_power` runs one workload immediately after initialization: 50
temperature conversions followed by one eight-byte UART transmission. It then
drops the peripheral drivers and remains in a quiet `WFE` loop. There is no
warm-up, delay, or GPIO measurement gate. Restart or power-cycle the board for
each independent PPK2 capture.

The inefficient Heliod workload resumes UART before the 50 conversions. The
efficient workload performs all conversions while UART is suspended and resumes
it only for transmission. The vanilla application uses normal upstream Embassy,
which leaves UART active after construction.

For this fixed workload Heliod reports costs of 4,033 for the inefficient
schedule and 3,284 for the efficient schedule: a predicted saving of 749 model
units, or 18.57% relative to the inefficient schedule.

Build and save the two Heliod configurations before flashing:

| Workload | Cargo features |
| --- | --- |
| Inefficient Heliod application | none |
| Efficient Heliod application | `power-benchmark-efficient` |

For example:

```sh
cargo build --release --bin temp_uart_power
cargo build --release --bin temp_uart_power --features power-benchmark-efficient
```

Build the vanilla application separately. It is pinned to upstream Embassy commit
`21863c31db0df77ba4922ce1dbc79a9e1dd972a1`, before this repository's Heliod HAL
changes, and has no dependency on Abacus or Heliod:

```sh
cargo build --release --manifest-path vanilla-power/Cargo.toml
```

Copy or rename the ELF after each build so the following build does not replace
it. Flash with the project's configured `probe-rs` runner. Configure the nRF52840
DK so the PPK2 measures the intended power domain, and ensure no second supply
bypasses the instrument.

For each physical run, start recording before reset or power-on and stop after
the trace reaches its quiet tail. Select the same logical interval in the Power
Profiler app: either power-on through completion for total application energy,
or the visible workload region for workload-only energy. Record energy, charge,
duration, and average current from that selection. Alternate configuration order
and collect at least five independent runs per configuration.

For optional offline analysis, normalize an exported CSV to SI units:

```text
time_s,current_a,voltage_v
0.000000,0.000005,3.000
0.000010,0.004100,3.000
0.000020,0.004200,3.000
```

Then integrate the manually selected region:

```sh
python3 scripts/analyze_power.py capture.csv --start-s 0.0 --end-s 0.1
```

Repeat independent captures and alternate configuration order. Compare efficient
with inefficient Heliod builds for workload savings. Compare the vanilla build
with the Heliod builds as a system-level reference. Vanilla Embassy leaves UARTE
enabled after construction, so its physical schedule is not identical to either
Heliod schedule; the energy difference includes both HAL integration and
peripheral-scheduling behavior and must not be reported as pure instrumentation
overhead. Compare loadable section sizes, execution time, and disassembly as well.
