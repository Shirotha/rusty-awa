use std::{
    io::{BufRead, Write},
    num::NonZero,
};

use awa_core::{Abyss, AwaTism, Program};
pub use fallible_iterator::FallibleIterator;
use num_traits::cast;

use crate::{ContinueAt, Error, Interpreter};

#[inline]
pub fn run_single<A: Abyss, I: BufRead, O: Write>(
    interpreter: &mut Interpreter<A, I, O>,
    awatism: AwaTism,
    labels: &[Option<NonZero<usize>>],
    pc: usize,
) -> Result<Option<usize>, Error> {
    match interpreter.next(awatism) {
        Ok(ContinueAt::Next) => Ok(Some(pc + 1)),
        Ok(ContinueAt::SkipNext) => Ok(Some(pc + 2)),
        Ok(ContinueAt::None) => Ok(None),
        Ok(ContinueAt::Label(label)) => {
            let index = cast::<_, usize>(label).unwrap();
            let Some(next) = labels[index] else {
                return Err(Error::UnknownLabel(label));
            };
            Ok(Some(next.get()))
        }
        Err(error) => Err(error),
    }
}

#[derive(Debug)]
pub struct Iter<'a, A: Abyss, I: BufRead, O: Write> {
    pub(crate) interpreter: &'a mut Interpreter<A, I, O>,
    pub(crate) program: &'a Program,
    pub(crate) pc: Option<usize>,
}
impl<'a, A, I, O> FallibleIterator for Iter<'a, A, I, O>
where
    A: Abyss,
    I: BufRead,
    O: Write,
{
    type Item = (usize, AwaTism);
    type Error = Error;
    #[inline]
    fn next(&mut self) -> Result<Option<Self::Item>, Self::Error> {
        let Some(current) = self.pc else {
            return Ok(None);
        };
        let Some(&awatism) = self.program.get(current) else {
            return Ok(None);
        };
        self.pc = run_single(self.interpreter, awatism, self.program.labels(), current)?;
        Ok(Some((current, awatism)))
    }
}

#[derive(Debug, Clone)]
pub struct Cursor<'a> {
    program: &'a Program,
    pub pc: Option<usize>,
}
impl<'a> Cursor<'a> {
    #[inline(always)]
    pub fn new(program: &'a Program) -> Self {
        Self {
            program,
            pc: Some(0),
        }
    }
    #[inline(always)]
    pub fn len(&self) -> usize {
        self.program.len()
    }
    #[inline(always)]
    pub fn is_empty(&self) -> bool {
        self.program.is_empty()
    }
    #[inline]
    pub fn next<A, I, O>(&mut self, interpreter: &mut Interpreter<A, I, O>) -> Result<bool, Error>
    where
        A: Abyss,
        I: BufRead,
        O: Write,
    {
        let Some((pc, awatism)) = self.current() else {
            return Ok(false);
        };
        self.pc = run_single(interpreter, awatism, self.program.labels(), pc)?;
        Ok(true)
    }
    #[inline]
    pub fn current(&self) -> Option<(usize, AwaTism)> {
        let pc = self.pc?;
        self.program.get(pc).cloned().map(|awatism| (pc, awatism))
    }
}
