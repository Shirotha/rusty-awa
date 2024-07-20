use bitbuffer::{BitError, BitReadBuffer, BitWriteStream, Endianness};
use thiserror::Error;

use customasm::*;

/// List of grammar rules to for use with [`Assembler`].
pub const GRAMMAR: [(&str, &str); 4] = [
    ("awatism", include_str!("awatism.asm")),
    ("awascii", include_str!("awascii.asm")),
    ("macro", include_str!("macro.asm")),
    ("bank", include_str!("bank.asm")),
];
// TODO: support real files
/// Represent a assembler that generates a binary from AwaTism source code.
pub struct Assembler {
    fileserver: util::FileServerMock,
    opts: asm::AssemblyOptions,
}
impl Assembler {
    /// Create a new [`Assembler`] with standard AwaTism grammar rules.
    #[inline]
    pub fn new() -> Self {
        let (mut fileserver, opts) = (util::FileServerMock::new(), asm::AssemblyOptions::new());
        fileserver.add_std_files(&GRAMMAR);
        Self { fileserver, opts }
    }
    #[inline(always)]
    pub fn get_opts(&mut self) -> &asm::AssemblyOptions {
        &self.opts
    }
    #[inline(always)]
    pub fn get_opts_mut(&mut self) -> &mut asm::AssemblyOptions {
        &mut self.opts
    }
    #[inline(always)]
    pub fn fileserver(&self) -> &util::FileServerMock {
        &self.fileserver
    }
    /// Builds a binary from the given source code
    /// # Returns
    /// On successful assembly will return the binary and its length in bits.
    /// Also a report of all assembler messages and errors will always be returned.
    #[inline]
    pub fn assemble<E: Endianness>(
        &mut self,
        src: impl Into<Vec<u8>>,
    ) -> (Option<(BitReadBuffer<'static, E>, usize)>, diagn::Report) {
        self.fileserver.add("src", src);
        let mut report = diagn::Report::new();
        let assembly = asm::assemble(
            &mut report,
            &self.opts,
            &mut self.fileserver,
            &["awatism", "awascii", "macro", "bank", "src"],
        );
        let result = assembly.output.map(|bits| {
            (
                BitReadBuffer::new_owned(bits.format_binary(), E::endianness()),
                bits.len(),
            )
        });
        (result, report)
    }
}
impl Default for Assembler {
    fn default() -> Self {
        Self::new()
    }
}

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
