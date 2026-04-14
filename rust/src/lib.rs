#![no_std]
#![allow(missing_docs, unused)]

//! Text in Nested Dictionaries and Lists - with Important Comments

/*
extern crate alloc;
use alloc::string::String;
*/
use core::cell::Cell;
use core::fmt::{self, Debug, Display, Formatter, Write};
use core::ops::{Deref, DerefMut, Range};
use core::sync::atomic;

#[doc(inline)]
/// main module re-exports the proc_macro from sub-crate.
pub use tindalwic_macros::json;
#[doc(inline)]
/// main module re-exports the proc_macro from sub-crate.
pub use tindalwic_macros::walk;

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
    pub const fn wrap(utf8: &'a str) -> Self {
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
    pub fn assign(&mut self, utf8: &'a str) {
        let wrap = Encoded::wrap(utf8);
        self.utf8 = wrap.utf8;
        self.dedent = wrap.dedent;
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
    /*
    /// Allocates a [String], filled with the UTF-8 copied from `self`.
    pub fn lines_joined(&self) -> String {
        let mut result = String::with_capacity(self.utf8.len());
        for line in self.lines() {
            result.push_str(line);
            result.push('\n');
        }
        if !result.is_empty() {
            result.truncate(result.len() - 1);
        }
        result
    }
    */
}

macro_rules! cell_helpers {
    ($Name:ident, $NameInList:ident, $NameInDict:ident) => {
        pub struct $NameInList<'a> {
            pub value: $Name<'a>,
            pub cell: &'a Cell<Value<'a>>,
        }
        impl<'a> Deref for $NameInList<'a> {
            type Target = $Name<'a>;
            fn deref(&self) -> &Self::Target {
                &self.value
            }
        }
        impl<'a> DerefMut for $NameInList<'a> {
            fn deref_mut(&mut self) -> &mut Self::Target {
                &mut self.value
            }
        }
        impl<'a> $NameInList<'a> {
            pub fn from(cell: &'a Cell<Value<'a>>) -> Option<Self> {
                if let Value::$Name(value) = cell.get() {
                    Some($NameInList { value, cell })
                } else {
                    None
                }
            }
            pub fn persist(&self) {
                self.cell.set(Value::$Name(self.value))
            }
        }
        pub struct $NameInDict<'a> {
            pub value: $Name<'a>,
            pub cell: &'a Cell<Keyed<'a>>,
        }
        impl<'a> Deref for $NameInDict<'a> {
            type Target = $Name<'a>;
            fn deref(&self) -> &Self::Target {
                &self.value
            }
        }
        impl<'a> DerefMut for $NameInDict<'a> {
            fn deref_mut(&mut self) -> &mut Self::Target {
                &mut self.value
            }
        }
        impl<'a> $NameInDict<'a> {
            pub fn from(cell: &'a Cell<Keyed<'a>>) -> Option<Self> {
                let keyed = cell.get();
                if let Value::$Name(value) = keyed.value {
                    Some($NameInDict { value, cell })
                } else {
                    None
                }
            }
            // pub fn persist(&self) {
            //     self.cell.set(Value::$Name(self.value))
            // }
        }
    };
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
/// ``
/// let comment = tindalwic::Comment::wrap("with ~strikethrough~ extension");
///
/// let html = markdown::to_html_with_options(&comment.lines_joined(), &markdown::Options::gfm())
///   .expect("should never error, according to:
///      https://docs.rs/markdown/latest/markdown/fn.to_html_with_options.html#errors");
///
/// assert_eq!(html, "<p>with <del>strikethrough</del> extension</p>");
/// ``
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
    pub fn assign(&mut self, utf8: &'a str) {
        self.encoded.assign(utf8);
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
    /*
    /// Allocates a [String], filled with the UTF-8 copied from `self`.
    pub fn lines_joined(&self) -> String {
        self.encoded.lines_joined()
    }
     */
}
/// Serialize using the "#" marker (ignoring any actual position).
///
/// # Examples
///
/// ```
/// fn check(gfm: &str) {
///     let expected = format!("#{}\n", gfm.replace("\n", "\n\t"));
///     assert_eq!(tindalwic::Comment::wrap(gfm).to_string(), expected);
/// }
/// check("one-liner");
/// check("two\nlines");
/// check(
///     "
/// # heading
/// paragraph
///     blockquote
/// ",
/// );
/// ```
impl<'a> Display for Comment<'a> {
    fn fmt(&self, out: &mut Formatter<'_>) -> fmt::Result {
        Output { out, indent: 0 }.some_comment("#", self)
    }
}

