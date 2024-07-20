use std::{fmt::Display, mem::replace};

use awa_core::{u5, Value};
use num_traits::{cast, Zero};

use crate::{Arena, Index};

type Ref = Option<Index>;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum Bubble<T: Value> {
    Single {
        value: T,
        next: Ref,
    },
    Double {
        inner: (Index, Index),
        next: Ref,
        #[cfg(feature = "cache_count")]
        count: T,
    },
}
impl<T: Value> Bubble<T> {
    #[inline]
    const fn next(&self) -> Ref {
        match self {
            Self::Single { next, .. } => *next,
            Self::Double { next, .. } => *next,
        }
    }
    #[inline]
    fn next_mut(&mut self) -> &mut Ref {
        match self {
            Self::Single { next, .. } => next,
            Self::Double { next, .. } => next,
        }
    }
    #[cfg(feature = "cache_count")]
    #[inline]
    fn count(&self, _arena: &Arena<Self>) -> T {
        match self {
            Self::Single { .. } => T::zero(),
            Self::Double { count, .. } => *count,
        }
    }
    #[cfg(not(feature = "cache_count"))]
    #[inline]
    fn count(&self, arena: &Arena<Self>) -> T {
        match self {
            Self::Single { .. } => T::zero(),
            Self::Double {
                inner: (first, _), ..
            } => find_count(arena, *first),
        }
    }
}

#[inline]
fn deep_copy(arena: &mut Arena<Bubble<impl Value>>, root: Index) -> Index {
    let copy = arena[root];
    let index = arena.insert(copy);
    if let Bubble::Double {
        inner: (inner, _), ..
    } = copy
    {
        let mut last = deep_copy(arena, inner);
        let first = last;
        loop {
            let Some(next) = arena[last].next() else {
                break;
            };
            let index = deep_copy(arena, next);
            *arena[last].next_mut() = Some(index);
            last = index;
        }
        // SAFETY: index is a double bubble by construction
        let Some(Bubble::Double { inner, .. }) = arena.get_mut(index) else {
            unreachable!()
        };
        *inner = (first, last);
    }
    index
}
#[inline]
fn move_next<T: Value>(arena: &Arena<Bubble<T>>, mut first: Index, count: usize) -> (Index, T) {
    let (mut result, one) = (T::zero(), T::one());
    for _ in 0..count {
        let Some(next) = arena[first].next() else {
            break;
        };
        (first, result) = (next, result + one);
    }
    (first, result)
}
#[inline]
fn remove_all(arena: &mut Arena<Bubble<impl Value>>, mut first: Index) {
    loop {
        match arena.remove(first) {
            Some(Bubble::Single { next, .. }) => {
                let Some(next) = next else { return };
                first = next;
            }
            Some(Bubble::Double {
                inner: (inner, _),
                next,
                ..
            }) => {
                remove_all(arena, inner);
                let Some(next) = next else { return };
                first = next;
            }
            None => unreachable!(),
        }
    }
}
#[cfg(not(feature = "cache_count"))]
#[inline]
fn find_count<T>(arena: &Arena<Bubble<T>>, mut first: Index) -> T
where
    T: Value,
{
    let (mut count, step) = (T::zero(), T::one());
    loop {
        if let Some(next) = arena[first].next() {
            (first, count) = (next, count + step);
        } else {
            return count;
        }
    }
}

