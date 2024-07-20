use std::mem::replace;

#[cfg_attr(
    target_pointer_width = "64",
    rustc_layout_scalar_valid_range_end(0xffffffff_fffffffe)
)]
#[cfg_attr(
    target_pointer_width = "32",
    rustc_layout_scalar_valid_range_end(0xfffffffe)
)]
#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Index(usize);

type Ref = Option<Index>;

#[derive(Debug, Clone, Copy)]
enum Entry<T> {
    Occupied(T),
    Free(Ref),
}
impl<T> Entry<T> {
    #[inline]
    pub fn as_mut(&mut self) -> Entry<&mut T> {
        match self {
            Self::Occupied(value) => Entry::Occupied(value),
            Self::Free(free) => Entry::Free(*free),
        }
    }
    #[inline]
    pub fn into_occupied(self) -> Option<T> {
        match self {
            Self::Occupied(value) => Some(value),
            Self::Free(_) => None,
        }
    }
    #[inline]
    pub fn into_free(self) -> Option<Ref> {
        match self {
            Self::Occupied(_) => None,
            Self::Free(free) => Some(free),
        }
    }
}

#[derive(Debug, Clone)]
pub struct Arena<T> {
    heap: Vec<Entry<T>>,
    free_head: Ref,
}
impl<T> Arena<T> {
    #[inline(always)]
    pub const fn new() -> Self {
        Self {
            heap: Vec::new(),
            free_head: None,
        }
    }
    #[inline(always)]
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            heap: Vec::with_capacity(capacity),
            free_head: None,
        }
    }
    #[inline]
    pub fn insert(&mut self, value: T) -> Index {
        match self.free_head {
            Some(index) => {
                let free = replace(&mut self.heap[index.0], Entry::Occupied(value));
                // SAFETY: unwrap: free has to be a Free by construction
                self.free_head = free.into_free().unwrap();
                index
            }
            None => {
                // SAFETY: the index limit will not reasonably be reached
                let index = unsafe { Index(self.heap.len()) };
                self.heap.push(Entry::Occupied(value));
                index
            }
        }
    }
    #[inline]
    pub fn remove(&mut self, index: Index) -> Option<T> {
        let entry = self.heap.get_mut(index.0)?;
        match entry {
            Entry::Occupied(_) => {
                let value = replace(entry, Entry::Free(self.free_head));
                self.free_head = Some(index);
                // SAFETY: unwrap: value is an Occupied by construction
                Some(value.into_occupied().unwrap())
            }
            Entry::Free(_) => None,
        }
    }
    #[inline]
    pub fn get(&self, index: Index) -> Option<&T> {
        let entry = self.heap.get(index.0)?;
        match entry {
            Entry::Occupied(value) => Some(value),
            Entry::Free(_) => None,
        }
    }
    #[inline]
    pub fn get_mut(&mut self, index: Index) -> Option<&mut T> {
        let entry = self.heap.get_mut(index.0)?;
        match entry {
            Entry::Occupied(value) => Some(value),
            Entry::Free(_) => None,
        }
    }
    /// # Safety
    /// This doesn't check for out-of-bounds or aliased indices
    #[inline]
    pub unsafe fn get_many_unchecked_mut<const N: usize>(
        &mut self,
        indices: [Index; N],
    ) -> [&mut T; N] {
        let indices = indices.map(|i| i.0);
        // SAFETY: indices are in-bounds by assumption
        let entries = self.heap.get_many_unchecked_mut(indices);
        // SAFETY: unwrap: entries are occupied by assumption
        entries.map(|entry| entry.as_mut().into_occupied().unwrap_unchecked())
    }
}
impl<T> Default for Arena<T> {
    #[inline(always)]
    fn default() -> Self {
        Self::new()
    }
}
impl<T> std::ops::Index<Index> for Arena<T> {
    type Output = T;
    #[inline]
    fn index(&self, index: Index) -> &Self::Output {
        match self.heap.get(index.0).unwrap() {
            Entry::Occupied(value) => value,
            Entry::Free(_) => panic!("invalid index"),
        }
    }
}
impl<T> std::ops::IndexMut<Index> for Arena<T> {
    #[inline]
    fn index_mut(&mut self, index: Index) -> &mut Self::Output {
        match self.heap.get_mut(index.0).unwrap() {
            Entry::Occupied(value) => value,
            Entry::Free(_) => panic!("invalid index"),
        }
    }
}
