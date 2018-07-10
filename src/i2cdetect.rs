#![no_main]
#![no_std]

extern crate f3;
#[macro_use]
extern crate cortex_m;
extern crate embedded_hal;
extern crate panic_abort;

#[macro_use(entry, exception)]
extern crate cortex_m_rt;

use core::cell;

use cortex_m::asm;

use f3::hal::prelude::*;
use f3::hal::stm32f30x;

entry!(main);

fn main() -> ! {
    let cp = cortex_m::Peripherals::take().unwrap();
    let dp = stm32f30x::Peripherals::take().unwrap();

    let mut flash = dp.FLASH.constrain();
    let mut rcc = dp.RCC.constrain();
    let mut itm = cp.ITM;

    let clocks = rcc.cfgr.freeze(&mut flash.acr);

    let delay = f3::hal::delay::Delay::new(cp.SYST, clocks);
    let delay = cell::RefCell::new(delay);

    let mut gpiob = dp.GPIOB.split(&mut rcc.ahb);
    let scl = gpiob.pb6.into_af4(&mut gpiob.moder, &mut gpiob.afrl);
    let sda = gpiob.pb7.into_af4(&mut gpiob.moder, &mut gpiob.afrl);

    let i2c = f3::hal::i2c::I2c::i2c1(dp.I2C1, (scl, sda), 90.khz(), clocks, &mut rcc.apb1);

    let (i2c, _pins) = i2c.free();

    iprintln!(
        &mut itm.stim[0],
        "\
i2cdetect util for bare-metal
-----------
   _0 _1 _2 _3 _4 _5 _6 _7 _8 _9 _A _B _C _D _E _F"
    );

    for addr in 0x00..0x78 {
        if addr & 0xf == 0x0 {
            iprint!(&mut itm.stim[0], "{:1X}_ ", addr >> 4);
        }
        if addr < 0x04 {
            iprint!(&mut itm.stim[0], "   ");
            continue;
        }
        i2c.cr2.write(|w| {
            w.start().set_bit();
            w.sadd1().bits(addr);
            w.rd_wrn().clear_bit();
            w.nbytes().bits(1);
            w.autoend().clear_bit();
            w
        });

        loop {
            let isr = i2c.isr.read();
            if !isr.busy().bit() {
                // Not found
                iprint!(&mut itm.stim[0], "-- ");
                break;
            }
            if isr.txis().bit() {
                // Found
                iprint!(&mut itm.stim[0], "{:02X} ", addr);
                // Reset
                i2c.txdr.write(|w| w.txdata().bits(0x00));
                while i2c.isr.read().tc().bit_is_clear() {}
                break;
            }
        }

        if (addr & 0xf) == 0xf {
            iprint!(&mut itm.stim[0], "\n");
        }

        delay.borrow_mut().delay_ms(100u16);
    }
    iprint!(&mut itm.stim[0], "\n");

    asm::bkpt();

    loop {
        asm::wfi();
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
