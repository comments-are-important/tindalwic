//! provide (controlled) access to the things inside private `cells` fields.

use crate::{Group, Lines, Member};
use core::cell::Cell;

/// a sequence of Group providing interior mutability
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[tindalwic_macros::check_fields]
pub struct Groups<'a> {
    items: &'a [Cell<Group<'a>>],
}

/// a sequence of Member providing interior mutability
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[tindalwic_macros::check_fields]
pub struct Members<'a> {
    items: &'a [Cell<Member<'a>>],
}

tindalwic_macros::template! { ? array ^ {^
    impl<'a> From<&'a mut [$Link<'a>]> for $Entity<'a> {
        fn from(value: &'a mut [$Link<'a>]) -> Self {
            let items = Cell::from_mut(value).as_slice_of_cells();
            $Entity { items }
        }
    }
    impl<'a> $Entity<'a> {
        /// wrap the given cells as a single grouping
        pub(crate) fn new(items: &'a [Cell<$Link<'a>>]) -> Self {
            $Entity { items }
        }
        /// are there no items?
        pub fn is_empty(&self) -> bool {
            self.items.is_empty()
        }
        /// number of items
        pub fn len(&self) -> usize {
            self.items.len()
        }
        /// return Some item at the given index (or None if out of bounds)
        pub fn get(&self, index: usize) -> Option<$Link<'a>> {
            match self.items.get(index) {
                Some(cell) => Some(cell.get()),
                None => None,
            }
        }
        /// set item at index and return Some (or None if out of bounds)
        pub fn set(&self, index: usize, item: $Link<'a>) -> Option<()> {
            match self.items.get(index) {
                Some(cell) => Some(cell.set(item)),
                None => None,
            }
        }
        /// iterate the items
        pub fn iterator(&self) -> Entity@Iterator<'a> {
            Entity@Iterator {
                pending: self.items,
            }
        }
    }
    /// iterates over the elements
    pub struct Entity@Iterator<'a> {
        pending: &'a [Cell<$Link<'a>>],
    }
    impl<'a> Iterator for Entity@Iterator<'a> {
        type Item = $Link<'a>;
        fn next(&mut self) -> Option<Self::Item> {
            let Some((first, rest)) = self.pending.split_first() else {
                return None;
            };
            self.pending = rest;
            Some(first.get())
        }
    }
    impl<'a> IntoIterator for $Entity<'a> {
        type Item = $Link<'a>;
        type IntoIter = Entity@Iterator<'a>;
        fn into_iter(self) -> Entity@Iterator<'a> {
            self.iterator()
        }
    }
    impl<'a> IntoIterator for &$Entity<'a> {
        type Item = $Link<'a>;
        type IntoIter = Entity@Iterator<'a>;
        fn into_iter(self) -> Entity@Iterator<'a> {
            self.iterator()
        }
    }
}}

impl<'a> Groups<'a> {
    /// flatten groups
    pub fn members(&self) -> impl Iterator<Item = Member<'a>> {
        self.into_iter().flat_map(|it| it.members.into_iter())
    }
    /// O(N) lookup
    pub fn named<T>(&self, name: T) -> impl Iterator<Item = Member<'a>>
    where
        Lines<'a>: From<T>,
    {
        let search: Lines<'a> = name.into();
        self.members().filter(move |it| it.name == search)
    }
}
