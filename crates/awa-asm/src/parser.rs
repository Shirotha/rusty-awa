use core::str;
use std::{
    env::{current_dir, set_current_dir},
    fs::File,
    io::Read,
    path::Path,
    rc::Rc,
};

use awa_core::{u5, AwaTism};

use crate::{Error, MacroTable, Result, Spanned};

#[inline]
pub fn awatism(line: Spanned<&[u8]>) -> Result<AwaTism> {
    let (name, mut arg) = line.split_at_whitespace();
    arg.trim();
    let ident = str::from_utf8(name.item).map_err(|e| Error::EncodingError {
        span: name.span.clone(),
        inner: e,
    })?;
    let awatism = match ident {
        "nop" => AwaTism::NoOp,
        "prn" => AwaTism::Print,
        "pr1" => AwaTism::PrintNum,
        "red" => AwaTism::Read,
        "r3d" => AwaTism::ReadNum,
        "trm" => AwaTism::Terminate,
        "blo" => AwaTism::Blow(arg.parse::<i8>()?),
        "sbm" => AwaTism::Submerge(arg.parse::<u5>()?),
        "pop" => AwaTism::Pop,
        "dpl" => AwaTism::Duplicate,
        "srn" => AwaTism::Surround(arg.parse::<u5>()?),
        "mrg" => AwaTism::Merge,
        "4dd" => AwaTism::Add,
        "sub" => AwaTism::Subtract,
        "mul" => AwaTism::Multiply,
        "div" => AwaTism::Divide,
        "cnt" => AwaTism::Count,
        "lbl" => AwaTism::Label(arg.parse::<u5>()?),
        "jmp" => AwaTism::Jump(arg.parse::<u5>()?),
        "eql" => AwaTism::EqualTo,
        "lss" => AwaTism::LessThan,
        "gr8" => AwaTism::GreaterThan,
        "p0p" => AwaTism::DoublePop,
        _ => {
            return Err(Error::UnknownIdentifier {
                span: name.span,
                identifier: ident.to_string(),
            })
        }
    };
    Ok(awatism)
}
#[inline]
pub fn _macro(line: Spanned<&[u8]>, macros: &MacroTable) -> Result<Vec<AwaTism>> {
    let (_exclaim, rest) = line.split_at(1);
    let (name, mut rest) = rest.split_at_whitespace();
    let ident = str::from_utf8(name.item).map_err(|e| Error::EncodingError {
        span: name.span.clone(),
        inner: e,
    })?;
    rest.trim();
    macros
        .get(ident)
        .map(|f| f(rest, macros))
        .transpose()?
        .ok_or_else(|| Error::UnknownIdentifier {
            span: name.span,
            identifier: format!("!{}", ident),
        })
}
#[inline]
pub fn push_line(
    buffer: &mut Vec<AwaTism>,
    mut line: Spanned<&[u8]>,
    macros: &MacroTable,
) -> Result<()> {
    line.trim_start();
    match line.first() {
        Some(b'!') => buffer.append(&mut _macro(line, macros)?),
        Some(b';') | None => (),
        Some(_) => buffer.push(awatism(line)?),
    }
    Ok(())
}
#[inline]
pub fn lines(file: Rc<str>, src: &[u8], macros: &MacroTable) -> Result<Vec<AwaTism>> {
    let mut buffer = Vec::new();
    for (i, line) in src.split(|c| *c == b'\n').enumerate() {
        push_line(
            &mut buffer,
            Spanned::from_line(file.clone(), i + 1, line),
            macros,
        )?;
    }
    Ok(buffer)
}
pub fn file(file: Spanned<&Path>, macros: &MacroTable) -> Result<Vec<AwaTism>> {
    let mut handle = File::open(file.item).map_err(|e| Error::IOError {
        span: file.span.clone(),
        inner: e,
    })?;
    let mut buffer = Vec::new();
    handle
        .read_to_end(&mut buffer)
        .map_err(|e| Error::IOError {
            span: file.span.clone(),
            inner: e,
        })?;
    let cwd = current_dir().map_err(|e| Error::IOError {
        span: file.span.clone(),
        inner: e,
    })?;
    set_current_dir(file.item.parent().unwrap()).map_err(|e| Error::IOError {
        span: file.span.clone(),
        inner: e,
    })?;
    let result = lines(file.item.to_str().unwrap().into(), &buffer, macros);
    set_current_dir(cwd).map_err(|e| Error::IOError {
        span: file.span,
        inner: e,
    })?;
    result
}
