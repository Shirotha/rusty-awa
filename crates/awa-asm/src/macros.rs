use core::str;
use std::{collections::HashMap, path::Path};

use awa_core::{u5, AwaTism};

use crate::{parser::file, Error, MacroTable, Result, Spanned};

pub fn chr(mut input: Spanned<&[u8]>, _macros: &MacroTable) -> Result<Vec<AwaTism>> {
    input.trim();
    let (begin, rest) = input.split_at_char(b'\'');
    if !begin.is_empty() {
        return Err(Error::SyntaxError {
            span: begin.span,
            msg: "expected single-quote".to_string(),
        });
    }
    let (mut inner, end) = rest.split_at_char(b'\'');
    if !end.is_empty() {
        return Err(Error::SyntaxError {
            span: end.span,
            msg: "expected single-quote".to_string(),
        });
    }
    let awascii = inner.take_awascii()?.ok_or_else(|| Error::SyntaxError {
        span: inner.span,
        msg: "empty character".to_string(),
    })?;
    Ok(vec![AwaTism::Blow(*awascii as i8)])
}
pub fn str(mut input: Spanned<&[u8]>, _macros: &MacroTable) -> Result<Vec<AwaTism>> {
    input.trim();
    let (begin, rest) = input.split_at_char(b'"');
    if !begin.is_empty() {
        return Err(Error::SyntaxError {
            span: begin.span,
            msg: "expected double-quote".to_string(),
        });
    }
    let (mut inner, end) = rest.split_at_char(b'"');
    if !end.is_empty() {
        return Err(Error::SyntaxError {
            span: end.span,
            msg: "extra content at end of line".to_string(),
        });
    }
    let mut buffer = Vec::new();
    let mut count = 0;
    let mut first_chunk = true;
    // SAFETY: 31 is a valid u5
    let chunk_size = unsafe { u5::new_unchecked(31) };
    while let Some(awascii) = inner.take_awascii()? {
        buffer.push(AwaTism::Blow(*awascii as i8));
        count += 1;
        if count == 31 {
            buffer.push(AwaTism::Surround(chunk_size));
            count = 0;
            if first_chunk {
                first_chunk = false;
            } else {
                buffer.push(AwaTism::Merge);
            }
        }
    }
    if count > 1 {
        // SAFETY: count is always a valid u5
        buffer.push(AwaTism::Surround(unsafe { u5::new_unchecked(count) }));
    }
    if count != 0 && !first_chunk {
        buffer.push(AwaTism::Merge);
    }
    Ok(buffer)
}
pub fn include(mut input: Spanned<&[u8]>, macros: &MacroTable) -> Result<Vec<AwaTism>> {
    input.trim();
    let (begin, rest) = input.split_at_char(b'<');
    if !begin.is_empty() {
        return Err(Error::SyntaxError {
            span: begin.span,
            msg: "expected left angle-bracket".to_string(),
        });
    }
    let (path, end) = rest.split_at_char(b'>');
    if !begin.is_empty() {
        return Err(Error::SyntaxError {
            span: end.span,
            msg: "extra content at end of line".to_string(),
        });
    }
    let span = path.span;
    let path = Path::new(str::from_utf8(path.item).map_err(|e| Error::EncodingError {
        span: span.clone(),
        inner: e,
    })?);
    file(Spanned { item: path, span }, macros)
}

impl Default for MacroTable {
    fn default() -> Self {
        let mut result = HashMap::new();
        result.insert("chr".into(), Box::new(chr) as Box<_>);
        result.insert("str".into(), Box::new(str) as Box<_>);
        result.insert("include".into(), Box::new(include) as Box<_>);
        MacroTable(result)
    }
}
