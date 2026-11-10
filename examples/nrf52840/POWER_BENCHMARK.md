# Temperature/UART power benchmark

`temp_uart_power` runs ten warm-up iterations, waits two seconds, and then runs
1,000 measured iterations. P0.03 is high around the measured block. Each
iteration performs one temperature conversion, formats the result into eight
bytes, transmits those bytes, and leaves the UART suspended.

Build and save all four configurations before flashing:

| Workload | Application Heliod analysis | Cargo features |
| --- | --- | --- |
| Inefficient | enabled | none |
| Efficient | enabled | `power-benchmark-efficient` |
| Inefficient | disabled | `power-benchmark-no-heliod` |
| Efficient | disabled | `power-benchmark-efficient,power-benchmark-no-heliod` |

For example:

```sh
cargo build --release --bin temp_uart_power
cargo build --release --bin temp_uart_power --features power-benchmark-efficient
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
with inefficient builds for workload savings. Compare Heliod-enabled and disabled
builds of the same workload for application-analysis overhead. Also compare
loadable section sizes and disassembly; this feature only disables the outer
application analysis and does not provide a vanilla Embassy/Abacus-free baseline.
