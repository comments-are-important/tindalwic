#![allow(missing_docs)]
use bumpalo::Bump;
use rand::prelude::IndexedRandom;
use rand::rngs::SmallRng;
use rand::{Rng, RngExt, SeedableRng as _};
use std::fmt::{self, Write};
use std::fs;
use std::io::Write as _;
use tindalwic::bumpalo::Arena;
use tindalwic::parse::Parse as _;
use tindalwic::text::Lines;
use tindalwic::{Data, File, Group, List, Member, VERSION};

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
    /// repeat: write, parse, error if different
    #[cfg(debug_assertions)]
    #[arg(long)]
    check: Option<usize>,
}
impl Args {
    const fn sample(unicode: bool) -> &'static str {
        if unicode {
            ""
        } else {
            "abcdefghijklmnopqrstuvwxyz"
        }
    }
    pub fn run(&self) -> anyhow::Result<()> {
        let seed = match self.seed {
            Some(value) => value,
            None => rand::make_rng::<SmallRng>().random(),
        };
        #[cfg(debug_assertions)]
        if let Some(check) = self.check {
            let mut rng = SmallRng::seed_from_u64(seed);
            for count in 0..check {
                let bump = Bump::new();
                let mut arena = Arena::new(&bump);
                let sample = Args::sample(self.unicode);
                let mut random = Random::new(&mut arena, &mut rng, sample)?;
                let original = random.file(self.items)?;
                let expected = original.to_string();
                let (good, parsed, errors) = match arena.format_errors("", &expected, usize::MAX) {
                    Ok(parsed) => (true, parsed, "".into()),
                    Err(errors) => (false, File::default(), errors),
                };
                let actually = parsed.to_string();
                if good && /*original == parsed &&*/ expected == actually {
                    continue;
                }
                if count != 0 {
                    println!("after {count} successes");
                }
                if !fs::exists("target/debug/tindalwic-cli")? {
                    eprintln!("can't write to ./target/chk.*");
                } else {
                    fs::write("target/chk.original.debug_api", format!("{original:#?}"))?;
                    fs::write("target/chk.original.tindalwic", &expected)?;
                    fs::write("target/chk.parse.errors", &errors)?;
                    fs::write("target/chk.parsed.debug_api", format!("{parsed:#?}"))?;
                    fs::write("target/chk.parsed.tindalwic", &actually)?;
                    eprintln!("wrote to ./target/chk.*");
                }
                anyhow::bail! {
                    if !good { "parse error"}
                    else if original != parsed {"API difference"}
                    else if expected != actually { "recoding difference"}
                    else {"unknown difference"}
                }
            }
            return Ok(());
        }
        let bump = Bump::new();
        let mut arena = Arena::new(&bump);
        let file = self.file(seed, &mut arena)?;
        std::io::stdout().write(file.to_string().as_bytes())?;
        Ok(())
    }
    fn file<'a>(&self, seed: u64, arena: &mut Arena<'a>) -> anyhow::Result<File<'a>> {
        let mut shebang = String::new();
        for arg in std::env::args() {
            if shebang.is_empty() {
                shebang.push_str("/usr/bin/env -S tindalwic");
            } else {
                shebang.push(' ');
                shebang.push_str(&arg);
            }
        }
        shebang.push_str(" --seed=");
        write!(shebang, "{:X}", seed)?;
        shebang.push_str("\nat ");
        shebang.push_str(&super::now());
        shebang.push_str(" by version ");
        shebang.push_str(VERSION);
        let mut rng = SmallRng::seed_from_u64(seed);
        let sample = Args::sample(self.unicode);
        let mut random = Random::new(arena, &mut rng, sample)?;
        let mut file = random.file(self.items)?;
        file.shebang = Some(arena.intern(&shebang).into());
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
    fn comment(&mut self) -> Option<Lines<'a>> {
        if self.rng.random_bool(0.5) {
            Some(self.value().into())
        } else {
            None
        }
    }
    fn data(&mut self, shape: &Option<Silhouette>) -> anyhow::Result<Data<'a>> {
        Ok(if let Some(parent) = shape {
            let count = self.groups(&parent.children)?;
            self.list(count)?
        } else {
            Data::Text(self.value().into())
        })
    }
    fn member(&mut self, _shape: &Option<Silhouette>) -> anyhow::Result<Member<'a>> {
        self.data(&None)?;
        anyhow::bail!("todo member")
    }
    fn members(&mut self, kids: &[Option<Silhouette>]) -> anyhow::Result<usize> {
        for kid in kids {
            let member = self.member(kid)?;
            self.arena
                .builder()
                .push_member(member)
                .map_err(anyhow::Error::msg)?;
        }
        Ok(kids.len())
    }
    fn group(&mut self, _shape: &Option<Silhouette>) -> anyhow::Result<Group<'a>> {
        self.members(&[][..])?;
        anyhow::bail!("todo group")
    }
    fn groups(&mut self, _kids: &[Option<Silhouette>]) -> anyhow::Result<usize> {
        self.group(&None)?;
        anyhow::bail!("todo groups")
    }
    fn list(&mut self, count: usize) -> anyhow::Result<Data<'a>> {
        Ok(Data::List(List {
            prolog: self.comment(),
            groups: self
                .arena
                .builder()
                .finish_groups(count)
                .map_err(anyhow::Error::msg)?,
            epilog: self.comment(),
        }))
    }
    pub fn file(&mut self, grow: usize) -> anyhow::Result<File<'a>> {
        let shape = Silhouette::random(grow, self.rng);
        let count = self.groups(&shape.children)?;
        let shebang = if self.rng.random_bool(0.5) {
            None
        } else {
            Some(self.value().into())
        };
        Ok(File {
            shebang,
            prolog: self.comment(),
            groups: self
                .arena
                .builder()
                .finish_groups(count)
                .map_err(anyhow::Error::msg)?,
            // trailing: self.rng.random_range(0..3),
        })
    }
}
