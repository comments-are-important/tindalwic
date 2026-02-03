//! ALACS file format and tools
//!
//! Data structures for representing text with comments.
//! Values can be Text, List, or Dict - augmented with comment metadata.
//! Users can manipulate these like standard collections while comments are preserved.
//!
//! All structures borrow from a source buffer via lifetime `'a`.
//! The source must be valid UTF-8 (validated once at parse time).

use indexmap::IndexMap;
use std::error::Error;
use std::fmt;

pub const MAX_INDENT: usize = 0x01FF;
pub const MAX_NEWLINES: usize = 0x3FFF;
pub const MAX_BYTES: usize = MAX_NEWLINES as usize;

#[derive(Clone, Debug)]
pub struct Comment<'a> {
    verbatim: &'a str,
    newlines: usize,
    dedent: usize,
}

#[derive(Debug)]
pub enum CommentErr {
    IndentTooLarge(usize),
    TooManyLines(usize),
}

impl fmt::Display for CommentErr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CommentErr::IndentTooLarge(x) => write!(
                f,
                "Indent ({}) must never exceed MAX_INDENT ({}).",
                x, MAX_INDENT
            ),
            CommentErr::TooManyLines(x) => write!(
                f,
                "Line count ({}) must never exceed MAX_NEWLINES ({}).",
                x, MAX_NEWLINES
            ),
        }
    }
}

impl Error for CommentErr {}

impl<'a> Comment<'a> {
    pub fn len(&self) -> usize {
        self.verbatim.len()
    }

    pub fn parse(stream: &'a str, indent: usize) -> Self {
        let bytes = stream.as_bytes();
        debug_assert!(bytes.len() <= MAX_BYTES);
        debug_assert!(indent < MAX_INDENT);
        let more = indent + 1;
        let mut newlines = 0;
        let mut cursor = 0;
        'outer: while cursor < bytes.len() {
            if bytes[cursor] != b'\n' {
                cursor += 1;
                continue;
            }
            if cursor + more >= bytes.len() {
                break;
            }
            for offset in 0..more {
                if bytes[cursor + 1 + offset] != b'\t' {
                    break 'outer;
                }
            }
            newlines += 1;
            cursor += 1 + more;
        }
        Comment {
            verbatim: &stream[..cursor],
            newlines: newlines,
            dedent: indent,
        }
    }

    pub fn build(&self, indent: usize, hashbang: bool, into: &mut String) {
        debug_assert!(indent < MAX_INDENT);
        let tabs = (indent as isize - self.dedent as isize) * self.newlines as isize;
        let delta = indent as isize + if hashbang { 3 } else { 2 } + tabs;
        let additional = self.verbatim.len().wrapping_add_signed(delta);
        let expected = into.len() + additional;
        into.reserve(additional);
        for _ in 0..indent {
            into.push('\t')
        }
        into.push('#');
        if hashbang {
            into.push('!')
        }
        if indent == self.dedent {
            into.push_str(self.verbatim);
            into.push('\n');
        } else {
            let mut lines = self.verbatim.split('\n');
            let Some(first) = lines.next() else { return };
            into.push_str(first);
            into.push('\n');
            let more = indent + 1;
            let dedent = self.dedent + 1;
            for line in lines {
                for _ in 0..more {
                    into.push('\t')
                }
                into.push_str(&line[dedent..]);
                into.push('\n');
            }
        }
        debug_assert_eq!(expected, into.len());
    }
}

#[derive(Clone, Debug)]
pub struct Key<'a> {
    pub key: &'a str,
    pub gap: bool,
    pub before: Option<Comment<'a>>,
}

#[derive(Clone, Debug)]
pub struct Map<'a> {
    pub map: IndexMap<&'a str, (Key<'a>, Value<'a>)>,
}
impl<'a> Map<'a> {
    pub fn new() -> Self {
        Map {
            map: IndexMap::new(),
        }
    }
}

#[derive(Clone, Debug)]
pub enum Value<'a> {
    Text {
        text: &'a str,
        dedent: usize,
        after: Option<Comment<'a>>,
    },
    List {
        list: Vec<Value<'a>>,
        intro: Option<Comment<'a>>,
        after: Option<Comment<'a>>,
    },
    Dict {
        dict: Map<'a>,
        intro: Option<Comment<'a>>,
        after: Option<Comment<'a>>,
    },
}
impl<'a> Value<'a> {
    pub fn text(text: &'a str) -> Self {
        Value::Text {
            text,
            dedent: 0,
            after: None,
        }
    }
    pub fn list() -> Self {
        Value::List {
            list: Vec::new(),
            intro: None,
            after: None,
        }
    }
    pub fn dict() -> Self {
        Value::Dict {
            dict: Map::new(),
            intro: None,
            after: None,
        }
    }
}