// ------------------------------------------------------------------------------------

cell_helpers! {Text,TextInList,TextInDict}
/// the fields of a [Value::Text]
#[derive(Clone, Copy, Debug)]
pub struct Text<'a> {
    encoded: Encoded<'a>,
    /// A Text can have a Comment after it.
    pub epilog: Option<Comment<'a>>,
}
impl<'a> Display for Text<'a> {
    fn fmt(&self, out: &mut Formatter<'_>) -> fmt::Result {
        Output { out, indent: 0 }.text_in_list(self)
    }
}
impl<'a> Text<'a> {
    /// Return a zero-copy instance using the provided literal (not indented) slice.
    pub const fn wrap(utf8: &'a str) -> Value<'a> {
        Value::Text(Text {
            encoded: Encoded::wrap(utf8),
            epilog: None,
        })
    }
    pub fn assign(&mut self, utf8: &'a str) {
        self.encoded.assign(utf8);
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
    /*
    /// Allocates a [String], filled with the UTF-8 copied from `self`.
    pub fn lines_joined(&self) -> String {
        self.encoded.lines_joined()
    }
    */
    pub fn to_value(&self) -> Value<'a> {
        Value::Text(self.clone())
    }
}

// ------------------------------------------------------------------------------------

cell_helpers! {List,ListInList,ListInDict}
/// the fields of a [Value::List]
#[derive(Clone, Copy, Debug)]
pub struct List<'a> {
    /// The contents of the Value::List.
    pub list: &'a [Cell<Value<'a>>],
    /// A List can have an introductory Comment.
    pub prolog: Option<Comment<'a>>,
    /// A List can have a Comment after it.
    pub epilog: Option<Comment<'a>>,
}
impl<'a> Display for List<'a> {
    fn fmt(&self, out: &mut Formatter<'_>) -> fmt::Result {
        Output { out, indent: 0 }.list_in_list(self)
    }
}
impl<'a> List<'a> {
    pub const fn wrap(list: &'a [Cell<Value<'a>>]) -> Value<'a> {
        Value::List(List {
            list,
            prolog: None,
            epilog: None,
        })
    }
    pub fn to_value(&self) -> Value<'a> {
        Value::List(self.clone())
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
    pub fn blank<'b>(_: usize) -> Cell<Keyed<'b>> {
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
    pub fn array<const N: usize>() -> [Cell<Keyed<'a>>; N] {
        ::core::array::from_fn::<_, N, _>(Keyed::blank)
    }
    /// convert a key and a value into an entry (for a Dict).
    pub const fn from(key: &'a str, value: Value<'a>) -> Self {
        Keyed {
            key,
            gap: false,
            before: None,
            value,
        }
    }
}

