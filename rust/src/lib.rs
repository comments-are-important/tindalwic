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
///  + then use [Cell::set] to affect the change.
pub use tindalwic_macros::json;

#[doc(inline)]
pub use tindalwic_macros::arena;

pub mod internals; // macro generated code needs access.

mod fmt;
mod parse;

#[cfg(feature = "alloc")]
mod alloc;

/// hopefully change to `pub use core::range::Range` when that becomes stable.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct Range<Idx> {
    /// The lower bound of the range (inclusive).
    pub start: Idx,
    /// The upper bound of the range (exclusive).
    pub end: Idx,
}
impl<Idx> From<core::ops::Range<Idx>> for Range<Idx> {
    fn from(r: core::ops::Range<Idx>) -> Self {
        Range {
            start: r.start,
            end: r.end,
        }
    }
}
impl<Idx> From<Range<Idx>> for core::ops::Range<Idx> {
    fn from(value: Range<Idx>) -> Self {
        value.start..value.end
    }
}

/// parsing problem
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub struct ParseError {
    /// span of the problem
    pub lines: Range<usize>,
    /// English description of the problem
    pub message: &'static str,
}
impl core::error::Error for ParseError {}
impl ParseError {
    /// make an Error with an arbitrary span of lines.
    pub fn new(lines: impl Into<Range<usize>>, message: &'static str) -> Self {
        ParseError {
            lines: lines.into(),
            message,
        }
    }
    /// make an Error for a single line.
    pub fn at(line: usize, message: &'static str) -> Self {
        ParseError::new(line..line + 1, message)
    }
}

/// Hidden parts of [Comment] and [Text].
///
/// These are zero-copy slices from an external buffer of Tindalwic UTF-8. The iterator
/// returned by [Encoded::lines()] is the most efficient way to strip the indentation
/// from a multi-line slice.
#[derive(Clone, Copy, Debug, PartialEq)]
struct UTF8<'a> {
    slice: &'a str,
    dedent: usize, // usize::MAX => one_liner
}
impl<'a> UTF8<'a> {
    /// Return a zero-copy instance using the provided literal (not indented) slice.
    fn wrap(utf8: &'a str) -> Self {
        let bytes = utf8.as_bytes();
        let mut newline = 0usize;
        while newline < bytes.len() && bytes[newline] != b'\n' {
            newline += 1;
        }
        UTF8 {
            slice: utf8,
            dedent: if newline < bytes.len() { 0 } else { usize::MAX },
        }
    }
    /// return true when there are zero UTF-8 bytes.
    pub fn is_empty(&self) -> bool {
        self.slice.is_empty()
    }
    fn one_liner(&self) -> bool {
        if self.dedent == usize::MAX {
            debug_assert!(!self.slice.contains('\n'), "one_liner contains newline");
            true
        } else {
            debug_assert!(self.slice.contains('\n'), "missing newline in !one_liner");
            false
        }
    }
    /// Returned iterator produces a sub-slice for each line, stripped of indentation
    /// and line separators, in order, from `self`. Always produces at least one line.
    fn lines(&self) -> impl Iterator<Item = &'a str> {
        // that return type is tricky to satisfy: having two branches here (one
        // optimized for absent indentation) causes E0308 incompatible types:
        //   "distinct uses of `impl Trait` result in different opaque types"
        // attempting to hide them behind closures does not help either:
        //   "no two closures, even if identical, have the same type"
        let d = if self.one_liner() { 0 } else { self.dedent };
        self.slice
            .split('\n')
            .enumerate()
            .map(move |(i, s)| if i == 0 || d == 0 { s } else { &s[d..] })
    }
}

// ====================================================================================

/// Metadata about an [Item], [Name] or [File].
///
/// A serialized [Comment] will start with one of three possible markers, depending
/// on its position:
///  + `#!` for [File::hashbang],
///  + `//` for [Name::before].
///  + `#` for the various `prolog` and `epilog` fields,
///
/// The content is UTF-8 Github Flavored Markdown and kept in the encoded form.
///
/// A field within the [Item] or File will hold the Comment, there is no mechanism to
/// navigate from a Comment to the thing it describes.
///
/// # Examples
///
/// ```
/// # #[cfg(feature="alloc")]
/// # {
/// let comment = tindalwic::Comment::wrap("with ~strikethrough~ extension");
///
/// let html = markdown::to_html_with_options(&comment.joined(), &markdown::Options::gfm()).expect(
///     "should never error, according to:
///      <https://docs.rs/markdown/latest/markdown/fn.to_html_with_options.html#errors>",
/// );
///
/// assert_eq!(html, "<p>with <del>strikethrough</del> extension</p>");
/// # }
/// ```
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Comment<'a> {
    utf8: UTF8<'a>,
}
impl<'a> Comment<'a> {
    /// Return a zero-copy instance using the provided literal (not indented) slice.
    pub fn wrap(utf8: &'a str) -> Self {
        Comment {
            utf8: UTF8::wrap(utf8),
        }
    }
    /// wrap into [Option::Some].
    pub fn some(utf8: &'a str) -> Option<Self> {
        Some(Comment::wrap(utf8))
    }
    /// return true when there are zero UTF-8 bytes.
    pub fn is_empty(&self) -> bool {
        self.utf8.is_empty()
    }
    /// Returned iterator produces a sub-slice for each line, stripped of indentation
    /// and line separators, in order, from `self`. Always produces at least one line.
    pub fn lines(&self) -> impl Iterator<Item = &'a str> {
        self.utf8.lines()
    }
}

