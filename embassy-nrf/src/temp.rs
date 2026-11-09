//! Builtin temperature sensor driver.

use core::future::poll_fn;
use core::task::Poll;

use embassy_sync::waitqueue::AtomicWaker;

use abacus_registers::{heliod, process_register_block};

use crate::interrupt::InterruptExt;
use crate::peripherals::TEMP;
use crate::{Peri, interrupt, pac};

#[process_register_block(
    peripheral_name = "Nrf5xTemp",
    backend = external(pac::temp::Temp),
    states = [
        (Off, cost: temp_off),
        (Reading, *T*, cost: temp_reading)
    ],
    transitions = [Off => Reading, Reading => Off]
)]
struct Nrf5xTempModel;

#[heliod(Nrf5xTempCost)]
mod checked {
    use super::*;
    use abacus_registers::heliod_operation;

    #[heliod_operation(
        peripheral = super::Nrf5xTempCost,
        from = Off,
        to = Reading,
        cost = temp_start
    )]
    pub(super) fn start(registers: Nrf5xTempRegisters<Off>) -> Nrf5xTempRegisters<Reading> {
        let peripheral = *registers.peripheral();
        peripheral.intenset().write(|w| w.set_datardy(true));
        peripheral.tasks_start().write_value(1);
        registers.into_reading()
    }

    #[heliod_operation(
        peripheral = super::Nrf5xTempCost,
        from = Reading,
        to = Reading,
        cost = temp_interrupt
    )]
    pub(super) fn handle_interrupt() {
        pac::TEMP.intenclr().write(|w| w.set_datardy(true));
        WAKER.wake();
    }

    #[heliod_operation(
        peripheral = super::Nrf5xTempCost,
        from = Reading,
        to = Reading,
        cost = temp_poll
    )]
    pub(super) fn data_ready(registers: &Nrf5xTempRegisters<Reading>) -> bool {
        registers.peripheral().events_datardy().read() != 0
    }

    #[heliod_operation(
        peripheral = super::Nrf5xTempCost,
        from = Reading,
        to = Off,
        cost = temp_complete
    )]
    pub(super) fn complete(registers: Nrf5xTempRegisters<Reading>) -> (Nrf5xTempRegisters<Off>, i32) {
        let peripheral = *registers.peripheral();
        peripheral.events_datardy().write_value(0);
        let value = peripheral.temp().read() as i32;
        (registers.into_off(), value)
    }

    #[heliod_operation(
        peripheral = super::Nrf5xTempCost,
        from = Reading,
        to = Off,
        cost = temp_cancel
    )]
    pub(super) fn cancel(registers: Nrf5xTempRegisters<Reading>) -> Nrf5xTempRegisters<Off> {
        let peripheral = *registers.peripheral();
        peripheral.tasks_stop().write_value(1);
        peripheral.events_datardy().write_value(0);
        registers.into_off()
    }
}

impl abacus_registers::SyncState for Nrf5xTempRegisters<Reading> {
    type SyncStateEnum = Nrf5xTempStateEnum;

    fn sync_state(self) -> Self::SyncStateEnum {
        checked::cancel(self).into()
    }
}

/// Interrupt handler.
pub struct InterruptHandler {
    _private: (),
}

impl interrupt::typelevel::Handler<interrupt::typelevel::TEMP> for InterruptHandler {
    unsafe fn on_interrupt() {
        checked::handle_interrupt();
    }
}

/// Builtin temperature sensor driver.
pub struct Temp<'d> {
    _peri: Peri<'d, TEMP>,
    registers: Option<Nrf5xTempRegisters<Off>>,
}

static WAKER: AtomicWaker = AtomicWaker::new();

impl<'d> Temp<'d> {
    /// Create a new temperature sensor driver.
    pub fn new(
        _peri: Peri<'d, TEMP>,
        _irq: impl interrupt::typelevel::Binding<interrupt::typelevel::TEMP, InterruptHandler> + 'd,
    ) -> Self {
        // Enable interrupt that signals temperature values
        interrupt::TEMP.unpend();
        unsafe { interrupt::TEMP.enable() };

        Self {
            _peri,
            // SAFETY: `Peri` provides exclusive ownership and a newly claimed
            // TEMP peripheral is not performing a measurement.
            registers: Some(unsafe { Nrf5xTempRegisters::new(pac::TEMP) }),
        }
    }

    /// Perform an asynchronous temperature measurement. The returned future
    /// can be awaited to obtain the measurement.
    ///
    /// The result is in units of 0.25 °C, i.e. divide by 4 to get °C.
    ///
    /// If the future is dropped, the measurement is cancelled.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use embassy_nrf::{bind_interrupts, temp};
    /// use embassy_nrf::temp::Temp;
    ///
    /// bind_interrupts!(struct Irqs {
    ///     TEMP => temp::InterruptHandler;
    /// });
    ///
    /// # async {
    /// # let p: embassy_nrf::Peripherals = todo!();
    /// let mut t = Temp::new(p.TEMP, Irqs);
    /// let celsius: i32 = t.read_celsius_x4().await / 4;
    /// # };
    /// ```
    pub async fn read_celsius_x4(&mut self) -> i32 {
        let off = self
            .registers
            .take()
            .expect("temperature measurement already in progress");
        let reading = ReadingGuard {
            registers: Some(checked::start(off)),
            destination: &mut self.registers,
        };

        poll_fn(|cx| {
            WAKER.register(cx.waker());
            if !checked::data_ready(reading.registers()) {
                Poll::Pending
            } else {
                Poll::Ready(())
            }
        })
        .await;
        reading.complete()
    }
}

struct ReadingGuard<'a> {
    registers: Option<Nrf5xTempRegisters<Reading>>,
    destination: &'a mut Option<Nrf5xTempRegisters<Off>>,
}

impl ReadingGuard<'_> {
    fn registers(&self) -> &Nrf5xTempRegisters<Reading> {
        self.registers.as_ref().unwrap()
    }

    fn complete(mut self) -> i32 {
        let (registers, value) = checked::complete(self.registers.take().unwrap());
        *self.destination = Some(registers);
        value
    }
}

impl Drop for ReadingGuard<'_> {
    fn drop(&mut self) {
        if let Some(registers) = self.registers.take() {
            *self.destination = Some(checked::cancel(registers));
        }
    }
}