cell_helpers! {Dict,DictInList,DictInDict}
/// the fields of a [Value::Dict]
#[derive(Clone, Copy, Debug)]
pub struct Dict<'a> {
    /// The contents of the Value::Dict.
    pub dict: &'a [Cell<Keyed<'a>>],
    /// A Dict can have an introductory Comment.
    pub prolog: Option<Comment<'a>>,
    /// A Dict can have a Comment after it.
    pub epilog: Option<Comment<'a>>,
}
impl<'a> Display for Dict<'a> {
    fn fmt(&self, out: &mut Formatter<'_>) -> fmt::Result {
        Output { out, indent: 0 }.dict_in_list(self)
    }
}
fn position<'a>(dict: &'a [Cell<Keyed<'a>>], key: &str) -> Option<usize> {
    dict.iter().position(|x| x.get().key == key)
}
impl<'a> Dict<'a> {
    pub fn wrap(dict: &'a [Cell<Keyed<'a>>]) -> Value<'a> {
        Value::Dict(Dict {
            dict,
            prolog: None,
            epilog: None,
        })
    }
    /// returns the position of the entry with the given key.
    pub fn position(&self, key: &str) -> Option<usize> {
        position(self.dict, key)
    }
    /// returns a reference to the entry with the given key.
    pub fn find(&self, key: &str) -> Option<&'a Cell<Keyed<'a>>> {
        self.position(key).map(|i| &self.dict[i])
    }
    pub fn to_value(&self) -> Value<'a> {
        Value::Dict(self.clone())
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
impl<'a> Display for Value<'a> {
    fn fmt(&self, out: &mut Formatter<'_>) -> fmt::Result {
        let mut out = Output { out, indent: 0 };
        match self {
            Value::Text(text) => out.text_in_list(text),
            Value::List(list) => out.list_in_list(list),
            Value::Dict(dict) => out.dict_in_list(dict),
        }
    }
}
impl<'a> Value<'a> {
    pub fn blank<'b>(_: usize) -> Cell<Value<'b>> {
        Cell::new(Value::Text(Text {
            encoded: Encoded {
                utf8: "",
                dedent: usize::MAX,
            },
            epilog: None,
        }))
    }
    pub fn array<const N: usize>() -> [Cell<Value<'a>>; N] {
        ::core::array::from_fn::<_, N, _>(Value::blank)
    }
    pub fn to_value(self) -> Self {
        self
    }
}

// ------------------------------------------------------------------------------------

/// the outermost context.
///
/// very similar to a [Value::Dict], just with different comments.
#[derive(Clone, Copy, Debug)]
pub struct File<'a> {
    /// The contents of the Value::File.
    pub dict: &'a [Cell<Keyed<'a>>],
    /// A File can start with a Unix `#!` Comment.
    pub hashbang: Option<Comment<'a>>,
    /// A File can have an introductory Comment.
    pub prolog: Option<Comment<'a>>,
}
impl<'a> Display for File<'a> {
    fn fmt(&self, out: &mut Formatter<'_>) -> fmt::Result {
        Output { out, indent: 0 }.file(self)
    }
}
impl<'a> File<'a> {
    pub fn new(dict: &'a [Cell<Keyed<'a>>]) -> Self {
        File {
            dict,
            hashbang: None,
            prolog: None,
        }
    }
    /// return true when there are no entries and no comments.
    pub fn is_empty(&self) -> bool {
        self.dict.is_empty() && self.hashbang.is_none() && self.prolog.is_none()
    }
    /// returns the position of the entry with the given key.
    pub fn position(&self, key: &str) -> Option<usize> {
        position(self.dict, key)
    }
    /// returns a reference to the entry with the given key.
    pub fn find(&self, key: &str) -> Option<&'a Cell<Keyed<'a>>> {
        self.position(key).map(|i| &self.dict[i])
    }
    pub fn to_value(&self) -> Value<'a> {
        Dict::wrap(self.dict)
    }
}

// ====================================================================================

