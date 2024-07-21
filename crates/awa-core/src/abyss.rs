use std::fmt::Display;

use num_traits::{cast, Num, NumCast};

use crate::AwaSCII;

pub trait Value = Num + NumCast + PartialOrd + Copy + Display;

macro_rules! impl_copied {
    ($single:ident, $batched:ident) => {
        #[inline]
        #[doc = "Batch [`Abyss`] operation while re-using the arguments."]
        fn $batched(&mut self, count: usize) -> Option<()> {
            for _ in 0..count {
                self.$single()?;
            }
            Some(())
        }
    };
}
macro_rules! impl_buffered {
    ($single:ident, $batched:ident, $buffer:ident: $buffer_type:ty) => {
        #[inline]
        #[doc = "Batch [`Abyss`] operation while distributing the arguments."]
        fn $batched<B>(&mut self, $buffer: B) -> Option<()>
        where
            B: AsRef<[$buffer_type]>,
        {
            for single in $buffer.as_ref() {
                self.$single(*single)?
            }
            Some(())
        }
    };
}

/// Minimal functionallity for an Abyss data structure that is required to run an AWA program.
pub trait Abyss {
    type Value: Value;
    fn is_empty(&self) -> bool;
    /// Push AwaSCII string as a double bubble, empty string will push a single bubble with value zero.
    /// Returns `None` if the abyss is full.
    fn blow_awascii<B>(&mut self, awascii: B) -> Option<()>
    where
        B: AsRef<[AwaSCII]>;
    /// Push number as a new bubble.
    /// Returns `None` if the abyss is full.
    fn blow(&mut self, value: Self::Value) -> Option<()>;
    /// Move top bubble down, pass `0` to move to bottom.
    /// Returns `None` if there is no top bubble.
    fn submerge(&mut self, distance: usize) -> Option<()>;
    /// Remove the top bubble.
    /// Returns `None` if there is no top bubble.
    fn pop(&mut self) -> Option<()>;
    /// Remove the top bubble and in case of a double bubble will also remove all inner bubbles
    /// Returns `None` if there is no top bubble.
    fn double_pop(&mut self) -> Option<()>;
    /// Duplicates the top bubble.
    /// Returns `None` if there is no top bubble.
    fn duplicate(&mut self) -> Option<()>;
    /// Create a double bubble from the top bubbles.
    /// Returns `None` if there not enough bubbles.
    fn surround(&mut self, count: usize) -> Option<()>;
    /// Merges the top two bubbles into a single double bubble.
    /// Returns `None` if there are less then two bubbles on top.
    fn merge(&mut self) -> Option<()>;
    /// Pushes the size of the top bubble on top (single bubble will push zero).
    /// Return `None` if there is no top bubble.
    fn count(&mut self) -> Option<()>;
    /// Map the top two bubbles into one bubble.
    /// Returns `None` if there are less then two bubbles on top.
    fn combine_single<F>(&mut self, op: F) -> Option<()>
    where
        F: Fn(Self::Value, Self::Value) -> Self::Value;
    /// Map the top two bubbles into one bubble, creates a double bubble for each single bubble.
    /// Returns `None` if there are less then two bubbles on top.
    fn combine_double<F1, F2>(&mut self, op1: F1, op2: F2) -> Option<()>
    where
        F1: Fn(Self::Value, Self::Value) -> Self::Value,
        F2: Fn(Self::Value, Self::Value) -> Self::Value;
    /// Tests the top two bubbles and removes them, returning the result of the test.
    /// Returns `None` if there are less then two bubbles on top.
    fn test<F>(&mut self, test: F) -> Option<bool>
    where
        F: Fn(&Self::Value, &Self::Value) -> bool;
    /// Iterate over all values in the top bubble and removing it after, returning a possible error during iteration.
    /// Returns `None` if there is no top bubble.
    fn consume<F, E>(&mut self, fun: F) -> Result<Option<()>, E>
    where
        F: FnMut(Self::Value) -> Result<(), E>;

    impl_buffered!(blow, blow_many, values: Self::Value);
    impl_buffered!(submerge, submerge_many, distances: usize);
    impl_copied!(pop, pop_many);
    impl_copied!(double_pop, double_pop_many);
    impl_copied!(duplicate, duplicate_many);
    impl_buffered!(surround, surround_many, counts: usize);
    impl_copied!(merge, merge_many);
    /// Push new double bubble with the given elements.
    /// The last element will end up as the front.
    /// Will return `None` when the abyss is full or the double is too big.
    #[inline]
    fn blow_double<B>(&mut self, inner: B) -> Option<()>
    where
        B: AsRef<[Self::Value]>,
    {
        let count = cast(inner.as_ref().len())?;
        self.blow_many(inner)?;
        self.surround(count)
    }
}
