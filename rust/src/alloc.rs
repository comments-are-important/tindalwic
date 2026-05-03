//! all this stuff is enabled by the "alloc" feature.

extern crate alloc;

use super::{internals::Builder, *};
use alloc::string::String;
use alloc::vec::Vec;
use bumpalo::Bump;

/// a flavor of Arena that uses bumpalo to put things in the heap.
/// TODO think about fleshing this out with more convenient methods.
pub struct Arena<'a, 'bump> {
    items: Vec<Item<'a, 'bump>>,
    entries: Vec<Entry<'a, 'bump>>,
    /// expose bump assuming our owner knows what they're doing
    pub bump: &'bump Bump,
}
impl<'a, 'bump> Builder<'a, 'bump> for Arena<'a, 'bump> {
    fn list(&mut self, count: usize) -> Option<List<'a, 'bump>> {
        let start = self.items.len().checked_sub(count)?;
        let cells = self
            .bump
            .alloc_slice_fill_with(count, |i| Cell::new(self.items[start + i]));
        self.items.truncate(start);
        Some(List::wrap(cells))
    }

    fn dict(&mut self, count: usize) -> Option<Dict<'a, 'bump>> {
        let start = self.entries.len().checked_sub(count)?;
        let cells = self.bump.alloc_slice_fill_with(count, |i| {
            Cell::new(self.entries[self.entries.len() - count + i])
        });
        self.entries.truncate(start);
        Some(Dict::wrap(cells))
    }

    fn item(&mut self, item: Item<'a, 'bump>) -> Option<()> {
        self.items.push(item);
        Some(())
    }

    fn entry(&mut self, entry: Entry<'a, 'bump>) -> Option<()> {
        self.entries.push(entry);
        Some(())
    }
}
impl<'a, 'bump> Arena<'a, 'bump> {
    /// the Bump needs an outside let binding so it lives long enough.
    pub fn new(bump: &'bump Bump) -> Self {
        Arena {
            items: Vec::new(),
            entries: Vec::new(),
            bump,
        }
    }
    /// call the parser on the provided content, panic if the content isn't legit.
    pub fn parse_or_panic(&mut self, content: &'a str) -> Option<File<'a, 'bump>> {
        parse::Input::parse(self, content, |error| panic!("{error}"))
    }
}

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

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::string::ToString;
    #[test]
    fn parse_alloc() {
        let bump = Bump::new();
        let mut arena = Arena::new(&bump);
        let file = arena.parse_or_panic("k=v\n").unwrap();
        assert_eq!(file.to_string(), "k=v\n");
    }
}
