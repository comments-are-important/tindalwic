use crate::comments::Comment;

encoded_utf8_struct! {
    /// the fields of a [Value::Text]
    pub struct Text<'a> {
        /// a Text Value can have a Comment after it
        epilog
    }
}

impl<'a> Text<'a> {
    /// write the encoding of this Text into the given String.
    pub(crate) fn encode(&self, indent: usize, keyed: Option<&Keyed<'a>>, into: &mut String) {
        into.extend(std::iter::repeat_n('\t', indent));
        into.push('<');
        if let Some(keyed) = keyed {
            into.push_str(keyed.key)
        }
        into.push_str(">\n");
        let indent = indent + 1;
        self.encode_utf8(indent, "", into);
        if let Some(epilog) = &self.epilog {
            epilog.encode_utf8(indent, "#", into);
        }
    }

    /// add an epilog Comment.
    pub fn with_epilog(mut self, epilog: &'a str) -> Self {
        self.epilog = Comment::some(epilog);
        self
    }
}

/// the fields of a [Value::List]
#[derive(Clone, Debug)]
pub struct List<'a> {
    /// the items contained in the List
    pub vec: Vec<Value<'a>>,
    /// a List Value can start with a Comment
    pub prolog: Option<Comment<'a>>,
    /// a List Value can have a Comment after it
    pub epilog: Option<Comment<'a>>,
}
impl<'a> From<Vec<Value<'a>>> for List<'a> {
    /// convert a vector of items into a List.
    fn from(list: Vec<Value<'a>>) -> Self {
        List {
            vec: list,
            prolog: None,
            epilog: None,
        }
    }
}
impl<'a> List<'a> {
    /// write the encoding of this List into the given String.
    pub(crate) fn encode(&self, indent: usize, keyed: Option<&Keyed<'a>>, into: &mut String) {
        into.extend(std::iter::repeat_n('\t', indent));
        into.push('[');
        if let Some(keyed) = keyed {
            into.push_str(keyed.key)
        }
        into.push_str("]\n");
        let indent = indent + 1;
        if let Some(prolog) = &self.prolog {
            prolog.encode_utf8(indent, "#", into);
        }
        for item in &self.vec {
            item.encode(indent, None, into);
        }
        if let Some(epilog) = &self.epilog {
            epilog.encode_utf8(indent, "#", into);
        }
    }

    /// add a prolog Comment.
    pub fn with_prolog(mut self, prolog: &'a str) -> Self {
        self.prolog = Comment::some(prolog);
        self
    }

    /// add an epilog Comment.
    pub fn with_epilog(mut self, epilog: &'a str) -> Self {
        self.epilog = Comment::some(epilog);
        self
    }
}

/// an association.
///
/// for performance reasons these are stored in a [Vec].
#[derive(Clone, Debug)]
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
impl<'a> Keyed<'a> {
    /// convert a key and a value into an entry (for a Dict).
    pub fn from(key:&'a str, value: Value<'a>) -> Self {
        Keyed {
            key,
            gap: false,
            before: None,
            value,
        }
    }
}

