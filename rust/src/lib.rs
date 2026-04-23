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

#[cfg(feature = "alloc")]
mod alloc;
mod fmt;
pub mod internals; // macro generated code needs access.
mod parse;

/// Hidden parts of [Comment] and [Text].
///
/// These are zero-copy slices from an external buffer of Tindalwic UTF-8. The iterator
/// returned by [Encoded::lines()] is the most efficient way to strip the indentation
/// from a multi-line slice.
#[derive(Clone, Copy, Debug)]
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
    /// return true when there are no UTF-8 bytes.
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
#[derive(Clone, Copy, Debug)]
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
    /// return true when there are no UTF-8 bytes.
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
#[derive(Clone, Copy, Debug)]
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
    /// return true when there are no UTF-8 bytes and no comments.
    pub fn is_empty(&self) -> bool {
        self.utf8.is_empty() && self.epilog.is_none()
    }
    /// Returned iterator produces a sub-slice for each line, stripped of indentation
    /// and line separators, in order, from `self`. Always produces at least one line.
    pub fn lines(&self) -> impl Iterator<Item = &'a str> {
        self.utf8.lines()
    }
}

// ------------------------------------------------------------------------------------

/// [Item::List] wraps a sequence of `Cell<Item>`, and optional prolog and epilog comments.
#[derive(Clone, Copy, Debug)]
pub struct List<'a> {
    /// The contents of the Item::List.
    pub cells: &'a [Cell<Item<'a>>],
    /// A List can have an introductory Comment.
    pub prolog: Option<Comment<'a>>,
    /// A List can have a Comment after it.
    pub epilog: Option<Comment<'a>>,
}
impl<'a> List<'a> {
    /// Return a zero-copy instance using the provided cells (and no comments).
    pub fn wrap(cells: &'a [Cell<Item<'a>>]) -> Self {
        List {
            cells,
            prolog: None,
            epilog: None,
        }
    }
    /// return true when there are no items and no comments.
    pub fn is_empty(&self) -> bool {
        self.cells.is_empty() && self.prolog.is_none() && self.epilog.is_none()
    }
}

// ------------------------------------------------------------------------------------

/// the part of an association inside a Dict that is used for lookup.
pub type Key<'a> = &'a str;

/// aggregates the Key with the metadata.
#[derive(Clone, Copy, Debug)]
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
#[derive(Clone, Copy, Debug)]
pub struct Entry<'a> {
    /// the name given to the [Item].
    pub name: Name<'a>,
    /// the item associated to the [Name]
    pub item: Item<'a>,
}
impl<'a> Entry<'a> {
    fn blank<'b>(_: usize) -> Cell<Entry<'b>> {
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
    pub fn array<const N: usize>() -> [Cell<Entry<'a>>; N] {
        ::core::array::from_fn::<_, N, _>(Entry::blank)
    }
    /// convert a key and an item into an entry for a Dict.
    pub fn wrap(key: Key<'a>, item: Item<'a>) -> Self {
        Entry {
            name: Name {
                key,
                gap: false,
                before: None,
            },
            item,
        }
    }
    fn position(cells: &'a [Cell<Entry<'a>>], key: Key<'_>) -> Option<usize> {
        cells.iter().position(|x| x.get().name.key == key)
    }
}

/// [Item::Dict] wraps a sequence of `Cell<Entry>`, and optional prolog and epilog comments.
#[derive(Clone, Copy, Debug)]
pub struct Dict<'a> {
    /// The contents of the Item::Dict.
    pub cells: &'a [Cell<Entry<'a>>],
    /// A Dict can have an introductory Comment.
    pub prolog: Option<Comment<'a>>,
    /// A Dict can have a Comment after it.
    pub epilog: Option<Comment<'a>>,
}
impl<'a> Dict<'a> {
    /// Return a zero-copy instance using the provided cells (and no comments).
    pub fn wrap(cells: &'a [Cell<Entry<'a>>]) -> Self {
        Dict {
            cells,
            prolog: None,
            epilog: None,
        }
    }
    /// return true when there are no entries and no comments.
    pub fn is_empty(&self) -> bool {
        self.cells.is_empty() && self.prolog.is_none() && self.epilog.is_none()
    }
    /// returns a reference to the entry with the given key.
    pub fn find(&self, key: Key<'_>) -> Option<&'a Cell<Entry<'a>>> {
        Entry::position(self.cells, key).map(|i| &self.cells[i])
    }
}

// ------------------------------------------------------------------------------------

/// the three Item variants
#[derive(Clone, Copy, Debug)]
pub enum Item<'a> {
    /// a [Text] Item holds UTF-8 content
    Text(Text<'a>),
    /// a [List] Item is a linear array of [Item]
    List(List<'a>),
    /// a [Dict] Item is an associative array of [Entry]
    Dict(Dict<'a>),
}
impl<'a> Item<'a> {
    fn blank<'b>(_: usize) -> Cell<Item<'b>> {
        Cell::new(Item::Text(Text {
            utf8: UTF8 {
                slice: "",
                dedent: usize::MAX,
            },
            epilog: None,
        }))
    }
    /// Make a fixed-size array of cells on the stack.
    pub fn array<const N: usize>() -> [Cell<Item<'a>>; N] {
        ::core::array::from_fn::<_, N, _>(Item::blank)
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
#[derive(Clone, Copy, Debug)]
pub struct File<'a> {
    /// The contents of the Item::File.
    pub cells: &'a [Cell<Entry<'a>>],
    /// A File can start with a Unix `#!` Comment.
    pub hashbang: Option<Comment<'a>>,
    /// A File can have an introductory Comment.
    pub prolog: Option<Comment<'a>>,
}
impl<'a> File<'a> {
    /// stub for decoding from tindalwic UTF-8
    pub fn parse<F>(content: &'a str, then: F)
    where
        F: FnOnce(File),
    {
        let items = Item::array::<15>();
        let entries = Entry::array::<15>();
        let mut arena = internals::Arena::wrap(&items, &entries);
        let file = parse::Input::parse(&mut arena, content, |(line, message)| {
            panic!("{line}:{message}")
        });
        (then)(file.unwrap());
    }
    /// Return a zero-copy instance using the provided cells (and no comments).
    pub fn wrap(cells: &'a [Cell<Entry<'a>>]) -> Self {
        File {
            cells,
            hashbang: None,
            prolog: None,
        }
    }
    /// return true when there are no entries and no comments.
    pub fn is_empty(&self) -> bool {
        self.cells.is_empty() && self.hashbang.is_none() && self.prolog.is_none()
    }
    /// returns a reference to the entry with the given key.
    pub fn find(&self, key: Key<'_>) -> Option<&'a Cell<Entry<'a>>> {
        Entry::position(self.cells, key).map(|i| &self.cells[i])
    }
}

// ====================================================================================
