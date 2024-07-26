use bitbuffer::{BitError, BitRead, BitReadStream, BitWrite, BitWriteStream, Endianness};
use num_traits::{
    Bounded, ConstOne, ConstZero, FromPrimitive, Num, NumCast, One, ToPrimitive, Unsigned, Zero,
};
use std::{
    fmt::Display,
    num::IntErrorKind,
    ops::{Add, Deref, Div, Mul, Rem, Sub},
    str::FromStr,
};

use crate::Error;

/// Represents a 5 bit unsigned integer to be used in arguments of [`AwaTism`].
#[allow(non_camel_case_types)]
#[rustc_layout_scalar_valid_range_end(0b11111)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct u5(u8);
impl u5 {
    // SAFETY: 2 is a valid 5 bit number
    pub const TWO: u5 = unsafe { u5(2) };
    /// # Safety
    /// `value` has to be a valid 5 bit number
    #[inline(always)]
    pub const unsafe fn new_unchecked(value: u8) -> Self {
        u5(value)
    }
}
impl TryFrom<u8> for u5 {
    type Error = Error;
    #[inline]
    fn try_from(value: u8) -> Result<Self, Self::Error> {
        if value > 0b11111 {
            return Err(Error::OutOfBounds(5));
        }
        // SAFETY: value fits into 5 bits here
        Ok(unsafe { Self(value) })
    }
}
impl FromStr for u5 {
    type Err = Error;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let value = s.parse::<u8>()?;
        Self::try_from(value)
    }
}
impl Deref for u5 {
    type Target = u8;
    #[inline(always)]
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
impl<'a, E: Endianness> BitRead<'a, E> for u5 {
    #[inline]
    fn read(stream: &mut BitReadStream<'a, E>) -> Result<Self, BitError> {
        Ok(unsafe { Self(stream.read_int(5)?) })
    }
    #[inline(always)]
    fn bit_size() -> Option<usize> {
        Some(5)
    }
}
impl<E: Endianness> BitWrite<E> for u5 {
    #[inline(always)]
    fn write(&self, stream: &mut BitWriteStream<E>) -> Result<(), BitError> {
        stream.write_int(self.0, 5)
    }
}
impl Display for u5 {
    #[inline(always)]
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

macro_rules! impl_op {
    ($trait:ident, $fun:ident) => {
        impl $trait for u5 {
            type Output = u5;
            #[inline]
            fn $fun(self, rhs: Self) -> Self::Output {
                #[allow(clippy::suspicious_arithmetic_impl)]
                let result = self.0.$fun(rhs.0) & 0b11111;
                // SAFETY: result is masked to 5 bits
                unsafe { u5(result) }
            }
        }
    };
}
impl_op!(Add, add);
impl_op!(Sub, sub);
impl_op!(Mul, mul);
impl_op!(Div, div);
impl_op!(Rem, rem);

impl Zero for u5 {
    #[inline(always)]
    fn is_zero(&self) -> bool {
        self.0 == 0
    }
    #[inline(always)]
    fn zero() -> Self {
        // SAFETY: 0 is a valid 5 bit number
        unsafe { u5(0) }
    }
}
impl ConstZero for u5 {
    // SAFETY: 0 is a valid 5 bit number
    const ZERO: u5 = unsafe { u5(0) };
}
impl One for u5 {
    #[inline(always)]
    fn one() -> Self {
        // SAFETY: 1 is a valid 5 bit number
        unsafe { u5(1) }
    }
}
impl ConstOne for u5 {
    // SAFETY: 0 is a valid 5 bit number
    const ONE: u5 = unsafe { u5(1) };
}
impl Num for u5 {
    type FromStrRadixErr = Error;
    #[inline]
    fn from_str_radix(str: &str, radix: u32) -> Result<Self, Self::FromStrRadixErr> {
        match u8::from_str_radix(str, radix) {
            Err(error) if *error.kind() == IntErrorKind::PosOverflow => Err(Error::OutOfBounds(5)),
            Err(error) => Err(error.into()),
            Ok(num) if num >= 32 => Err(Error::OutOfBounds(5)),
            // SAFETY: num is a 5 bits number here
            Ok(num) => Ok(unsafe { u5(num) }),
        }
    }
}
impl Bounded for u5 {
    #[inline(always)]
    fn min_value() -> Self {
        // SAFETY: 0 is a valid 5 bit number
        unsafe { u5(0) }
    }
    #[inline(always)]
    fn max_value() -> Self {
        // SAFETY: 31 is a valid 5 bit number
        unsafe { u5(31) }
    }
}
impl Unsigned for u5 {}
impl ToPrimitive for u5 {
    #[inline(always)]
    fn to_i8(&self) -> Option<i8> {
        Some(self.0 as i8)
    }
    #[inline(always)]
    fn to_u8(&self) -> Option<u8> {
        Some(self.0)
    }
    #[inline(always)]
    fn to_i64(&self) -> Option<i64> {
        Some(self.0 as i64)
    }
    #[inline(always)]
    fn to_u64(&self) -> Option<u64> {
        Some(self.0 as u64)
    }
}
impl FromPrimitive for u5 {
    #[inline]
    fn from_i8(n: i8) -> Option<Self> {
        match n {
            0..=31 => Some(unsafe { u5(n as u8) }),
            _ => None,
        }
    }
    #[inline]
    fn from_u8(n: u8) -> Option<Self> {
        match n {
            0..=31 => Some(unsafe { u5(n) }),
            _ => None,
        }
    }
    #[inline]
    fn from_i64(n: i64) -> Option<Self> {
        match n {
            0..=31 => Some(unsafe { u5(n as u8) }),
            _ => None,
        }
    }
    #[inline]
    fn from_u64(n: u64) -> Option<Self> {
        match n {
            0..=31 => Some(unsafe { u5(n as u8) }),
            _ => None,
        }
    }
}
impl NumCast for u5 {
    #[inline]
    fn from<T: ToPrimitive>(n: T) -> Option<Self> {
        let num = n.to_u8()?;
        if num >= 32 {
            return None;
        }
        // SAFETY: num is a 5 bit number here
        Some(unsafe { u5(num) })
    }
}
