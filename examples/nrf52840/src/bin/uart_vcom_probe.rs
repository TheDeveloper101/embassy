#![no_std]
#![no_main]

use defmt::info;
use embassy_executor::Spawner;
use embassy_nrf::uarte::{self, Uarte};
use embassy_time::Timer;
use {defmt_rtt as _, panic_probe as _};

#[embassy_executor::main]
async fn main(_spawner: Spawner) {
    let p = embassy_nrf::init(Default::default());
    let mut config = uarte::Config::default();
    config.baudrate = uarte::Baudrate::Baud115200;
    config.parity = uarte::Parity::Excluded;

    // The blocking API bypasses the Heliod async UART implementation. P0.06
    // and P0.08 are the nRF52840 DK interface MCU's VCOM0 TX and RX pins.
    let mut uart = Uarte::new_blocking(p.UARTE0, p.P0_06, p.P0_08, config);

    let mut sequence = 0u32;
    loop {
        info!("sending blocking UART VCOM probe {}", sequence);
        uart.blocking_write(b"UART VCOM PROBE\r\n").unwrap();
        sequence = sequence.wrapping_add(1);
        Timer::after_secs(1).await;
    }
}
