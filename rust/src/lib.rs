#![no_std]

//! Text in Nested Dictionaries and Lists - with Important Comments

use core::cell::Cell;

#[doc(inline)]
/// main module re-exports the proc_macro from sub-crate.
pub use tindalwic_macros::json;
#[doc(inline)]
/// main module re-exports the proc_macro from sub-crate.
pub use tindalwic_macros::set;
#[doc(inline)]
/// main module re-exports the proc_macro from sub-crate.
pub use tindalwic_macros::walk;

#[cfg(feature = "alloc")]
mod alloc;
pub mod internals; // macro generated code needs access.
mod fmt;
mod parse;

/// Hidden parts of [Comment] and [Text].
///
/// These are zero-copy slices from an external buffer of Tindalwic UTF-8. The iterator
/// returned by [Encoded::lines()] is the most efficient way to strip the indentation
/// from a multi-line slice.
#[derive(Clone, Copy, Debug)]
struct Encoded<'a> {
    utf8: &'a str,
    dedent: usize, // usize::MAX => one_liner
}
impl<'a> Encoded<'a> {
    /// Return a zero-copy instance using the provided literal (not indented) slice.
    pub fn wrap(utf8: &'a str) -> Self {
        let bytes = utf8.as_bytes();
        let mut newline = 0usize;
        while newline < bytes.len() && bytes[newline] != b'\n' {
            newline += 1;
        }
        Encoded {
            utf8,
            dedent: if newline < bytes.len() { 0 } else { usize::MAX },
        }
    }
    fn one_liner(&self) -> bool {
        if self.dedent == usize::MAX {
            debug_assert!(!self.utf8.contains('\n'), "one_liner contains newline");
            true
        } else {
            debug_assert!(self.utf8.contains('\n'), "missing newline in !one_liner");
            false
        }
    }
    /// Returned iterator produces a sub-slice for each line, stripped of indentation
    /// and line separators, in order, from `self`. Always produces at least one line.
    pub fn lines(&self) -> impl Iterator<Item = &'a str> {
        // that return type is tricky to satisfy: having two branches here (one
        // optimized for absent indentation) causes E0308 incompatible types:
        //   "distinct uses of `impl Trait` result in different opaque types"
        // attempting to hide them behind closures does not help either:
        //   "no two closures, even if identical, have the same type"
        let d = if self.one_liner() { 0 } else { self.dedent };
        self.utf8
            .split('\n')
            .enumerate()
            .map(move |(i, s)| if i == 0 || d == 0 { s } else { &s[d..] })
    }
}

// ====================================================================================

/// Metadata about a Value or a File.
///
/// A serialized Comment will start with one of three possible markers, depending
/// on its position:
///  + `#!` for the "hashbang" of a File,
///  + `#` for all prolog and epilog comments,
///  + `//` before (and about) the keys in a dictionary.
///
/// The content is UTF-8 Github Flavored Markdown and kept in the serialized form.
///
/// A field within the Value or File will hold the Comment, there is no mechanism to
/// navigate from a Comment to the thing it describes.
///
/// # Examples
///
/// ```
/// let comment = tindalwic::Comment::wrap("with ~strikethrough~ extension");
///
/// let html = markdown::to_html_with_options(&comment.joined(), &markdown::Options::gfm())
///   .expect("should never error, according to:
///      <https://docs.rs/markdown/latest/markdown/fn.to_html_with_options.html#errors>");
///
/// assert_eq!(html, "<p>with <del>strikethrough</del> extension</p>");
/// ```
#[derive(Clone, Copy, Debug)]
pub struct Comment<'a> {
    encoded: Encoded<'a>,
}
impl<'a> Comment<'a> {
    /// Return a zero-copy instance using the provided literal (not indented) slice.
    pub fn wrap(utf8: &'a str) -> Self {
        Comment {
            encoded: Encoded::wrap(utf8),
        }
    }
    /// instantiate into [Option::Some].
    pub fn some(utf8: &'a str) -> Option<Self> {
        Some(Comment::wrap(utf8))
    }
    /// Returned iterator produces a sub-slice for each line, stripped of indentation
    /// and line separators, in order, from `self`. Always produces at least one line.
    pub fn lines(&self) -> impl Iterator<Item = &'a str> {
        self.encoded.lines()
    }
}


// ------------------------------------------------------------------------------------

/// the fields of a [Value::Text]
#[derive(Clone, Copy, Debug)]
pub struct Text<'a> {
    encoded: Encoded<'a>,
    /// A Text can have a Comment after it.
    pub epilog: Option<Comment<'a>>,
}
impl<'a> Text<'a> {
    /// Return a zero-copy instance using the provided literal (not indented) slice.
    pub fn wrap(utf8: &'a str) -> Self {
        Text {
            encoded: Encoded::wrap(utf8),
            epilog: None,
        }
    }
    fn one_liner_in_list(&self) -> bool {
        if !self.encoded.one_liner() {
            false
        } else if self.encoded.utf8.is_empty() {
            true
        } else {
            !matches!(
                self.encoded.utf8.as_bytes()[0],
                b'\t' | b'#' | b'<' | b'>' | b'[' | b']' | b'{' | b'}' | b'/' | b'='
            )
        }
    }
    fn one_liner_in_dict(&self, key: &str) -> bool {
        if !self.encoded.one_liner() {
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
    /// Returned iterator produces a sub-slice for each line, stripped of indentation
    /// and line separators, in order, from `self`. Always produces at least one line.
    pub fn lines(&self) -> impl Iterator<Item = &'a str> {
        self.encoded.lines()
    }
}

// ------------------------------------------------------------------------------------

/// the fields of a [Value::List]
#[derive(Clone, Copy, Debug)]
pub struct List<'a> {
    /// The contents of the Value::List.
    pub cells: &'a [Cell<Value<'a>>],
    /// A List can have an introductory Comment.
    pub prolog: Option<Comment<'a>>,
    /// A List can have a Comment after it.
    pub epilog: Option<Comment<'a>>,
}
impl<'a> List<'a> {
    /// Return a zero-copy instance using the provided cells (and no comments).
    pub fn wrap(cells: &'a [Cell<Value<'a>>]) -> Self {
        List {
            cells,
            prolog: None,
            epilog: None,
        }
    }
}