pub struct File<'a> {
    pub dict: Map<'a>,
    pub hashbang: Option<Comment<'a>>,
    pub intro: Option<Comment<'a>>,
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    const ZERO: &str = "#A\n\tB\nX";
    const ONE: &str = "\t#A\n\t\tB\nX";
    const TWO: &str = "\t\t#A\n\t\t\tB\nX";

    fn comment(stream: &str, line: usize, tabs: usize, indent: usize, expect: &str) {
        let lines: Vec<&str> = stream.split('\n').collect();
        let skip: usize = lines[..line].iter().map(|x| x.len()).sum();
        let chop: Vec<&str> = lines[line].splitn(2, '#').collect();
        assert_eq!(tabs, chop[0].len());
        let offset = skip + line + tabs + 1;
        let comment = Comment::parse(&stream[offset..], tabs);
        let mut buffer = String::new();
        comment.build(indent, false, &mut buffer);
        assert_eq!(buffer, expect);
    }

    #[test]
    fn comments_0_0() {
        comment(ZERO, 0, 0, 0, &ZERO[..ZERO.len() - 1]);
    }
    #[test]
    fn comments_0_1() {
        comment(ZERO, 0, 0, 1, &ONE[..ONE.len() - 1]);
    }
    #[test]
    fn comments_0_2() {
        comment(ZERO, 0, 0, 2, &TWO[..TWO.len() - 1]);
    }
    #[test]
    fn comments_1_0() {
        comment(ONE, 0, 1, 0, &ZERO[..ZERO.len() - 1]);
    }
    #[test]
    fn comments_1_1() {
        comment(ONE, 0, 1, 1, &ONE[..ONE.len() - 1]);
    }
    #[test]
    fn comments_1_2() {
        comment(ONE, 0, 1, 2, &TWO[..TWO.len() - 1]);
    }

    #[test]
    fn comments_z() {
        comment("Z\n\t\t\t#A\nX", 1, 3, 0, "#A\n");
    }
    #[test]
    fn comments_zz() {
        comment("Z\nZZ\n\t\t#A\nX", 2, 2, 0, "#A\n");
    }
    #[test]
    fn comments_zzz() {
        comment("Z\nZZ\nZZZ\n\t#A\nX", 3, 1, 0, "#A\n");
    }

    // #[test]
    // fn make_a_text_with_comment() {
    //     let buffer = "one\ntwo\n# comment";
    //     let address = buffer.as_ptr() as usize;
    //     let text = Value::Text {
    //         text: &buffer[0..6],
    //         dedent: 0,
    //         after: Some(Comment {
    //             markdown: &buffer[9..],
    //             dedent: 0,
    //         }),
    //     };
    //     match text {
    //         Value::Text {
    //             text,
    //             dedent: _,
    //             after,
    //         } => {
    //             assert_eq!(text.as_ptr() as usize, address);
    //             let after = after.expect("should have comment");
    //             assert_eq!(after.markdown.as_ptr() as usize, address + 9);
    //         }
    //         _ => panic!("oops"),
    //     }
    // }
}

// // =============================================================================
// // Value - the enum of possible value types
// // =============================================================================

// /// A value that can appear in an ALACS structure.
// /// All values can have a trailing comment.
// #[derive(Clone)]
// pub enum Value<'a> {
//     Text(Text<'a>),
//     List(List<'a>),
//     Dict(Dict<'a>),
// }

// impl<'a> Value<'a> {
//     /// Get the comment after this value, if any.
//     pub fn comment_after(&self) -> Option<&Comment<'a>> {
//         match self {
//             Value::Text(t) => t.comment_after.as_ref(),
//             Value::List(l) => l.comment_after.as_ref(),
//             Value::Dict(d) => d.comment_after.as_ref(),
//         }
//     }

//     /// Set the comment after this value.
//     pub fn set_comment_after(&mut self, comment: Option<Comment<'a>>) {
//         match self {
//             Value::Text(t) => t.comment_after = comment,
//             Value::List(l) => l.comment_after = comment,
//             Value::Dict(d) => d.comment_after = comment,
//         }
//     }
// }

// impl fmt::Debug for Value<'_> {
//     fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
//         match self {
//             Value::Text(t) => t.fmt(f),
//             Value::List(l) => l.fmt(f),
//             Value::Dict(d) => d.fmt(f),
//         }
//     }
// }

