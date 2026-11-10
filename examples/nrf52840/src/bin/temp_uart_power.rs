#![no_std]
#![no_main]

use abacus_registers::{heliod, heliod_ignore, heliod_operation};
use embassy_executor::Spawner;
use embassy_nrf::gpio::{Level, Output, OutputDrive};
use embassy_nrf::mode::Async;
use embassy_nrf::peripherals::UARTE0;
use embassy_nrf::temp::{Nrf5xTempCost, Temp};
use embassy_nrf::uarte::{Nrf52UarteCost, Uarte};
use embassy_nrf::{bind_interrupts, temp, uarte};
use embassy_time::Timer;
use {defmt_rtt as _, panic_probe as _};

const WARMUP_ITERATIONS: usize = 10;
const MEASURED_ITERATIONS: usize = 1_000;

bind_interrupts!(struct Irqs {
    TEMP => temp::InterruptHandler;
    UARTE0 => uarte::InterruptHandler<UARTE0>;
});

fn format_temperature_x4(value: i32, buffer: &mut [u8; 8]) {
    const HEX: &[u8; 16] = b"0123456789ABCDEF";
    let value = value as u16;
    buffer[0] = b'T';
    buffer[1] = b'=';
    buffer[2] = HEX[((value >> 12) & 0x0f) as usize];
    buffer[3] = HEX[((value >> 8) & 0x0f) as usize];
    buffer[4] = HEX[((value >> 4) & 0x0f) as usize];
    buffer[5] = HEX[(value & 0x0f) as usize];
    buffer[6] = b'\r';
    buffer[7] = b'\n';
}

#[heliod(Nrf5xTempCost, Nrf52UarteCost)]
mod workload {
    #![allow(dead_code)]

    use super::*;

    #[heliod_operation(
        peripheral = Nrf5xTempCost,
        from = Off,
        to = Off,
        cost = 62,
        background_steps = 15
    )]
    async fn read_temperature(temp: &mut Temp<'_>) -> i32 {
        temp.read_celsius_x4().await
    }

    #[heliod_operation(
        peripheral = Nrf52UarteCost,
        from = Off,
        to = ActiveIdle,
        cost = 1
    )]
    fn resume(uart: &mut Uarte<'_, Async>) {
        uart.resume();
    }

    #[heliod_operation(
        peripheral = Nrf52UarteCost,
        from = ActiveIdle,
        to = ActiveIdle,
        cost = 144,
        background_steps = 36
    )]
    async fn write(uart: &mut Uarte<'_, Async>, buffer: &[u8]) {
        uart.write(buffer).await.unwrap();
    }

    #[heliod_operation(
        peripheral = Nrf52UarteCost,
        from = ActiveIdle,
        to = Off,
        cost = 1
    )]
    fn suspend(uart: &mut Uarte<'_, Async>) {
        uart.suspend();
    }

    pub async fn inefficient(temp: &mut Temp<'_>, uart: &mut Uarte<'_, Async>, buffer: &mut [u8; 8]) {
        resume(uart);
        let value = read_temperature(temp).await;
        heliod_ignore!(format_temperature_x4(value, buffer));
        write(uart, buffer).await;
        suspend(uart);
    }

    pub async fn efficient(temp: &mut Temp<'_>, uart: &mut Uarte<'_, Async>, buffer: &mut [u8; 8]) {
        let value = read_temperature(temp).await;
        heliod_ignore!(format_temperature_x4(value, buffer));
        resume(uart);
        write(uart, buffer).await;
        suspend(uart);
    }
}

async fn iteration(temp: &mut Temp<'_>, uart: &mut Uarte<'_, Async>, buffer: &mut [u8; 8]) {
    #[cfg(feature = "power-benchmark-efficient")]
    workload::efficient(temp, uart, buffer).await;

    #[cfg(not(feature = "power-benchmark-efficient"))]
    workload::inefficient(temp, uart, buffer).await;
}

#[embassy_executor::main]
async fn main(_spawner: Spawner) {
    let p = embassy_nrf::init(Default::default());
    // P0.03 is high only during the measured block. Connect it and GND to the
    // power instrument's digital input. The input must be high impedance.
    let mut measurement_gate = Output::new(p.P0_03, Level::Low, OutputDrive::Standard);
    let mut temp = Temp::new(p.TEMP, Irqs);
    let mut uart = Uarte::new(p.UARTE0, p.P0_06, p.P0_08, Irqs, uarte::Config::default());
    let mut buffer = [0; 8];

    // Uarte::new leaves the peripheral active; both schedules require Off at
    // entry and restore Off before returning.
    uart.suspend();

    for _ in 0..WARMUP_ITERATIONS {
        iteration(&mut temp, &mut uart, &mut buffer).await;
    }

    // Make the start edge easy to identify after reset and initialization.
    Timer::after_secs(2).await;
    measurement_gate.set_high();
    for _ in 0..MEASURED_ITERATIONS {
        iteration(&mut temp, &mut uart, &mut buffer).await;
    }
    measurement_gate.set_low();

    // Leave a quiet, recognizable tail in the capture and never repeat the run.
    loop {
        cortex_m::asm::wfe();
    }
}
