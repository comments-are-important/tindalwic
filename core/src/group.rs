//! provide (controlled) access to the things inside private `cells` fields.

use crate::{Group, Lines, Member};
use core::cell::Cell;
// use core::ops::Deref;

macro_rules! cellular {
    ($(#[$meta:meta])* $name:ident<$inner:ident>$iterator:ident) => {
        #[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
        $(#[$meta])*
        pub struct $name<'a> {
            slice: &'a [Cell<$inner<'a>>],
        }
        impl<'a> From<&'a mut [$inner<'a>]> for $name<'a> {
            fn from(value: &'a mut [$inner<'a>]) -> Self {
                let slice = Cell::from_mut(value).as_slice_of_cells();
                $name { slice }
            }
        }
        impl<'a> $name<'a> {
            /// wrap the given cells as a single grouping
            pub(crate) fn new(slice: &'a [Cell<$inner<'a>>]) -> Self {
                $name { slice }
            }
            /// are there no items?
            pub fn is_empty(&self) -> bool {
                self.slice.is_empty()
            }
            /// number of items
            pub fn len(&self) -> usize {
                self.slice.len()
            }
            /// return Some item at the given index (or None if out of bounds)
            pub fn get(&self, index: usize) -> Option<$inner<'a>> {
                match self.slice.get(index) {
                    Some(cell) => Some(cell.get()),
                    None => None,
                }
            }
            /// set item at index and return Some (or None if out of bounds)
            pub fn set(&self, index: usize, item: $inner<'a>) -> Option<()> {
                match self.slice.get(index) {
                    Some(cell) => Some(cell.set(item)),
                    None => None,
                }
            }
            /// iterate the items
            pub fn iterator(&self) -> $iterator<'a> {
                $iterator {
                    pending: self.slice,
                }
            }
        }
        /// iterates over the elements
        pub struct $iterator<'a> {
            pending: &'a [Cell<$inner<'a>>],
        }
        impl<'a> Iterator for $iterator<'a> {
            type Item = $inner<'a>;
            fn next(&mut self) -> Option<Self::Item> {
                let Some((first, rest)) = self.pending.split_first() else {
                    return None;
                };
                self.pending = rest;
                Some(first.get())
            }
        }
        impl<'a> IntoIterator for $name<'a> {
            type Item = $inner<'a>;
            type IntoIter = $iterator<'a>;
            fn into_iter(self) -> $iterator<'a> {
                self.iterator()
            }
        }
        impl<'a> IntoIterator for &$name<'a> {
            type Item = $inner<'a>;
            type IntoIter = $iterator<'a>;
            fn into_iter(self) -> $iterator<'a> {
                self.iterator()
            }
        }
    };
}

cellular! {
    /// a sequence of Group providing interior mutability
    Groups<Group>GroupIterator
}
cellular! {
    /// a sequence of Member providing interior mutability
    Members<Member>MemberIterator
}

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
