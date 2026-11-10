#![no_std]
#![no_main]

use defmt::*;
use defmt_rtt as _;
use embassy_executor::Spawner;
use embassy_nrf::{bind_interrupts, peripherals, uarte};
use panic_probe as _;

use abacus_registers::{heliod, heliod_ignore, heliod_operation};
use embassy_nrf::uarte::Nrf52UarteCost;

bind_interrupts!(struct Irqs {
    UARTE0 => uarte::InterruptHandler<peripherals::UARTE0>;
});

#[heliod(Nrf52UarteCost)]
mod checked {
    use super::*;

    #[heliod_operation(
        peripheral = Nrf52UarteCost,
        from = ActiveIdle,
        to = ActiveIdle,
        cost = 131091,
        background_steps = 32773
    )]
    async fn read(uart: &mut uarte::Uarte<'_, embassy_nrf::mode::Async>, buffer: &mut [u8]) {
        unwrap!(uart.read(buffer).await);
    }

    #[heliod_operation(
        peripheral = Nrf52UarteCost,
        from = ActiveIdle,
        to = ActiveIdle,
        cost = 144,
        background_steps = 36
    )]
    async fn write(uart: &mut uarte::Uarte<'_, embassy_nrf::mode::Async>, buffer: &[u8]) {
        unwrap!(uart.write(buffer).await);
    }

    pub async fn run(uart: &mut uarte::Uarte<'_, embassy_nrf::mode::Async>, buffer: &mut [u8]) {
        loop {
            heliod_ignore!(info!("reading..."));
            read(uart, buffer).await;
            heliod_ignore!(info!("writing..."));
            write(uart, buffer).await;
        }
    }
}

#[embassy_executor::main]
async fn main(_spawner: Spawner) {
    let p = embassy_nrf::init(Default::default());
    let mut config = uarte::Config::default();
    config.parity = uarte::Parity::Excluded;
    config.baudrate = uarte::Baudrate::Baud115200;

    let mut uart = uarte::Uarte::new(p.UARTE0, p.P0_06, p.P0_08, Irqs, config);

    info!("uarte initialized!");

    // Message must be in SRAM
    let mut buf = [0; 8];
    buf.copy_from_slice(b"Hello!\r\n");

    unwrap!(uart.write(&buf).await);
    info!("wrote hello in uart!");
    checked::run(&mut uart, &mut buf).await;
}