macro_rules! keyed_vec_struct {
    {
        $(#[$struct_meta:meta])*
        $vis:vis struct $name:ident<$lifetime:lifetime> {
            $( $(#[$comment_meta:meta])* $comment:ident ),* $(,)?
        }
    } => {
        $(#[$struct_meta])*
        #[derive(Clone, Debug)]
        pub struct $name<$lifetime> {
            #[doc = concat!("the entries contained in this ", stringify!($name))]
            pub vec: Vec<Keyed<'a>>,
            $( $(#[$comment_meta])* $comment: Option<Comment<$lifetime>>, )*
        }
        impl<'a> From<Vec<Keyed<'a>>> for $name<'a> {
            /// convert a [Vec] of entries into a Dict.
            fn from(entries: Vec<Keyed<'a>>) -> Self {
                $name {
                    vec: entries,
                    $( $comment: None, )*
                }
            }
        }
        impl<'a> $name<'a> {
            /// returns number of entries.
            pub fn len(&self) -> usize {
                self.vec.len()
            }
            /// returns the position of the entry with the given key.
            pub fn position(&self, key: &str) -> Option<usize> {
                self.vec.iter().position(|x| x.key == key)
            }
            /// returns a reference to the entry with the given key.
            pub fn find(&self, key: &str) -> Option<&Keyed<'a>> {
                self.position(key).map(|i| &self.vec[i])
            }
            /// returns a mutable reference to the entry with the given key.
            pub fn find_mut(&mut self, key: &str) -> Option<&mut Keyed<'a>> {
                self.position(key).map(|i| &mut self.vec[i])
            }
            /// append the given entry to the end of the vec.
            pub fn push(&mut self, keyed: Keyed<'a>) {
                self.vec.push(keyed);
            }
            pub(crate) fn encode_keyed(&self, indent: usize, into: &mut String) {
                for keyed in &self.vec {
                    if keyed.gap {
                        into.push('\n');
                    }
                    if let Some(before) = &keyed.before {
                        before.encode_utf8(indent, "//", into);
                    }
                    keyed.value.encode(indent, Some(&keyed), into);
                }
            }
        }
    };
}

keyed_vec_struct!{
    /// the fields of a [Value::Dict]
    pub struct Dict<'a> {
        /// a Dict Value can start with a Comment
        prolog,
        /// a Dict Value can have a Comment after it
        epilog,
    }
}

impl<'a> Dict<'a> {
    /// write the encoding of this Dict into the given String.
    pub(crate) fn encode(&self, indent: usize, keyed: Option<&Keyed<'a>>, into: &mut String) {
        into.extend(std::iter::repeat_n('\t', indent));
        into.push('{');
        if let Some(keyed) = keyed {
            into.push_str(keyed.key)
        }
        into.push_str("}\n");
        let indent = indent + 1;
        if let Some(prolog) = &self.prolog {
            prolog.encode_utf8(indent, "#", into);
        }
        self.encode_keyed(indent, into);
        if let Some(epilog) = &self.epilog {
            epilog.encode_utf8(indent, "#", into);
        }
    }

    /// add a prolog Comment.
    pub fn with_prolog(mut self, prolog: &'a str) -> Self {
        self.prolog = Comment::some(prolog);
        self
    }

    /// add an epilog Comment.
    pub fn with_epilog(mut self, epilog: &'a str) -> Self {
        self.epilog = Comment::some(epilog);
        self
    }
}

/// the three possible Value types
#[derive(Clone, Debug)]
pub enum Value<'a> {
    /// a [Text] value holds UTF-8 content
    Text(Text<'a>),
    /// a [List] value is a linear array of values
    List(List<'a>),
    /// a [Dict] value is an associative array of Keyed values
    Dict(Dict<'a>),
}

impl<'a> Value<'a> {
    /// write the encoding of this Value into the given String.
    pub(crate) fn encode(&self, indent: usize, keyed: Option<&Keyed<'a>>, into: &mut String) {
        match self {
            Value::Text(text) => text.encode(indent, keyed, into),
            Value::List(list) => list.encode(indent, keyed, into),
            Value::Dict(dict) => dict.encode(indent, keyed, into),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn zzz() {
        let mut hi = String::from("hi");
        let mut text = Text::from(&hi[..]);
        text.epilog = Comment::some("comment");
        let mut root = Value::List(List::from(vec![Value::Text(text)]));
        //hi.clear(); // won't compile
        let result = path!([0]).text_mut(&mut root).unwrap();
        result.epilog = Comment::some("changed");
        //assert_eq!(text.epilog.unwrap().gfm.to_string(), "hi");
        hi.clear();
    }
    // fn duplicate_a_value() {
    //    app might want to clone a value (at some index or under some key)
    //    and add it again (at a different index or under different key)
    //    and that will need impl Clone for Value
    // }
}