// ------------------------------------------------------------------------------------

/// [Item::Text] wraps a sequence of lines of UTF-8, and optional epilog comment.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Text<'a> {
    utf8: UTF8<'a>,
    /// A Text can have a Comment after it.
    pub epilog: Option<Comment<'a>>,
}
impl<'a> Text<'a> {
    /// Return a zero-copy instance using the provided literal (not indented) slice.
    pub fn wrap(utf8: &'a str) -> Self {
        Text {
            utf8: UTF8::wrap(utf8),
            epilog: None,
        }
    }
    fn one_liner_in_list(&self) -> bool {
        if !self.utf8.one_liner() {
            false
        } else if self.utf8.slice.is_empty() {
            true
        } else {
            !matches!(
                self.utf8.slice.as_bytes()[0],
                b'\t' | b'#' | b'<' | b'>' | b'[' | b']' | b'{' | b'}' | b'/' | b'='
            )
        }
    }
    fn one_liner_in_dict(&self, key: Key<'_>) -> bool {
        if !self.utf8.one_liner() {
            false
        } else if key.is_empty() {
            true
        } else if key.contains('=') {
            false
        } else {
            !matches!(
                key.as_bytes()[0],
                b'\t' | b'#' | b'<' | b'>' | b'[' | b']' | b'{' | b'}' | b'/' | b'='
            )
        }
    }
    /// return true when there are zero UTF-8 bytes.
    pub fn is_empty(&self) -> bool {
        self.utf8.is_empty()
    }
    /// return true when there is some comment or some UTF-8 bytes.
    pub fn has_content(&self) -> bool {
        !(self.is_empty() && self.epilog.is_none())
    }
    /// Returned iterator produces a sub-slice for each line, stripped of indentation
    /// and line separators, in order, from `self`. Always produces at least one line.
    pub fn lines(&self) -> impl Iterator<Item = &'a str> {
        self.utf8.lines()
    }
}

// ------------------------------------------------------------------------------------

/// [Item::List] wraps a sequence of `Cell<Item>`, and optional prolog and epilog comments.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct List<'a, 'store> {
    /// The contents of the Item::List.
    pub cells: &'store [Cell<Item<'a, 'store>>],
    /// A List can have an introductory Comment.
    pub prolog: Option<Comment<'a>>,
    /// A List can have a Comment after it.
    pub epilog: Option<Comment<'a>>,
}
impl<'a, 'store> List<'a, 'store> {
    /// Return a zero-copy instance using the provided cells (and no comments).
    pub fn wrap(cells: &'store [Cell<Item<'a, 'store>>]) -> Self {
        List {
            cells,
            prolog: None,
            epilog: None,
        }
    }
    /// return number of items.
    pub fn len(&self) -> usize {
        self.cells.len()
    }
    /// return true when there are zero items.
    pub fn is_empty(&self) -> bool {
        self.cells.is_empty()
    }
    /// return true when there is some comment or some items.
    pub fn has_content(&self) -> bool {
        !(self.is_empty() && self.prolog.is_none() && self.epilog.is_none())
    }
    /// return None if index is out of bounds, else Some(item at that index).
    /// same as Self::get, provided for parity with Dict::at and File::at
    pub fn at(&self, index: usize) -> Option<Item<'a, 'store>> {
        self.cells.get(index).map(Cell::get)
    }
    /// iterate over each item.
    pub fn iter(&self) -> impl Iterator<Item = Item<'a, 'store>> {
        self.cells.iter().map(Cell::get)
    }
    /// returns Option of the item at the given index.
    pub fn get(&self, index: usize) -> Option<Item<'a, 'store>> {
        self.at(index)
    }
}
impl<'a, 'store> IntoIterator for List<'a, 'store> {
    type Item = Item<'a, 'store>;
    type IntoIter = internals::CellIter<'store, Self::Item>;
    fn into_iter(self) -> Self::IntoIter {
        internals::CellIter {
            inner: self.cells.iter(),
        }
    }
}
impl<'a, 'store> IntoIterator for &List<'a, 'store> {
    type Item = Item<'a, 'store>;
    type IntoIter = internals::CellIter<'store, Self::Item>;
    fn into_iter(self) -> Self::IntoIter {
        internals::CellIter {
            inner: self.cells.iter(),
        }
    }
}

