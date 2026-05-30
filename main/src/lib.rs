#![no_std]

//! Text in Nested Dictionaries and Lists - with Important Comments

use core::cell::Cell;

#[doc(inline)]
/// traverse a path from the root down into the data structure.
///
/// the syntax is very close to that of the encoded data.
pub use tindalwic_macros::walk;

#[doc(inline)]
/// build an [Item] using a subset of the JSON syntax.
///
/// this helps to write code snippets that make a structural change to a [File].
/// a typical snippet would:
///  + [walk!] into a [File] to the place to be changed,
///  + use [json!] to build a new [Item],
///  + then use [core::cell::Cell::set] to affect the change.
pub use tindalwic_macros::json;

#[doc(inline)]
pub use tindalwic_macros::arena;

pub mod capped;
pub mod fmt;
pub mod parse;
pub mod walk;

#[cfg(feature = "alloc")]
pub mod alloc;
#[cfg(feature = "bumpalo")]
pub mod bumpalo;
#[cfg(feature = "serde")]
pub mod serde;

mod value {
    /// All primitive values in Tindalwic are string slice references, not owned.
    ///
    ///  + [Comment::value](super::Comment::value)
    ///  + [Text::value](super::Text::value)
    ///  + [Entry::key](super::Entry::key)
    ///
    /// They often contain embedded indentation because the parser is zero-copy from
    /// the encoded data. The methods here will strip indentation as necessary.
    /// Apps that modify only a few values do not have to pay for any processing of
    /// unmodified values that are already appropriately indented.
    #[derive(Clone, Copy, Debug)]
    pub struct Value<'a> {
        slice: &'a str,
        indent: usize, // usize::MAX => single line
    }
    impl<'a> PartialEq for Value<'a> {
        fn eq(&self, other: &Self) -> bool {
            if self.indent == other.indent {
                self.slice == other.slice
            } else {
                self.lines().eq(other.lines())
            }
        }
    }
    impl<'a> Value<'a> {
        /// `true` when zero chars (see [str::is_empty]).
        pub fn is_empty(&self) -> bool {
            self.slice.is_empty()
        }
        pub(crate) fn byte_count(&self) -> usize {
            self.slice.as_bytes().len()
        }
        /// `true` if prefix matches (see [str::starts_with]).
        ///
        /// Restricted to char until [core::str::pattern::Pattern] is stable.
        pub fn starts_with(&self, pat: char) -> bool {
            self.slice.starts_with(pat)
        }
        /// the format sometimes allows shorter encoding for single line values
        pub fn only_line(&self) -> Option<&'a str> {
            if self.indent == usize::MAX {
                Some(self.slice)
            } else {
                None
            }
        }
        /// if the value was captured at this indent, then it can be used as is.
        pub fn verbatim(&self, indent: usize) -> Option<&'a str> {
            let only = self.only_line();
            if only.is_some() {
                only
            } else if indent == self.indent {
                Some(self.slice)
            } else {
                None
            }
        }
        /// Returned iterator produces one sub-slice for each line.
        ///
        /// Always produces at least one line. Omits indentation and newline chars.
        pub fn lines(&self) -> impl Iterator<Item = &'a str> {
            // that return type is tricky to satisfy: having two branches here (one
            // optimized for absent indentation) causes E0308 incompatible types:
            //   "distinct uses of `impl Trait` result in different opaque types"
            // attempting to hide them behind closures does not help either:
            //   "no two closures, even if identical, have the same type"
            let d = if self.only_line().is_some() {
                0
            } else {
                self.indent
            };
            self.slice
                .split('\n')
                .enumerate()
                .map(move |(i, s)| if i == 0 || d == 0 { s } else { &s[d..] })
        }
        /// Take as many chars as possible from beginning of slice.
        ///
        /// No indentation is expected at the beginning, subsequent indented lines
        /// (even those with excess indentation) are included.
        pub fn slice_prefix(indent: usize, slice: &'a str) -> Self {
            assert!(indent != usize::MAX, "indent can't be MAX");
            if slice.is_empty() {
                return Value::default();
            }
            let bytes = slice.as_bytes();
            let limit = bytes.len();
            let mut offset = 0usize;
            while bytes[offset] != b'\n' {
                offset += 1;
                if offset >= limit {
                    let indent = usize::MAX;
                    return Value { slice, indent };
                }
            }
            if indent == 0 {
                return Value { slice, indent };
            }
            let mut tabs = crate::parse::indentation(bytes, offset + 1, limit);
            if tabs < indent {
                return Value {
                    slice: &slice[..offset],
                    indent: usize::MAX,
                };
            }
            loop {
                offset += tabs + 1;
                if offset >= limit {
                    return Value { slice, indent };
                }
                while bytes[offset] != b'\n' {
                    offset += 1;
                    if offset >= limit {
                        return Value { slice, indent };
                    }
                }
                tabs = crate::parse::indentation(bytes, offset + 1, limit);
                if tabs < indent {
                    let slice = &slice[..offset];
                    return Value { slice, indent };
                }
            }
        }
    }
    impl<'a> Default for Value<'a> {
        fn default() -> Self {
            Value {
                slice: "",
                indent: usize::MAX,
            }
        }
    }
}
pub use value::Value;
impl<'a> Value<'a> {
    /// linear `O(n)` scan.
    // TODO: add link to `alloc` map view, say it "offers `O(1)`."
    pub fn find_linearly_in(self, cells: &'a [Cell<Entry<'a>>]) -> Option<usize> {
        cells.iter().position(|cell| cell.get().key == self)
    }
}
impl<'a> From<&'a str> for Value<'a> {
    fn from(value: &'a str) -> Self {
        Value::slice_prefix(0, value)
    }
}
impl<'a> Eq for Value<'a> {}
impl<'a> core::hash::Hash for Value<'a> {
    fn hash<H: core::hash::Hasher>(&self, state: &mut H) {
        for line in self.lines() {
            line.hash(state);
        }
    }
}

