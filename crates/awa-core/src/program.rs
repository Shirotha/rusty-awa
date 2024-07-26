use std::{num::NonZero, ops::Index, slice::SliceIndex};

use bitbuffer::{BitError, BitReadBuffer, BitReadStream, Endianness};
use num_traits::cast;

use crate::AwaTism;

#[derive(Debug, Clone)]
pub struct Program {
    instructions: Vec<AwaTism>,
    labels: Box<[Option<NonZero<usize>>; 32]>,
}
impl Program {
    #[inline]
    pub fn new() -> Self {
        Program {
            instructions: Vec::new(),
            labels: [None; 32].into(),
        }
    }
    #[inline]
    pub fn with_capacity(capacity: usize) -> Self {
        Program {
            instructions: Vec::with_capacity(capacity),
            labels: [None; 32].into(),
        }
    }
    #[inline]
    pub fn from_vec(instructions: Vec<AwaTism>) -> Self {
        let mut labels = Box::new([None; 32]);
        for (pc, awatism) in instructions.iter().enumerate() {
            if let AwaTism::Label(label) = awatism {
                // SAFETY: pc + 1 can never be zero
                labels[**label as usize] = Some(unsafe { NonZero::new_unchecked(pc + 1) });
            }
        }
        Self {
            instructions,
            labels,
        }
    }
    #[inline]
    pub fn from_bitbuffer(buffer: BitReadBuffer<impl Endianness>) -> Result<Self, BitError> {
        let (mut stream, mut program) = (BitReadStream::new(buffer), Self::new());
        loop {
            match stream.read() {
                Ok(awatism) => program.push(awatism),
                Err(error @ BitError::NotEnoughData { bits_left, .. }) => {
                    // SAFETY: unwrap: no AwaTism needs more than 16 bits
                    if stream.read_int::<u16>(bits_left).unwrap() == 0 {
                        return Ok(program);
                    }
                    return Err(error);
                }
                Err(BitError::IndexOutOfBounds { .. }) => return Ok(program),
                Err(error) => return Err(error),
            }
        }
    }
    #[inline]
    pub fn from_bitbuffer_with_length(
        buffer: BitReadBuffer<impl Endianness>,
        length: usize,
    ) -> Result<Self, BitError> {
        if length == 0 {
            return Ok(Self::new());
        }
        // NOTE: biggest instruction is 13 bits, so this is the minimum size required
        let (mut stream, mut program) =
            (BitReadStream::new(buffer), Self::with_capacity(length / 13));
        while stream.pos() < length {
            match stream.read() {
                Ok(awatism) => program.push(awatism),
                Err(error) => return Err(error),
            }
        }
        Ok(program)
    }
    #[inline(always)]
    pub fn len(&self) -> usize {
        self.instructions.len()
    }
    #[inline(always)]
    pub fn is_empty(&self) -> bool {
        self.instructions.is_empty()
    }
    #[inline(always)]
    pub fn iter(&self) -> impl Iterator<Item = &AwaTism> {
        self.instructions.iter()
    }
    #[inline(always)]
    pub fn get<I: SliceIndex<[AwaTism]>>(&self, index: I) -> Option<&I::Output> {
        self.instructions.get(index)
    }
    /// Returns label table.
    /// Numbers represent the first instruction to execute after jumping to a label, not the label itself.
    /// Will be `None` when no matching label was found.
    #[inline(always)]
    pub fn labels(&self) -> &[Option<NonZero<usize>>] {
        self.labels.as_slice()
    }
    /// Push instruction to the end of the program and update the label table.
    #[inline]
    pub fn push(&mut self, awatism: AwaTism) {
        self.instructions.push(awatism);
        if let AwaTism::Label(label) = awatism {
            // SAFETY: unwrap: usize is wider than u5
            // SAFETY: the index limit will not reasonably be reached
            self.labels[cast::<_, usize>(label).unwrap()] =
                Some(unsafe { NonZero::new_unchecked(self.instructions.len()) });
        }
    }
}
impl<I: SliceIndex<[AwaTism]>> Index<I> for Program {
    type Output = I::Output;
    #[inline(always)]
    fn index(&self, index: I) -> &Self::Output {
        &self.instructions[index]
    }
}
impl Default for Program {
    #[inline(always)]
    fn default() -> Self {
        Self::new()
    }
}
impl IntoIterator for Program {
    type Item = AwaTism;
    type IntoIter = <Vec<AwaTism> as IntoIterator>::IntoIter;
    #[inline(always)]
    fn into_iter(self) -> Self::IntoIter {
        self.instructions.into_iter()
    }
}
impl<'a> IntoIterator for &'a Program {
    type Item = &'a AwaTism;
    type IntoIter = <&'a Vec<AwaTism> as IntoIterator>::IntoIter;
    #[inline(always)]
    fn into_iter(self) -> Self::IntoIter {
        self.instructions.iter()
    }
}
