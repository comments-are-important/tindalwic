//! provide (controlled) access to the things inside private `cells` fields.

use crate::{Entry, Item};
use core::cell::Cell;
use core::ops::Deref;

mod sealed {
    pub(super) trait Sealed {}
    impl<'a> Sealed for crate::Item<'a> {}
    impl<'a> Sealed for crate::Entry<'a> {}
}

/// things that can be grouped
#[allow(private_bounds)]
pub trait Grouped: sealed::Sealed + Copy + Eq {
    // type Array;
}
impl<'a> Grouped for Item<'a> {
    // type Array = Items<'a>;
}
impl<'a> Grouped for Entry<'a> {
    // type Array = Entries<'a>;
}

/// a bit of encapsulation around the storage particulars.
/// TODO stop remaining leakage of Cell, keep it to privates in this file.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Elements<'a, T: Grouped> {
    slice: &'a [Cell<T>],
}
impl<'a, T: Grouped> From<&'a mut [T]> for Elements<'a, T> {
    fn from(value: &'a mut [T]) -> Self {
        let slice = Cell::from_mut(value).as_slice_of_cells();
        Elements { slice }
    }
}
impl<'a, T: Grouped> Elements<'a, T> {
    /// are there no items?
    pub fn is_empty(&self) -> bool {
        self.slice.is_empty()
    }
    /// number of items
    pub fn len(&self) -> usize {
        self.slice.len()
    }
    /// return Some element at the given index (or None if out of bounds)
    pub fn get(&self, index: usize) -> Option<T> {
        match self.slice.get(index) {
            Some(cell) => Some(cell.get()),
            None => None,
        }
    }
    /// set element at index and return Some (or None if out of bounds)
    pub fn set(&self, index: usize, element: T) -> Option<()> {
        match self.slice.get(index) {
            Some(cell) => Some(cell.set(element)),
            None => None,
        }
    }
    /// iterate the elements
    pub fn iterator(&self) -> GroupedIterator<'a, T> {
        GroupedIterator {
            pending: self.slice,
        }
    }
}

/// iterates over the grouped elements
pub struct GroupedIterator<'a, T: Grouped> {
    pending: &'a [Cell<T>],
}
impl<'a, T: Grouped> Iterator for GroupedIterator<'a, T> {
    type Item = T;
    fn next(&mut self) -> Option<Self::Item> {
        let Some((first, rest)) = self.pending.split_first() else {
            return None;
        };
        self.pending = rest;
        Some(first.get())
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Groups<'a, T: Grouped> {
    Single,
    #[expect(dead_code)]
    Multiple(&'a [&'a [Cell<T>]]),
}
impl<'a, T: Grouped> Default for Groups<'a, T> {
    fn default() -> Self {
        Groups::Single
    }
}

/// a sequence of items that can be divided into groups
#[derive(Clone, Copy, Default, Eq)]
pub struct Items<'a> {
    elements: Elements<'a, Item<'a>>,
    #[expect(dead_code)]
    groups: Groups<'a, Item<'a>>,
}
impl<'a> Deref for Items<'a> {
    type Target = Elements<'a, Item<'a>>;
    #[inline]
    fn deref(&self) -> &Elements<'a, Item<'a>> {
        &self.elements
    }
}
impl<'a> PartialEq for Items<'a> {
    fn eq(&self, other: &Self) -> bool {
        self.elements == other.elements
    }
}
impl<'a> IntoIterator for Items<'a> {
    type Item = Item<'a>;
    type IntoIter = GroupedIterator<'a, Self::Item>;
    fn into_iter(self) -> Self::IntoIter {
        self.iterator()
    }
}
impl<'a> IntoIterator for &Items<'a> {
    type Item = Item<'a>;
    type IntoIter = GroupedIterator<'a, Self::Item>;
    fn into_iter(self) -> Self::IntoIter {
        self.iterator()
    }
}
impl<'a> Items<'a> {
    /// wrap the given cells as a single grouping
    pub(crate) fn new(slice: &'a [Cell<Item<'a>>]) -> Self {
        Items {
            elements: Elements { slice },
            ..Default::default()
        }
    }
}
impl<'a> From<&'a mut [Item<'a>]> for Items<'a> {
    fn from(value: &'a mut [Item<'a>]) -> Self {
        Items {
            elements: value.into(),
            ..Default::default()
        }
    }
}

/// a sequence of entries that can be divided into groups
#[derive(Clone, Copy, Default, Eq)]
pub struct Entries<'a> {
    elements: Elements<'a, Entry<'a>>,
    #[expect(dead_code)]
    groups: Groups<'a, Entry<'a>>,
}
impl<'a> Deref for Entries<'a> {
    type Target = Elements<'a, Entry<'a>>;
    #[inline]
    fn deref(&self) -> &Elements<'a, Entry<'a>> {
        &self.elements
    }
}
impl<'a> PartialEq for Entries<'a> {
    fn eq(&self, other: &Self) -> bool {
        self.elements.eq(&other.elements)
    }
}
impl<'a> IntoIterator for Entries<'a> {
    type Item = Entry<'a>;
    type IntoIter = GroupedIterator<'a, Self::Item>;
    fn into_iter(self) -> Self::IntoIter {
        self.iterator()
    }
}
impl<'a> IntoIterator for &Entries<'a> {
    type Item = Entry<'a>;
    type IntoIter = GroupedIterator<'a, Self::Item>;
    fn into_iter(self) -> Self::IntoIter {
        self.iterator()
    }
}
impl<'a> Entries<'a> {
    /// wrap the given cells as a single grouping
    pub fn new(slice: &'a [Cell<Entry<'a>>]) -> Self {
        Entries {
            elements: Elements { slice },
            ..Default::default()
        }
    }
}
impl<'a> From<&'a mut [Entry<'a>]> for Entries<'a> {
    fn from(value: &'a mut [Entry<'a>]) -> Self {
        Entries {
            elements: value.into(),
            ..Default::default()
        }
    }
}
