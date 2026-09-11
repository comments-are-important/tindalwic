//! generate random files, run them through the library algorithms.
//! the randomness is not attempting to produce data that mimics the real world in any
//! way. other benchmarks do that. instead, the ratios here are chosen to even out the
//! library profiling line hit counts that would happen during this test: all the
//! branches coming from each decision point in the algorithms should be taken roughly
//! the same number of times.

use bumpalo::Bump;
use rand::prelude::IndexedRandom;
use rand::{Rng, RngExt};
use std::fmt::{self, Write};
use tindalwic::bumpalo::Arena;
use tindalwic::parse::Parse as _;
use tindalwic::{Comment, Entry, File, Item, Value};

/// a very blurry outline of some data. created first to be able to call the
/// Arena API in the order it requires.
#[derive(Debug)]
struct Silhouette {
    branches: usize, // recursive count excluding leaf nodes but including self
    children: Vec<Option<Silhouette>>, // None indicates position of a leaf
}
impl Silhouette {
    fn new() -> Self {
        Silhouette {
            branches: 1,
            children: Vec::new(),
        }
    }
    fn grow(&mut self, mut at: usize, leaf: bool) {
        // the `at` corresponds to index within post-order traversal (ignore leaf nodes)
        if at >= self.branches {
            panic!("can't grow at {at} - no such branch exists");
        }
        for kid in &mut self.children {
            let Some(kid) = kid else { continue };
            if at < kid.branches {
                kid.grow(at, leaf);
                if !leaf {
                    self.branches += 1;
                }
                return;
            }
            at -= kid.branches;
        }
        if at != 0 {
            panic!("bad math somewhere");
        }
        if leaf {
            self.children.push(None);
        } else {
            self.children.push(Some(Silhouette::new()));
            self.branches += 1;
        }
    }
    fn random<R: Rng + ?Sized>(grow: usize, rng: &mut R) -> Self {
        let mut root = Silhouette::new();
        for _ in 0..grow {
            let at = rng.random_range(..root.branches);
            let leaf = rng.random_ratio(1, 3);
            root.grow(at, leaf);
            // println!("grow({at},{leaf}) -> {root}");
        }
        root
    }
}
impl fmt::Display for Silhouette {
    fn fmt(&self, out: &mut fmt::Formatter<'_>) -> fmt::Result {
        out.write_char('(')?;
        usize::fmt(&self.branches, out)?;
        for kid in &self.children {
            if let Some(kid) = kid {
                Silhouette::fmt(kid, out)?;
            } else {
                out.write_char('.')?;
            }
        }
        out.write_char(')')?;
        Ok(())
    }
}

/// generate random files containing the requested number of items.
pub struct Random<'a, 'r, R: Rng + ?Sized> {
    bump: &'a Bump,
    arena: &'r mut Arena<'a>,
    rng: &'r mut R,
    sample: Vec<char>,
}
impl<'a, 'r, R: Rng + ?Sized> Random<'a, 'r, R> {
    pub fn new(
        bump: &'a Bump,
        arena: &'r mut Arena<'a>,
        rng: &'r mut R,
        sample: &'static str,
    ) -> Result<Self, &'static str> {
        let sample: Vec<char> = sample.chars().collect();
        if sample.contains(&'\n') {
            return Err("can't have LF char in sample");
        }
        Ok(Random {
            bump,
            arena,
            rng,
            sample,
        })
    }
    fn not_linefeed(&mut self) -> char {
        if !self.sample.is_empty() {
            if self.rng.random_bool(0.1) {
                return ' ';
            }
            // constructor already verified no linefeed...
            return *self.sample.choose(&mut self.rng).unwrap();
        }
        let mut result: char = self.rng.random();
        while result == '\n' {
            result = self.rng.random();
        }
        result
    }
    fn value(&mut self) -> &'a str {
        let mut value = String::new();
        if self.rng.random_bool(0.92) {
            for _ in 0..self.rng.random_range(20..85) {
                value.push(self.not_linefeed());
            }
            if self.rng.random_bool(0.3) {
                for _ in 0..self.rng.random_range(1..=4) {
                    value.push('\n');
                    for _ in 0..self.rng.random_range(20..85) {
                        value.push(self.not_linefeed());
                    }
                }
            }
        }
        self.bump.alloc_str(&value)
    }
    fn comment(&mut self) -> Option<Comment<'a>> {
        if self.rng.random_bool(0.5) {
            Some(Comment {
                value: self.value().into(),
            })
        } else {
            None
        }
    }
    fn item(&mut self, shape: &Option<Silhouette>) -> Result<Item<'a>, &'static str> {
        Ok(if let Some(parent) = shape {
            if self.rng.random_ratio(1, 2) {
                let count = self.entries(&parent.children)?;
                self.dict(count)?
            } else {
                let count = self.items(&parent.children)?;
                self.list(count)?
            }
        } else {
            Item::text(self.value())
        })
    }
    fn items(&mut self, kids: &[Option<Silhouette>]) -> Result<usize, &'static str> {
        for kid in kids {
            let item = self.item(kid)?;
            self.arena.builder().push_item(item)?;
        }
        Ok(kids.len())
    }
    fn list(&mut self, count: usize) -> Result<Item<'a>, &'static str> {
        Ok(Item::List {
            prolog: self.comment(),
            cells: self.arena.builder().finish_items(count)?,
            epilog: self.comment(),
        })
    }
    fn entries(&mut self, kids: &[Option<Silhouette>]) -> Result<usize, &'static str> {
        for kid in kids {
            let before = self.comment();
            let key = self.value().into();
            let item = self.item(kid)?;
            self.arena.builder().push_entry(Entry {
                gap: self.rng.random_bool(0.2),
                before,
                key,
                item,
            })?;
        }
        Ok(kids.len())
    }
    fn dict(&mut self, count: usize) -> Result<Item<'a>, &'static str> {
        Ok(Item::Dict {
            prolog: self.comment(),
            cells: self.arena.builder().finish_entries(count)?,
            epilog: self.comment(),
        })
    }
    pub fn file(&mut self, grow: usize) -> Result<File<'a>, &'static str> {
        let hashbang = self.comment();
        let shape = Silhouette::random(grow, self.rng);
        let count = self.entries(&shape.children)?;
        Ok(File {
            hashbang,
            prolog: self.comment(),
            cells: self.arena.builder().finish_entries(count)?,
        })
    }
    pub fn embedded(
        &mut self,
        key: Value<'a>,
        grow: usize,
        meta: Entry<'a>,
    ) -> Result<File<'a>, &'static str> {
        self.arena.builder().push_entry(meta)?;
        let hashbang = self.comment();
        let shape = Silhouette::random(grow, self.rng);
        let count = self.entries(&shape.children)?;
        let item = self.dict(count)?;
        self.arena.builder().push_entry(Entry {
            key,
            item,
            ..Default::default()
        })?;
        Ok(File {
            hashbang,
            prolog: None,
            cells: self.arena.builder().finish_entries(2)?,
        })
    }
}
