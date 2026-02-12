#![warn(missing_docs)] //, unused
//! Text in Nested Dicts and Lists - with Important Comments

mod comments;
mod encoded;
#[macro_use]
mod paths;
#[macro_use]
mod values;

pub use comments::Comment;
pub use paths::{Path, PathErr, PathStep};
pub use values::{Dict, Keyed, List, Text, Value};

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
