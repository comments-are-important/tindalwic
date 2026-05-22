//! all this stuff is enabled by the "alloc" feature.

extern crate alloc;

use super::internals::Builder;
use crate::{Comment, Dict, Entry, File, Item, List, ParseError, Text, UTF8};
use alloc::string::String;
use alloc::vec::Vec;
use bumpalo::Bump;
use core::cell::Cell;

/// this pattern is typically implemented atop RefCell, but because this is in a
/// critical path, small unsafe blocks avoid the cost of those runtime checks.
struct CellVec<T>(Cell<Vec<T>>);
impl<T: Copy> CellVec<T> {
    fn new() -> Self {
        CellVec(Cell::new(Vec::new()))
    }
    fn push(&self, value: T) -> Option<()> {
        let CellVec(cell) = self;
        // SAFETY: Cell instance is private, no ref to its Vec value leaks outside this
        // impl, except via this let, only as receiver in Vec methods, which are safe.
        let vec = unsafe { &mut *cell.as_ptr() };
        vec.push(value);
        Some(())
    }
    fn finish<'b>(&self, count: usize, bump: &'b Bump) -> Option<&'b [Cell<T>]> {
        let CellVec(cell) = self;
        // SAFETY: Cell instance is private, no ref to its Vec value leaks outside this
        // impl, except via this let, only as receiver in Vec methods, which are safe.
        let vec = unsafe { &mut *cell.as_ptr() };
        let start = vec.len().checked_sub(count)?;
        let cells = bump.alloc_slice_fill_with(count, |i| Cell::new(vec[start + i]));
        vec.truncate(start);
        Some(cells)
    }
}

/// a flavor of Arena that uses bumpalo to put things in the heap.
/// TODO think about fleshing this out with more convenient methods.
pub struct Arena<'a, 'bump> {
    items: CellVec<Item<'a, 'bump>>,
    entries: CellVec<Entry<'a, 'bump>>,
    bump: &'bump Bump,
}
impl<'a, 'bump> Builder<'a, 'bump> for Arena<'a, 'bump> {
    fn list(&self, count: usize) -> Option<List<'a, 'bump>> {
        Some(List::wrap(self.items.finish(count, self.bump)?))
    }

    fn dict(&self, count: usize) -> Option<Dict<'a, 'bump>> {
        Some(Dict::wrap(self.entries.finish(count, self.bump)?))
    }

    fn item(&self, item: Item<'a, 'bump>) -> Option<()> {
        self.items.push(item)
    }

    fn entry(&self, entry: Entry<'a, 'bump>) -> Option<()> {
        self.entries.push(entry)
    }
}
impl<'a, 'bump> Arena<'a, 'bump> {
    /// the Bump needs an outside let binding so it lives long enough.
    pub fn new(bump: &'bump Bump) -> Self {
        Arena {
            items: CellVec::new(),
            entries: CellVec::new(),
            bump,
        }
    }
    /// copy a str into the bump
    pub fn intern(&self, value: &'_ str) -> &'bump str {
        self.bump.alloc_str(value)
    }
    /// call the parser on the provided content, panic if the content isn't legit.
    pub fn parse_or_panic(&self, content: &'a str) -> Option<File<'a, 'bump>> {
        self.parse(content, |error| panic!("{error}"))
    }
    /// call the parser on the provided content, with a callback for errors.
    pub fn parse<F: FnMut(ParseError)>(
        &self,
        content: &'a str,
        report: F,
    ) -> Option<File<'a, 'bump>> {
        crate::parse::Input::parse(self, content, report)
    }
}

impl<'a> UTF8<'a> {
    /// Allocates a [String], filled with the UTF-8 copied from `self`.
    pub(crate) fn joined(&self) -> String {
        if self.dedent == 0 || self.dedent == usize::MAX {
            String::from(self.slice)
        } else {
            let mut result = String::with_capacity(self.slice.len());
            for line in self.lines() {
                result.push_str(line);
                result.push('\n');
            }
            if !result.is_empty() {
                result.truncate(result.len() - 1);
            }
            result
        }
    }
}

impl<'a> Comment<'a> {
    /// Allocates a [String], filled with the UTF-8 copied from `self`.
    pub fn joined(&self) -> String {
        self.utf8.joined()
    }
}

impl<'a> Text<'a> {
    /// Allocates a [String], filled with the UTF-8 copied from `self`.
    pub fn joined(&self) -> String {
        self.utf8.joined()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::string::ToString;
    #[test]
    fn parse_alloc() {
        let bump = Bump::new();
        let arena = Arena::new(&bump);
        let file = arena.parse_or_panic("k=v\n").unwrap();
        assert_eq!(file.to_string(), "k=v\n");
    }
}