/// support for the macro. public so macro can use it, but think of it as hidden.
pub struct Arena<'a> {
    value_cells: &'a [Cell<Value<'a>>],
    keyed_cells: &'a [Cell<Keyed<'a>>],
    value_next: usize,
    keyed_next: usize,
}
impl<'a> Arena<'a> {
    pub fn new(value_cells: &'a [Cell<Value<'a>>], keyed_cells: &'a [Cell<Keyed<'a>>]) -> Self {
        Arena { value_cells, keyed_cells, value_next: 0, keyed_next: 0 }
    }
    pub fn value_in_list(&mut self, value: Value<'a>) {
        self.value_cells[self.value_next].set(value);
        self.value_next += 1;
    }
    pub fn text_in_list(&mut self, utf8: &'a str) {
        self.value_in_list(Text::wrap(utf8));
    }
    pub fn list_in_list(&mut self, list: Range<usize>) {
        self.value_in_list(List::wrap(&self.value_cells[list]));
    }
    pub fn dict_in_list(&mut self, dict: Range<usize>) {
        self.value_in_list(Dict::wrap(&self.keyed_cells[dict]));
    }
    pub fn value_in_dict(&mut self, key: &'a str, value: Value<'a>) {
        self.keyed_cells[self.keyed_next].set(Keyed::from(key, value));
        self.keyed_next += 1;
    }
    pub fn text_in_dict(&mut self, key: &'a str, utf8: &'a str) {
        self.value_in_dict(key, Text::wrap(utf8));
    }
    pub fn list_in_dict(&mut self, key: &'a str, list: Range<usize>) {
        self.value_in_dict(key, List::wrap(&self.value_cells[list]));
    }
    pub fn dict_in_dict(&mut self, key: &'a str, dict: Range<usize>) {
        self.value_in_dict(key, Dict::wrap(&self.keyed_cells[dict]));
    }
    pub fn end(&self) -> &'a Cell<Value<'a>> {
        &self.value_cells[self.value_next - 1]
    }
    pub fn value(&self) -> Value<'a> {
        self.end().get()
    }
    pub fn text(&self) -> Option<Text<'a>> {
        if let Value::Text(text) = self.end().get() {
            Some(text)
        } else {
            None
        }
    }
    pub fn list(&self) -> Option<List<'a>> {
        if let Value::List(list) = self.end().get() {
            Some(list)
        } else {
            None
        }
    }
    pub fn dict(&self) -> Option<Dict<'a>> {
        if let Value::Dict(dict) = self.end().get() {
            Some(dict)
        } else {
            None
        }
    }
}

#[derive(Debug)]
pub enum Branch<'a> {
    List(usize),
    Dict(&'a str),
}
#[derive(Debug)]
pub struct Error<'a> {
    failed: &'a [Branch<'a>],
    message: &'static str,
}
#[derive(Debug)]
pub struct Path<'a> {
    pub branches: &'a [Branch<'a>],
}
impl<'a> Path<'a> {
    pub fn new(branches: &'a [Branch<'a>]) -> Self {
        Path { branches }
    }
    pub fn error_full(&'a self, message: &'static str) -> Error<'a> {
        Error { failed: &self.branches[..], message }
    }
    pub fn error(&'a self, bad:usize, message: &'static str) -> Error<'a> {
        Error { failed: &self.branches[..=bad], message }
    }
    pub fn text_value(&'a self, from: Value<'a>) -> Result<TextInList<'a>, Error<'a>> {
        TextInList::from(self.value(from)?).ok_or(self.error_full("path does not end at text"))
    }
    pub fn list_value(&'a self, from: Value<'a>)-> Result<ListInList<'a>, Error<'a>> {
        ListInList::from(self.value(from)?).ok_or(self.error_full("path does not end at list"))
    }
    pub fn dict_value(&'a self, from: Value<'a>)-> Result<DictInList<'a>, Error<'a>> {
        DictInList::from(self.value(from)?).ok_or(self.error_full("path does not end at dict"))
    }
    fn value(&'a self, mut from: Value<'a>) -> Result<&'a Cell<Value<'a>>, Error<'a>> {
        if self.branches.is_empty() {
            return Err(self.error_full("empty path can't be resolved"));
        }
        for (step, branch) in self.branches.iter().enumerate() {
            match &from {
                Value::Text(text) => {
                    return Err(self.error(step, "path ended prematurely by a text value"));
                }
                Value::List(list) => match branch {
                    Branch::List(at) => match list.list.get(*at) {
                        None => return Err(self.error(step, "index out of bounds")),
                        Some(found) => {
                            if step + 1 == self.branches.len() {
                                return Ok(found);
                            }
                            from = found.get();
                        }
                    },
                    Branch::Dict(_) => {
                        return Err(self.error(step, "path expected dict but found list"));
                    }
                },
                Value::Dict(dict) => match branch {
                    Branch::Dict(key) => {
                        match dict.find(key) {
                            None => return Err(self.error(step, "key not found")),
                            Some(found) => {
                                from = found.get().value;
                            }
                        };
                    }
                    Branch::List(_) => {
                        return Err(self.error(step, "path expected list but found dict"));
                    }
                },
            }
        }
        Err(self.error_full("path did not end at a value inside a list"))
    }
    pub fn text_keyed(&'a self, from: Value<'a>) -> Result<TextInDict<'a>, Error<'a>> {
        TextInDict::from(self.keyed(from)?).ok_or(self.error_full("path does not end at text"))
    }
    pub fn list_keyed(&'a self, from: Value<'a>)-> Result<ListInDict<'a>, Error<'a>> {
        ListInDict::from(self.keyed(from)?).ok_or(self.error_full("path does not end at list"))
    }
    pub fn dict_keyed(&'a self, from: Value<'a>)-> Result<DictInDict<'a>, Error<'a>> {
        DictInDict::from(self.keyed(from)?).ok_or(self.error_full("path does not end at dict"))
    }
    fn keyed(&'a self, mut from: Value<'a>) -> Result<&'a Cell<Keyed<'a>>, Error<'a>> {
        if self.branches.is_empty() {
            return Err(self.error_full("empty path can't be resolved"));
        }
        for (step, branch) in self.branches.iter().enumerate() {
            match &from {
                Value::Text(text) => {
                    return Err(self.error(step, "path ended prematurely by a text value"));
                }
                Value::List(list) => match branch {
                    Branch::List(at) => match list.list.get(*at) {
                        None => return Err(self.error(step, "index out of bounds")),
                        Some(found) => {
                            from = found.get();
                        }
                    },
                    Branch::Dict(_) => {
                        return Err(self.error(step, "path expected dict but found list"));
                    }
                },
                Value::Dict(dict) => match branch {
                    Branch::Dict(key) => {
                        match dict.find(key) {
                            None => return Err(self.error(step, "key not found")),
                            Some(found) => {
                                if step + 1 == self.branches.len() {
                                    return Ok(found);
                                }
                                from = found.get().value;
                            }
                        };
                    }
                    Branch::List(_) => {
                        return Err(self.error(step, "path expected list but found dict"));
                    }
                },
            }
        }
        Err(self.error_full("path did not end at a value inside a dict"))
    }
}