/// Represent an [`awa_core::Abyss`] that uses a linked list backed by an arena allocator to store bubbles.
#[derive(Debug, Clone)]
pub struct Abyss<T: Value> {
    arena: Arena<Bubble<T>>,
    top: Ref,
}
impl<T: Value> Abyss<T> {
    #[inline(always)]
    pub const fn new() -> Self {
        Self {
            arena: Arena::new(),
            top: None,
        }
    }
    #[inline(always)]
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            arena: Arena::with_capacity(capacity),
            top: None,
        }
    }
}
impl<T: Value> Default for Abyss<T> {
    #[inline(always)]
    fn default() -> Self {
        Self::new()
    }
}
impl<T: Value> awa_core::Abyss for Abyss<T> {
    type Value = T;
    #[inline]
    fn blow_awascii<B>(&mut self, awascii: B) -> Option<()>
    where
        B: AsRef<[awa_core::AwaSCII]>,
    {
        let awascii = awascii.as_ref();
        let inner = awascii
            .iter()
            .rev()
            .fold((None, None), |(first, last), char| {
                let bubble = Bubble::Single {
                    // SAFETY: unwrap: even i8 can hold all valid AwaSCII characters
                    value: cast(**char).unwrap(),
                    next: last,
                };
                let index = Some(self.arena.insert(bubble));
                (first.or(index), index)
            });
        let bubble = if let (Some(first), Some(last)) = inner {
            Bubble::Double {
                inner: (first, last),
                next: self.top,
                #[cfg(feature = "cache_count")]
                count: cast(awascii.len())?,
            }
        } else {
            Bubble::Single {
                value: T::zero(),
                next: self.top,
            }
        };
        self.top = Some(self.arena.insert(bubble));
        Some(())
    }
    #[inline]
    fn blow(&mut self, value: Self::Value) -> Option<()> {
        let bubble = Bubble::Single {
            value,
            next: self.top,
        };
        self.top = Some(self.arena.insert(bubble));
        Some(())
    }
    #[inline]
    fn submerge(&mut self, distance: u5) -> Option<()> {
        let first = self.top?;
        let count = if distance.is_zero() {
            usize::MAX
        } else {
            // SAFETY: unwrap: usize is wider than u5
            cast(distance).unwrap()
        };
        let (before, _) = move_next(&self.arena, first, count);
        let after = replace(self.arena[before].next_mut(), Some(first));
        self.top = replace(self.arena[first].next_mut(), after);
        Some(())
    }
    #[inline]
    fn pop(&mut self) -> Option<()> {
        match self.arena.remove(self.top?)? {
            Bubble::Single { next, .. } => self.top = next,
            Bubble::Double {
                inner: (first, last),
                next,
                ..
            } => {
                self.top = Some(first);
                *self.arena[last].next_mut() = next;
            }
        }
        Some(())
    }
    #[inline]
    fn duplicate(&mut self) -> Option<()> {
        let index = self.top?;
        let copy = deep_copy(&mut self.arena, index);
        *self.arena[copy].next_mut() = Some(index);
        self.top = Some(copy);
        Some(())
    }
    #[inline]
    fn surround(&mut self, count: u5) -> Option<()> {
        if count.is_zero() {
            return Some(());
        }
        let first = self.top?;
        // SAFETY: unwrap: usize is always wider than u5
        #[cfg_attr(not(feature = "cache_count"), allow(unused_variables))]
        let (last, count) = move_next(&self.arena, first, cast::<_, usize>(count).unwrap() - 1);
        let bubble = Bubble::Double {
            inner: (first, last),
            next: self.arena[last].next_mut().take(),
            #[cfg(feature = "cache_count")]
            count: count + T::one(),
        };
        self.top = Some(self.arena.insert(bubble));
        Some(())
    }
    #[inline]
    fn merge(&mut self) -> Option<()> {
        let first = self.top?;
        match self.arena[first] {
            Bubble::Single { next, .. } => {
                let second = next?;
                match &mut self.arena[second] {
                    Bubble::Single { next, .. } => {
                        let third = next.take();
                        let bubble = Bubble::Double {
                            inner: (first, second),
                            next: third,
                            // SAFETY: unwrap: every number type should be able to store 2
                            #[cfg(feature = "cache_count")]
                            count: cast(2).unwrap(),
                        };
                        self.top = Some(self.arena.insert(bubble));
                    }
                    Bubble::Double {
                        inner: (inner_first, _),
                        #[cfg(feature = "cache_count")]
                        count,
                        ..
                    } => {
                        let inner_first = replace(inner_first, first);
                        #[cfg(feature = "cache_count")]
                        (*count = *count + T::one());
                        *self.arena[first].next_mut() = Some(inner_first);
                        self.top = Some(second);
                    }
                }
            }
            Bubble::Double { next, .. } => {
                let second = next?;
                match &mut self.arena[second] {
                    Bubble::Single { next, .. } => {
                        let third = next.take();
                        // SAFETY: first is a double bubble by construction
                        let Some(Bubble::Double {
                            inner: (_, inner_last),
                            next,
                            #[cfg(feature = "cache_count")]
                            count,
                        }) = self.arena.get_mut(first)
                        else {
                            unreachable!()
                        };
                        let inner_last = replace(inner_last, second);
                        *next = third;
                        #[cfg(feature = "cache_count")]
                        (*count = *count + T::one());
                        *self.arena[inner_last].next_mut() = Some(second)
                    }
                    Bubble::Double { .. } => {
                        // SAFETY: second is a double bubble by construction
                        let Some(Bubble::Double {
                            inner: (right_first, right_last),
                            next: third,
                            #[cfg(feature = "cache_count")]
                                count: right_count,
                        }) = self.arena.remove(second)
                        else {
                            unreachable!()
                        };
                        // SAFETY: first is a bouble bubble by construction
                        let Some(Bubble::Double {
                            inner: (_, left_last),
                            next,
                            #[cfg(feature = "cache_count")]
                            count,
                        }) = self.arena.get_mut(first)
                        else {
                            unreachable!()
                        };
                        let left_last = replace(left_last, right_last);
                        *next = third;
                        #[cfg(feature = "cache_count")]
                        (*count = *count + right_count);
                        *self.arena[left_last].next_mut() = Some(right_first);
                    }
                }
            }
        }
        Some(())
    }
    #[inline]
    fn count(&mut self) -> Option<()> {
        let count = self.arena[self.top?].count(&self.arena);
        let bubble = Bubble::Single {
            value: count,
            next: self.top,
        };
        self.top = Some(self.arena.insert(bubble));
        Some(())
    }
    #[inline]
    fn combine_single<F>(&mut self, op: F) -> Option<()>
    where
        F: Fn(Self::Value, Self::Value) -> Self::Value,
    {
        /// Handle `single op double` case.
        /// `rhs` is first bubble in double, not the root.
        fn map_right<T: Value, F>(arena: &mut Arena<Bubble<T>>, lhs: T, mut rhs: Index, op: &F)
        where
            F: Fn(T, T) -> T,
        {
            loop {
                let next = match &mut arena[rhs] {
                    Bubble::Single { value, next } => {
                        *value = op(lhs, *value);
                        *next
                    }
                    Bubble::Double {
                        inner: (inner, _),
                        next,
                        ..
                    } => {
                        let (inner, next) = (*inner, *next);
                        map_right(arena, lhs, inner, op);
                        next
                    }
                };
                let Some(next) = next else { return };
                rhs = next;
            }
        }
        /// Handle `double op double` case.
        /// `lhs`/`rhs` is first bubble in double, not the root.
        /// # Returns
        /// In case of bubbles with different sizes, will return the first bubble without partner.
        #[inline]
        fn map_double<T: Value>(
            arena: &mut Arena<Bubble<T>>,
            mut lhs: Index,
            mut rhs: Index,
            op: &impl Fn(T, T) -> T,
            #[cfg(feature = "cache_count")] count: &mut T,
        ) -> Ref {
            #[cfg_attr(not(feature = "cache_count"), allow(unused_variables))]
            let one = T::one();
            loop {
                #[cfg(feature = "cache_count")]
                (*count = *count + one);
                let (next, _) = inner(arena, lhs, rhs, op);
                match next {
                    (Some(next_lhs), Some(next_rhs)) => (lhs, rhs) = (next_lhs, next_rhs),
                    (Some(rest), None) | (None, Some(rest)) => return Some(rest),
                    (None, None) => return None,
                }
            }
        }
        /// Handle unknown bubbles.
        /// # Returns
        /// Will return next pointers for both operands.
        /// Also returns `true` when `rhs` was removed.
        fn inner<T: Value>(
            arena: &mut Arena<Bubble<T>>,
            lhs: Index,
            rhs: Index,
            op: &impl Fn(T, T) -> T,
        ) -> ((Ref, Ref), bool) {
            // SAFETY: lhs and rhs exist and are distinct by construction
            match unsafe { arena.get_many_unchecked_mut([lhs, rhs]) } {
                [Bubble::Single {
                    value: value_lhs,
                    next: next_lhs,
                }, Bubble::Single {
                    value: value_rhs,
                    next: next_rhs,
                }] => {
                    let next = (*next_lhs, *next_rhs);
                    *value_rhs = op(*value_lhs, *value_rhs);
                    arena.remove(lhs);
                    (next, false)
                }
                [Bubble::Single {
                    value,
                    next: next_lhs,
                }, Bubble::Double {
                    inner: (inner, _),
                    next: next_rhs,
                    ..
                }] => {
                    let (next, value, inner) = ((*next_lhs, *next_rhs), *value, *inner);
                    arena.remove(lhs);
                    map_right(arena, value, inner, op);
                    (next, false)
                }
                [Bubble::Double {
                    inner: (inner, _),
                    next: next_lhs,
                    ..
                }, Bubble::Single {
                    value,
                    next: next_rhs,
                }] => {
                    let (next, value, inner) = ((*next_lhs, *next_rhs), *value, *inner);
                    arena.remove(rhs);
                    map_right(arena, value, inner, &|a, b| op(b, a));
                    (next, true)
                }
                [Bubble::Double {
                    inner: (inner_lhs, _),
                    next: next_lhs,
                    ..
                }, Bubble::Double {
                    inner: (inner_rhs, _),
                    next: next_rhs,
                    ..
                }] => {
                    let (next, inner_lhs, inner_rhs) =
                        ((*next_lhs, *next_rhs), *inner_lhs, *inner_rhs);
                    arena.remove(lhs);
                    #[cfg(feature = "cache_count")]
                    let mut new_count = T::zero();
                    let rest = map_double(
                        arena,
                        inner_lhs,
                        inner_rhs,
                        op,
                        #[cfg(feature = "cache_count")]
                        &mut new_count,
                    );
                    if let Some(rest) = rest {
                        remove_all(arena, rest);
                    }
                    #[cfg(feature = "cache_count")]
                    {
                        // SAFETY: rhs is a double bubble by construction
                        let Some(Bubble::Double { count, .. }) = arena.get_mut(rhs) else {
                            unreachable!()
                        };
                        *count = new_count
                    }
                    (next, false)
                }
            }
        }
        let lhs = self.top?;
        let rhs = self.arena[lhs].next()?;
        let ((_, third), relink) = inner(&mut self.arena, lhs, rhs, &op);
        if relink {
            *self.arena[rhs].next_mut() = third;
        } else {
            self.top = Some(rhs);
        }
        Some(())
    }

