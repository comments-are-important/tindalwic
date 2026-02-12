use crate::comments::Comment;
use crate::encoded::Encoded;
use super::Keyed;

/// the fields of a [Value::Text]
#[derive(Debug, PartialEq, Eq, Clone, Default)]
pub struct Text<'a> {
    /// the encoded UTF-8 content
    pub utf8: Encoded<'a>,
    /// a Text Value can have a Comment after it
    pub epilog: Option<Comment<'a>>,
}
impl<'a> Text<'a> {
    /// wrap a reference to content into a Text
    pub fn adopt(utf8: &'a str) -> Self {
        Text {
            utf8: Encoded::adopt(utf8),
            epilog: None,
        }
    }
    /// write the encoding of this Text into the given String.
    pub(crate) fn encode(&self, indent: usize, keyed: Option<&Keyed<'a>>, into: &mut String) {
        into.extend(std::iter::repeat_n('\t', indent));
        into.push('<');
        if let Some(keyed) = keyed {
            into.push_str(keyed.key)
        }
        into.push_str(">\n");
        let indent = indent + 1;
        self.utf8.encode(indent, "", into);
        if let Some(epilog) = self.epilog {
            epilog.encode(indent, "#", into);
        }
    }
}

/// the fields of a [Value::List]
#[derive(Debug, PartialEq, Eq, Clone, Default)]
pub struct List<'a> {
    /// the items contained in the List
    pub vec: Vec<Value<'a>>,
    /// a List Value can start with a Comment
    pub prolog: Option<Comment<'a>>,
    /// a List Value can have a Comment after it
    pub epilog: Option<Comment<'a>>,
}
impl<'a> List<'a> {
    /// take ownership of the items
    pub fn adopt(list: Vec<Value<'a>>) -> Self {
        List {
            vec: list,
            prolog: None,
            epilog: None,
        }
    }
    /// write the encoding of this List into the given String.
    pub(crate) fn encode(&self, indent: usize, keyed: Option<&Keyed<'a>>, into: &mut String) {
        into.extend(std::iter::repeat_n('\t', indent));
        into.push('[');
        if let Some(keyed) = keyed {
            into.push_str(keyed.key)
        }
        into.push_str("]\n");
        let indent = indent + 1;
        if let Some(prolog) = self.prolog {
            prolog.encode(indent, "#", into);
        }
        for item in &self.vec {
            item.encode(indent, None, into);
        }
        if let Some(epilog) = self.epilog {
            epilog.encode(indent, "#", into);
        }
    }
}

/// the fields of a [Value::Dict]
#[derive(Debug, PartialEq, Eq, Clone, Default)]
pub struct Dict<'a> {
    /// the entries contained in the Dict
    pub vec: Vec<Keyed<'a>>,
    /// a Dict Value can start with a Comment
    pub prolog: Option<Comment<'a>>,
    /// a Dict Value can have a Comment after it
    pub epilog: Option<Comment<'a>>,
}

impl<'a> Dict<'a>{
impl_keyed_vec!();
    /// write the encoding of this Dict into the given String.
    pub(crate) fn encode(&self, indent: usize, keyed: Option<&Keyed<'a>>, into: &mut String) {
        into.extend(std::iter::repeat_n('\t', indent));
        into.push('{');
        if let Some(keyed) = keyed {
            into.push_str(keyed.key)
        }
        into.push_str("}\n");
        let indent = indent + 1;
        if let Some(prolog) = self.prolog {
            prolog.encode(indent, "#", into);
        }
        self.encode_keyed(indent, into);
        if let Some(epilog) = self.epilog {
            epilog.encode(indent, "#", into);
        }
    }
}

/// the three possible Value types
#[derive(Debug, PartialEq, Eq, Clone)]
pub enum Value<'a> {
    /// a [Text] value holds UTF-8 content
    Text(Text<'a>),
    /// a [List] value is a linear array of values
    List(List<'a>),
    /// a [Dict] value is an associative array of Keyed values
    Dict(Dict<'a>),
}

/// an empty Text value
impl Default for Value<'static> {
    fn default() -> Self {
        Value::Text(Text::default())
    }
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
        let mut text = Text::adopt(&hi);
        text.epilog = Comment::adopt("comment");
        let mut root = Value::List(List::adopt(vec![Value::Text(text)]));
        //hi.clear(); // won't compile
        let result = path!([0]).text_mut(&mut root).unwrap();
        result.epilog = Comment::adopt("changed");
        //assert_eq!(text.epilog.unwrap().gfm.to_string(), "hi");
        hi.clear();
    }
    // fn duplicate_a_value() {
    //    app might want to clone a value (at some index or under some key)
    //    and add it again (at a different index or under different key)
    //    and that will need impl Clone for Value
    // }
}
