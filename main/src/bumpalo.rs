//! all this stuff is enabled by the "bumpalo" feature.

extern crate alloc;

use crate::alloc::Intern;
use crate::parse::{Builder, Input, ParseError, Reported};
use crate::{Dict, Entry, File, Item, List};
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

pub(crate) struct HeapBuilder<'a> {
    items: CellVec<Item<'a>>,
    entries: CellVec<Entry<'a>>,
    bump: &'a Bump,
}
impl<'a> Builder<'a> for HeapBuilder<'a> {
    fn list(&self, count: usize) -> Result<List<'a>, ParseError> {
        let Some(cells) = self.items.finish(count, self.bump) else {
            return Err(ParseError::mem("not enough items to make that list"));
        };
        Ok(List {
            cells,
            ..Default::default()
        })
    }
    fn dict(&self, count: usize) -> Result<Dict<'a>, ParseError> {
        let Some(cells) = self.entries.finish(count, self.bump) else {
            return Err(ParseError::mem("not enough entries to make that dict"));
        };
        Ok(Dict {
            cells,
            ..Default::default()
        })
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
impl<'a> Intern<'a> for HeapBuilder<'a> {
    fn str(&self, value: &'_ str) -> &'a str {
        self.bump.alloc_str(value)
    }
}

/// a flavor of Arena that uses bumpalo to put things in the heap.
/// TODO think about fleshing this out with more convenient methods.
pub struct Arena<'a> {
    pub(crate) builder: HeapBuilder<'a>,
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
    pub fn str(&self, value: &'_ str) -> &'a str {
        self.builder.str(value)
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
    #[test]
    fn invalid() {
        let bump = Bump::new();
        let arena = Arena::new(&bump);
        let Err(errors) = arena.parse_collect("nope", usize::MAX) else {
            panic!("got a file expected parse error")
        };
        assert_eq!(errors.len(), 1);
    }
}
