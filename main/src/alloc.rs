//! all this stuff is enabled by the "alloc" feature.

extern crate alloc;

use crate::parse::{Builder, Input, ParseError, Reported};
use crate::{Comment, Dict, Entry, File, Item, List, Text, UTF8};
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

struct HeapBuilder<'a> {
    items: CellVec<Item<'a>>,
    entries: CellVec<Entry<'a>>,
    bump: &'a Bump,
}
impl<'a> Builder<'a> for HeapBuilder<'a> {
    fn list(&self, count: usize) -> Result<List<'a>, ParseError> {
        match self.items.finish(count, self.bump) {
            Some(list) => Ok(List::wrap(list)),
            None => Err(ParseError::mem("not enough items to make that list")),
        }
    }
    fn dict(&self, count: usize) -> Result<Dict<'a>, ParseError> {
        match self.entries.finish(count, self.bump) {
            Some(dict) => Ok(Dict::wrap(dict)),
            None => Err(ParseError::mem("not enough entries to make that dict")),
        }
    }
    fn item(&self, item: Item<'a>) -> Result<(), ParseError> {
        self.items
            .push(item)
            .ok_or_else(|| ParseError::mem("no room for item"))
    }
    fn entry(&self, entry: Entry<'a>) -> Result<(), ParseError> {
        self.entries
            .push(entry)
            .ok_or_else(|| ParseError::mem("no room for entry"))
    }
}

/// a flavor of Arena that uses bumpalo to put things in the heap.
/// TODO think about fleshing this out with more convenient methods.
pub struct Arena<'a> {
    builder: HeapBuilder<'a>,
}
impl<'a> Arena<'a> {
    /// the Bump needs an outside let binding so it lives long enough.
    pub fn new(bump: &'a Bump) -> Self {
        let builder = HeapBuilder {
            items: CellVec::new(),
            entries: CellVec::new(),
            bump,
        };
        Arena { builder }
    }
    /// after `count` calls to .item, call this to build a list of those.
    pub fn list(&self, count: usize) -> Result<List<'a>, ParseError> {
        self.builder.list(count)
    }
    /// after `count` calls to .entry, call this to build a dict of those.
    pub fn dict(&self, count: usize) -> Result<Dict<'a>, ParseError> {
        self.builder.dict(count)
    }
    /// push an item into builder memory for future .list call to use.
    pub fn item(&self, item: Item<'a>) -> Result<(), ParseError> {
        self.builder.item(item)
    }
    /// push an entry into builder memory for future .dict call to use.
    pub fn entry(&self, entry: Entry<'a>) -> Result<(), ParseError> {
        self.builder.entry(entry)
    }
    /// copy a str into the bump
    pub fn intern(&self, value: &'_ str) -> &'a str {
        self.builder.bump.alloc_str(value)
    }
    /// call the parser on the provided content, panic if the content isn't legit.
    pub fn parse_or_panic(&self, content: &'a str) -> File<'a> {
        self.parse(content, |error| panic!("{error}"))
            .expect("panic should have already happened in report")
    }
    /// call the parser on the provided content, collect first `count` errors.
    pub fn parse_collect(
        &self,
        content: &'a str,
        count: usize,
    ) -> Result<File<'a>, Vec<ParseError>> {
        let mut errors = Vec::new();
        self.parse(content, |err| {
            if errors.len() >= count {
                return Reported::Abort;
            }
            errors.push(err);
            Reported::Continue
        })
        .ok_or_else(|| errors)
    }
    /// call the parser on the provided content, with a callback for errors.
    pub fn parse<F: FnMut(ParseError) -> Reported>(
        &self,
        content: &'a str,
        report: F,
    ) -> Option<File<'a>> {
        Input::parse(&self.builder, content, report)
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
        let file = arena.parse_or_panic("k=v\n");
        assert_eq!(file.to_string(), "k=v\n");
    }
}