// ====================================================================================

/// Metadata about an [Item], [Entry] or [File].
///
/// A serialized [Comment] will start with one of three possible markers, depending
/// on its position:
///  + `#!` for [File::hashbang],
///  + `//` for [Entry::before].
///  + `#` for the various `prolog` and `epilog` fields,
///
/// The content is UTF-8 Github Flavored Markdown.
///
/// A field within the [Item] or File will hold the Comment, there is no mechanism to
/// navigate from a Comment to the thing it describes.
///
/// # Examples
///
/// ```
/// # #[cfg(feature="alloc")]
/// # {
/// use tindalwic::*;
/// let comment = Comment {
///     value: "with ~strikethrough~ extension".into(),
/// };
///
/// let html = markdown::to_html_with_options(&comment.joined(), &markdown::Options::gfm()).expect(
///     "should never error, according to:
///      <https://docs.rs/markdown/latest/markdown/fn.to_html_with_options.html#errors>",
/// );
///
/// assert_eq!(html, "<p>with <del>strikethrough</del> extension</p>");
/// # }
/// ```
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Comment<'a> {
    /// the string value
    pub value: Value<'a>,
}

// ------------------------------------------------------------------------------------

/// [Item::Text] wraps a sequence of lines of UTF-8, and optional epilog comment.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Text<'a> {
    /// the string value
    pub value: Value<'a>,
    /// A Text can have a Comment after it.
    pub epilog: Option<Comment<'a>>,
}

// ------------------------------------------------------------------------------------

/// [Item::List] wraps a sequence of `Cell<Item>`, and optional prolog and epilog comments.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct List<'a> {
    /// A List can have an introductory Comment.
    pub prolog: Option<Comment<'a>>,
    /// The contents of the Item::List.
    pub cells: &'a [Cell<Item<'a>>],
    /// A List can have a Comment after it.
    pub epilog: Option<Comment<'a>>,
}

