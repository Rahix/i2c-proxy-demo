#![no_main]
#![no_std]

extern crate f3;
extern crate cortex_m;
extern crate panic_abort;
extern crate embedded_hal;
extern crate lsm303dlhc;

#[macro_use(entry, exception)]
extern crate cortex_m_rt;

mod proxy;

use cortex_m::asm;

use f3::hal::prelude::*;
use f3::hal::stm32f30x;

// the program entry point is ...
entry!(main);

// ... this never ending function
fn main() -> ! {
    let cp = cortex_m::Peripherals::take().unwrap();
    let dp = stm32f30x::Peripherals::take().unwrap();

    let mut flash = dp.FLASH.constrain();
    let mut rcc = dp.RCC.constrain();

    let clocks = rcc.cfgr.freeze(&mut flash.acr);

    let mut gpiob = dp.GPIOB.split(&mut rcc.ahb);
    let scl = gpiob.pb6.into_af4(&mut gpiob.moder, &mut gpiob.afrl);
    let sda = gpiob.pb7.into_af4(&mut gpiob.moder, &mut gpiob.afrl);

    let i2c = f3::hal::i2c::I2c::i2c1(dp.I2C1, (scl, sda), 400.khz(), clocks, &mut rcc.apb1);

    let bus = proxy::I2cBusManager::<cortex_m::interrupt::Mutex<_>, _>::new(i2c);

    let a = bus.acquire();
    let b = bus.acquire();

    let sensor = lsm303dlhc::Lsm303dlhc::new(a);

    loop {
        asm::bkpt();
    }
}

exception!(HardFault, hard_fault);

fn hard_fault(_ef: &cortex_m_rt::ExceptionFrame) -> ! {
    asm::bkpt();

    loop {}
}

exception!(*, default_handler);

fn default_handler(_irqn: i16) {
    loop {}
}
