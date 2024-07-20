#![allow(internal_features)]
#![feature(rustc_attrs)]
#![feature(nonzero_internals)]
#![feature(trait_alias)]

use std::num::ParseIntError;

pub use bitbuffer::{
    BigEndian, BitError, BitReadBuffer, BitReadStream, BitWriteStream, Endianness, LittleEndian,
};

mod _u5;
pub use _u5::*;
mod awatism;
pub use awatism::*;
mod awascii;
pub use awascii::*;
mod abyss;
pub use abyss::*;
mod asm;
pub use asm::*;
mod program;
pub use program::*;

use thiserror::Error;

#[derive(Debug, Clone, Error)]
pub enum Error {
    #[error("Value is too big to fit in {0} bits")]
    OutOfBounds(u8),
    #[error("ASCII char {0} has no equivalent AwaSCII char")]
    InvalidAwaSCII(u8),
    #[error(transparent)]
    ParseError(#[from] ParseIntError),
}
