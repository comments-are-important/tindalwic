//extern crate alloc;
#![allow(unused)]

use super::internals::Arena;
use super::{Comment, Dict, Entry, File, Item, List, Name, Text};
use rand::distr::uniform::{UniformSampler, UniformUsize};
use rand::{Rng, RngExt};

/// generate data
pub struct Random<'a, 'r, R: Rng + ?Sized> {
    pub utf8: &'a str,
    pub arena: &'r mut Arena<'a>,
    pub rng: &'r mut R,
    pub width: UniformUsize,
    pub deepest: usize,
}
impl<'a, 'r, R: Rng + ?Sized> Random<'a, 'r, R> {
    fn utf8(&mut self, newline: bool) -> &'a str {
        let one: usize = self.rng.random_range(..=self.utf8.len());
        let two: usize = self.rng.random_range(..=self.utf8.len());
        let mut slice = &self.utf8[one.min(two)..one.max(two)];
        if !newline && let Some(index) = slice.find('\n') {
            slice = &slice[..index]
        }
        slice
    }
    fn comment(&mut self) -> Option<Comment<'a>> {
        if self.rng.random_bool(0.5) {
            Comment::some(self.utf8(true))
        } else {
            None
        }
    }
    fn item(&mut self, depth: usize) -> Option<Item<'a>> {
        match if depth >= self.deepest {
            0
        } else {
            self.rng.random_range(0..3)
        } {
            0 => Some(Item::Text(Text::wrap(self.utf8(true)))),
            1 => Some(Item::List(self.list(depth)?)),
            2 => Some(Item::Dict(self.dict(depth)?)),
            _ => unreachable!(),
        }
    }
    fn width(&mut self, limit: usize) -> usize {
        self.width.sample(self.rng).min(limit)
    }
    fn list(&mut self, depth: usize) -> Option<List<'a>> {
        let mut count = 0;
        for _ in 0..self.width.sample(self.rng) {
            let item = self.item(depth + 1)?;
            if self.arena.item_slots() == 0 {
                break;
            }
            self.arena.item(item)?;
            count += 1;
        }
        let mut list = self.arena.list(count)?;
        list.prolog = self.comment();
        list.epilog = self.comment();
        Some(list)
    }
    fn dict(&mut self, depth: usize) -> Option<Dict<'a>> {
        let mut count = 0;
        for _ in 0..self.width.sample(self.rng) {
            let item = self.item(depth + 1)?;
            if self.arena.entry_slots() == 0 {
                break;
            }
            let gap = self.rng.random_bool(0.2);
            let before = self.comment();
            let key = self.utf8(false);
            self.arena.entry(Entry {
                name: Name { gap, before, key },
                item,
            })?;
            count += 1;
        }
        let mut dict = self.arena.dict(count)?;
        dict.prolog = self.comment();
        dict.epilog = self.comment();
        Some(dict)
    }
    /// should never panic, assuming impl Random has no bugs
    pub fn file(&mut self) -> File<'a> {
        // code above respects the Arena *_slots so the unwrap should not panic
        match self.dict(0) {
            None => unreachable!(),
            Some(dict) => File {
                cells: dict.cells,
                hashbang: dict.epilog,
                prolog: dict.prolog,
            },
        }
    }
}

#[cfg(test)]
mod tests {
    extern crate alloc;
    extern crate std;
    use super::Random;
    use crate::parse::Input;
    use crate::{File, arena};
    use alloc::string::ToString;
    use rand::distr::uniform::{UniformSampler, UniformUsize};
    use rand::rngs::SmallRng;
    use rand::{RngExt, SeedableRng};
    use std::eprintln;

    fn bail(args: &(usize, &str)) {
        let (line, message) = args;
        panic!("{line}: {message}");
    }

    #[test]
    fn zzz() {for _ in 0..800000{
        arena! {
            $crate = crate;
            let mut original_arena = <50list,50dict>;
        }
        let seed: u64 = rand::rng().random();
        eprintln!("seed = {}", seed);
        let mut rng = SmallRng::seed_from_u64(seed);
        let mut random = Random {
            utf8: "abcdefghijklmnopqrstuvwxyz",
            arena: &mut original_arena,
            rng: &mut rng,
            width: UniformUsize::new(0, 6).unwrap(),
            deepest: 5,
        };
        let original: File = random.file();
        arena! {
            $crate = crate;
            let mut parsed_arena = <50list,50dict>;
        }
        let original_string = original.to_string();
        let parsed = Input::parse(&mut parsed_arena, &original_string, bail).unwrap();
        if original != parsed {
            eprintln!("======= original\n{original}\n======= parsed\n{parsed}");
        }
        assert_eq!(original, parsed);
    }}
}
