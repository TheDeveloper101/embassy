#![no_std]
#![no_main]

use embassy_executor::Spawner;
use embassy_nrf::gpio::{Level, Output, OutputDrive};
use embassy_nrf::mode::Async;
use embassy_nrf::peripherals::UARTE0;
use embassy_nrf::temp::Temp;
use embassy_nrf::uarte::Uarte;
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

async fn iteration(temp: &mut Temp<'_>, uart: &mut Uarte<'_, Async>, buffer: &mut [u8; 8]) {
    let value = temp.read_celsius_x4().await;
    format_temperature_x4(value, buffer);
    uart.write(buffer).await.unwrap();
}

#[embassy_executor::main]
async fn main(_spawner: Spawner) {
    let p = embassy_nrf::init(Default::default());
    let mut measurement_gate = Output::new(p.P0_03, Level::Low, OutputDrive::Standard);
    let mut temp = Temp::new(p.TEMP, Irqs);
    let mut uart = Uarte::new(p.UARTE0, p.P0_06, p.P0_08, Irqs, uarte::Config::default());
    let mut buffer = [0; 8];

    for _ in 0..WARMUP_ITERATIONS {
        iteration(&mut temp, &mut uart, &mut buffer).await;
    }

    Timer::after_secs(2).await;
    measurement_gate.set_high();
    for _ in 0..MEASURED_ITERATIONS {
        iteration(&mut temp, &mut uart, &mut buffer).await;
    }
    measurement_gate.set_low();

    loop {
        cortex_m::asm::wfe();
    }
}
