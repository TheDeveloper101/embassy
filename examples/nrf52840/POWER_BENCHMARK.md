# Temperature/UART power benchmark

`temp_uart_power` runs ten warm-up iterations, waits two seconds, and then runs
1,000 measured iterations. P0.03 is high around the measured block. Each
iteration performs one temperature conversion, formats the result into eight
bytes, transmits those bytes, and leaves the UART suspended.

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
it. Flash with the project's configured `probe-rs` runner. Connect P0.03 and a
common ground to the power instrument's digital input. Configure the nRF52840 DK
so the instrument measures the intended power domain, and ensure no second supply
bypasses the instrument. P0.03 must not drive an LED or another appreciable load.

Export synchronized instrument samples to a CSV containing SI units:

```text
time_s,current_a,voltage_v,gate
0.000000,0.000005,3.000,0
0.000010,0.004100,3.000,1
0.000020,0.004200,3.000,1
0.100000,0.000005,3.000,0
```

Then integrate the marked region:

```sh
python3 scripts/analyze_power.py capture.csv
```

Repeat independent captures and alternate configuration order. Compare efficient
with inefficient Heliod builds for workload savings. Compare the vanilla build
with the Heliod builds as a system-level reference. Vanilla Embassy leaves UARTE
enabled after construction, so its physical schedule is not identical to either
Heliod schedule; the energy difference includes both HAL integration and
peripheral-scheduling behavior and must not be reported as pure instrumentation
overhead. Compare loadable section sizes, execution time, and disassembly as well.
