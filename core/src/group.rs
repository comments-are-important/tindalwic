//! provide (controlled) access to the things inside private `cells` fields.

use super::{Entry, Item};
use core::cell::Cell;

/// seals Groups/Cells/CellsIterator to just our two structs
pub trait Grouped: Default + Copy + PartialEq {}
impl<'a> Grouped for Item<'a> {}
impl<'a> Grouped for Entry<'a> {}

#[derive(Clone, Copy, Debug)]
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

#[derive(Clone, Copy, Debug, Default)]
struct Cells<'a, T: Grouped> {
    slice: &'a [Cell<T>],
}
impl<'a, T: Grouped> From<&'a mut [T]> for Cells<'a, T> {
    fn from(value: &'a mut [T]) -> Self {
        let slice = Cell::from_mut(value).as_slice_of_cells();
        Cells { slice }
    }
}
impl<'a, T: Grouped> Cells<'a, T> {
    /// are there no items?
    pub fn is_empty(&self) -> bool {
        self.slice.is_empty()
    }
    /// number of items
    pub fn len(&self) -> usize {
        self.slice.len()
    }
    /// get item at index
    pub fn get(&self, index: usize) -> Option<T> {
        self.slice.get(index).map(|cell| cell.get())
    }
    /// set item at index
    pub fn set(&self, index: usize, grouped: T) {
        // TODO silently does nothing if index is OOB
        self.slice.get(index).map(|cell| cell.set(grouped));
    }
    fn eq(&self, other: &Self) -> bool {
        let length = self.slice.len();
        if length != other.slice.len() {
            return false;
        }
        for index in 0..length {
            if self.get(index) != other.get(index) {
                return false;
            }
        }
        true
    }
}

/// hides the Cell slice, iterates over the inner value
pub struct CellsIterator<'a, T: Grouped> {
    pending: &'a [Cell<T>],
}
impl<'a, T: Grouped> Iterator for CellsIterator<'a, T> {
    type Item = T;
    fn next(&mut self) -> Option<Self::Item> {
        let Some((first, rest)) = self.pending.split_first() else {
            return None;
        };
        self.pending = rest;
        let item = first.take();
        first.set(item);
        Some(item)
    }
}

/// a sequence of items that can be divided into groups
#[derive(Clone, Copy, Default)]
pub struct Items<'a> {
    cells: Cells<'a, Item<'a>>,
    #[expect(dead_code)]
    groups: Groups<'a, Item<'a>>,
}
impl<'a> From<&'a mut [Item<'a>]> for Items<'a> {
    fn from(value: &'a mut [Item<'a>]) -> Self {
        Items {
            cells: value.into(),
            ..Default::default()
        }
    }
}
impl<'a> IntoIterator for &Items<'a> {
    type Item = Item<'a>;
    type IntoIter = CellsIterator<'a, Self::Item>;
    fn into_iter(self) -> Self::IntoIter {
        CellsIterator {
            pending: self.cells.slice,
        }
    }
}
impl<'a> Eq for Items<'a> {}
impl<'a> PartialEq for Items<'a> {
    fn eq(&self, other: &Self) -> bool {
        self.cells.eq(&other.cells)
    }
}
impl<'a> Items<'a> {
    /// wrap the given cells as a single grouping
    pub(crate) fn new(slice: &'a [Cell<Item<'a>>]) -> Self {
        Items {
            cells: Cells { slice },
            ..Default::default()
        }
    }
    /// are there no items?
    pub fn is_empty(&self) -> bool {
        self.cells.is_empty()
    }
    /// number of items
    pub fn len(&self) -> usize {
        self.cells.len()
    }
    /// get item at index
    pub fn get(&self, index: usize) -> Option<Item<'a>> {
        self.cells.get(index)
    }
    /// set item at index
    pub fn set(&self, index: usize, item: Item<'a>) {
        self.cells.set(index, item);
    }
}

/// a sequence of entries that can be divided into groups
#[derive(Clone, Copy, Default)]
pub struct Entries<'a> {
    cells: Cells<'a, Entry<'a>>,
    #[expect(dead_code)]
    groups: Groups<'a, Entry<'a>>,
}
impl<'a> From<&'a mut [Entry<'a>]> for Entries<'a> {
    fn from(value: &'a mut [Entry<'a>]) -> Self {
        Entries {
            cells: value.into(),
            ..Default::default()
        }
    }
}
impl<'a> IntoIterator for &Entries<'a> {
    type Item = Entry<'a>;
    type IntoIter = CellsIterator<'a, Self::Item>;
    fn into_iter(self) -> Self::IntoIter {
        CellsIterator {
            pending: self.cells.slice,
        }
    }
}
impl<'a> Eq for Entries<'a> {}
impl<'a> PartialEq for Entries<'a> {
    fn eq(&self, other: &Self) -> bool {
        self.cells.eq(&other.cells)
    }
}
impl<'a> Entries<'a> {
    /// wrap the given cells as a single grouping
    pub fn new(slice: &'a [Cell<Entry<'a>>]) -> Self {
        Entries {
            cells: Cells { slice },
            ..Default::default()
        }
    }
    /// are there no entries?
    pub fn is_empty(&self) -> bool {
        self.cells.is_empty()
    }
    /// number of entries
    pub fn len(&self) -> usize {
        self.cells.len()
    }
    /// get entry at index
    pub fn get(&self, index: usize) -> Option<Entry<'a>> {
        self.cells.get(index)
    }
    /// set entry at index
    pub fn set(&self, index: usize, entry: Entry<'a>) {
        self.cells.set(index, entry);
    }
}
