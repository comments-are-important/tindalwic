#![allow(missing_docs)]

use bumpalo::Bump;
use rand::prelude::IndexedRandom;
use rand::rngs::SmallRng;
use rand::{Rng, RngExt, SeedableRng as _};
use std::fmt::{self, Write};
use std::io::Write as _;
use tindalwic::bumpalo::Arena;
use tindalwic::parse::Parse as _;
use tindalwic::{Comment, Entry, File, Item, Name, VERSION};

#[derive(clap::Args, Debug)]
pub struct Args {
    /// how many items?
    #[arg(default_value_t = 10)]
    items: usize,
    /// use full range of chars instead of just lower-case ascii
    #[arg(long)]
    unicode: bool,
    /// specify the random seed
    #[arg(long, value_parser = |s:&str|u64::from_str_radix(s, 16))]
    seed: Option<u64>,
}
impl Args {
    pub fn run(&self) -> anyhow::Result<()> {
        let bump = Bump::new();
        let mut arena = Arena::new(&bump);
        let file = self.file(&mut arena)?;
        std::io::stdout().write(file.to_string().as_bytes())?;
        Ok(())
    }
    pub fn file<'a>(&self, arena: &mut Arena<'a>) -> anyhow::Result<File<'a>> {
        let mut hashbang = String::new();
        for arg in std::env::args() {
            if hashbang.is_empty() {
                hashbang.push_str("/usr/bin/env -S tindalwic");
            } else {
                hashbang.push(' ');
                hashbang.push_str(&arg);
            }
        }
        let seed = match self.seed {
            Some(value) => value,
            None => rand::make_rng::<SmallRng>().random(),
        };
        hashbang.push_str(" --seed=");
        write!(hashbang, "{:X}", seed)?;
        hashbang.push_str("\nat ");
        hashbang.push_str(&super::now());
        hashbang.push_str(" by version ");
        hashbang.push_str(VERSION);
        let mut rng = SmallRng::seed_from_u64(seed);
        let sample = if self.unicode {
            ""
        } else {
            "abcdefghijklmnopqrstuvwxyz"
        };
        let mut file = Random::new(arena, &mut rng, sample)?.file(self.items)?;
        file.hashbang = Comment::some(arena.intern(&hashbang));
        Ok(file)
    }
}

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
    arena: &'r mut Arena<'a>,
    rng: &'r mut R,
    sample: Vec<char>,
}
impl<'a, 'r, R: Rng + ?Sized> Random<'a, 'r, R> {
    pub fn new(
        arena: &'r mut Arena<'a>,
        rng: &'r mut R,
        sample: &'static str,
    ) -> anyhow::Result<Self> {
        let sample: Vec<char> = sample.chars().collect();
        if sample.contains(&'\n') {
            anyhow::bail!("can't have LF char in sample");
        }
        Ok(Random { arena, rng, sample })
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
        self.arena.intern(&value)
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
    fn item(&mut self, shape: &Option<Silhouette>) -> anyhow::Result<Item<'a>> {
        Ok(if let Some(parent) = shape {
            if self.rng.random_ratio(1, 2) {
                let count = self.entries(&parent.children)?;
                self.dict(count)?
            } else {
                let count = self.items(&parent.children)?;
                self.list(count)?
            }
        } else {
            Item::text(self.value().into())
        })
    }
    fn items(&mut self, kids: &[Option<Silhouette>]) -> anyhow::Result<usize> {
        for kid in kids {
            let item = self.item(kid)?;
            self.arena
                .builder()
                .push_item(item)
                .map_err(anyhow::Error::msg)?;
        }
        Ok(kids.len())
    }
    fn list(&mut self, count: usize) -> anyhow::Result<Item<'a>> {
        Ok(Item::List {
            prolog: self.comment(),
            cells: self
                .arena
                .builder()
                .finish_items(count)
                .map_err(anyhow::Error::msg)?,
            epilog: self.comment(),
        })
    }
    fn entries(&mut self, kids: &[Option<Silhouette>]) -> anyhow::Result<usize> {
        for kid in kids {
            let key = Name {
                gap: self.rng.random_bool(0.2),
                comment: self.comment(),
                key: self.value().into(),
            };
            let item = self.item(kid)?;
            self.arena
                .builder()
                .push_entry(Entry { name: key, item })
                .map_err(anyhow::Error::msg)?;
        }
        Ok(kids.len())
    }
    fn dict(&mut self, count: usize) -> anyhow::Result<Item<'a>> {
        Ok(Item::Dict {
            prolog: self.comment(),
            cells: self
                .arena
                .builder()
                .finish_entries(count)
                .map_err(anyhow::Error::msg)?,
            epilog: self.comment(),
        })
    }
    pub fn file(&mut self, grow: usize) -> anyhow::Result<File<'a>> {
        let shape = Silhouette::random(grow, self.rng);
        let count = self.entries(&shape.children)?;
        Ok(File {
            hashbang: self.comment(),
            prolog: self.comment(),
            cells: self
                .arena
                .builder()
                .finish_entries(count)
                .map_err(anyhow::Error::msg)?,
        })
    }
}
