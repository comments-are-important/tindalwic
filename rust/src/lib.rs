#![deny(missing_docs)] //, unused

//! Text in Nested Dicts and Lists - with Important Comments

mod comments;
mod encoded;
mod maps;
mod paths;
mod values;

pub use comments::Comment;
pub use paths::{Path, PathErr, Step};
pub use values::{Dict, List, Text, Value};

/// build a [Path] from steps
#[macro_export]
macro_rules! path {
    ($($step:tt),+) => {
        $crate::Path::from(&[$($crate::path!(@step $step)),+][..])
    };
    (@step [$n:expr]) => {
        $crate::Step::List($n)
    };
    (@step $s:literal) => {
        $crate::Step::Dict($s)
    };
}

/// wrap Github Flavored Markdown in a Comment
pub fn comment<'a>(gfm: &'a str) -> Option<Comment<'a>> {
    Comment::adopt(gfm)
}
/// wrap UTF-8 in a Text
pub fn text<'a>(utf8: &'a str) -> Value<'a> {
    Value::Text(Text::adopt(utf8))
}
/// wrap a linear array of values into a List
pub fn list<'a>(vec: Vec<Value<'a>>) -> Value<'a> {
    Value::List(List::adopt(vec))
}

/// the outermost context.
///
/// very similar to a [Dict], just with different comments.
#[derive(Debug, PartialEq, Eq, Clone)]
pub struct File<'a> {
    /// the entries contained in the File
    pub map: maps::Map<'a>,
    /// a File can start with a Unix `#!` comment
    pub hashbang: Option<Comment<'a>>,
    /// a file can have an introductory Comment
    pub prolog: Option<Comment<'a>>,
}
