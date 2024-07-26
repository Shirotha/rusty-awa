use core::str;
use std::{
    collections::HashMap,
    fmt::{Display, Write},
    ops::Deref,
    path::Path,
    rc::Rc,
    str::FromStr,
};

use awa_core::{AwaSCII, AwaTism, Program};
use thiserror::Error;

pub mod macros;
pub mod parser;

/// Source location stored as right-exclusive range
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Span {
    pub file: Rc<str>,
    pub line: usize,
    pub start: usize,
    pub end: usize,
}
impl Span {
    #[inline]
    pub const fn new(file: Rc<str>, line: usize, start: usize, end: usize) -> Self {
        assert!(start <= end);
        Self {
            file,
            line,
            start,
            end,
        }
    }
    #[inline]
    pub fn skip(&self, start: usize) -> Self {
        let mut this = self.clone();
        this.start += start;
        if this.start > this.end {
            this.start = this.end;
        }
        this
    }
    #[inline]
    pub fn truncate(&self, end: usize) -> Self {
        let mut this = self.clone();
        let end = this.start + end;
        if end < this.end {
            this.end = end;
        }
        this
    }
    #[inline]
    pub const fn len(&self) -> usize {
        self.end.saturating_sub(self.start)
    }
    #[inline]
    pub const fn is_empty(&self) -> bool {
        self.start >= self.end
    }
}
impl Display for Span {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.file.fmt(f)?;
        f.write_char(':')?;
        self.line.fmt(f)?;
        f.write_char(':')?;
        self.start.fmt(f)?;
        f.write_str("..")?;
        self.end.fmt(f)
    }
}
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Spanned<T> {
    pub item: T,
    pub span: Span,
}
impl<'i> Spanned<&'i [u8]> {
    #[inline]
    pub fn from_line(file: Rc<str>, number: usize, line: &'i [u8]) -> Self {
        Self {
            item: line,
            span: Span::new(file, number, 0, line.len()),
        }
    }
    #[inline]
    pub fn trim_start(&mut self) {
        let start = self
            .item
            .iter()
            .take_while(|c| c.is_ascii_whitespace())
            .count();
        self.item = &self.item[start..];
        self.span = self.span.skip(start);
    }
    #[inline]
    pub fn trim_end(&mut self) {
        let mut len = self.item.len();
        if len == 0 {
            return;
        }
        if self.item[len - 1] == b'\n' {
            *self = self.split_at(len - 1).0;
            len -= 1;
            if len == 0 {
                return;
            }
        }
        if self.item[len - 1] == b'\r' {
            *self = self.split_at(len - 1).0;
            len -= 1;
            if len == 0 {
                return;
            }
        }
        let cut = self
            .item
            .iter()
            .rev()
            .take_while(|c| c.is_ascii_whitespace())
            .count();
        self.item = &self.item[..(len - cut)];
        self.span = self.span.truncate(self.item.len());
    }
    #[inline]
    pub fn trim(&mut self) {
        self.trim_start();
        self.trim_end();
    }
    #[inline]
    pub fn split_at(&self, middle: usize) -> (Self, Self) {
        (
            Self {
                item: &self.item[..middle],
                span: self.span.truncate(middle),
            },
            Self {
                item: &self.item[middle..],
                span: self.span.skip(middle),
            },
        )
    }
    pub fn split_at_char(&self, char: u8) -> (Self, Self) {
        let middle = self.item.iter().take_while(|c| **c != char).count();
        let (before, after) = self.split_at(middle);
        (before, after.split_at(1).1)
    }
    #[inline]
    pub fn split_at_whitespace(&self) -> (Self, Self) {
        let middle = self
            .item
            .iter()
            .take_while(|c| !c.is_ascii_whitespace())
            .count();
        self.split_at(middle)
    }
    #[inline]
    pub fn first(&self) -> Option<u8> {
        self.item.first().copied()
    }
    #[inline]
    pub fn len(&self) -> usize {
        self.span.len()
    }
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.span.is_empty()
    }
    #[inline]
    pub fn parse<T: FromStr>(&self) -> Result<T>
    where
        <T as FromStr>::Err: Display,
    {
        str::from_utf8(self.item)
            .map_err(|e| Error::EncodingError {
                span: self.span.clone(),
                inner: e,
            })?
            .parse::<T>()
            .map_err(|e| Error::ParseError {
                span: self.span.clone(),
                msg: e.to_string(),
            })
    }
    #[inline]
    pub fn take_awascii(&mut self) -> Result<Option<AwaSCII>> {
        let len = self.len();
        match self.item.get(len.saturating_sub(1)) {
            Some(b'n') if self.item.get(len.saturating_sub(2)) == Some(&b'\\') => {
                *self = self.split_at(len - 2).1;
                // SAFETY: 63 is a valid AwaSCII character
                Ok(Some(unsafe { AwaSCII::new_unchecked(63) }))
            }
            Some(ascii) => {
                let (rest, last) = self.split_at(len - 1);
                let awascii = AwaSCII::from_ascii(*ascii).ok_or_else(|| Error::ParseError {
                    span: last.span,
                    msg: "invalid AwaSCII".to_string(),
                })?;
                *self = rest;
                Ok(Some(awascii))
            }
            _ => Ok(None),
        }
    }
}

#[derive(Debug, Error)]
pub enum Error {
    #[error("{span}: syntax error: {msg}")]
    SyntaxError { span: Span, msg: String },
    #[error("{span}: unknown identifier: {identifier}")]
    UnknownIdentifier { span: Span, identifier: String },
    #[error("{span}: parsing failed: {msg}")]
    ParseError { span: Span, msg: String },
    #[error("{span}: {inner}")]
    IOError { span: Span, inner: std::io::Error },
    #[error("{span}: {inner}")]
    EncodingError {
        span: Span,
        inner: std::str::Utf8Error,
    },
}

pub type Result<T> = std::result::Result<T, Error>;
pub type Macro = Box<dyn Fn(Spanned<&[u8]>, &MacroTable) -> Result<Vec<AwaTism>>>;
pub struct MacroTable(HashMap<String, Macro>);
impl Deref for MacroTable {
    type Target = HashMap<String, Macro>;
    #[inline(always)]
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
#[inline]
pub fn load_program(file: &Path, src: &[u8], macros: &MacroTable) -> Result<Program> {
    let awatisms = parser::lines(file.to_str().unwrap().into(), src, macros)?;
    Ok(Program::from_vec(awatisms))
}
