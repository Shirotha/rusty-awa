use bitbuffer::{BitError, BitRead, BitReadStream, BitWrite, BitWriteStream, Endianness};
use std::{cell::LazyCell, fmt::Display, ops::Deref};

use crate::Error;

/// Represents a chatacter encoded in the 6 bit AwaSCII character set.
#[rustc_layout_scalar_valid_range_end(0b111111)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct AwaSCII(u8);
impl AwaSCII {
    const TO_ASCII: [u8; 64] = [
        b'A', b'W', b'a', b'w', b'J', b'E', b'L', b'Y', b'H', b'O', b'S', b'I', b'U', b'M', b'j',
        b'e', b'l', b'y', b'h', b'o', b's', b'i', b'u', b'm', b'P', b'C', b'N', b'T', b'p', b'c',
        b'n', b't', b'B', b'D', b'F', b'G', b'R', b'b', b'd', b'f', b'g', b'r', b'0', b'1', b'2',
        b'3', b'4', b'5', b'6', b'7', b'8', b'9', b' ', b'.', b',', b'!', b'`', b'(', b')', b'~',
        b'_', b'/', b';', b'\n',
    ];
    #[allow(clippy::declare_interior_mutable_const)]
    const FROM_ASCII: LazyCell<[u8; 128]> = LazyCell::new(|| {
        let mut t = [255; 128];
        for (awascii, ascii) in Self::TO_ASCII.iter().enumerate() {
            t[*ascii as usize] = awascii as u8;
        }
        t
    });
    /// Create a new character from its character code.
    /// # Safety
    /// `awascii` has to be a valid 6 bit number
    #[inline(always)]
    pub const unsafe fn new_unchecked(awascii: u8) -> Self {
        AwaSCII(awascii)
    }
    #[inline]
    pub fn new(awascii: u8) -> Option<Self> {
        if awascii >= 32 {
            return None;
        }
        // SAFETY: awascii is a valid 6 bit number here
        Some(unsafe { AwaSCII(awascii) })
    }
    /// Create a new chracter from an ASCII character, when a chatacter cannot be represented in AwaSCII `None` will be returned.
    #[inline]
    pub fn from_ascii(ascii: u8) -> Option<Self> {
        #[allow(clippy::borrow_interior_mutable_const)]
        let awascii = (*Self::FROM_ASCII)[ascii as usize];
        if awascii == 255 {
            return None;
        }
        // SAFETY: FROM_ASCII only contains valid AwaSCII characters
        Some(unsafe { Self(awascii) })
    }
    /// Return the matching ASCII chatacter.
    #[inline]
    pub const fn to_ascii(&self) -> u8 {
        Self::TO_ASCII[self.0 as usize]
    }
}
impl Deref for AwaSCII {
    type Target = u8;
    #[inline(always)]
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
impl TryFrom<u8> for AwaSCII {
    type Error = Error;
    #[inline]
    fn try_from(value: u8) -> Result<Self, Self::Error> {
        if value > 0b111111 {
            return Err(Error::OutOfBounds(6));
        }
        // SAFETY: value fits into 6 bits here
        Ok(unsafe { Self(value) })
    }
}
impl<'a, E: Endianness> BitRead<'a, E> for AwaSCII {
    #[inline]
    fn read(stream: &mut BitReadStream<'a, E>) -> Result<Self, BitError> {
        Ok(unsafe { Self(stream.read_int(6)?) })
    }
    #[inline(always)]
    fn bit_size() -> Option<usize> {
        Some(6)
    }
}
impl<E: Endianness> BitWrite<E> for AwaSCII {
    #[inline(always)]
    fn write(&self, stream: &mut BitWriteStream<E>) -> Result<(), BitError> {
        stream.write_int(self.0, 6)
    }
}
impl Display for AwaSCII {
    #[inline(always)]
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        (self.to_ascii() as char).fmt(f)
    }
}