// ------------------------------------------------------------------------------------

/// an association.
///
/// these are stored in an array (instead of using a hash table).
#[derive(Clone, Copy, Debug)]
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
    fn blank<'b>(_: usize) -> Cell<Keyed<'b>> {
        Cell::new(Keyed {
            key: "",
            gap: false,
            before: None,
            value: Value::Text(Text {
                encoded: Encoded {
                    utf8: "",
                    dedent: usize::MAX,
                },
                epilog: None,
            }),
        })
    }
    /// Make a fixed-size array of cells on the stack.
    pub fn array<const N: usize>() -> [Cell<Keyed<'a>>; N] {
        ::core::array::from_fn::<_, N, _>(Keyed::blank)
    }
    /// convert a key and a value into an entry (for a Dict).
    pub fn from(key: &'a str, value: Value<'a>) -> Self {
        Keyed {
            key,
            gap: false,
            before: None,
            value,
        }
    }
}

/// the fields of a [Value::Dict]
#[derive(Clone, Copy, Debug)]
pub struct Dict<'a> {
    /// The contents of the Value::Dict.
    pub cells: &'a [Cell<Keyed<'a>>],
    /// A Dict can have an introductory Comment.
    pub prolog: Option<Comment<'a>>,
    /// A Dict can have a Comment after it.
    pub epilog: Option<Comment<'a>>,
}
fn position<'a>(dict: &'a [Cell<Keyed<'a>>], key: &str) -> Option<usize> {
    dict.iter().position(|x| x.get().key == key)
}
impl<'a> Dict<'a> {
    /// Return a zero-copy instance using the provided cells (and no comments).
    pub fn wrap(cells: &'a [Cell<Keyed<'a>>]) -> Self {
        Dict {
            cells,
            prolog: None,
            epilog: None,
        }
    }
    /// returns the position of the entry with the given key.
    pub fn position(&self, key: &str) -> Option<usize> {
        position(self.cells, key)
    }
    /// returns a reference to the entry with the given key.
    pub fn find(&self, key: &str) -> Option<&'a Cell<Keyed<'a>>> {
        self.position(key).map(|i| &self.cells[i])
    }
}

// ------------------------------------------------------------------------------------

/// the three possible Value types
#[derive(Clone, Copy, Debug)]
pub enum Value<'a> {
    /// a [Text] value holds UTF-8 content
    Text(Text<'a>),
    /// a [List] value is a linear array of values
    List(List<'a>),
    /// a [Dict] value is an associative array of Keyed values
    Dict(Dict<'a>),
}
impl<'a> Value<'a> {
    fn blank<'b>(_: usize) -> Cell<Value<'b>> {
        Cell::new(Value::Text(Text {
            encoded: Encoded {
                utf8: "",
                dedent: usize::MAX,
            },
            epilog: None,
        }))
    }
    /// Make a fixed-size array of cells on the stack.
    pub fn array<const N: usize>() -> [Cell<Value<'a>>; N] {
        ::core::array::from_fn::<_, N, _>(Value::blank)
    }
}
impl<'a> From<Text<'a>> for Value<'a> {
    fn from(value: Text<'a>) -> Self {
        Value::Text(value)
    }
}
impl<'a> From<List<'a>> for Value<'a> {
    fn from(value: List<'a>) -> Self {
        Value::List(value)
    }
}
impl<'a> From<Dict<'a>> for Value<'a> {
    fn from(value: Dict<'a>) -> Self {
        Value::Dict(value)
    }
}
impl<'a> From<File<'a>> for Value<'a> {
    fn from(value: File<'a>) -> Self {
        Value::Dict(Dict::wrap(value.cells))
    }
}

// ------------------------------------------------------------------------------------

/// the outermost context.
///
/// very similar to a [Value::Dict], just with different comments.
#[derive(Clone, Copy, Debug)]
pub struct File<'a> {
    /// The contents of the Value::File.
    pub cells: &'a [Cell<Keyed<'a>>],
    /// A File can start with a Unix `#!` Comment.
    pub hashbang: Option<Comment<'a>>,
    /// A File can have an introductory Comment.
    pub prolog: Option<Comment<'a>>,
}
impl<'a> File<'a> {
    /// stub for decoding from tindalwic UTF-8
    pub fn parse(_content: &'a str) -> Self {
        todo!()
    }
    /// Return a zero-copy instance using the provided cells (and no comments).
    pub fn wrap(cells: &'a [Cell<Keyed<'a>>]) -> Self {
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
    /// returns the position of the entry with the given key.
    pub fn position(&self, key: &str) -> Option<usize> {
        position(self.cells, key)
    }
    /// returns a reference to the entry with the given key.
    pub fn find(&self, key: &str) -> Option<&'a Cell<Keyed<'a>>> {
        self.position(key).map(|i| &self.cells[i])
    }
}

// ====================================================================================
