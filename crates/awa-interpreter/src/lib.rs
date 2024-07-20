#![feature(const_mut_refs)]

mod iter;
pub use iter::*;

use std::{
    fmt::{Error as FmtError, Write as FmtWrite},
    io::{BufRead, Error as IOError, Write},
    ops::{Add, Div, Mul, Rem, Sub},
};

use num_traits::{cast, ConstOne};
use thiserror::Error;

use awa_core::{u5, Abyss, AwaSCII, AwaTism, Error as CoreError, Program, Value};

#[derive(Debug, Error)]
pub enum Error {
    #[error(transparent)]
    IOError(#[from] IOError),
    #[error("expect input to be a number")]
    NoNumber,
    #[error("expected the abyss to have at least {0} bubbles")]
    NotEnoughBubbles(u5),
    #[error("abyss is full")]
    NoSpace,
    #[error(transparent)]
    FmtError(#[from] FmtError),
    #[error(transparent)]
    CoreError(#[from] CoreError),
    #[error("label with id {0} not found")]
    UnknownLabel(u5),
}

/// Represents location of next instruction to execute.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ContinueAt {
    /// No next instruction, program is done.
    None,
    /// Execute next instruction in order.
    Next,
    /// Skip over next instruction, executing the one after that.
    SkipNext,
    /// Jump to a label.
    Label(u5),
}

/// Convert ASCII string to AwaSCII string.
#[inline]
pub fn parse_awascii_input(src: impl AsRef<str>, out: &mut Vec<AwaSCII>) {
    for char in src.as_ref().chars() {
        if !char.is_ascii() {
            continue;
        };
        let Some(awascii) = AwaSCII::from_ascii(char as u8) else {
            continue;
        };
        out.push(awascii);
    }
}
/// Convert ASCII string to number.
#[inline(always)]
pub fn parse_number_input<T: Value>(src: impl AsRef<str>) -> Option<T> {
    let mut result = T::zero();
    // SAFETY: unwrap: every number type can hold 10
    let ten = cast::<_, T>(10).unwrap();
    let src = src.as_ref();
    for chr in src.chars() {
        match chr {
            '0'..='9' => {
                let digit = (chr as u8) - b'0';
                // SAFETY: unwrap: every number type can hold 10
                result = ten * result + cast(digit).unwrap();
            }
            _ => return Some(result),
        }
    }
    Some(result)
}

/// Represents an instruction interpreter that can run [`AwaTism`]s one at a time.
#[derive(Debug)]
pub struct Interpreter<A: Abyss, I: BufRead, O: Write> {
    abyss: A,
    input: I,
    output: O,
    iobuffer: String,
    awabuffer: Vec<AwaSCII>,
}
impl<A: Abyss, I: BufRead, O: Write> Interpreter<A, I, O> {
    #[inline(always)]
    pub const fn new(abyss: A, input: I, output: O) -> Self {
        Self {
            input,
            output,
            abyss,
            iobuffer: String::new(),
            awabuffer: Vec::new(),
        }
    }
    #[inline(always)]
    pub fn run<'a>(&'a mut self, program: &'a Program) -> Iter<'a, A, I, O> {
        Iter {
            interpreter: self,
            program,
            pc: Some(0),
        }
    }
    #[inline(always)]
    pub fn abyss(&self) -> &A {
        &self.abyss
    }
    #[inline]
    pub fn next(&mut self, awatism: AwaTism) -> Result<ContinueAt, Error> {
        match awatism {
            AwaTism::NoOp => (),
            AwaTism::Print => {
                self.iobuffer.clear();
                match self.abyss.consume(|v| {
                    let awascii = match cast(v) {
                        None => return Err(CoreError::OutOfBounds(6)),
                        Some(v) if v >= 64 => return Err(CoreError::OutOfBounds(6)),
                        // SAFETY: v is a valid 6 bit number here
                        Some(v) => unsafe { AwaSCII::new_unchecked(v) },
                    };
                    self.iobuffer.push(awascii.to_ascii() as char);
                    Ok(())
                })? {
                    Some(_) => {
                        self.output.write_all(self.iobuffer.as_bytes())?;
                        self.output.flush()?;
                    }
                    None => return Err(Error::NotEnoughBubbles(u5::ONE)),
                }
            }
            AwaTism::PrintNum => {
                self.iobuffer.clear();
                let mut first = true;
                match self.abyss.consume::<_, FmtError>(|v| {
                    if first {
                        first = false;
                    } else {
                        self.iobuffer.push(' ');
                    }
                    write!(self.iobuffer, "{}", v)?;
                    Ok(())
                })? {
                    Some(_) => {
                        self.output.write_all(self.iobuffer.as_bytes())?;
                        self.output.flush()?;
                    }
                    None => return Err(Error::NotEnoughBubbles(u5::ONE)),
                }
            }
            AwaTism::Read => {
                self.iobuffer.clear();
                // SAFETY: no limit on read bytes
                let count = self.input.read_line(&mut self.iobuffer)?;
                if count > 0 {
                    self.awabuffer.clear();
                    parse_awascii_input(&self.iobuffer, &mut self.awabuffer);
                    if self.abyss.blow_awascii(&self.awabuffer).is_none() {
                        return Err(Error::NoSpace);
                    }
                }
            }
            AwaTism::ReadNum => {
                self.iobuffer.clear();
                // SAFETY: no limit on read bytes
                let count = self.input.read_line(&mut self.iobuffer)?;
                if count == 0 {
                    return Err(Error::NoNumber);
                }
                let Some(value) = parse_number_input::<A::Value>(&self.iobuffer) else {
                    return Err(Error::NoNumber);
                };
                if self.abyss.blow(value).is_none() {
                    return Err(Error::NoSpace);
                }
            }
            AwaTism::Terminate => return Ok(ContinueAt::None),
            AwaTism::Blow(value) => {
                // SAFETY: unwrap: A::Value should be able to represent an i8, thats its whole purpose
                if self.abyss.blow(cast(value).unwrap()).is_none() {
                    return Err(Error::NoSpace);
                }
            }
            AwaTism::Submerge(distance) => {
                if self.abyss.submerge(distance).is_none() {
                    return Err(Error::NotEnoughBubbles(distance));
                }
            }
            AwaTism::Pop => {
                if self.abyss.pop().is_none() {
                    return Err(Error::NotEnoughBubbles(u5::ONE));
                }
            }
            AwaTism::Duplicate => {
                if self.abyss.duplicate().is_none() {
                    return Err(Error::NotEnoughBubbles(u5::ONE));
                }
            }
            AwaTism::Surround(count) => {
                if self.abyss.surround(count).is_none() {
                    return Err(Error::NotEnoughBubbles(count));
                }
            }
            AwaTism::Merge => {
                if self.abyss.merge().is_none() {
                    return Err(Error::NotEnoughBubbles(u5::TWO));
                }
            }
            AwaTism::Add => {
                if self.abyss.combine_single(<A::Value as Add>::add).is_none() {
                    return Err(Error::NotEnoughBubbles(u5::TWO));
                }
            }
            AwaTism::Subtract => {
                if self.abyss.combine_single(<A::Value as Sub>::sub).is_none() {
                    return Err(Error::NotEnoughBubbles(u5::TWO));
                }
            }
            AwaTism::Multiply => {
                if self.abyss.combine_single(<A::Value as Mul>::mul).is_none() {
                    return Err(Error::NotEnoughBubbles(u5::TWO));
                }
            }
            AwaTism::Divide => {
                if self
                    .abyss
                    .combine_double(<A::Value as Div>::div, <A::Value as Rem>::rem)
                    .is_none()
                {
                    return Err(Error::NotEnoughBubbles(u5::TWO));
                }
            }
            AwaTism::Count => {
                if self.abyss.count().is_none() {
                    return Err(Error::NotEnoughBubbles(u5::ONE));
                }
            }
            AwaTism::Label(_label) => (),
            AwaTism::Jump(label) => return Ok(ContinueAt::Label(label)),
            AwaTism::EqualTo => match self.abyss.test(<A::Value as PartialEq>::eq) {
                Some(true) => (),
                Some(false) => return Ok(ContinueAt::SkipNext),
                None => return Err(Error::NotEnoughBubbles(u5::TWO)),
            },
            AwaTism::LessThan => match self.abyss.test(<A::Value as PartialOrd>::lt) {
                Some(true) => (),
                Some(false) => return Ok(ContinueAt::SkipNext),
                None => return Err(Error::NotEnoughBubbles(u5::TWO)),
            },
            AwaTism::GreaterThan => match self.abyss.test(<A::Value as PartialOrd>::gt) {
                Some(true) => (),
                Some(false) => return Ok(ContinueAt::SkipNext),
                None => return Err(Error::NotEnoughBubbles(u5::TWO)),
            },
            AwaTism::DoublePop => {
                if self.abyss.double_pop().is_none() {
                    return Err(Error::NotEnoughBubbles(u5::ONE));
                }
            }
        }
        Ok(ContinueAt::Next)
    }
}
