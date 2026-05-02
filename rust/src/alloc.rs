#![allow(unused)]
extern crate alloc;

use super::*;
use alloc::string::String;
use alloc::vec::Vec;
use core::cell::Cell;

/*
struct StoreItems<'a> {
    vec: Vec<Vec<Cell<Item<'a>>>>,
}
impl<'a> StoreItems<'a> {
    fn new() -> Self {
        StoreItems{ vec: Vec::new()}
    }
    fn more(&mut self, capacity:usize) -> &[Cell<Item<'a>>] {
        let last = self.vec.len();
        self.vec.push(Vec::with_capacity(capacity));
        &self.vec[last][..]
    }
}

struct Grow<'a, T:Copy> {
    next: Vec<Cell<T>>, // can safely be moved
    done: Vec<&'a [Cell<T>]>,
}
impl<'a, T:Copy> Grow<'a, T> {
    fn new() -> Self {
        Grow {
            next: Vec::new(),
            done: Vec::new(),
        }
    }
    fn push(&mut self, value: T) -> Option<()> {
        if self.next.len() == self.next.capacity() {
            if self.next.try_reserve(32).is_err() {
                return None;
            }
        }
        self.next.push(Cell::new(value));
        Some(())
    }
    fn hunt(&mut self, count:usize) -> Option<&'a [Cell<T>]> {
        for index in (0..self.done.len()).rev() {
            let done = self.done[index];
            if count == done.len() {
                self.done.swap_remove(index);
                return Some(done);
            } else if count < done.len() {
                self.done[index] = &done[count..];
                return Some(&done[..count]);
            }
        }
        return None;
    }
    fn finish(&mut self, count: usize) -> Option<&'a [Cell<T>]> {
        let start = self.next.len().checked_sub(count)?;
        let mut found = self.hunt(count);
        if found.is_none() {
            self.done.push((self.more)(count.max(80)));
            found = self.hunt(count);
        }
        let found = found?;
        for offset in 0..count {
            self.next[start+offset].swap(&found[offset]);
        }
        self.next.truncate(start);
        return Some(found);
    }
}

pub struct Arena<'a, I, E>
where
    I: FnMut(usize) -> &'a [Cell<Item<'a>>],
    E: FnMut(usize) -> &'a [Cell<Entry<'a>>],
 {
    items: Grow<'a, I, Item<'a>>,
    entries: Grow<'a, E, Entry<'a>>,
}
impl <'a, I, E> Arena<'a, I, E>
where
    I: FnMut(usize) -> &'a [Cell<Item<'a>>],
    E: FnMut(usize) -> &'a [Cell<Entry<'a>>],
 {
    pub fn new(items: I, entries: E) -> Self {
        Arena { items: Grow::new(items), entries: Grow::new(entries) }
    }
    pub fn list(&mut self, count: usize) -> Option<List<'a>> {
        Some(List::wrap(self.items.finish(count)?))
    }
    pub fn dict(&mut self, count: usize) -> Option<Dict<'a>> {
        Some(Dict::wrap(self.entries.finish(count)?))
    }
    pub fn item(&mut self, item: Item<'a>) -> Option<()> {
        self.items.push(item)
    }
    pub fn keyed(&mut self, key: &'a str, item: Item<'a>) -> Option<()> {
        self.entry(Entry::wrap(key, item))
    }
    pub fn entry(&mut self, entry: Entry<'a>) -> Option<()> {
        self.entries.push(entry)
    }
    pub fn text_item(&mut self, utf8: &'a str) -> Option<()> {
        self.item(Text::wrap(utf8).into())
    }
    pub fn list_item(&mut self, count: usize) -> Option<()> {
        let list = self.list(count)?;
        self.item(list.into())
    }
    pub fn dict_item(&mut self, count: usize) -> Option<()> {
        let dict = self.dict(count)?;
        self.item(dict.into())
    }
    pub fn text_entry(&mut self, key: &'a str, utf8: &'a str) -> Option<()> {
        self.keyed(key, Text::wrap(utf8).into())
    }
    pub fn list_entry(&mut self, key: &'a str, count: usize) -> Option<()> {
        let list = self.list(count)?;
        self.keyed(key, list.into())
    }
    pub fn dict_entry(&mut self, key: &'a str, count: usize) -> Option<()> {
        let dict = self.dict(count)?;
        self.keyed(key, dict.into())
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn draft_macro_expansion() {
        let mut item_store = StoreItems::new();
        let mut entry_store: Vec<Vec<Cell<Entry<'_>>>> = Vec::new();
        let arena = Arena::new(
            |x| item_store.more(x),
            |x|{
                let mut vec: Vec<Cell<Entry<'_>>> = Vec::with_capacity(x);
                while vec.len() < vec.capacity() {
                    vec.push(Entry::blank(0));
                }
                entry_store.push(vec);
                &entry_store[0][..]
            },
        );
    }
}
*/

impl<'a> UTF8<'a> {
    /// Allocates a [String], filled with the UTF-8 copied from `self`.
    fn joined(&self) -> String {
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
