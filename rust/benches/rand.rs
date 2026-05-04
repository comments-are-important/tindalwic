//! generate random files, run them through the library algorithms, and collect timing.
//! the randomness is not attempting to produce data that mimics the real world in any
//! way. other benchmarks do that. instead, the ratios here are chosen to even out the
//! library profiling line hit counts that would happen during this test: all the
//! branches coming from each decision point in the algorithms should be taken roughly
//! the same number of times.

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

/// a very blurry outline of some data. created first to be able to call the
/// Arena/Builder API in the order it requires.
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

/// generate random files containing the requested number of items.
pub struct Random<'a, 'store: 'a, 'r, R: Rng + ?Sized> {
    bump: &'store Bump,
    arena: &'r mut Arena<'a, 'store>,
    rng: &'r mut R,
}
impl<'a, 'store, 'r, R: Rng + ?Sized> Random<'a, 'store, 'r, R> {
    fn utf8(&mut self, newline: bool) -> &'a str {
        let mut utf8 = String::new();
        let lines = if !newline {
            1
        } else {
            1//self.rng.random_range(1..=4)
        };
        for line in 0..lines {
            if line != 0 {
                utf8.push('\n');
            }
            let mut len: usize = self.rng.random_range(..5);
            while len > 0 {
                let c: char = self.rng.random();
                if c != '\n' {
                    utf8.push(c);
                    len -= 1;
                }
            }
        }
        self.bump.alloc_str(&utf8)
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
    let seed: u64 = 0;//rand::rng().random();
    println!("seed={seed}");
    let mut rng = SmallRng::seed_from_u64(seed);
    c.bench_function("round-trip", |b| {
        b.iter(|| {
            let bump = Bump::new();
            let mut arena = Arena::new(&bump);
            let mut random = Random {
                bump: &bump,
                arena: &mut arena,
                rng: &mut rng,
            };
            let original: File = random.file(2);
            let original_string = original.to_string();
            let parsed = arena.parse_or_panic(&original_string).unwrap();
            assert_eq!(
                original, parsed,
                "failed round-trip:\n===\n{original}\n===\n{parsed}\n==="
            );
        })
    });
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
