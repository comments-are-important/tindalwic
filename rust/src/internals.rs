#![doc(hidden)] // only public so macro generated code can access.

use super::*;

struct Bump<'a, T> {
    cells: &'a [Cell<T>],
    next: usize,
    done: usize,
}
impl<'a, T> Bump<'a, T> {
    fn wrap(cells: &'a [Cell<T>]) -> Self {
        Bump {
            cells,
            next: 0,
            done: cells.len(),
        }
    }
    fn push(&mut self, value: T) -> Option<()> {
        if self.next >= self.done {
            return None;
        }
        let next = self.next;
        self.cells[next].set(value);
        self.next = next + 1;
        Some(())
    }
    fn finish(&mut self, count: usize) -> Option<&'a [Cell<T>]> {
        if self.next < count || self.next > self.done {
            return None;
        }
        if self.next == self.done {
            let both = self.next - count;
            self.next = both;
            self.done = both;
            return Some(&self.cells[both..both + count]);
        }
        let next = self.next - count;
        let done = self.done - count;
        for offset in (0..count).rev() {
            self.cells[next + offset].swap(&self.cells[done + offset]);
        }
        self.next = next;
        self.done = done;
        Some(&self.cells[done..done + count])
    }
}

pub struct Arena<'a> {
    items: Bump<'a, Item<'a>>,
    entries: Bump<'a, Entry<'a>>,
}
impl<'a> Arena<'a> {
    pub fn wrap(items: &'a [Cell<Item<'a>>], entries: &'a [Cell<Entry<'a>>]) -> Self {
        let items = Bump::wrap(items);
        let entries = Bump::wrap(entries);
        Arena { items, entries }
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

#[derive(Debug)]
pub enum Branch<'a> {
    List(usize),
    Dict(Key<'a>),
}
#[derive(Debug)]
pub struct Error<'a> {
    pub failed: &'a [Branch<'a>],
    pub message: &'static str,
}
#[derive(Debug)]
pub struct Path<'a> {
    pub branches: &'a [Branch<'a>],
}

impl<'a> Path<'a> {
    pub fn wrap(branches: &'a [Branch<'a>]) -> Self {
        Path { branches }
    }
    pub fn error_full(&'a self, message: &'static str) -> Error<'a> {
        Error {
            failed: &self.branches[..],
            message,
        }
    }
    pub fn error_at(&'a self, bad: usize, message: &'static str) -> Error<'a> {
        Error {
            failed: &self.branches[..=bad],
            message,
        }
    }
    pub fn text(&'a self, item: &'a Item<'a>) -> Result<Text<'a>, Error<'a>> {
        let Item::Text(text) = item else {
            return Err(self.error_full("path does not end at Text"));
        };
        Ok(*text)
    }
    pub fn list(&'a self, item: &'a Item<'a>) -> Result<List<'a>, Error<'a>> {
        let Item::List(list) = item else {
            return Err(self.error_full("path does not end at List"));
        };
        Ok(*list)
    }
    pub fn dict(&'a self, item: &'a Item<'a>) -> Result<Dict<'a>, Error<'a>> {
        let Item::Dict(dict) = item else {
            return Err(self.error_full("path does not end at Dict"));
        };
        Ok(*dict)
    }
    pub fn item_cell(&'a self, mut from: Item<'a>) -> Result<&'a Cell<Item<'a>>, Error<'a>> {
        if self.branches.is_empty() {
            return Err(self.error_full("empty path can't be resolved"));
        }
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
                        match dict.find(key) {
                            None => return Err(self.error_at(step, "key not found")),
                            Some(found) => {
                                from = found.get().item;
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
    pub fn entry_cell(&'a self, mut from: Item<'a>) -> Result<&'a Cell<Entry<'a>>, Error<'a>> {
        if self.branches.is_empty() {
            return Err(self.error_full("empty path can't be resolved"));
        }
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
                        match dict.find(key) {
                            None => return Err(self.error_at(step, "key not found")),
                            Some(found) => {
                                if step + 1 == self.branches.len() {
                                    return Ok(found);
                                }
                                from = found.get().item;
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
