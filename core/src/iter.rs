//! provide (controlled) access to the things inside private `cells` fields.

use super::*;

/// an iterator for Items
pub struct ItemsIterator<'a> {
    pending: &'a [Cell<Item<'a>>],
}
impl<'a> Iterator for ItemsIterator<'a> {
    type Item /* same name but not ours */ = crate::Item<'a>;
    fn next(&mut self) -> Option<Self::Item /* okay */> {
        let Some((first, rest)) = self.pending.split_first() else {
            return None;
        };
        self.pending = rest;
        let item = first.take();
        first.set(item.clone());
        Some(item)
    }
}
impl<'a> IntoIterator for &Items<'a> {
    type Item /* same name but not ours */ = crate::Item<'a>;
    type IntoIter = ItemsIterator<'a>;
    fn into_iter(self) -> Self::IntoIter {
        ItemsIterator {
            pending: self.cells,
        }
    }
}
impl<'a> Eq for Items<'a> {}
impl<'a> PartialEq for Items<'a> {
    fn eq(&self, other: &Self) -> bool {
        self.len() == other.len() && self.into_iter().eq(other.into_iter())
    }
}
impl<'a> From<&'a [Cell<Item<'a>>]> for Items<'a> {
    fn from(cells: &'a [Cell<Item<'a>>]) -> Self {
        Items { cells }
    }
}
impl<'a> Items<'a> {
    /// are there no items?
    pub fn is_empty(&self) -> bool {
        self.cells.is_empty()
    }
    /// number of items
    pub fn len(&self) -> usize {
        self.cells.len()
    }
    /// get item (clone) at index
    pub fn get(&self, index: usize) -> Option<Item<'a>> {
        self.cells.get(index).map(|cell| {
            let item = cell.take();
            cell.set(item.clone());
            item
        })
    }
    /// set item at index
    pub fn set(&self, index: usize, item: &Item<'a>) {
        self.cells.get(index).map(|cell| cell.set(item.clone()));
    }
}

/// an iterator for Entries
pub struct EntriesIterator<'a> {
    pending: &'a [Cell<Entry<'a>>],
}
impl<'a> Iterator for EntriesIterator<'a> {
    type Item /* same name but not ours */ = crate::Entry<'a>;
    fn next(&mut self) -> Option<Self::Item /* confusing */> {
        let Some((first, rest)) = self.pending.split_first() else {
            return None;
        };
        self.pending = rest;
        let entry = first.take();
        first.set(entry.clone());
        Some(entry)
    }
}
impl<'a> IntoIterator for &Entries<'a> {
    type Item /* same name but not ours */ = crate::Entry<'a>;
    type IntoIter = EntriesIterator<'a>;
    fn into_iter(self) -> Self::IntoIter {
        EntriesIterator {
            pending: self.cells,
        }
    }
}
impl<'a> Eq for Entries<'a> {}
impl<'a> PartialEq for Entries<'a> {
    fn eq(&self, other: &Self) -> bool {
        self.len() == other.len() && self.into_iter().eq(other.into_iter())
    }
}
impl<'a> From<&'a [Cell<Entry<'a>>]> for Entries<'a> {
    fn from(cells: &'a [Cell<Entry<'a>>]) -> Self {
        Entries { cells }
    }
}
impl<'a> Entries<'a> {
    /// are there no entries?
    pub fn is_empty(&self) -> bool {
        self.cells.is_empty()
    }
    /// number of entries
    pub fn len(&self) -> usize {
        self.cells.len()
    }
    /// get entry (clone) at index
    pub fn get(&self, index: usize) -> Option<Entry<'a>> {
        self.cells.get(index).map(|cell| {
            let entry = cell.take();
            cell.set(entry.clone());
            entry
        })
    }
    /// set entry at index
    pub fn set(&self, index: usize, entry: &Entry<'a>) {
        self.cells.get(index).map(|cell| cell.set(entry.clone()));
    }
    /// `O(N)` lookup.
    pub fn linear_search<T>(&self, key: T) -> Option<Entry<'a>>
    where
        Value<'a>: From<T>,
    {
        let value: Value<'a> = key.into();
        for entry in self {
            if value == entry.name.key {
                return Some(entry);
            }
        }
        None
    }
}
