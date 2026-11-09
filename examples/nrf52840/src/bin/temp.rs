#![no_std]
#![no_main]

use defmt::info;
use defmt_rtt as _;
use embassy_executor::Spawner;
use embassy_nrf::temp::Temp;
use embassy_nrf::{bind_interrupts, temp};
use embassy_time::Timer;
use panic_probe as _;

use abacus_registers::{heliod, heliod_ignore, heliod_operation};
use embassy_nrf::temp::Nrf5xTempCost;

bind_interrupts!(struct Irqs {
    TEMP => temp::InterruptHandler;
});

#[heliod(Nrf5xTempCost)]
mod checked {
    use super::*;

    // This is the bound derived for the checked Embassy driver implementation.
    // It is passed across the crate boundary as an opaque operation summary.
    #[heliod_operation(
        peripheral = Nrf5xTempCost,
        from = Off,
        to = Off,
        cost = 62
    )]
    async fn read_celsius_x4(temp: &mut Temp<'_>) -> i32 {
        temp.read_celsius_x4().await
    }

    pub async fn run(temp: &mut Temp<'_>) {
        loop {
            let value = read_celsius_x4(temp).await;
            heliod_ignore!(info!("temperature: {}℃", value as f32 / 4.0));
            heliod_ignore!(Timer::after_secs(1).await);
        }
    }
}

#[embassy_executor::main]
async fn main(_spawner: Spawner) {
    let p = embassy_nrf::init(Default::default());
    let mut temp = Temp::new(p.TEMP, Irqs);

    checked::run(&mut temp).await;
}