// ------------------------------------------------------------------------------------

/// the part of an association inside a Dict that is used for lookup.
pub type Key<'a> = &'a str;

/// aggregates the Key with the metadata.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Name<'a> {
    /// a key can have a blank line before it (before its comment)
    pub gap: bool,
    /// a key can have a comment before it (after its blank line).
    pub before: Option<Comment<'a>>,
    /// the key being associated to an [Item].
    pub key: Key<'a>,
}

/// an association (from name.key to item) including metadata.
///
/// at the lowest level, these are stored in an array. TODO a Map view can be
/// built (if the `alloc` feature is enabled).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Entry<'a, 'store> {
    /// the name given to the [Item].
    pub name: Name<'a>,
    /// the item associated to the [Name]
    pub item: Item<'a, 'store>,
}
impl<'a, 'store> Entry<'a, 'store> {
    fn blank<'b>(_: usize) -> Cell<Entry<'b, 'store>> {
        Cell::new(Entry {
            name: Name {
                key: "",
                gap: false,
                before: None,
            },
            item: Item::Text(Text {
                utf8: UTF8 {
                    slice: "",
                    dedent: usize::MAX,
                },
                epilog: None,
            }),
        })
    }
    /// Make a fixed-size array of cells on the stack.
    pub fn array<const N: usize>() -> [Cell<Entry<'a, 'store>>; N] {
        ::core::array::from_fn::<_, N, _>(Entry::blank)
    }
    /// convert a key and an item into an entry for a Dict.
    pub fn wrap(key: Key<'a>, item: Item<'a, 'store>) -> Self {
        Entry {
            name: Name {
                key,
                gap: false,
                before: None,
            },
            item,
        }
    }
    fn position(cells: &'store [Cell<Entry<'a, 'store>>], key: Key<'_>) -> Option<usize> {
        cells.iter().position(|x| x.get().name.key == key)
    }
}

/// [Item::Dict] wraps a sequence of `Cell<Entry>`, and optional prolog and epilog comments.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Dict<'a, 'store> {
    /// The contents of the Item::Dict.
    pub cells: &'store [Cell<Entry<'a, 'store>>],
    /// A Dict can have an introductory Comment.
    pub prolog: Option<Comment<'a>>,
    /// A Dict can have a Comment after it.
    pub epilog: Option<Comment<'a>>,
}
impl<'a, 'store> Dict<'a, 'store> {
    /// Return a zero-copy instance using the provided cells (and no comments).
    pub fn wrap(cells: &'store [Cell<Entry<'a, 'store>>]) -> Self {
        Dict {
            cells,
            prolog: None,
            epilog: None,
        }
    }
    /// return number of entries.
    pub fn len(&self) -> usize {
        self.cells.len()
    }
    /// return true when there are zero entries.
    pub fn is_empty(&self) -> bool {
        self.cells.is_empty()
    }
    /// return true when there is some comment or some entries.
    pub fn has_content(&self) -> bool {
        !(self.is_empty() && self.prolog.is_none() && self.epilog.is_none())
    }
    /// return None if index is out of bounds, else Some(entry at that index).
    pub fn at(&self, index: usize) -> Option<Entry<'a, 'store>> {
        self.cells.get(index).map(Cell::get)
    }
    /// iterate over each entry.
    pub fn iter(&self) -> impl Iterator<Item = Entry<'a, 'store>> {
        self.cells.iter().map(Cell::get)
    }
    /// return Some(index of entry) of the first one matching the given key.
    pub fn position(&self, key: Key<'_>) -> Option<usize> {
        Entry::position(self.cells, key)
    }
    /// returns Option of the entry with the given key.
    pub fn get(&self, key: Key<'_>) -> Option<Entry<'a, 'store>> {
        Entry::position(self.cells, key).map(|i| self.cells[i].get())
    }
}
impl<'a, 'store> IntoIterator for Dict<'a, 'store> {
    type Item = Entry<'a, 'store>;
    type IntoIter = internals::CellIter<'store, Self::Item>;
    fn into_iter(self) -> Self::IntoIter {
        internals::CellIter {
            inner: self.cells.iter(),
        }
    }
}
impl<'a, 'store> IntoIterator for &Dict<'a, 'store> {
    type Item = Entry<'a, 'store>;
    type IntoIter = internals::CellIter<'store, Self::Item>;
    fn into_iter(self) -> Self::IntoIter {
        internals::CellIter {
            inner: self.cells.iter(),
        }
    }
}