// ------------------------------------------------------------------------------------

/// an association (from key to item) and its metadata.
///
/// at the lowest level, these are stored in an array.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Entry<'a> {
    /// a key can have a blank line before it (before its comment)
    pub gap: bool,
    /// a key can have a comment before it (after its blank line).
    pub before: Option<Comment<'a>>,
    /// the key being associated to an [Item].
    pub key: Value<'a>,
    /// the item associated to the [Entry::key]
    pub item: Item<'a>,
}
impl<'a> Default for Entry<'a> {
    fn default() -> Self {
        Entry {
            gap: false,
            before: None,
            key: Value::default(),
            item: Item::default(),
        }
    }
}
impl<'a> Entry<'a> {
    /// Make a fixed-size array of cells on the stack.
    pub fn array<const N: usize>() -> [Cell<Entry<'a>>; N] {
        ::core::array::from_fn::<_, N, _>(|_| Cell::default())
    }
}

/// [Item::Dict] wraps a sequence of `Cell<Entry>`, and optional prolog and epilog comments.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Dict<'a> {
    /// A Dict can have an introductory Comment.
    pub prolog: Option<Comment<'a>>,
    /// The contents of the Item::Dict.
    pub cells: &'a [Cell<Entry<'a>>],
    /// A Dict can have a Comment after it.
    pub epilog: Option<Comment<'a>>,
}

// ------------------------------------------------------------------------------------

/// the three Item variants
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Item<'a> {
    /// a [Text] Item holds UTF-8 content
    Text(Text<'a>),
    /// a [List] Item is a linear array of [Item]
    List(List<'a>),
    /// a [Dict] Item is an associative array of [Entry]
    Dict(Dict<'a>),
}
impl<'a> Default for Item<'a> {
    fn default() -> Self {
        Item::Text(Text::default())
    }
}
impl<'a> Item<'a> {
    /// Make a fixed-size array of cells on the stack.
    pub fn array<const N: usize>() -> [Cell<Item<'a>>; N] {
        ::core::array::from_fn::<_, N, _>(|_| Cell::default())
    }
    /// wrap a value (no epilog) into an Item::Text
    pub fn text(value: &'a str) -> Self {
        Item::Text(Text {
            value: value.into(),
            ..Default::default()
        })
    }
}
impl<'a> From<Text<'a>> for Item<'a> {
    fn from(value: Text<'a>) -> Self {
        Item::Text(value)
    }
}
impl<'a> From<List<'a>> for Item<'a> {
    fn from(value: List<'a>) -> Self {
        Item::List(value)
    }
}
impl<'a> From<Dict<'a>> for Item<'a> {
    fn from(value: Dict<'a>) -> Self {
        Item::Dict(value)
    }
}

// ------------------------------------------------------------------------------------

/// the outermost context.
///
/// similar to a [Item::Dict], but with different comments.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct File<'a> {
    /// A File can start with a Unix `#!` Comment.
    pub hashbang: Option<Comment<'a>>,
    /// A File can have an introductory Comment.
    pub prolog: Option<Comment<'a>>,
    /// The contents of the Item::File.
    pub cells: &'a [Cell<Entry<'a>>],
}

#[cfg(test)]
#[allow(unused_extern_crates)]
extern crate self as test_rename_of_tindalwic_dependency;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::json;

    #[test]
    fn rename() {
        json! {
            $crate = test_rename_of_tindalwic_dependency;
            let empty = {}.unwrap();
            completed.unwrap();
        }
        assert!(empty.cells.is_empty());
    }

    #[test]
    fn value_eq() {
        let zero: Value<'_> = "ONE\nTWO".into();
        let one = Value::slice_prefix(1, "ONE\n\tTWO");
        let two = Value::slice_prefix(2, "ONE\n\t\tTWO");
        assert_eq!(zero, one);
        assert_eq!(one, two);
    }
}