// ====================================================================================

struct Output<'o, 'f> {
    out: &'o mut Formatter<'f>,
    indent: usize,
}
impl<'o, 'f> Output<'o, 'f> {
    fn indent(&mut self) -> fmt::Result {
        for _ in 0..self.indent {
            self.out.write_char('\t')?;
        }
        Ok(())
    }
    fn encoded<'a>(&mut self, encoded: &Encoded<'a>) -> fmt::Result {
        if self.indent == encoded.dedent || encoded.one_liner() {
            self.out.write_str(encoded.utf8)?;
            self.out.write_char('\n')?;
        } else {
            let mut lines = encoded.lines();
            if let Some(first) = lines.next() {
                self.out.write_str(first)?;
                self.out.write_char('\n')?;
                for line in lines {
                    self.indent()?;
                    self.out.write_str(&line[encoded.dedent..])?;
                    self.out.write_char('\n')?;
                }
            };
        }
        Ok(())
    }
    fn some_comment<'a>(&mut self, marker: &'a str, comment: &Comment<'a>) -> fmt::Result {
        self.indent()?;
        self.out.write_str(marker)?;
        self.indent += 1;
        self.encoded(&comment.encoded)?;
        self.indent -= 1;
        Ok(())
    }
    fn comment<'a>(&mut self, marker: &'a str, option: &Option<Comment<'a>>) -> fmt::Result {
        if let Some(comment) = option {
            self.some_comment(marker, comment)?;
        }
        Ok(())
    }
    fn text_in_list<'a>(&mut self, text: &Text<'a>) -> fmt::Result {
        self.indent()?;
        if text.one_liner_in_list() {
            self.out.write_str(text.encoded.utf8)?;
            self.out.write_char('\n')?;
        } else {
            self.out.write_str("<>\n")?;
            self.indent += 1;
            self.indent()?;
            self.encoded(&text.encoded)?;
            self.indent -= 1;
        }
        self.comment("#", &text.epilog)
    }
    fn text_in_dict<'a>(&mut self, key: &'a str, text: &Text<'a>) -> fmt::Result {
        self.indent()?;
        if text.one_liner_in_dict(key) {
            self.out.write_str(key)?;
            self.out.write_char('=')?;
            self.out.write_str(text.encoded.utf8)?;
            self.out.write_char('\n')?;
        } else {
            self.out.write_char('<')?;
            self.out.write_str(key)?;
            self.out.write_str(">\n")?;
            self.indent += 1;
            self.indent()?;
            self.encoded(&text.encoded)?;
            self.indent -= 1;
        }
        self.comment("#", &text.epilog)
    }
    fn list_in_list<'a>(&mut self, list: &List<'a>) -> fmt::Result {
        self.comment("#", &list.prolog)?;
        self.indent()?;
        self.out.write_str("[]\n")?;
        self.indent += 1;
        for value in list.list {
            self.value_in_list(value)?;
        }
        self.indent -= 1;
        self.comment("#", &list.epilog)
    }
    fn list_in_dict<'a>(&mut self, key: &'a str, list: &List<'a>) -> fmt::Result {
        self.indent()?;
        self.out.write_char('[')?;
        self.out.write_str(key)?;
        self.out.write_str("]\n")?;
        self.indent += 1;
        for value in list.list {
            self.value_in_list(value)?;
        }
        self.indent -= 1;
        self.comment("#", &list.epilog)
    }
    fn dict_in_list<'a>(&mut self, dict: &Dict<'a>) -> fmt::Result {
        self.indent()?;
        self.out.write_str("{}\n")?;
        self.indent += 1;
        for keyed in dict.dict {
            self.value_in_dict(keyed)?;
        }
        self.indent -= 1;
        self.comment("#", &dict.epilog)
    }
    fn dict_in_dict<'a>(&mut self, key: &'a str, dict: &Dict<'a>) -> fmt::Result {
        self.indent()?;
        self.out.write_char('{')?;
        self.out.write_str(key)?;
        self.out.write_str("}\n")?;
        self.indent += 1;
        for keyed in dict.dict {
            self.value_in_dict(keyed)?;
        }
        self.indent -= 1;
        self.comment("#", &dict.epilog)
    }
    fn value_in_list<'a>(&mut self, cell: &Cell<Value<'a>>) -> fmt::Result {
        let value = cell.get();
        match value {
            Value::Text(text) => self.text_in_list(&text),
            Value::List(list) => self.list_in_list(&list),
            Value::Dict(dict) => self.dict_in_list(&dict),
        }
    }
    fn value_in_dict<'a>(&mut self, cell: &Cell<Keyed<'a>>) -> fmt::Result {
        let keyed = cell.get();
        if keyed.gap {
            // TODO be strict? f.write_indent(self.indent)?;
            self.out.write_char('\n')?;
        }
        self.comment("//", &keyed.before)?;
        match &keyed.value {
            Value::Text(text) => self.text_in_dict(keyed.key, text),
            Value::List(list) => self.list_in_dict(keyed.key, list),
            Value::Dict(dict) => self.dict_in_dict(keyed.key, dict),
        }
    }
    fn file<'a>(&mut self, file: &File<'a>) -> fmt::Result {
        self.comment("#!", &file.hashbang)?;
        self.comment("#", &file.prolog)?;
        for keyed in file.dict {
            self.value_in_dict(&keyed)?;
        }
        Ok(())
    }
}

#[allow(unused)]
struct Input<'a> {
    src: &'a str,
    next: usize,
    indent: usize,
}
#[allow(unused)]
impl<'a> Input<'a> {
    #[allow(unused)]
    fn encoded(&mut self, from: &'a str, start: usize) -> Encoded<'a> {
        let bytes = &from.as_bytes()[start..];
        let mut newlines = 0usize;
        let indent = self.indent + 1;
        let mut cursor = 0usize;
        'outer: while cursor < bytes.len() {
            if bytes[cursor] != b'\n' {
                cursor += 1;
                continue;
            }
            if cursor + indent >= bytes.len() {
                break;
            }
            for offset in 0..indent {
                if bytes[cursor + 1 + offset] != b'\t' {
                    break 'outer;
                }
            }
            cursor += 1 + indent;
            newlines += 1;
        }
        Encoded {
            utf8: &from[..cursor],
            dedent: if newlines == 0 { usize::MAX } else { indent },
        }
    }
}

// ====================================================================================