// ------------------------------------------------------------------------------------

/// the three Item variants
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Item<'a, 'store> {
    /// a [Text] Item holds UTF-8 content
    Text(Text<'a>),
    /// a [List] Item is a linear array of [Item]
    List(List<'a, 'store>),
    /// a [Dict] Item is an associative array of [Entry]
    Dict(Dict<'a, 'store>),
}
impl<'a, 'store> Item<'a, 'store> {
    fn blank<'b>(_: usize) -> Cell<Item<'b, 'store>> {
        Cell::new(Item::Text(Text {
            utf8: UTF8 {
                slice: "",
                dedent: usize::MAX,
            },
            epilog: None,
        }))
    }
    /// Make a fixed-size array of cells on the stack.
    pub fn array<const N: usize>() -> [Cell<Item<'a, 'store>>; N] {
        ::core::array::from_fn::<_, N, _>(Item::blank)
    }
}
impl<'a, 'store> From<Text<'a>> for Item<'a, 'store> {
    fn from(value: Text<'a>) -> Self {
        Item::Text(value)
    }
}
impl<'a, 'store> From<List<'a, 'store>> for Item<'a, 'store> {
    fn from(value: List<'a, 'store>) -> Self {
        Item::List(value)
    }
}
impl<'a, 'store> From<Dict<'a, 'store>> for Item<'a, 'store> {
    fn from(value: Dict<'a, 'store>) -> Self {
        Item::Dict(value)
    }
}

// ------------------------------------------------------------------------------------

/// the outermost context.
///
/// similar to a [Item::Dict], but with different comments.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct File<'a, 'store> {
    /// The contents of the Item::File.
    pub cells: &'store [Cell<Entry<'a, 'store>>],
    /// A File can start with a Unix `#!` Comment.
    pub hashbang: Option<Comment<'a>>,
    /// A File can have an introductory Comment.
    pub prolog: Option<Comment<'a>>,
}
impl<'a, 'store> File<'a, 'store> {
    /// Return a zero-copy instance using the provided cells (and no comments).
    pub fn wrap(cells: &'store [Cell<Entry<'a, 'store>>]) -> Self {
        File {
            cells,
            hashbang: None,
            prolog: None,
        }
    }
    /// return number of entries.
    pub fn len(&self) -> usize {
        self.cells.len()
    }
    /// return true when there are zero entries.
    pub fn is_empty(&self) -> bool {
        self.cells.is_empty()
    }
    /// return true when there is some comment or some entries.
    pub fn has_content(&self) -> bool {
        !(self.is_empty() && self.hashbang.is_none() && self.prolog.is_none())
    }
    /// return None if index is out of bounds, else Some(entry at that index).
    pub fn at(&self, index: usize) -> Option<Entry<'a, 'store>> {
        self.cells.get(index).map(Cell::get)
    }
    /// iterate over each entry.
    pub fn iter(&self) -> impl Iterator<Item = Entry<'a, 'store>> {
        self.cells.iter().map(Cell::get)
    }
    /// return Some(index of entry) of the first one matching the given key.
    pub fn position(&self, key: Key<'_>) -> Option<usize> {
        Entry::position(self.cells, key)
    }
    /// returns Option of the entry with the given key.
    pub fn get(&self, key: Key<'_>) -> Option<Entry<'a, 'store>> {
        Entry::position(self.cells, key).map(|i| self.cells[i].get())
    }
}
impl<'a, 'store> IntoIterator for File<'a, 'store> {
    type Item = Entry<'a, 'store>;
    type IntoIter = internals::CellIter<'store, Self::Item>;
    fn into_iter(self) -> Self::IntoIter {
        internals::CellIter {
            inner: self.cells.iter(),
        }
    }
}
impl<'a, 'store> IntoIterator for &File<'a, 'store> {
    type Item = Entry<'a, 'store>;
    type IntoIter = internals::CellIter<'store, Self::Item>;
    fn into_iter(self) -> Self::IntoIter {
        internals::CellIter {
            inner: self.cells.iter(),
        }
    }
}

// ====================================================================================

#[cfg(test)]
#[allow(unused_extern_crates)]
extern crate self as test_rename_of_tindalwic_dependency;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rename() {
        json! {
            $crate = test_rename_of_tindalwic_dependency;
            let empty = {}.unwrap();
            completed.unwrap();
        }
        assert!(empty.cells.is_empty());
    }
}