// // =============================================================================
// // Text - string lines with optional trailing comment
// // =============================================================================

// /// Text value: lines of text with an optional trailing comment.

// impl<'a> Text<'a> {
//     pub fn from_lines(lines: impl IntoIterator<Item = &'a str>) -> Self {
//         Self {
//             lines: lines.into_iter().collect(),
//             comment_after: None,
//         }
//     }
// }

// impl<'a> From<Text<'a>> for Value<'a> {
//     fn from(t: Text<'a>) -> Self {
//         Value::Text(t)
//     }
// }

// // =============================================================================
// // List - a sequence of Values with optional comments
// // =============================================================================

// /// A list of values with optional intro and trailing comments.
// #[derive(Clone, Default)]
// pub struct List<'a> {
//     pub items: Vec<Value<'a>>,
//     pub comment_intro: Option<Comment<'a>>,
//     pub comment_after: Option<Comment<'a>>,
// }

// impl<'a> List<'a> {
//     /// Create empty list.
//     pub fn new() -> Self {
//         Self {
//             items: Vec::new(),
//             comment_intro: None,
//             comment_after: None,
//         }
//     }

//     /// Number of items.
//     pub fn len(&self) -> usize {
//         self.items.len()
//     }

//     /// True if empty.
//     pub fn is_empty(&self) -> bool {
//         self.items.is_empty()
//     }

//     /// Append a value.
//     pub fn push(&mut self, value: Value<'a>) {
//         self.items.push(value);
//     }

//     /// Get item by index.
//     pub fn get(&self, index: usize) -> Option<&Value<'a>> {
//         self.items.get(index)
//     }

//     /// Get mutable item by index.
//     pub fn get_mut(&mut self, index: usize) -> Option<&mut Value<'a>> {
//         self.items.get_mut(index)
//     }

//     /// Iterate over items.
//     pub fn iter(&self) -> impl Iterator<Item = &Value<'a>> {
//         self.items.iter()
//     }

//     /// Iterate mutably over items.
//     pub fn iter_mut(&mut self) -> impl Iterator<Item = &mut Value<'a>> {
//         self.items.iter_mut()
//     }
// }

// impl fmt::Debug for List<'_> {
//     fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
//         write!(f, "List(")?;
//         for (i, item) in self.items.iter().enumerate() {
//             if i > 0 {
//                 write!(f, ",")?;
//             }
//             write!(f, "{:?}", item)?;
//         }
//         if let Some(ref c) = self.comment_intro {
//             write!(f, ",intro={:?}", c)?;
//         }
//         if let Some(ref c) = self.comment_after {
//             write!(f, ",after={:?}", c)?;
//         }
//         write!(f, ")")
//     }
// }

// impl<'a> From<List<'a>> for Value<'a> {
//     fn from(l: List<'a>) -> Self {
//         Value::List(l)
//     }
// }

// // =============================================================================
// // Key - a string key with optional preceding blank line and comment
// // =============================================================================

// /// A dictionary key with optional formatting metadata.
// #[derive(Clone)]
// pub struct Key<'a> {
//     pub name: &'a str,
//     pub blank_line_before: bool,
//     pub comment_before: Option<Comment<'a>>,
// }

// impl<'a> Key<'a> {
//     /// Create a key from a string slice.
//     /// Panics if the name contains a newline.
//     pub fn new(name: &'a str) -> Self {
//         if name.contains('\n') {
//             panic!("newline in key");
//         }
//         Self {
//             name,
//             blank_line_before: false,
//             comment_before: None,
//         }
//     }

//     /// Get the key name.
//     pub fn as_str(&self) -> &'a str {
//         self.name
//     }
// }

// impl fmt::Debug for Key<'_> {
//     fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
//         write!(f, "Key({:?})", self.name)
//     }
// }

// impl fmt::Display for Key<'_> {
//     fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
//         write!(f, "{}", self.name)
//     }
// }

// // =============================================================================
// // Dict - a map of Key to Value with optional comments
// // =============================================================================

// /// A dictionary mapping keys to values, with optional intro and trailing comments.
// /// Preserves insertion order.
// #[derive(Clone, Default)]
// pub struct Dict<'a> {
//     pub entries: IndexMap<&'a str, (Key<'a>, Value<'a>)>,
//     pub comment_intro: Option<Comment<'a>>,
//     pub comment_after: Option<Comment<'a>>,
// }

// impl<'a> Dict<'a> {
//     /// Create empty dict.
//     pub fn new() -> Self {
//         Self {
//             entries: IndexMap::new(),
//             comment_intro: None,
//             comment_after: None,
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

