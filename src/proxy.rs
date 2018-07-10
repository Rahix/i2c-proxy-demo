use core::cell;

use cortex_m;
use embedded_hal::blocking::i2c;

/// Abstraction over synchronisation primitves
///
/// On a system where std is available, this is implemented by
/// `std::sync::Mutex` (although this implementation
/// does not allow sharing across threads), on a bare metal
/// system it could be `cortex_m::interrupt::Mutex`
pub trait BusMutex<T> {
    /// Create a mutex from the given value
    fn create(v: T) -> Self;

    /// Lock the mutex and allow access to the value inside
    /// the given closure
    fn lock<R, F: FnOnce(&T) -> R>(&self, f: F) -> R;
}

/*
/// Implementation for `std::sync::Mutex`
impl<'a, T: 'a> BusMutex<'a, T> for ::std::sync::Mutex<T> {
    fn create(v: T) -> Self {
        ::std::sync::Mutex::new(v)
    }

    fn lock<R, F: FnOnce(&T) -> R>(&'a self, f: F) -> R {
        let v = self.lock().unwrap();
        f(&v)
    }
}
*/

/// Implementation for `cortex_m::interrupt::Mutex`
impl<T> BusMutex<T> for cortex_m::interrupt::Mutex<T> {
    fn create(v: T) -> Self {
        cortex_m::interrupt::Mutex::new(v)
    }

    fn lock<R, F: FnOnce(&T) -> R>(&self, f: F) -> R {
        cortex_m::interrupt::free(|cs| {
            let v = self.borrow(cs);
            f(v)
        })
    }
}

/// The Bus Manager
///
/// The bus manager is the owner of the i2c peripheral. It has to stay alive
/// longer than all devices making use of the bus.
///
/// When creating the bus manager, you have to specify what mutex you want
/// to use:
///
/// ```
/// let bus = I2cBusManager::<cortex_m::interrupt::Mutex<_>, _>::new(i2c);
/// ```
pub struct I2cBusManager< M: BusMutex<cell::RefCell<T>>, T>(M, ::core::marker::PhantomData<T>);

impl<M: BusMutex<cell::RefCell<T>>, T:> I2cBusManager<M, T> {
    /// Create a new I2C bus manager from the given peripheral
    pub fn new(i: T) -> I2cBusManager<M, T> {
        let mutex = M::create(cell::RefCell::new(i));

        I2cBusManager(mutex, ::core::marker::PhantomData)
    }

    /// Acquire an instance of this bus for a device
    ///
    /// This instance will implement the i2c traits
    pub fn acquire<'a>(&'a self) -> I2cProxy<'a, M, T> {
        I2cProxy(&self.0, ::core::marker::PhantomData)
    }
}

/// A Proxy that implements the I2C traits from `embedded_hal`
///
/// The proxy is crated like this:
///
/// ```
/// let device = MyI2cDevice::new(bus.acquire());
/// ```
pub struct I2cProxy<'a, M: 'a + BusMutex<cell::RefCell<T>>, T>(
    &'a M,
    ::core::marker::PhantomData<T>,
);

impl<'a, M: 'a + BusMutex<cell::RefCell<T>>, T: i2c::Write> i2c::Write
    for I2cProxy<'a, M, T>
{
    type Error = T::Error;

    fn write(&mut self, addr: u8, bytes: &[u8]) -> Result<(), Self::Error> {
        self.0.lock(|lock| {
            let mut i = lock.borrow_mut();
            i.write(addr, bytes)
        })
    }
}

impl<'a, M: 'a + BusMutex<cell::RefCell<T>>, T: i2c::Read> i2c::Read
    for I2cProxy<'a, M, T>
{
    type Error = T::Error;

    fn read(&mut self, address: u8, buffer: &mut [u8]) -> Result<(), Self::Error> {
        self.0.lock(|lock| {
            let mut i = lock.borrow_mut();
            i.read(address, buffer)
        })
    }
}

impl<'a, M: 'a + BusMutex<cell::RefCell<T>>, T: i2c::WriteRead> i2c::WriteRead
    for I2cProxy<'a, M, T>
{
    type Error = T::Error;

    fn write_read(
        &mut self,
        address: u8,
        bytes: &[u8],
        buffer: &mut [u8],
    ) -> Result<(), Self::Error> {
        self.0.lock(|lock| {
            let mut i = lock.borrow_mut();
            i.write_read(address, bytes, buffer)
        })
    }
}
