#![doc(hidden)] // only public so macro generated code can access.

use super::*;

struct Bump<'store, T> {
    cells: &'store [Cell<T>],
    next: usize,
    done: usize,
}
impl<'store, T> Bump<'store, T> {
    fn wrap(cells: &'store [Cell<T>]) -> Self {
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
    fn finish(&mut self, count: usize) -> Option<&'store [Cell<T>]> {
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

pub struct Arena<'a, 'store> {
    items: Bump<'store, Item<'a, 'store>>,
    entries: Bump<'store, Entry<'a, 'store>>,
}
impl<'a, 'store> Arena<'a, 'store> {
    pub fn wrap(
        items: &'store [Cell<Item<'a, 'store>>],
        entries: &'store [Cell<Entry<'a, 'store>>],
    ) -> Self {
        let items = Bump::wrap(items);
        let entries = Bump::wrap(entries);
        Arena { items, entries }
    }
    pub fn item_slots(&self) -> usize {
        self.items.done - self.items.next
    }
    pub fn entry_slots(&self) -> usize {
        self.entries.done - self.entries.next
    }
    pub fn completed(&self) -> Option<()> {
        if self.items.done == 0 && self.entries.done == 0 {
            Some(())
        } else {
            None
        }
    }
    pub fn list(&mut self, count: usize) -> Option<List<'a, 'store>> {
        Some(List::wrap(self.items.finish(count)?))
    }
    pub fn dict(&mut self, count: usize) -> Option<Dict<'a, 'store>> {
        Some(Dict::wrap(self.entries.finish(count)?))
    }
    pub fn item(&mut self, item: Item<'a, 'store>) -> Option<()> {
        self.items.push(item)
    }
    pub fn keyed(&mut self, key: &'a str, item: Item<'a, 'store>) -> Option<()> {
        self.entry(Entry::wrap(key, item))
    }
    pub fn entry(&mut self, entry: Entry<'a, 'store>) -> Option<()> {
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
    pub fn parse_or_panic(&mut self, content: &'a str) -> Option<File<'a, 'store>> {
        parse::Input::parse(self, content, |error| panic!("{error}"))
    }
}

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
