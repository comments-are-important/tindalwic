#![allow(missing_docs)]
#![warn(unused)]

use bumpalo::Bump;
use criterion::{Criterion, criterion_group, criterion_main};
use rand::rngs::SmallRng;
use rand::{Rng, RngExt, SeedableRng};
use std::fmt::{self, Write};
use tindalwic::alloc::Arena;
use tindalwic::internals::Builder as _;
use tindalwic::{Comment, Dict, Entry, File, Item, List, Name, Text};

#[derive(Debug)]
struct Silhouette {
    branches: usize, // recursive count excluding leafs but including self
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
    fn random<R: Rng + ?Sized>(size: usize, rng: &mut R) -> Self {
        let mut root = Silhouette::new();
        for _ in 1..size {
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

/// generate data
pub struct Random<'a, 'store, 'r, R: Rng + ?Sized> {
    pub utf8: &'a str, // TODO use a random String instead of asking caller
    pub arena: &'r mut Arena<'a, 'store>,
    pub rng: &'r mut R,
}
impl<'a, 'store, 'r, R: Rng + ?Sized> Random<'a, 'store, 'r, R> {
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
    fn item(&mut self, shape: &Option<Silhouette>) -> Option<Item<'a, 'store>> {
        Some(if let Some(parent) = shape {
            if self.rng.random_ratio(1, 2) {
                Item::Dict(self.dict(parent)?)
            } else {
                Item::List(self.list(parent)?)
            }
        } else {
            Item::Text(Text::wrap(self.utf8(true)))
        })
    }
    fn list(&mut self, shape: &Silhouette) -> Option<List<'a, 'store>> {
        for kid in &shape.children {
            let item = self.item(kid)?;
            self.arena.item(item)?;
        }
        let mut list = self.arena.list(shape.children.len())?;
        list.prolog = self.comment();
        list.epilog = self.comment();
        Some(list)
    }
    fn dict(&mut self, shape: &Silhouette) -> Option<Dict<'a, 'store>> {
        for kid in &shape.children {
            let item = self.item(kid)?;
            let gap = self.rng.random_bool(0.2);
            let before = self.comment();
            let key = self.utf8(false);
            self.arena.entry(Entry {
                name: Name { gap, before, key },
                item,
            })?;
        }
        let mut dict = self.arena.dict(shape.children.len())?;
        dict.prolog = self.comment();
        dict.epilog = self.comment();
        Some(dict)
    }
    pub fn file(&mut self, size: usize) -> File<'a, 'store> {
        let shape = Silhouette::random(size, self.rng);
        let dict = self.dict(&shape).unwrap();
        File {
            cells: dict.cells,
            hashbang: dict.epilog,
            prolog: dict.prolog,
        }
    }
}

fn criterion_benchmark(c: &mut Criterion) {
    c.bench_function("round-trip", |b| {
        b.iter(|| {
            let bump = Bump::new();
            let mut arena = Arena::new(&bump);
            let seed: u64 = rand::rng().random();
            let mut rng = SmallRng::seed_from_u64(seed);
            let mut random = Random {
                utf8: "abcdefghijklmnopqrstuvwxyz",
                arena: &mut arena,
                rng: &mut rng,
            };
            let original: File = random.file(20);
            let original_string = original.to_string();
            let parsed = arena.parse_or_panic(&original_string).unwrap();
            assert_eq!(
                original, parsed,
                "failed round-trip:\n===\n{original}\n===\n{parsed}"
            );
        })
    });
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
