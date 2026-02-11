//#![deny(missing_docs)]
///! Text in Nested Dicts and Lists - with Important Comments
mod comments;
mod encoded;
mod maps;
mod paths;
mod values;

pub use comments::Comment;
use maps::Map;
pub use paths::{Path, PathErr, Step};
pub use values::{Dict, List, Text};

#[macro_export]
macro_rules! path {
    ($($step:tt),+) => {
        $crate::paths::Path::from(&[$($crate::path!(@step $step)),+][..])
    };
    (@step [$n:expr]) => {
        $crate::paths::Step::List($n)
    };
    (@step $s:literal) => {
        $crate::paths::Step::Dict($s)
    };
}

/// wrap Github Flavored Markdown in a Comment
pub fn comment<'a>(gfm: &'a str) -> Option<Comment<'a>> {
    Comment::adopt(gfm)
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct File<'a> {
    pub map: Map<'a>,
    pub hashbang: Option<Comment<'a>>,
    pub prolog: Option<Comment<'a>>,
}
