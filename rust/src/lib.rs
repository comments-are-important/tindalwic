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

pub struct File<'a> {
    pub dict: Map<'a>,
    pub hashbang: Option<Comment<'a>>,
    pub intro: Option<Comment<'a>>,
}

// // =============================================================================
// // File - top-level document with optional hashbang
// // =============================================================================

// /// A top-level tindalwic file, which is a Dict with an optional hashbang.
// /// Preserves insertion order.
// #[derive(Clone, Default)]
// pub struct File<'a> {
//     pub entries: IndexMap<&'a str, (Key<'a>, Value<'a>)>,
//     pub hashbang: Option<Comment<'a>>,
//     pub comment_intro: Option<Comment<'a>>,
// }

// impl<'a> File<'a> {
//     /// Create empty file.
//     pub fn new() -> Self {
//         Self {
//             entries: IndexMap::new(),
//             hashbang: None,
//             comment_intro: None,
//         }
//     }

//     /// Number of entries.
//     pub fn len(&self) -> usize {
//         self.entries.len()
//     }

//     /// True if empty.
//     pub fn is_empty(&self) -> bool {
//         self.entries.is_empty()
//     }

//     /// Insert a key-value pair.
//     pub fn insert(&mut self, key: Key<'a>, value: Value<'a>) {
//         self.entries.insert(key.name, (key, value));
//     }

//     /// Get value by key name.
//     pub fn get(&self, key: &str) -> Option<&Value<'a>> {
//         self.entries.get(key).map(|(_, v)| v)
//     }

//     /// Get mutable value by key name.
//     pub fn get_mut(&mut self, key: &str) -> Option<&mut Value<'a>> {
//         self.entries.get_mut(key).map(|(_, v)| v)
//     }

//     /// Get key and value by key name.
//     pub fn get_entry(&self, key: &str) -> Option<(&Key<'a>, &Value<'a>)> {
//         self.entries.get(key).map(|(k, v)| (k, v))
//     }

//     /// Iterate over (key, value) pairs in insertion order.
//     pub fn iter(&self) -> impl Iterator<Item = (&Key<'a>, &Value<'a>)> {
//         self.entries.values().map(|(k, v)| (k, v))
//     }

//     /// Iterate mutably over values in insertion order.
//     pub fn values_mut(&mut self) -> impl Iterator<Item = &mut Value<'a>> {
//         self.entries.values_mut().map(|(_, v)| v)
//     }
// }

// impl fmt::Debug for File<'_> {
//     fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
//         write!(f, "File(")?;
//         if let Some(ref c) = self.hashbang {
//             write!(f, "hashbang={:?},", c)?;
//         }
//         if let Some(ref c) = self.comment_intro {
//             write!(f, "intro={:?},", c)?;
//         }
//         for (key, value) in self.iter() {
//             write!(f, "{}={:?},", key.name, value)?;
//         }
//         write!(f, ")")
//     }
// }
