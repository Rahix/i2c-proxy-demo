#![no_main]
#![no_std]

extern crate f3;
#[macro_use]
extern crate cortex_m;
extern crate panic_abort;
extern crate embedded_hal;
extern crate lsm303dlhc;
extern crate pcf8574;

#[macro_use(entry, exception)]
extern crate cortex_m_rt;

mod proxy;

use core::cell;

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
    let mut itm = cp.ITM;

    iprintln!(&mut itm.stim[0], "Start");

    let clocks = rcc.cfgr.freeze(&mut flash.acr);

    let delay = f3::hal::delay::Delay::new(cp.SYST, clocks);
    let delay = cell::RefCell::new(delay);

    let mut gpiob = dp.GPIOB.split(&mut rcc.ahb);
    let scl = gpiob.pb6.into_af4(&mut gpiob.moder, &mut gpiob.afrl);
    let sda = gpiob.pb7.into_af4(&mut gpiob.moder, &mut gpiob.afrl);

    let i2c = f3::hal::i2c::I2c::i2c1(dp.I2C1, (scl, sda), 90.khz(), clocks, &mut rcc.apb1);

    let bus = proxy::I2cBusManager::<cortex_m::interrupt::Mutex<_>, _>::new(i2c);

    iprintln!(&mut itm.stim[0], "Bus is set up");

    let mut lsm = lsm303dlhc::Lsm303dlhc::new(bus.acquire()).unwrap();
    let mut get_accel = || {
        use f3::hal::prelude::*;

        let mut x = 0i32;
        let mut y = 0i32;
        let mut z = 0i32;
        for _ in 0..10 {
            let a = lsm.accel().unwrap();
            x += a.x as i32 / 100;
            y += a.y as i32 / 100;
            z += a.z as i32 / 100;
            delay.borrow_mut().delay_ms(5u16);
        }
        (x / 10, y / 10, z / 10)
    };

    let a = get_accel();
    iprintln!(&mut itm.stim[0], "Acceleration: {} {} {}", a.0, a.1, a.2);

    let mut porta = pcf8574::Pcf8574::new(bus.acquire(), 0x39).unwrap();
    let mut portb = pcf8574::Pcf8574::new(bus.acquire(), 0x38).unwrap();

    let sequence = [0x20, 0x10, 0x02, 0x04, 0x08, 0x40];
    let mut i = 0;

    loop {
        i = (i + 1) % sequence.len();
        porta.set(!sequence[i]).unwrap();
        portb.set(!sequence[(sequence.len()-i) % sequence.len()]).unwrap();

        let a = get_accel();
        iprintln!(&mut itm.stim[0], "Acceleration: {} {} {}", a.0, a.1, a.2);
        delay.borrow_mut().delay_ms((a.2 / 5) as u16);
    }
}

exception!(HardFault, hard_fault);

fn hard_fault(_ef: &cortex_m_rt::ExceptionFrame) -> ! {
    asm::bkpt();

    loop {
        asm::wfi();
    }
}

exception!(*, default_handler);

fn default_handler(_irqn: i16) {
    loop {
        asm::wfi();
    }
}
