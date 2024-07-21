use std::{
    cmp::Ordering,
    fmt::{Display, Write},
    ops::{Deref, DerefMut},
};

use awa_core::{Abyss, AwaSCII, Value};
use num_traits::{cast, One, Zero};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
enum BufferKind {
    Empty,
    Singles,
    Double,
}
/// Store either multiple singles or a double bubble.
/// Having an empty buffer set to something different then [`BufferKind::Empty`] is undefined behaviour.
#[derive(Debug, Clone)]
struct Buffer<T: Value> {
    data: Vec<T>,
    kind: BufferKind,
}
impl<T: Value> Buffer<T> {
    #[inline(always)]
    pub const fn new() -> Self {
        Self {
            data: Vec::new(),
            kind: BufferKind::Empty,
        }
    }
    #[inline(always)]
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            data: Vec::with_capacity(capacity),
            kind: BufferKind::Empty,
        }
    }
    #[inline]
    pub fn pop(&mut self) -> Option<Option<T>> {
        match self.kind {
            BufferKind::Empty => None,
            BufferKind::Singles => match self.data.pop() {
                Some(value) => {
                    if self.data.is_empty() {
                        self.kind = BufferKind::Empty;
                    }
                    Some(Some(value))
                }
                None => None,
            },
            BufferKind::Double => {
                self.kind = BufferKind::Singles;
                Some(None)
            }
        }
    }
    #[inline]
    pub fn double_pop(&mut self) -> Option<Option<T>> {
        match self.kind {
            BufferKind::Empty => None,
            BufferKind::Singles => match self.data.pop() {
                Some(value) => {
                    if self.data.is_empty() {
                        self.kind = BufferKind::Empty;
                    }
                    Some(Some(value))
                }
                None => None,
            },
            BufferKind::Double => {
                self.clear();
                Some(None)
            }
        }
    }
    #[inline]
    pub fn clear(&mut self) {
        self.data.clear();
        self.kind = BufferKind::Empty;
    }
}
impl<T: Value> Default for Buffer<T> {
    #[inline(always)]
    fn default() -> Self {
        Self::new()
    }
}
impl<T: Value> Deref for Buffer<T> {
    type Target = Vec<T>;
    #[inline(always)]
    fn deref(&self) -> &Self::Target {
        &self.data
    }
}
impl<T: Value> DerefMut for Buffer<T> {
    #[inline(always)]
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.data
    }
}
impl<T: Value> AsRef<[T]> for &Buffer<T> {
    #[inline(always)]
    fn as_ref(&self) -> &[T] {
        &self.data
    }
}
/// Wrapper around any [`Abyss`] that stores the top data in an array.
///
/// In case the inner abyss has bad performance in blow/pop instructions this can improve it.
#[derive(Debug, Clone)]
pub struct Buffered<A: Abyss> {
    inner: A,
    buffer: Buffer<A::Value>,
}
impl<A: Abyss + Default> Buffered<A> {
    #[inline]
    pub fn new() -> Self {
        Self {
            inner: A::default(),
            buffer: Buffer::new(),
        }
    }
}
impl<A: Abyss + Default> Default for Buffered<A> {
    #[inline(always)]
    fn default() -> Self {
        Self::new()
    }
}
impl<A: Abyss> Buffered<A> {
    #[inline]
    pub fn from_inner(inner: A) -> Self {
        Self {
            inner,
            buffer: Buffer::new(),
        }
    }
    #[inline]
    pub fn with_capacity(inner: A, capacity: usize) -> Self {
        Self {
            inner,
            buffer: Buffer::with_capacity(capacity),
        }
    }
    #[inline(always)]
    pub fn into_inner(self) -> A {
        self.inner
    }
    #[inline]
    fn copy(&mut self) -> Option<()> {
        match self.buffer.kind {
            BufferKind::Empty => (),
            BufferKind::Singles => {
                self.inner.blow_many(&self.buffer)?;
            }
            BufferKind::Double => {
                self.inner.blow_double(&self.buffer)?;
            }
        }
        Some(())
    }
    #[inline]
    fn commit(&mut self) -> Option<()> {
        self.copy()?;
        self.buffer.clear();
        Some(())
    }
    #[inline]
    fn get_singles_mut(&mut self) -> Option<&mut Vec<A::Value>> {
        if matches!(self.buffer.kind, BufferKind::Double) {
            self.commit()?;
        }
        self.buffer.kind = BufferKind::Singles;
        Some(&mut self.buffer)
    }
    #[inline]
    fn get_double_mut(&mut self) -> Option<&mut Vec<A::Value>> {
        if matches!(self.buffer.kind, BufferKind::Singles | BufferKind::Double) {
            self.commit()?;
        }
        self.buffer.kind = BufferKind::Double;
        Some(&mut self.buffer)
    }
}
impl<A: Abyss> Abyss for Buffered<A> {
    type Value = A::Value;
    #[inline]
    fn is_empty(&self) -> bool {
        matches!(self.buffer.kind, BufferKind::Empty) && self.inner.is_empty()
    }
    #[inline]
    fn blow_awascii<B>(&mut self, awascii: B) -> Option<()>
    where
        B: AsRef<[AwaSCII]>,
    {
        let (string, buffer) = (awascii.as_ref(), self.get_double_mut()?);
        // SAFETY: unwrap: even an i8 can fit all AwaSCII characters
        buffer.extend(
            string
                .iter()
                .map(|char| cast::<_, Self::Value>(**char).unwrap()),
        );
        Some(())
    }
    #[inline]
    fn blow(&mut self, value: Self::Value) -> Option<()> {
        let buffer = self.get_singles_mut()?;
        buffer.push(value);
        Some(())
    }
    // TODO: if the jump goes past the buffer, reduce distance by length instead of committing
    #[inline]
    fn submerge(&mut self, distance: usize) -> Option<()> {
        match self.buffer.kind {
            BufferKind::Empty => self.inner.submerge(distance),
            BufferKind::Singles => {
                if distance.is_zero() {
                    let value = self.buffer.data.pop()?;
                    return if self.inner.is_empty() {
                        self.buffer.insert(0, value);
                        Some(())
                    } else {
                        if self.buffer.is_empty() {
                            self.buffer.kind = BufferKind::Empty;
                        }
                        self.inner.blow(value)?;
                        self.inner.submerge(0)
                    };
                }
                let (value, len) = (self.buffer.data.pop().unwrap(), self.buffer.len());
                if len >= distance {
                    self.buffer.insert(len - distance, value);
                    return Some(());
                }
                if len.is_zero() {
                    self.buffer.kind = BufferKind::Empty;
                }
                self.inner.blow(value)?;
                self.inner.submerge(distance - len)
            }
            BufferKind::Double => {
                self.commit()?;
                self.inner.submerge(distance)
            }
        }
    }
    #[inline]
    fn pop(&mut self) -> Option<()> {
        self.buffer.pop().map(|_| ()).or_else(|| self.inner.pop())
    }
    #[inline]
    fn double_pop(&mut self) -> Option<()> {
        self.buffer
            .double_pop()
            .map(|_| ())
            .or_else(|| self.inner.double_pop())
    }
    #[inline]
    fn duplicate(&mut self) -> Option<()> {
        match self.buffer.kind {
            BufferKind::Empty => self.inner.duplicate(),
            BufferKind::Singles => {
                // SAFETY: unwrap: buffer cannot be empty by construction
                let last = *self.buffer.last().unwrap();
                self.buffer.push(last);
                Some(())
            }
            BufferKind::Double => self.copy(),
        }
    }
    #[inline]
    fn surround(&mut self, count: usize) -> Option<()> {
        match self.buffer.kind {
            BufferKind::Empty => self.inner.surround(count),
            BufferKind::Singles => {
                let len = self.buffer.len();
                self.buffer.kind = BufferKind::Double;
                match len.cmp(&count) {
                    Ordering::Less => {
                        self.commit()?;
                        self.inner.merge_many(count - len - 1)?;
                    }
                    Ordering::Equal => (),
                    Ordering::Greater => {
                        let middle = len - count;
                        self.inner.blow_many(&self.buffer[..middle])?;
                        self.buffer.drain(..middle);
                    }
                }
                Some(())
            }
            BufferKind::Double => {
                self.commit()?;
                self.inner.surround(count)
            }
        }
    }
    #[inline]
    fn merge(&mut self) -> Option<()> {
        match self.buffer.kind {
            BufferKind::Empty => self.inner.merge(),
            BufferKind::Singles => match self.buffer.len() {
                0 => unreachable!(),
                1 => {
                    self.commit()?;
                    self.inner.merge()
                }
                2 => {
                    self.buffer.kind = BufferKind::Double;
                    Some(())
                }
                len => {
                    let middle = len - 2;
                    self.inner.blow_many(&self.buffer[..middle])?;
                    self.buffer.drain(..middle);
                    self.buffer.kind = BufferKind::Double;
                    Some(())
                }
            },
            BufferKind::Double => {
                self.commit()?;
                self.inner.merge()
            }
        }
    }
    #[inline]
    fn count(&mut self) -> Option<()> {
        match self.buffer.kind {
            BufferKind::Empty => self.inner.count(),
            BufferKind::Singles => {
                self.buffer.push(Self::Value::one());
                Some(())
            }
            BufferKind::Double => {
                let count = self.buffer.len();
                self.commit()?;
                self.buffer.push(cast(count)?);
                self.buffer.kind = BufferKind::Singles;
                Some(())
            }
        }
    }
    #[inline]
    fn combine_single<F>(&mut self, op: F) -> Option<()>
    where
        F: Fn(Self::Value, Self::Value) -> Self::Value,
    {
        if matches!(self.buffer.kind, BufferKind::Singles if self.buffer.len() >= 2) {
            // SAFETY: unwrap: buffer has at least two elements here
            let lhs = self.buffer.data.pop().unwrap();
            let rhs = *self.buffer.last().unwrap();
            *self.buffer.last_mut().unwrap() = op(lhs, rhs);
            Some(())
        } else {
            self.commit()?;
            self.inner.combine_single(op)
        }
    }
    #[inline]
    fn combine_double<F1, F2>(&mut self, op1: F1, op2: F2) -> Option<()>
    where
        F1: Fn(Self::Value, Self::Value) -> Self::Value,
        F2: Fn(Self::Value, Self::Value) -> Self::Value,
    {
        if matches!(self.buffer.kind, BufferKind::Singles if self.buffer.len() >= 2) {
            // SAFETY: unwrap: buffer has at least two elements here
            let (lhs, rhs) = (
                self.buffer.data.pop().unwrap(),
                self.buffer.data.pop().unwrap(),
            );
            if !self.buffer.is_empty() {
                self.commit()?;
            }
            self.buffer.push(op2(lhs, rhs));
            self.buffer.push(op1(lhs, rhs));
            self.buffer.kind = BufferKind::Double;
            Some(())
        } else {
            self.commit()?;
            self.inner.combine_double(op1, op2)
        }
    }
    #[inline]
    fn test<F>(&mut self, test: F) -> Option<bool>
    where
        F: Fn(&Self::Value, &Self::Value) -> bool,
    {
        match self.buffer.kind {
            BufferKind::Empty => self.inner.test(test),
            BufferKind::Singles => match self.buffer.len() {
                0 => unreachable!(),
                1 => {
                    self.commit()?;
                    self.test(test)
                }
                len => {
                    let middle = len - 2;
                    Some(test(&self.buffer[middle + 1], &self.buffer[middle]))
                }
            },
            BufferKind::Double => (!self.inner.is_empty()).then_some(false),
        }
    }
    #[inline]
    fn consume<F, E>(&mut self, mut fun: F) -> Result<Option<()>, E>
    where
        F: FnMut(Self::Value) -> Result<(), E>,
    {
        match self.buffer.kind {
            BufferKind::Empty => self.inner.consume(fun),
            BufferKind::Singles => {
                fun(*self.buffer.last().unwrap())?;
                self.buffer.pop();
                Ok(Some(()))
            }
            BufferKind::Double => {
                self.buffer.iter().rev().copied().try_for_each(fun)?;
                self.buffer.clear();
                Ok(Some(()))
            }
        }
    }
    #[inline]
    fn blow_many<B>(&mut self, values: B) -> Option<()>
    where
        B: AsRef<[Self::Value]>,
    {
        if matches!(self.buffer.kind, BufferKind::Double) {
            self.commit()?;
        }
        self.buffer.kind = BufferKind::Singles;
        self.buffer.extend_from_slice(values.as_ref());
        Some(())
    }
    #[inline]
    fn pop_many(&mut self, count: usize) -> Option<()> {
        let offset = match self.buffer.kind {
            BufferKind::Empty => return self.inner.pop_many(count),
            BufferKind::Singles => 0,
            BufferKind::Double => 1,
        };
        let (len, count) = (self.buffer.len() + offset, count);
        match len.cmp(&count) {
            Ordering::Less => {
                self.buffer.clear();
                self.inner.pop_many(count - len)?;
            }
            Ordering::Equal => self.buffer.clear(),
            Ordering::Greater => {
                let middle = len - count - offset;
                self.buffer.drain(..middle);
                self.buffer.kind = BufferKind::Singles;
            }
        }
        Some(())
    }
    #[inline]
    fn double_pop_many(&mut self, count: usize) -> Option<()> {
        match self.buffer.kind {
            BufferKind::Empty => self.inner.double_pop_many(count),
            BufferKind::Singles => {
                let len = self.buffer.len();
                match len.cmp(&count) {
                    Ordering::Less => {
                        self.buffer.clear();
                        self.inner.double_pop_many(count - len)?;
                    }
                    Ordering::Equal => self.buffer.clear(),
                    Ordering::Greater => {
                        let middle = len - count;
                        self.buffer.drain(..middle);
                    }
                }
                Some(())
            }
            BufferKind::Double => {
                self.buffer.clear();
                self.inner.double_pop_many(count - 1)
            }
        }
    }
    #[inline]
    fn duplicate_many(&mut self, count: usize) -> Option<()> {
        match self.buffer.kind {
            BufferKind::Empty => self.inner.duplicate_many(count),
            BufferKind::Singles => {
                // SAFETY: unwrap: buffer is not empty by construction
                let value = *self.buffer.last().unwrap();
                self.buffer.extend((0..count).map(|_| value));
                Some(())
            }
            BufferKind::Double => {
                for _ in 0..count {
                    self.inner.blow_double(&self.buffer)?;
                }
                Some(())
            }
        }
    }
}
impl<A: Abyss + Display> Display for Buffered<A> {
    #[inline(always)]
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.buffer.kind {
            BufferKind::Empty => (),
            BufferKind::Singles => {
                for value in self.buffer.iter().rev() {
                    value.fmt(f)?;
                    f.write_char('\n')?;
                }
            }
            BufferKind::Double => {
                f.write_char('[')?;
                let mut first = true;
                for value in self.buffer.iter().rev() {
                    if first {
                        first = false;
                    } else {
                        f.write_str(", ")?;
                    }
                    value.fmt(f)?;
                }
                f.write_str("]\n")?;
            }
        }
        f.write_str("-----\n")?;
        self.inner.fmt(f)
    }
}
