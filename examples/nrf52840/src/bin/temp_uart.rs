#![no_std]
#![no_main]

use abacus_registers::{heliod, heliod_ignore, heliod_operation};
use defmt::info;
use embassy_executor::Spawner;
use embassy_nrf::mode::Async;
use embassy_nrf::peripherals::UARTE0;
use embassy_nrf::temp::{Nrf5xTempCost, Temp};
use embassy_nrf::uarte::{Nrf52UarteCost, Uarte};
use embassy_nrf::{bind_interrupts, temp, uarte};
use {defmt_rtt as _, panic_probe as _};

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
mod checked {
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

    fn short_independent_work(called: usize) {
        let _step = 1 + 1;
        heliod_ignore!(info!("called this many times: {}", called));
    }

    /// Intentionally keeps the UART active during an unrelated temperature read.
    pub async fn inefficient_iteration(temp: &mut Temp<'_>, uart: &mut Uarte<'_, Async>, buffer: &mut [u8; 8]) {
        resume(uart);
        let value = read_temperature(temp).await;
        heliod_ignore!(format_temperature_x4(value, buffer));
        write(uart, buffer).await;
        suspend(uart);
    }

    /// Delays UART activation until the temperature result is ready to send.
    pub async fn optimized_iteration(temp: &mut Temp<'_>, uart: &mut Uarte<'_, Async>, buffer: &mut [u8; 8]) {
        let value = read_temperature(temp).await;
        heliod_ignore!(format_temperature_x4(value, buffer));
        resume(uart);
        write(uart, buffer).await;
        suspend(uart);
    }

    /// Keeps the UART active after its last use while another measurement runs.
    pub async fn late_suspend_iteration(temp: &mut Temp<'_>, uart: &mut Uarte<'_, Async>, buffer: &mut [u8; 8]) {
        let value = read_temperature(temp).await;
        heliod_ignore!(format_temperature_x4(value, buffer));
        resume(uart);
        write(uart, buffer).await;
        let _unused = read_temperature(temp).await;
        suspend(uart);
    }

    /// Suspends the UART immediately after its last use.
    pub async fn optimized_late_suspend_iteration(
        temp: &mut Temp<'_>,
        uart: &mut Uarte<'_, Async>,
        buffer: &mut [u8; 8],
    ) {
        let value = read_temperature(temp).await;
        heliod_ignore!(format_temperature_x4(value, buffer));
        resume(uart);
        write(uart, buffer).await;
        suspend(uart);
        let _unused = read_temperature(temp).await;
    }

    /// Tears down and restarts the UART across one short independent command.
    pub async fn unbatched_writes(uart: &mut Uarte<'_, Async>, buffer: &[u8; 8]) {
        let called = 0;
        resume(uart);
        write(uart, buffer).await;
        suspend(uart);
        short_independent_work(called);
        resume(uart);
        write(uart, buffer).await;
        suspend(uart);
        let called = called + 1;
    }

    /// Keeps one active UART session across the short independent command.
    pub async fn batched_writes(uart: &mut Uarte<'_, Async>, buffer: &[u8; 8]) {
        let called = 0;
        resume(uart);
        write(uart, buffer).await;
        short_independent_work(called);
        write(uart, buffer).await;
        suspend(uart);
        let called = called + 1;
    }

    pub async fn optimized_loop(temp: &mut Temp<'_>, uart: &mut Uarte<'_, Async>, buffer: &mut [u8; 8]) {
        loop {
            optimized_iteration(temp, uart, buffer).await;
        }
    }
}

#[embassy_executor::main]
async fn main(_spawner: Spawner) {
    let p = embassy_nrf::init(Default::default());
    let mut temp = Temp::new(p.TEMP, Irqs);
    let mut uart = Uarte::new(p.UARTE0, p.P0_06, p.P0_08, Irqs, uarte::Config::default());
    let mut buffer = [0; 8];

    info!("sending UART probe");
    uart.write(b"UART OK\r\n").await.unwrap();
    info!("UART probe completed");

    // Construction leaves the UART active. The evaluated schedules both begin
    // in Off so their transition placement can be compared directly.
    uart.suspend();

    info!("running intentionally inefficient schedule once");
    checked::inefficient_iteration(&mut temp, &mut uart, &mut buffer).await;

    info!("running late-suspend schedule once");
    checked::late_suspend_iteration(&mut temp, &mut uart, &mut buffer).await;

    info!("running optimized late-suspend schedule once");
    checked::optimized_late_suspend_iteration(&mut temp, &mut uart, &mut buffer).await;

    info!("running unbatched-write schedule once");
    checked::unbatched_writes(&mut uart, &buffer).await;

    info!("running batched-write schedule once");
    checked::batched_writes(&mut uart, &buffer).await;

    info!("running optimized schedule");
    checked::optimized_loop(&mut temp, &mut uart, &mut buffer).await;
}
