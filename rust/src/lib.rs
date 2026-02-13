#![warn(missing_docs)] //, unused
//! Text in Nested Dicts and Lists - with Important Comments

#[macro_use]
mod comments;
#[macro_use]
mod paths;
#[macro_use]
mod values;

pub use comments::Comment;
pub use paths::{Path, PathErr, PathStep};
pub use values::{Dict, Keyed, List, Text, Value};

keyed_vec_struct!{
    /// the outermost context.
    ///
    /// very similar to a [Dict], just with different comments.
    pub struct File<'a> {
        /// a File can start with a Unix `#!` comment
        hashbang,
        /// a file can have an introductory Comment
        prolog,
    }
}

impl<'a> File<'a> {
    /// write the encoding of this File `into` the String (clearing it first).
    pub fn encode(&self, into: &mut String) {
        into.clear();
        if let Some(hashbang) = &self.hashbang {
            hashbang.encode_utf8(0, "#!", into);
        }
        if let Some(prolog) = &self.prolog {
            prolog.encode_utf8(0, "#", into);
        }
        self.encode_keyed(0, into);
    }
    /// return the encoding of this File in a freshly allocated String.
    pub fn tindalwic(&self) -> String {
        let mut bytes = String::new();
        self.encode(&mut bytes);
        bytes
    }
    /// add a hashbang Comment.
    pub fn with_hashbang(mut self, hashbang: &'a str) -> Self {
        self.hashbang = Comment::some(hashbang);
        self
    }
    /// add a prolog Comment.
    pub fn with_prolog(mut self, prolog: &'a str) -> Self {
        self.prolog = Comment::some(prolog);
        self
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
            File::from(vec![Keyed::from(
                "one",
                Value::List(List::from(vec![Value::Text(Text::from(""))]))
            )])
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
        let mut file = File::from(vec![])
            .with_hashbang("/usr/bin/env -S app argument")
            .with_prolog(" this is the prolog for the file");
        file.push(Keyed {
            key: "one",
            gap: true,
            before: Comment::some(" about key one"),
            value: Value::Text(Text::from("one").with_epilog(" about value one")),
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
