//! this module is elided from release builds because it is a developer tool.

use anyhow::{Error, Result};
use std::path::PathBuf;
use tindalwic::CommentMarker;
use tindalwic::File;
use tindalwic::text::UTF8;

/// read tindalwic, print expected lezer tree to stdout
/// the core crate is authoritative so the lezer needs to conform.
/// a strategy for harmonizing is to generate the expected output for tests
/// mechanically then tweak the grammar code so it produces correct trees.
pub fn run(input: &PathBuf) -> Result<()> {
    let content = super::read_to_string(input)?;
    let bump = bumpalo::Bump::new();
    let mut arena = tindalwic::bumpalo::Arena::new(&bump);
    let parsed = arena.format_errors(&input.display(), &content, usize::MAX);
    let mut root = Parent::default();
    root.fill(Node::File(parsed.map_err(Error::msg)?));
    let mut buffer = String::new();
    Tree::Node(root).lezer(&mut buffer);
    println!("{}", buffer);
    Ok(())
}

enum Node<'a> {
    File(File<'a>),
    Lines(usize),
    Shebang(Option<UTF8<'a>>),
    Prolog(Option<UTF8<'a>>),
}
impl Parent {
    fn fill(&mut self, node: Node) {
        match node {
            Node::File(file) => {
                self.leaf("File");
                self.kid(Node::Shebang(file.shebang));
                self.kid(Node::Prolog(file.prolog));
            }
            Node::Lines(count) => {
                for _ in 0..count {
                    self.leaf("Line");
                }
            }
            Node::Shebang(Some(utf8)) => {
                self.leaf("Shebang");
                self.leaf(CommentMarker::Shebang.token());
                self.fill(Node::Lines(utf8.into_iter().count()));
            }
            Node::Prolog(Some(utf8)) => {
                self.leaf("Prolog");
                self.leaf(CommentMarker::Prolog.token());
                self.fill(Node::Lines(utf8.into_iter().count()));
            }
            _ => {}
        }
    }
    fn kid(&mut self, node: Node) {
        match node {
            Node::Lines(count) if count == 0 => {}
            Node::Shebang(None) => {}
            Node::Prolog(None) => {}
            _ => {
                let mut kid = Parent::default();
                kid.fill(node);
                self.kids.push(Tree::Node(kid));
            }
        }
    }
    fn leaf(&mut self, leaf: &'static str) {
        self.kids.push(Tree::Leaf(leaf));
    }
}
#[derive(Default)]
struct Parent {
    kids: Vec<Tree>,
}
enum Tree {
    Leaf(&'static str),
    Node(Parent),
}
impl Tree {
    fn lezer(&self, buffer: &mut String) {
        match self {
            Tree::Leaf(leaf) => buffer.push_str(leaf),
            Tree::Node(parent) => {
                let mut iter = parent.kids.iter();
                let Some(first) = iter.next() else { return };
                first.lezer(buffer);
                let Some(second) = iter.next() else { return };
                buffer.push('(');
                second.lezer(buffer);
                for rest in iter {
                    buffer.push(',');
                    rest.lezer(buffer);
                }
                buffer.push(')');
            }
        }
    }
}
