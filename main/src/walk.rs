//! supporting code for the walk! macro.
//! everything here must be pub so macro can use it,
//! but using these directly is not recommended.
//! using walk! is much easier.

use crate::{Dict, Entry, Item, List, Text};
use core::cell::Cell;

/// a decision along a walk.
#[derive(Debug)]
pub enum Branch<'p> {
    /// select list item by index
    List(usize),
    /// select dict entry by key
    Dict(&'p str),
}
/// information about where a walk went wrong.
#[derive(Debug)]
pub struct PathError<'p> {
    /// zero or more good steps and one last bad step
    pub failed: &'p [Branch<'p>],
    /// English description of the problem
    pub message: &'static str,
}
/// a sequence of Branch built and used by the walk macro.
#[derive(Debug)]
pub struct Path<'p> {
    /// all the decisions for a walk
    pub branches: &'p [Branch<'p>],
}
impl<'p> Path<'p> {
    /// construct a path
    pub fn wrap(branches: &'p [Branch<'p>]) -> Self {
        Path { branches }
    }
    /// construct an error indicating the last path step failed
    pub fn error_full(&self, message: &'static str) -> PathError<'p> {
        PathError {
            failed: &self.branches[..],
            message,
        }
    }
    /// construct an error indicating the given path step failed
    pub fn error_at(&self, bad: usize, message: &'static str) -> PathError<'p> {
        PathError {
            failed: &self.branches[..=bad],
            message,
        }
    }
    /// return a copy of the Text item (or Err if it isn't a text).
    pub fn text<'a>(&self, item: &Item<'a>) -> Result<Text<'a>, PathError<'p>> {
        let Item::Text(text) = item else {
            return Err(self.error_full("path does not end at Text"));
        };
        Ok(*text)
    }
    /// return a copy of the List item (or Err if it isn't a list).
    pub fn list<'a>(&self, item: &Item<'a>) -> Result<List<'a>, PathError<'p>> {
        let Item::List(list) = item else {
            return Err(self.error_full("path does not end at List"));
        };
        Ok(*list)
    }
    /// return a copy of the Dict item (or Err if it isn't a dict).
    pub fn dict<'a>(&self, item: &Item<'a>) -> Result<Dict<'a>, PathError<'p>> {
        let Item::Dict(dict) = item else {
            return Err(self.error_full("path does not end at Dict"));
        };
        Ok(*dict)
    }
    /// walk down a path that starts at a list
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
    /// walk down a path that starts at a dict
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
