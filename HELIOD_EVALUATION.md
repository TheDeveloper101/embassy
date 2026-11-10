# Heliod Embassy evaluation

The nRF52840 evaluation contains three binaries:

- `temp`: bounded temperature measurement.
- `uart`: bounded UART echo loop.
- `temp_uart`: cross-peripheral burn-bug demonstration.

Build the combined example from the Embassy repository root:

```sh
cargo build \
  --manifest-path examples/nrf52840/Cargo.toml \
  --release \
  --bin temp_uart \
  --target thumbv7em-none-eabi
```

The `temp_uart` binary contains two schedules with the same peripheral results.
`inefficient_iteration` enables the UART before an unrelated temperature
measurement. `optimized_iteration` measures the temperature first and enables
the UART immediately before transmission.

Expected analysis results with the concrete demonstration costs are:

```text
Heliod cost for checked::inefficient_iteration: 260
Heliod cost for checked::optimized_iteration: 246
Heliod cost per loop iteration for checked::optimized_loop: 246
```

Compilation also reports:

```text
Heliod: checked::inefficient_iteration enables Nrf52UarteRegisters too early.
Moving the transition from Off to ActiveIdle past 1 independent command,
immediately before the first peripheral use, saves 14.
```

No optimization diagnostic should name `checked::optimized_iteration`.

The TX timeout is 32 RTC1 ticks. At 115200 baud, an eight-byte 8-N-1
transfer requires 23 ticks; the remaining nine ticks provide scheduling
margin. RX uses a separate one-second protocol deadline (32768 ticks), because
the arrival time of data from an external sender cannot be derived from baud
rate alone.

The artifact fixes its evaluated workload at buffers of at most eight bytes and
115200 baud. With the `time` feature enabled, the Heliod-bounded implementation
is the UART driver's default path; the artifact does not provide a general bound
for other workloads.

The binary executes the inefficient schedule once and then repeats the optimized
schedule. Each iteration measures the temperature, encodes the raw quarter-degree
value as four little-endian bytes, and transmits those bytes on UARTE0 TX pin
P0.06 at 115200 baud. RTT logging identifies which schedule is executing.

Flash and run the resulting ELF with:

```sh
probe-rs run \
  --chip nRF52840_xxAA \
  examples/nrf52840/target/thumbv7em-none-eabi/release/temp_uart
```

The concrete values are abstract cost units. The evaluation demonstrates bound
composition and detection of an injected burn bug; it does not claim that these
values are calibrated physical-energy measurements.
