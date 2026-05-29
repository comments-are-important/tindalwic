//! macros depend on these, so must be public.
//! but you should probably not use these directly, macros are much easier.

use crate::parse::{Builder, Input, ParseError, Reported};
use crate::{Dict, Entry, File, Item, List, Value};

use core::cell::Cell;

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
        let Some(cells) = self.items.finish(count) else {
            return Err(ParseError::mem("not enough items to make that list"));
        };
        Ok(List {
            cells,
            ..Default::default()
        })
    }
    fn dict(&self, count: usize) -> Result<Dict<'a>, ParseError> {
        let Some(cells) = self.entries.finish(count) else {
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

/// a flavor of Arena that uses fixed-size array slices.
/// the arrays can live in the stack.
pub struct Arena<'a> {
    builder: StackBuilder<'a>,
}
impl<'a> Arena<'a> {
    /// provide the storage
    pub fn wrap(items: &'a [Cell<Item<'a>>], entries: &'a [Cell<Entry<'a>>]) -> Self {
        let builder = StackBuilder::wrap(items, entries);
        Arena { builder }
    }
    /// returns count of items that can still fit.
    pub fn item_slots(&self) -> usize {
        self.builder.items.done.get() - self.builder.items.next.get()
    }
    /// returns count of entries that can still fit.
    pub fn entry_slots(&self) -> usize {
        self.builder.entries.done.get() - self.builder.entries.next.get()
    }
    /// the json! macro uses this as a sanity check that no space gets wasted.
    pub fn completed(&self) -> Option<()> {
        if self.builder.items.done.get() == 0 && self.builder.entries.done.get() == 0 {
            Some(())
        } else {
            None
        }
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
    /// push an entry into builder memory for future .dict call to use.
    pub fn keyed(&self, key: &'a str, item: Item<'a>) -> Result<(), ParseError> {
        self.entry(Entry {
            key: Value::new(key),
            item,
            ..Default::default()
        })
    }
    /// push a text item into builder memory for future .list call to use.
    pub fn text_item(&self, value: &'a str) -> Result<(), ParseError> {
        self.item(Item::text(value))
    }
    /// push a list item into builder memory for future .list call to use.
    pub fn list_item(&self, count: usize) -> Result<(), ParseError> {
        let list = self.list(count)?;
        self.item(list.into())
    }
    /// push a dict item into builder memory for future .list call to use.
    pub fn dict_item(&self, count: usize) -> Result<(), ParseError> {
        let dict = self.dict(count)?;
        self.item(dict.into())
    }
    /// push a text entry into builder memory for future .dict call to use.
    pub fn text_entry(&self, key: &'a str, value: &'a str) -> Result<(), ParseError> {
        self.keyed(key, Item::text(value))
    }
    /// push a list entry into builder memory for future .dict call to use.
    pub fn list_entry(&self, key: &'a str, count: usize) -> Result<(), ParseError> {
        let list = self.list(count)?;
        self.keyed(key, list.into())
    }
    /// push a dict entry into builder memory for future .dict call to use.
    pub fn dict_entry(&self, key: &'a str, count: usize) -> Result<(), ParseError> {
        let dict = self.dict(count)?;
        self.keyed(key, dict.into())
    }
    /// call the parser on the provided content, panic if the content isn't legit.
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