    fn combine_double<F1, F2>(&mut self, op1: F1, op2: F2) -> Option<()>
    where
        F1: Fn(Self::Value, Self::Value) -> Self::Value,
        F2: Fn(Self::Value, Self::Value) -> Self::Value,
    {
        /// Handle `single op double` case.
        /// `rhs` is first bubble in double, not the root.
        /// # Returns
        /// Will return the pointer to thr wrapping double bubble
        fn map_right<T: Value>(
            arena: &mut Arena<Bubble<T>>,
            lhs: T,
            mut rhs: Index,
            op1: &impl Fn(T, T) -> T,
            op2: &impl Fn(T, T) -> T,
        ) {
            let mut last = None;
            let mut left_value;
            loop {
                let next = match &mut arena[rhs] {
                    Bubble::Single {
                        value: right_value,
                        next,
                    } => {
                        let next = next.take();
                        (left_value, *right_value) =
                            (op1(lhs, *right_value), op2(lhs, *right_value));
                        let left = Bubble::Single {
                            value: left_value,
                            next: Some(rhs),
                        };
                        let left_index = arena.insert(left);
                        let outer = Bubble::Double {
                            inner: (left_index, rhs),
                            next: None,
                            // SAFETY: unwrap: 2 should fit into any number type
                            #[cfg(feature = "cache_count")]
                            count: cast::<_, T>(2).unwrap(),
                        };
                        let index = arena.insert(outer);
                        if let Some(last) = last {
                            *arena[last].next_mut() = Some(index);
                        }
                        next
                    }
                    Bubble::Double {
                        inner: (inner, _),
                        next,
                        ..
                    } => {
                        let (inner, next) = (*inner, *next);
                        map_right(arena, lhs, inner, op1, op2);
                        next
                    }
                };
                let Some(next) = next else { return };
                (last, rhs) = (Some(rhs), next);
            }
        }
        /// Handle `double op double` case.
        /// `lhs`/`rhs` is first bubble in double, not the root.
        /// # Returns
        /// In case of bubbles with different sizes, will return the first bubble without partner.
        #[inline]
        fn map_double<T: Value>(
            arena: &mut Arena<Bubble<T>>,
            mut lhs: Index,
            mut rhs: Index,
            op1: &impl Fn(T, T) -> T,
            op2: &impl Fn(T, T) -> T,
            #[cfg(feature = "cache_count")] count: &mut T,
        ) -> Ref {
            let mut last = None;
            #[cfg_attr(not(feature = "cache_count"), allow(unused_variables))]
            let one = T::one();
            loop {
                #[cfg(feature = "cache_count")]
                (*count = *count + one);
                let (outer, next) = inner(arena, lhs, rhs, op1, op2);
                if let Some(last) = last {
                    *arena[last].next_mut() = Some(outer);
                }
                last = Some(outer);
                match next {
                    (Some(next_lhs), Some(next_rhs)) => (lhs, rhs) = (next_lhs, next_rhs),
                    (Some(rest), None) | (None, Some(rest)) => return Some(rest),
                    (None, None) => return None,
                }
            }
        }
        /// Handle unknown bubbles.
        /// # Returns
        /// Will return the pointer to the wrapping double bubble
        /// Will also return next pointers for both operands.
        fn inner<T: Value>(
            arena: &mut Arena<Bubble<T>>,
            lhs: Index,
            rhs: Index,
            op1: &impl Fn(T, T) -> T,
            op2: &impl Fn(T, T) -> T,
        ) -> (Index, (Ref, Ref)) {
            // SAFETY: lhs and rhs exist and are distinct by construction
            match unsafe { arena.get_many_unchecked_mut([lhs, rhs]) } {
                [Bubble::Single {
                    value: left_value,
                    next: left_next,
                }, Bubble::Single {
                    value: right_value,
                    next: right_next,
                }] => {
                    let next = (replace(left_next, Some(rhs)), right_next.take());
                    (*left_value, *right_value) = (
                        op1(*left_value, *right_value),
                        op2(*left_value, *right_value),
                    );
                    let outer = Bubble::Double {
                        inner: (lhs, rhs),
                        next: None,
                        // SAFETY: unwrap: 2 should fit into any number type
                        #[cfg(feature = "cache_count")]
                        count: cast::<_, T>(2).unwrap(),
                    };
                    let index = arena.insert(outer);
                    (index, next)
                }
                [Bubble::Single {
                    value,
                    next: left_next,
                }, Bubble::Double {
                    inner: (inner, _),
                    next: right_next,
                    ..
                }] => {
                    let (value, inner, next) = (*value, *inner, (*left_next, *right_next));
                    arena.remove(lhs);
                    map_right(arena, value, inner, op1, op2);
                    (rhs, next)
                }
                [Bubble::Double {
                    inner: (inner, _),
                    next: left_next,
                    ..
                }, Bubble::Single {
                    value,
                    next: right_next,
                }] => {
                    let (value, inner, next) = (*value, *inner, (*left_next, *right_next));
                    arena.remove(rhs);
                    map_right(arena, value, inner, &|a, b| op1(b, a), &|a, b| op2(b, a));
                    (lhs, next)
                }
                [Bubble::Double {
                    inner: (left_inner, _),
                    next: left_next,
                    ..
                }, Bubble::Double {
                    inner: (right_inner, _),
                    next: right_next,
                    ..
                }] => {
                    let (left_inner, right_inner, next) =
                        (*left_inner, *right_inner, (*left_next, *right_next));
                    arena.remove(lhs);
                    #[cfg(feature = "cache_count")]
                    let mut new_count = T::zero();
                    let rest = map_double(
                        arena,
                        left_inner,
                        right_inner,
                        op1,
                        op2,
                        #[cfg(feature = "cache_count")]
                        &mut new_count,
                    );
                    if let Some(rest) = rest {
                        remove_all(arena, rest);
                    }
                    #[cfg(feature = "cache_count")]
                    {
                        // SAFETY: rhs is a double bubble by construction
                        let Some(Bubble::Double { count, .. }) = arena.get_mut(rhs) else {
                            unreachable!()
                        };
                        *count = new_count
                    }
                    (rhs, next)
                }
            }
        }
        let lhs = self.top?;
        let rhs = self.arena[lhs].next()?;
        let (outer, (_, third)) = inner(&mut self.arena, lhs, rhs, &op1, &op2);
        *self.arena[outer].next_mut() = third;
        self.top = Some(outer);
        Some(())
    }