// impl fmt::Debug for Dict<'_> {
//     fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
//         write!(f, "Dict(")?;
//         if let Some(ref c) = self.comment_intro {
//             write!(f, "intro={:?},", c)?;
//         }
//         if let Some(ref c) = self.comment_after {
//             write!(f, "after={:?},", c)?;
//         }
//         for (key, value) in self.iter() {
//             write!(f, "{}={:?},", key.name, value)?;
//         }
//         write!(f, ")")
//     }
// }

// impl<'a> From<Dict<'a>> for Value<'a> {
//     fn from(d: Dict<'a>) -> Self {
//         Value::Dict(d)
//     }
// }

// // =============================================================================
// // File - top-level document with optional hashbang
// // =============================================================================

// /// A top-level ALACS file, which is a Dict with an optional hashbang.
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

// // =============================================================================
// // Tests
// // =============================================================================

// #[cfg(test)]
// mod tests {
//     use super::*;

//     #[test]
//     fn can_borrow_vec_from_text() {
//         let mut tac = Text::from_lines([]);
//         tac.comment_after = Some(Comment::from_line("hi"));
//         tac.push("value");
//         assert_eq!(tac.lines.len(), tac.len());
//         assert_eq!(tac.lines.capacity(), tac.capacity());
//     }
//     #[test]
//     fn utf8_from_line() {
//         let utf8 = Utf8::from_line("hello world");
//         assert_eq!(utf8.len(), 1);
//         assert_eq!(utf8.get(0), Some("hello world"));
//     }

//     #[test]
//     fn utf8_normalize_splits_newlines() {
//         let mut utf8 = Utf8::from_line("line1\nline2\nline3");
//         utf8.normalize();
//         assert_eq!(utf8.len(), 3);
//         assert_eq!(utf8.get(0), Some("line1"));
//         assert_eq!(utf8.get(1), Some("line2"));
//         assert_eq!(utf8.get(2), Some("line3"));
//     }

//     #[test]
//     fn utf8_normalize_clears_single_empty() {
//         let mut utf8 = Utf8::from_line("");
//         utf8.normalize();
//         assert!(utf8.is_empty());
//     }

//     #[test]
//     fn utf8_join() {
//         let utf8 = Utf8::from_lines(["a", "b", "c"]);
//         assert_eq!(utf8.join(), "a\nb\nc");
//     }

//     // #[test]
//     // fn text_display() {
//     //     let text = Text::from_line("hello");
//     //     assert_eq!(format!("{}", text), "hello");
//     // }

//     #[test]
//     fn dict_insert_and_get() {
//         let mut dict = Dict::new();
//         dict.insert(Key::new("name"), Text::from_lines(["value"]).into());
//         assert!(dict.get("name").is_some());
//     }

//     #[test]
//     #[should_panic(expected = "newline in key")]
//     fn key_rejects_newline() {
//         Key::new("bad\nkey");
//     }

//     #[test]
//     fn file_borrows_from_buffer() {
//         // Demonstrates the borrowing pattern
//         let buffer = "key=value";
//         let key_slice = &buffer[0..3];
//         let value_slice = &buffer[4..9];

//         let mut file = File::new();
//         file.insert(Key::new(key_slice), Text::from_lines([value_slice]).into());

//         assert_eq!(file.len(), 1);
//         // buffer is still accessible here - file borrows from it
//         assert_eq!(&buffer[0..3], "key");
//     }

//     #[test]
//     fn dict_preserves_insertion_order() {
//         let mut dict = Dict::new();
//         dict.insert(Key::new("charlie"), Text::from_lines(["3"]).into());
//         dict.insert(Key::new("alpha"), Text::from_lines(["1"]).into());
//         dict.insert(Key::new("bravo"), Text::from_lines(["2"]).into());

//         let keys: Vec<_> = dict.iter().map(|(k, _)| k.name).collect();
//         assert_eq!(keys, vec!["charlie", "alpha", "bravo"]);
//     }

//     #[test]
//     fn dict_update_preserves_position() {
//         let mut dict = Dict::new();
//         dict.insert(Key::new("a"), Text::from_lines(["1"]).into());
//         dict.insert(Key::new("b"), Text::from_lines(["2"]).into());
//         dict.insert(Key::new("a"), Text::from_lines(["updated"]).into());

//         let keys: Vec<_> = dict.iter().map(|(k, _)| k.name).collect();
//         assert_eq!(keys, vec!["a", "b"]); // "a" stays in original position
//     }
// }
