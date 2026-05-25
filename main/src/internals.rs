#![doc(hidden)] // only public so macro generated code can access.

use super::parse::{Builder, Input, ParseError, Reported};
use super::*;

/// push T into stack on low side of array, finish them into high side.
/// aligns to an in-order tree traversal: push on entry, visit kids, finish on exit.
/// all kids finished before visiting next sibling, so siblings are adjacent,
/// finish moves those adjacent cells to the high end of the array.
/// total O(n) moves, zero extra space, caller only needs to track child count.
/// basically two-stacks-in-one-array (Knuth, TAOCP Vol. 1, §2.2.2 p.246),
/// but keep siblings together by transferring, as group, from low to high.
struct LowToHigh<'a, T> {
    cells: &'a [Cell<T>],
    next: Cell<usize>,
    done: Cell<usize>,
}
impl<'a, T> LowToHigh<'a, T> {
    fn wrap(cells: &'a [Cell<T>]) -> Self {
        LowToHigh {
            cells,
            next: 0.into(),
            done: cells.len().into(),
        }
    }
    fn push(&self, value: T) -> Option<()> {
        if self.next >= self.done {
            return None;
        }
        let next = self.next.get();
        self.cells[next].set(value);
        self.next.set(next + 1);
        Some(())
    }
    fn finish(&self, count: usize) -> Option<&'a [Cell<T>]> {
        let next = self.next.get();
        let done = self.done.get();
        if next < count || next > done {
            return None;
        }
        if next == done {
            let both = next - count;
            self.next.set(both);
            self.done.set(both);
            return Some(&self.cells[both..both + count]);
        }
        let next = next - count;
        let done = done - count;
        for offset in (0..count).rev() {
            self.cells[next + offset].swap(&self.cells[done + offset]);
        }
        self.next.set(next);
        self.done.set(done);
        Some(&self.cells[done..done + count])
    }
}

