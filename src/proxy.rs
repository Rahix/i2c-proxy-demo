use core::cell;

use embedded_hal::blocking::i2c;
use cortex_m;

/// Abstraction over synchronisation primitves
///
/// On a system where std is available, this is implemented by
/// `std::sync::Mutex` (although this implementation
/// does not work across threads), on a bare metal system it could be
/// `cortex_m::interrupt::Mutex`
pub trait BusMutex<'a, T> {
    fn create(v: T) -> Self;
    fn lock<R, F: FnOnce(&T) -> R>(&'a self, f: F) -> R;
}

/*
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

impl<'a, T: 'a> BusMutex<'a, T> for cortex_m::interrupt::Mutex<T> {
    fn create(v: T) -> Self {
        cortex_m::interrupt::Mutex::new(v)
    }

    fn lock<R, F: FnOnce(&T) -> R>(&'a self, f: F) -> R {
        cortex_m::interrupt::free(|cs| {
            let v = self.borrow(cs);
            f(v)
        })
    }
}

/*
impl<'a, T: 'a> BusMutex<'a, T> for ::std::sync::Mutex<T> {
    type Guard = ::std::sync::MutexGuard<'a, T>;

    fn create(v: T) -> Self {
        ::std::sync::Mutex::new(v)
    }

    fn lock<R, F: FnOnce(Self::Guard) -> R>(&'a self, f: F) -> R {
        let v = self.lock().unwrap();
        f(v)
    }
}
*/

/// The Bus Manager
///
/// The bus-manager is created like this:
///
/// ```
/// let i2c = interface::DummyI2c;
///
/// let bus = proxy::I2cBusManager::<::std::sync::Mutex<_>, _>::new(i2c);
/// ```
pub struct I2cBusManager<'a, M: BusMutex<'a, cell::RefCell<T>>, T: 'a>(
    M,
    &'a ::core::marker::PhantomData<T>,
);

impl<'a, M: BusMutex<'a, cell::RefCell<T>>, T: 'a> I2cBusManager<'a, M, T> {
    pub fn new(i: T) -> I2cBusManager<'a, M, T> {
        let mutex = M::create(cell::RefCell::new(i));

        I2cBusManager(mutex, &::core::marker::PhantomData)
    }

    pub fn acquire<'b>(&'a self) -> I2cProxy<'a, 'b, M, T> {
        I2cProxy(&self.0, &::core::marker::PhantomData)
    }
}

/// A Proxy that implements the I2C traits from `embedded_hal`
///
/// The proxy is crated like this:
///
/// ```
/// let device = MyI2cDevice::new(bus.acquire());
/// ```
pub struct I2cProxy<'a, 'b, M: 'a + BusMutex<'a, cell::RefCell<T>>, T: 'b>(
    &'a M,
    &'b ::core::marker::PhantomData<T>,
)
where
    'a: 'b;

impl<'a, 'b, M: BusMutex<'a, cell::RefCell<T>>, T: i2c::Write + 'b> i2c::Write
    for I2cProxy<'a, 'b, M, T>
{
    type Error = T::Error;

    fn write(&mut self, addr: u8, bytes: &[u8]) -> Result<(), Self::Error> {
        self.0.lock(|lock| {
            let mut i = lock.borrow_mut();
            i.write(addr, bytes)
        })
    }
}

impl<'a, 'b, M: BusMutex<'a, cell::RefCell<T>>, T: i2c::Read + 'b> i2c::Read
    for I2cProxy<'a, 'b, M, T>
{
    type Error = T::Error;

    fn read(&mut self, address: u8, buffer: &mut [u8]) -> Result<(), Self::Error> {
        self.0.lock(|lock| {
            let mut i = lock.borrow_mut();
            i.read(address, buffer)
        })
    }
}

impl<'a, 'b, M: BusMutex<'a, cell::RefCell<T>>, T: i2c::WriteRead + 'b> i2c::WriteRead
    for I2cProxy<'a, 'b, M, T>
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
