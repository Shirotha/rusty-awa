use bitbuffer::{BitError, BitReadBuffer, BitWriteStream, Endianness};
use thiserror::Error;

/// Represents an error that can occure during interpretation of AwaTalk source code.
#[derive(Debug, Error)]
pub enum ParseError {
    #[error("missing header")]
    NoHeader,
    #[error(transparent)]
    BitError(#[from] BitError),
}

#[derive(Debug)]
struct StringMatcher {
    pattern: &'static [u8],
    index: usize,
}
impl StringMatcher {
    #[inline(always)]
    pub const fn new(pattern: &'static str) -> Self {
        Self {
            pattern: pattern.as_bytes(),
            index: 0,
        }
    }
    #[inline]
    pub fn push(&mut self, char: u8) -> bool {
        if self.pattern[self.index].eq_ignore_ascii_case(&char) {
            self.index += 1;
            return self.index == self.pattern.len();
        }
        false
    }
    #[inline(always)]
    pub fn reset(&mut self) {
        self.index = 0;
    }
}

pub const AWATALK_HEAD: &[u8] = "awa".as_bytes();
pub const AWATALK_ZERO: &str = " awa";
pub const AWATALK_ONE: &str = "wa";

/// Convert AwaTalk source code into a binary.
/// This will return the size in bits in addition to the resulting binary.
/// All invalid characters will be skipped over, including `"aw "` in wrong positions.
#[inline]
pub fn load_awatalk<E: Endianness>(
    src: impl AsRef<[u8]>,
) -> Result<(BitReadBuffer<'static, E>, usize), ParseError> {
    let Some(mut src) = src
        .as_ref()
        .split_at_checked(AWATALK_HEAD.len())
        .and_then(|(header, body)| header.eq_ignore_ascii_case(AWATALK_HEAD).then_some(body))
    else {
        return Err(ParseError::NoHeader);
    };
    // SAFETY: buffer: src only containing ones will take 16 bits per bit
    let mut buffer = vec![0; src.len() >> 4];
    let mut writer = BitWriteStream::from_slice(&mut buffer, E::endianness());
    let [mut zero, mut one] = [AWATALK_ZERO, AWATALK_ONE].map(StringMatcher::new);
    while let Some((char, rest)) = src.split_first() {
        src = rest;
        if zero.push(*char) {
            writer.write_int(0, 1)?;
        } else if one.push(*char) {
            writer.write_int(1, 1)?;
        } else {
            continue;
        }
        zero.reset();
        one.reset();
    }
    let (bits, len) = (writer.bit_len(), writer.byte_len());
    buffer.truncate(len);
    Ok((BitReadBuffer::new_owned(buffer, E::endianness()), bits))
}