    fn test<F>(&mut self, test: F) -> Option<bool>
    where
        F: Fn(&Self::Value, &Self::Value) -> bool,
    {
        let Some(Bubble::Single { value, next }) = self.arena.get(self.top?) else {
            return Some(false);
        };
        let (first, second) = (*value, (*next)?);
        let Some(Bubble::Single { value, .. }) = self.arena.get(second) else {
            return Some(false);
        };
        Some(test(&first, value))
    }
    #[inline]
    fn consume<F, E>(&mut self, mut fun: F) -> Result<Option<()>, E>
    where
        F: FnMut(Self::Value) -> Result<(), E>,
    {
        fn inner<T: Value, E>(
            arena: &mut Arena<Bubble<T>>,
            index: Index,
            fun: &mut impl FnMut(T) -> Result<(), E>,
        ) -> Result<Ref, E> {
            match arena.remove(index) {
                Some(Bubble::Single { value, next }) => {
                    fun(value)?;
                    Ok(next)
                }
                Some(Bubble::Double {
                    inner: (mut index, _),
                    next,
                    ..
                }) => loop {
                    if let Some(next) = inner(arena, index, fun)? {
                        index = next;
                    } else {
                        return Ok(next);
                    }
                },
                None => unreachable!(),
            }
        }
        let Some(top) = self.top else { return Ok(None) };
        self.top = inner(&mut self.arena, top, &mut fun)?;
        Ok(Some(()))
    }
}
impl<T: Value> Display for Abyss<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        #[inline]
        fn fmt_bubble<T: Value>(
            arena: &Arena<Bubble<T>>,
            index: Index,
            f: &mut std::fmt::Formatter<'_>,
        ) -> Result<Ref, std::fmt::Error> {
            match arena[index] {
                Bubble::Single { value, next } => {
                    value.fmt(f)?;
                    Ok(next)
                }
                Bubble::Double {
                    inner: (mut index, _),
                    next,
                    ..
                } => {
                    f.write_str("[")?;
                    loop {
                        let Some(next) = fmt_bubble(arena, index, f)? else {
                            break;
                        };
                        f.write_str(", ")?;
                        index = next;
                    }
                    f.write_str("]")?;
                    Ok(next)
                }
            }
        }
        let mut r#ref = self.top;
        while let Some(index) = r#ref {
            r#ref = fmt_bubble(&self.arena, index, f)?;
            f.write_str("\n")?;
        }
        Ok(())
    }
}
