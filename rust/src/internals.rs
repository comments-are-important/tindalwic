#![doc(hidden)] // only public so macro generated code can access.

use super::*;

/// TODO return Results, not Options, here and in Arenas.
pub trait Builder<'a, 'store> {
    fn list(&self, count: usize) -> Option<List<'a, 'store>>;
    fn dict(&self, count: usize) -> Option<Dict<'a, 'store>>;
    fn item(&self, item: Item<'a, 'store>) -> Option<()>;
    fn entry(&self, entry: Entry<'a, 'store>) -> Option<()>;
}

/// push T into stack on low side of array, finish them into high side.
/// aligns to an in-order tree traversal: push on entry, visit kids, finish on exit.
/// all kids finished before visiting next sibling, so siblings are adjacent,
/// finish moves those adjacent cells to the high end of the array.
/// total O(n) moves, zero extra space, caller only needs to track child count.
/// basically two-stacks-in-one-array (Knuth, TAOCP Vol. 1, §2.2.2 p.246),
/// but keep siblings together by transferring, as group, from low to high.
struct LowToHigh<'store, T> {
    cells: &'store [Cell<T>],
    next: Cell<usize>,
    done: Cell<usize>,
}
impl<'store, T> LowToHigh<'store, T> {
    fn wrap(cells: &'store [Cell<T>]) -> Self {
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
    fn finish(&self, count: usize) -> Option<&'store [Cell<T>]> {
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

pub struct Arena<'a, 'store> {
    items: LowToHigh<'store, Item<'a, 'store>>,
    entries: LowToHigh<'store, Entry<'a, 'store>>,
}
impl<'a, 'store> Builder<'a, 'store> for Arena<'a, 'store> {
    fn list(&self, count: usize) -> Option<List<'a, 'store>> {
        Some(List::wrap(self.items.finish(count)?))
    }
    fn dict(&self, count: usize) -> Option<Dict<'a, 'store>> {
        Some(Dict::wrap(self.entries.finish(count)?))
    }
    fn item(&self, item: Item<'a, 'store>) -> Option<()> {
        self.items.push(item)
    }
    fn entry(&self, entry: Entry<'a, 'store>) -> Option<()> {
        self.entries.push(entry)
    }
}
impl<'a, 'store> Arena<'a, 'store> {
    pub fn wrap(
        items: &'store [Cell<Item<'a, 'store>>],
        entries: &'store [Cell<Entry<'a, 'store>>],
    ) -> Self {
        let items = LowToHigh::wrap(items);
        let entries = LowToHigh::wrap(entries);
        Arena { items, entries }
    }
    pub fn item_slots(&self) -> usize {
        self.items.done.get() - self.items.next.get()
    }
    pub fn entry_slots(&self) -> usize {
        self.entries.done.get() - self.entries.next.get()
    }
    pub fn completed(&self) -> Option<()> {
        if self.items.done.get() == 0 && self.entries.done.get() == 0 {
            Some(())
        } else {
            None
        }
    }
    pub fn keyed(&self, key: &'a str, item: Item<'a, 'store>) -> Option<()> {
        self.entry(Entry::wrap(key, item))
    }
    pub fn text_item(&self, utf8: &'a str) -> Option<()> {
        self.item(Text::wrap(utf8).into())
    }
    pub fn list_item(&self, count: usize) -> Option<()> {
        let list = self.list(count)?;
        self.item(list.into())
    }
    pub fn dict_item(&self, count: usize) -> Option<()> {
        let dict = self.dict(count)?;
        self.item(dict.into())
    }
    pub fn text_entry(&self, key: &'a str, utf8: &'a str) -> Option<()> {
        self.keyed(key, Text::wrap(utf8).into())
    }
    pub fn list_entry(&self, key: &'a str, count: usize) -> Option<()> {
        let list = self.list(count)?;
        self.keyed(key, list.into())
    }
    pub fn dict_entry(&self, key: &'a str, count: usize) -> Option<()> {
        let dict = self.dict(count)?;
        self.keyed(key, dict.into())
    }
    pub fn parse_or_panic(&self, content: &'a str) -> Option<File<'a, 'store>> {
        parse::Input::parse(self, content, |error| panic!("{error}"))
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
pub struct Error<'p> {
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
    pub fn error_full(&self, message: &'static str) -> Error<'p> {
        Error {
            failed: &self.branches[..],
            message,
        }
    }
    pub fn error_at(&self, bad: usize, message: &'static str) -> Error<'p> {
        Error {
            failed: &self.branches[..=bad],
            message,
        }
    }
    pub fn text<'a, 'store>(&self, item: &Item<'a, 'store>) -> Result<Text<'a>, Error<'p>> {
        let Item::Text(text) = item else {
            return Err(self.error_full("path does not end at Text"));
        };
        Ok(*text)
    }
    pub fn list<'a, 'store>(&self, item: &Item<'a, 'store>) -> Result<List<'a, 'store>, Error<'p>> {
        let Item::List(list) = item else {
            return Err(self.error_full("path does not end at List"));
        };
        Ok(*list)
    }
    pub fn dict<'a, 'store>(&self, item: &Item<'a, 'store>) -> Result<Dict<'a, 'store>, Error<'p>> {
        let Item::Dict(dict) = item else {
            return Err(self.error_full("path does not end at Dict"));
        };
        Ok(*dict)
    }
    pub fn item_cell<'a, 'store>(
        &self,
        item: &Item<'a, 'store>,
    ) -> Result<&'a Cell<Item<'a, 'store>>, Error<'p>> {
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
    pub fn entry_cell<'a, 'store>(
        &self,
        item: &Item<'a, 'store>,
    ) -> Result<&'a Cell<Entry<'a, 'store>>, Error<'p>> {
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