struct StackBuilder<'a> {
    items: LowToHigh<'a, Item<'a>>,
    entries: LowToHigh<'a, Entry<'a>>,
}
impl<'a> StackBuilder<'a> {
    pub fn wrap(items: &'a [Cell<Item<'a>>], entries: &'a [Cell<Entry<'a>>]) -> Self {
        let items = LowToHigh::wrap(items);
        let entries = LowToHigh::wrap(entries);
        StackBuilder { items, entries }
    }
}
impl<'a> Builder<'a> for StackBuilder<'a> {
    fn list(&self, count: usize) -> Result<List<'a>, ParseError> {
        match self.items.finish(count) {
            Some(list) => Ok(List::wrap(list)),
            None => Err(ParseError::mem("not enough items to make that list")),
        }
    }
    fn dict(&self, count: usize) -> Result<Dict<'a>, ParseError> {
        match self.entries.finish(count) {
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

pub struct Arena<'a> {
    builder: StackBuilder<'a>,
}
impl<'a> Arena<'a> {
    pub fn wrap(items: &'a [Cell<Item<'a>>], entries: &'a [Cell<Entry<'a>>]) -> Self {
        let builder = StackBuilder::wrap(items, entries);
        Arena { builder }
    }
    pub fn item_slots(&self) -> usize {
        self.builder.items.done.get() - self.builder.items.next.get()
    }
    pub fn entry_slots(&self) -> usize {
        self.builder.entries.done.get() - self.builder.entries.next.get()
    }
    pub fn completed(&self) -> Option<()> {
        if self.builder.items.done.get() == 0 && self.builder.entries.done.get() == 0 {
            Some(())
        } else {
            None
        }
    }
    pub fn list(&self, count: usize) -> Result<List<'a>, ParseError> {
        self.builder.list(count)
    }
    pub fn dict(&self, count: usize) -> Result<Dict<'a>, ParseError> {
        self.builder.dict(count)
    }
    pub fn item(&self, item: Item<'a>) -> Result<(), ParseError> {
        self.builder.item(item)
    }
    pub fn entry(&self, entry: Entry<'a>) -> Result<(), ParseError> {
        self.builder.entry(entry)
    }
    pub fn keyed(&self, key: &'a str, item: Item<'a>) -> Result<(), ParseError> {
        self.entry(Entry::wrap(key, item))
    }
    pub fn text_item(&self, utf8: &'a str) -> Result<(), ParseError> {
        self.item(Text::wrap(utf8).into())
    }
    pub fn list_item(&self, count: usize) -> Result<(), ParseError> {
        let list = self.list(count)?;
        self.item(list.into())
    }
    pub fn dict_item(&self, count: usize) -> Result<(), ParseError> {
        let dict = self.dict(count)?;
        self.item(dict.into())
    }
    pub fn text_entry(&self, key: &'a str, utf8: &'a str) -> Result<(), ParseError> {
        self.keyed(key, Text::wrap(utf8).into())
    }
    pub fn list_entry(&self, key: &'a str, count: usize) -> Result<(), ParseError> {
        let list = self.list(count)?;
        self.keyed(key, list.into())
    }
    pub fn dict_entry(&self, key: &'a str, count: usize) -> Result<(), ParseError> {
        let dict = self.dict(count)?;
        self.keyed(key, dict.into())
    }
    pub fn parse_or_panic(&self, content: &'a str) -> File<'a> {
        Input::parse(&self.builder, content, |error| panic!("{error}"))
            .expect("panic should have already happened in report")
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

// ====================================================================================

/// an iter type to enable for-loops for List, Dict, and File.
#[derive(Clone, Debug)]
pub struct CellIter<'a, T: Copy> {
    pub(crate) inner: core::slice::Iter<'a, Cell<T>>,
}
impl<'a, T: Copy> Iterator for CellIter<'a, T> {
    type Item = T;
    fn next(&mut self) -> Option<Self::Item> {
        self.inner.next().map(Cell::get)
    }
    fn size_hint(&self) -> (usize, Option<usize>) {
        self.inner.size_hint()
    }
}
impl<'a, T: Copy> ExactSizeIterator for CellIter<'a, T> {}
impl<'a, T: Copy> DoubleEndedIterator for CellIter<'a, T> {
    fn next_back(&mut self) -> Option<Self::Item> {
        self.inner.next_back().map(Cell::get)
    }
}
impl<'a, T: Copy> core::iter::FusedIterator for CellIter<'a, T> {}

// ====================================================================================

#[derive(Debug)]
pub enum Branch<'p> {
    List(usize),
    Dict(Key<'p>),
}
#[derive(Debug)]
pub struct PathError<'p> {
    pub failed: &'p [Branch<'p>],
    pub message: &'static str,
}
#[derive(Debug)]
pub struct Path<'p> {
    pub branches: &'p [Branch<'p>],
}
impl<'p> Path<'p> {
    pub fn wrap(branches: &'p [Branch<'p>]) -> Self {
        Path { branches }
    }
    pub fn error_full(&self, message: &'static str) -> PathError<'p> {
        PathError {
            failed: &self.branches[..],
            message,
        }
    }
    pub fn error_at(&self, bad: usize, message: &'static str) -> PathError<'p> {
        PathError {
            failed: &self.branches[..=bad],
            message,
        }
    }
    pub fn text<'a>(&self, item: &Item<'a>) -> Result<Text<'a>, PathError<'p>> {
        let Item::Text(text) = item else {
            return Err(self.error_full("path does not end at Text"));
        };
        Ok(*text)
    }
    pub fn list<'a>(&self, item: &Item<'a>) -> Result<List<'a>, PathError<'p>> {
        let Item::List(list) = item else {
            return Err(self.error_full("path does not end at List"));
        };
        Ok(*list)
    }
    pub fn dict<'a>(&self, item: &Item<'a>) -> Result<Dict<'a>, PathError<'p>> {
        let Item::Dict(dict) = item else {
            return Err(self.error_full("path does not end at Dict"));
        };
        Ok(*dict)
    }
    pub fn item_cell<'a>(&self, item: &Item<'a>) -> Result<&'a Cell<Item<'a>>, PathError<'p>> {
        if self.branches.is_empty() {
            return Err(self.error_full("empty path can't be resolved"));
        }
        let mut from = *item;
        for (step, branch) in self.branches.iter().enumerate() {
            match &from {
                Item::Text(_text) => {
                    return Err(self.error_at(step, "path ended prematurely by a text item"));
                }
                Item::List(list) => match branch {
                    Branch::List(at) => match list.cells.get(*at) {
                        None => return Err(self.error_at(step, "index out of bounds")),
                        Some(found) => {
                            if step + 1 == self.branches.len() {
                                return Ok(found);
                            }
                            from = found.get();
                        }
                    },
                    Branch::Dict(_) => {
                        return Err(self.error_at(step, "path expected dict but found list"));
                    }
                },
                Item::Dict(dict) => match branch {
                    Branch::Dict(key) => {
                        match dict.position(key) {
                            None => return Err(self.error_at(step, "key not found")),
                            Some(found) => {
                                from = dict.cells[found].get().item;
                            }
                        };
                    }
                    Branch::List(_) => {
                        return Err(self.error_at(step, "path expected list but found dict"));
                    }
                },
            }
        }
        Err(self.error_full("path did not end at an item inside a list"))
    }
    pub fn entry_cell<'a>(&self, item: &Item<'a>) -> Result<&'a Cell<Entry<'a>>, PathError<'p>> {
        if self.branches.is_empty() {
            return Err(self.error_full("empty path can't be resolved"));
        }
        let mut from = *item;
        for (step, branch) in self.branches.iter().enumerate() {
            match &from {
                Item::Text(_text) => {
                    return Err(self.error_at(step, "path ended prematurely by a text item"));
                }
                Item::List(list) => match branch {
                    Branch::List(at) => match list.cells.get(*at) {
                        None => return Err(self.error_at(step, "index out of bounds")),
                        Some(found) => {
                            from = found.get();
                        }
                    },
                    Branch::Dict(_) => {
                        return Err(self.error_at(step, "path expected dict but found list"));
                    }
                },
                Item::Dict(dict) => match branch {
                    Branch::Dict(key) => {
                        match dict.position(key) {
                            None => return Err(self.error_at(step, "key not found")),
                            Some(found) => {
                                if step + 1 == self.branches.len() {
                                    return Ok(&dict.cells[found]);
                                }
                                from = dict.cells[found].get().item;
                            }
                        };
                    }
                    Branch::List(_) => {
                        return Err(self.error_at(step, "path expected list but found dict"));
                    }
                },
            }
        }
        Err(self.error_full("path did not end at an entry inside a dict"))
    }
}
