#![warn(missing_docs)] //, unused
//! Text in Nested Dicts and Lists - with Important Comments

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

macro_rules! impl_keyed_vec {
    () => {
        /// returns number of entries.
        pub fn len(&self) -> usize {
            self.vec.len()
        }
        /// returns the position of the entry with the given key.
        pub fn position(&self, key: &str) -> Option<usize> {
            self.vec.iter().position(|x|x.key == key)
        }
        /// returns a reference to the entry with the given key.
        pub fn find(&self, key: &str) -> Option<&Keyed<'a>> {
            self.position(key).map(|i|&self.vec[i])
        }
        /// returns a mutable reference to the entry with the given key.
        pub fn find_mut(&mut self, key: &str) -> Option<&mut Keyed<'a>> {
            self.position(key).map(|i|&mut self.vec[i])
        }
        /// append the given entry to the end of the vec.
        pub fn push(&mut self, keyed:Keyed<'a>) {
            self.vec.push(keyed);
        }
        pub(crate) fn encode_keyed(&self, indent:usize, into: &mut String) {
            for keyed in &self.vec {
                if keyed.gap {
                    into.push('\n');
                }
                if let Some(before) = keyed.before {
                    before.encode(indent, "//", into);
                }
                keyed.value.encode(indent, Some(&keyed), into);
            }
        }
    };
}

mod comments;
mod encoded;
mod paths;
mod values;

pub use comments::Comment;
pub use paths::{Path, PathErr, Step};
pub use values::{Dict, List, Text, Value};

/// an association.
///
/// for performance reasons these are stored in a [Vec].
#[derive(Debug, PartialEq, Eq, Clone, Default)]
pub struct Keyed<'a> {
    /// the key being associated to the value.
    pub key: &'a str,
    /// a key can have a blank line before it (before its comment)
    pub gap: bool,
    /// a key can have a comment before it (after its blank line).
    pub before: Option<Comment<'a>>,
    /// the value associated to the key
    pub value: Value<'a>,
}

/// the outermost context.
///
/// very similar to a [Dict], just with different comments.
#[derive(Debug, PartialEq, Eq, Clone, Default)]
pub struct File<'a> {
    /// the entries contained in the File
    pub vec: Vec<Keyed<'a>>,
    /// a File can start with a Unix `#!` comment
    pub hashbang: Option<Comment<'a>>,
    /// a file can have an introductory Comment
    pub prolog: Option<Comment<'a>>,
}

impl<'a> File<'a> {
impl_keyed_vec!();
    /// write the encoding of this File `into` the String (clearing it first).
    pub fn encode(&self, into: &mut String) {
        into.clear();
        if let Some(hashbang) = self.hashbang {
            hashbang.encode(0, "#!", into);
        }
        if let Some(prolog) = self.prolog {
            prolog.encode(0, "#", into);
        }
        self.encode_keyed(0, into);
    }
    /// return the encoding of this File in a freshly allocated String.
    pub fn tindalwic(&self) -> String {
        let mut bytes = String::new();
        self.encode(&mut bytes);
        bytes
    }
}

#[cfg(test)]
mod test {
    use super::*;

    fn visible(string: &str) -> String {
        string.replace("╶─▸", "\t").replace("                ", "")
    }

    #[test]
    fn encode_uncommented_file() {
        assert_eq!(
            File {
                vec: vec![Keyed {
                    key: "one",
                    value: Value::List(List {
                        vec: vec![Value::default()],
                        ..Default::default()
                    }),
                    ..Default::default()
                }],
                ..Default::default()
            }
            .tindalwic(),
            visible(
                "[one]
                ╶─▸<>
                ╶─▸╶─▸
                "
            )
        );
    }
    #[test]
    fn encode_fully_commented_file() {
        let mut file = File {
            hashbang: Comment::adopt("/usr/bin/env -S app argument"),
            prolog: Comment::adopt(" this is the prolog for the file"),
            ..Default::default()
        };
        file.push(Keyed {
            key: "one",
            gap: true,
            before: Comment::adopt(" about key one"),
            value: Value::Text(Text {
                utf8: encoded::Encoded::adopt("one"),
                epilog: Comment::adopt(" about value one"),
                ..Default::default()
            }),
            ..Default::default()
        });
        assert_eq!(
            file.tindalwic(),
            visible(
                "#!/usr/bin/env -S app argument
                # this is the prolog for the file

                // about key one
                <one>
                ╶─▸one
                ╶─▸# about value one
                "
            )
        )
    }
}
