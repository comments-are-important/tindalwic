#![no_std]
#![warn(missing_docs,unused)]

//! Text in Nested Dictionaries and Lists - with Important Comments

extern crate alloc;
use alloc::string::String;
use alloc::vec::Vec;
use core::fmt::{self, Debug, Display, Formatter, Write};
use core::ops::{Deref, DerefMut};

/// Hidden parts of [Comment] and [Text].
///
/// These are zero-copy slices from an external buffer of Tindalwic UTF-8. The iterator
/// returned by [Encoded::lines()] is the most efficient way to strip the indentation
/// from a multi-line slice.
#[derive(Clone, Debug)]
struct Encoded<'a> {
    utf8: &'a str,
    dedent: usize, // usize::MAX => one_liner
}
impl<'a> Encoded<'a> {
    /// Return a zero-copy instance using the provided literal (not indented) slice.
    pub fn wrap(utf8: &'a str) -> Self {
        Encoded {
            utf8,
            dedent: if utf8.contains('\n') { 0 } else { usize::MAX },
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
/// let html = markdown::to_html_with_options(&comment.lines_joined(), &markdown::Options::gfm())
///   .expect("should never error, according to:
///      https://docs.rs/markdown/latest/markdown/fn.to_html_with_options.html#errors");
///
/// assert_eq!(html, "<p>with <del>strikethrough</del> extension</p>");
/// ```
#[derive(Clone, Debug)]
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
    /// Allocates a [String], filled with the UTF-8 copied from `self`.
    pub fn lines_joined(&self) -> String {
        self.encoded.lines_joined()
    }
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

/// the fields of a [Value::Text]
#[derive(Clone, Debug)]
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
    /// Allocates a [String], filled with the UTF-8 copied from `self`.
    pub fn lines_joined(&self) -> String {
        self.encoded.lines_joined()
    }
}

// ------------------------------------------------------------------------------------

/// the fields of a [Value::List]
#[derive(Clone, Debug)]
pub struct List<'a> {
    /// The contents of the Value::List.
    pub vec: Vec<Value<'a>>,
    /// A List can have an introductory Comment.
    pub prolog: Option<Comment<'a>>,
    /// A List can have a Comment after it.
    pub epilog: Option<Comment<'a>>,
}
impl<'a> Deref for List<'a> {
    type Target = Vec<Value<'a>>;

    fn deref(&self) -> &Self::Target {
        &self.vec
    }
}
impl<'a> DerefMut for List<'a> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.vec
    }
}
impl<'a> Display for List<'a> {
    fn fmt(&self, out: &mut Formatter<'_>) -> fmt::Result {
        Output { out, indent: 0 }.list_in_list(self)
    }
}

// ------------------------------------------------------------------------------------

/// an association.
///
/// these are stored in a [Vec] (instead of using a hash table).
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
#[derive(Clone, Debug)]
pub struct Dict<'a> {
    /// The contents of the Value::Dict.
    pub vec: Vec<Keyed<'a>>,
    /// A Dict can have an introductory Comment.
    pub prolog: Option<Comment<'a>>,
    /// A Dict can have a Comment after it.
    pub epilog: Option<Comment<'a>>,
}
impl<'a> Deref for Dict<'a> {
    type Target = Vec<Keyed<'a>>;

    fn deref(&self) -> &Self::Target {
        &self.vec
    }
}
impl<'a> DerefMut for Dict<'a> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.vec
    }
}
impl<'a> Display for Dict<'a> {
    fn fmt(&self, out: &mut Formatter<'_>) -> fmt::Result {
        Output { out, indent: 0 }.dict_in_list(self)
    }
}
impl<'a> Dict<'a> {
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
}

// ------------------------------------------------------------------------------------

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
impl<'a> Display for Value<'a> {
    fn fmt(&self, out: &mut Formatter<'_>) -> fmt::Result {
        Output { out, indent: 0 }.value_in_list(self)
    }
}

// ------------------------------------------------------------------------------------

/// the outermost context.
///
/// very similar to a [Value::Dict], just with different comments.
#[derive(Clone, Debug)]
pub struct File<'a> {
    /// The contents of the Value::File.
    pub vec: Vec<Keyed<'a>>,
    /// A File can start with a Unix `#!` Comment.
    pub hashbang: Option<Comment<'a>>,
    /// A File can have an introductory Comment.
    pub prolog: Option<Comment<'a>>,
}
impl<'a> Deref for File<'a> {
    type Target = Vec<Keyed<'a>>;

    fn deref(&self) -> &Self::Target {
        &self.vec
    }
}
impl<'a> DerefMut for File<'a> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.vec
    }
}
impl<'a> Display for File<'a> {
    fn fmt(&self, out: &mut Formatter<'_>) -> fmt::Result {
        Output { out, indent: 0 }.file(self)
    }
}
impl<'a> File<'a> {
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
}

// ------------------------------------------------------------------------------------

/// build a Tindalwic [File] or [Value] from JSON literals.
///
/// This macro does not provide any mechanism to set [Comment]s.
#[macro_export]
macro_rules! tindalwic {
    ( $name:ident ) => {
        $name
    };
    ( $text:literal ) => {
        Value::Text(Text::wrap($text))
    };
    ( [ $( $items:tt ),* ] ) => {
        Value::List(List{
            vec: vec![ $( tindalwic!($items) ),* ],
            prolog: None,
            epilog: None,
        })
    };
    ( { $( $key:literal : $value:tt ),* } ) => {
        Value::Dict(Dict{
            vec: vec![ $( Keyed::from($key, tindalwic!($value)) ),* ],
            prolog: None,
            epilog: None,
        })
    };
    ( $( $key:literal : $value:tt ),* ) => {
        File{
            vec: vec![ $( Keyed::from($key, tindalwic!($value)) ),* ],
            hashbang: None,
            prolog: None,
        }
    };
}

// ====================================================================================

struct Output<'a, 'f> {
    out: &'a mut Formatter<'f>,
    indent: usize,
}
impl<'a, 'f> Output<'a, 'f> {
    fn indent(&mut self) -> fmt::Result {
        for _ in 0..self.indent {
            self.out.write_char('\t')?;
        }
        Ok(())
    }
    fn encoded(&mut self, encoded: &Encoded<'a>) -> fmt::Result {
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
    fn some_comment(&mut self, marker: &'a str, comment: &Comment<'a>) -> fmt::Result {
        self.indent()?;
        self.out.write_str(marker)?;
        self.indent += 1;
        self.encoded(&comment.encoded)?;
        self.indent -= 1;
        Ok(())
    }
    fn comment(&mut self, marker: &'a str, option: &Option<Comment<'a>>) -> fmt::Result {
        if let Some(comment) = option {
            self.some_comment(marker, comment)?;
        }
        Ok(())
    }
    fn text_in_list(&mut self, text: &Text<'a>) -> fmt::Result {
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
    fn text_in_dict(&mut self, key: &'a str, text: &Text<'a>) -> fmt::Result {
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
    fn list_in_list(&mut self, list: &List<'a>) -> fmt::Result {
        self.indent()?;
        self.out.write_str("[]\n")?;
        self.indent += 1;
        for value in &list.vec {
            self.value_in_list(value)?;
        }
        self.indent -= 1;
        self.comment("#", &list.epilog)
    }
    fn list_in_dict(&mut self, key: &'a str, list: &List<'a>) -> fmt::Result {
        self.indent()?;
        self.out.write_char('[')?;
        self.out.write_str(key)?;
        self.out.write_str("]\n")?;
        self.indent += 1;
        for value in &list.vec {
            self.value_in_list(value)?;
        }
        self.indent -= 1;
        self.comment("#", &list.epilog)
    }
    fn dict_in_list(&mut self, dict: &Dict<'a>) -> fmt::Result {
        self.indent()?;
        self.out.write_str("{}\n")?;
        self.indent += 1;
        for keyed in &dict.vec {
            self.value_in_dict(keyed)?;
        }
        self.indent -= 1;
        self.comment("#", &dict.epilog)
    }
    fn dict_in_dict(&mut self, key: &'a str, dict: &Dict<'a>) -> fmt::Result {
        self.indent()?;
        self.out.write_char('{')?;
        self.out.write_str(key)?;
        self.out.write_str("}\n")?;
        self.indent += 1;
        for keyed in &dict.vec {
            self.value_in_dict(keyed)?;
        }
        self.indent -= 1;
        self.comment("#", &dict.epilog)
    }
    fn value_in_list(&mut self, value: &Value<'a>) -> fmt::Result {
        match value {
            Value::Text(text) => self.text_in_list(text),
            Value::List(list) => self.list_in_list(list),
            Value::Dict(dict) => self.dict_in_list(dict),
        }
    }
    fn value_in_dict(&mut self, keyed: &Keyed<'a>) -> fmt::Result {
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
    fn file(&mut self, file: &File<'a>) -> fmt::Result {
        self.comment("#!", &file.hashbang)?;
        self.comment("#", &file.prolog)?;
        for keyed in &file.vec {
            self.value_in_dict(keyed)?;
        }
        Ok(())
    }
}

struct Input<'a> {
    src: &'a str,
    next: usize,
    indent:usize,
}
impl<'a> Input<'a> {
    fn encoded(&mut self, from: &'a str, start: usize) -> Encoded {
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

/// an [Err] [Result] for path resolution
#[derive(Clone, Debug)]
pub struct PathErr {
    good: &'static [PathStep],
    have: &'static str,
    fail: Option<&'static PathStep>,
}
impl Display for PathErr {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match &self.fail {
            None => {
                write!(
                    f,
                    "Path `{}` leads to {}.",
                    Path::from(self.good),
                    self.have
                )
            }
            Some(fail) => {
                write!(
                    f,
                    "Path `{}` leads to {}, can't {:?}.",
                    Path::from(self.good),
                    self.have,
                    fail
                )
            }
        }
    }
}
impl PathErr {
    fn some(good: &'static [PathStep], have: &'static str, fail: &'static PathStep) -> Self {
        PathErr {
            good,
            have,
            fail: Some(fail),
        }
    }
    fn none(good: &'static [PathStep], have: &'static str) -> Self {
        PathErr {
            good,
            have,
            fail: None,
        }
    }
}

/// a single step in a [Path]
#[derive(Clone, Debug)]
pub enum PathStep {
    /// an index into a linear array
    List(usize),
    /// the key into an associative array
    Dict(&'static str),
}
impl From<usize> for PathStep {
    fn from(value: usize) -> Self {
        PathStep::List(value)
    }
}
impl From<&'static str> for PathStep {
    fn from(value: &'static str) -> Self {
        PathStep::Dict(value)
    }
}

/// one or more [PathStep]s
#[derive(Clone, Debug)]
pub struct Path {
    steps: &'static [PathStep],
}
impl From<&'static [PathStep]> for Path {
    fn from(steps: &'static [PathStep]) -> Self {
        if steps.is_empty() {
            panic!("need at least one step")
        }
        Path { steps }
    }
}
impl Path {
    /// resolve this path, if possible, to a [Value]
    pub fn value<'v>(&self, root: &'v Value<'v>) -> Result<&'v Value<'v>, PathErr> {
        let mut value = root;
        let mut passed = &self.steps[0..0];
        for step in self.steps {
            value = match (step, value) {
                (PathStep::List(index), Value::List(list)) => list
                    .vec
                    .get(*index)
                    .ok_or(PathErr::some(passed, "List too short", step)),
                (PathStep::Dict(lookup), Value::Dict(dict)) => dict
                    .find(lookup)
                    .map(|k| &k.value)
                    .ok_or(PathErr::some(passed, "Dict missing key", step)),
                (_, Value::Text(_)) => Err(PathErr::some(passed, "Text", step)),
                (_, Value::List(_)) => Err(PathErr::some(passed, "List", step)),
                (_, Value::Dict(_)) => Err(PathErr::some(passed, "Dict", step)),
            }?;
            passed = &self.steps[0..passed.len() + 1]
        }
        Ok(value)
    }

    /// resolve this path, if possible, to a mutable [Value]
    pub fn value_mut<'v>(&self, root: &'v mut Value<'v>) -> Result<&'v mut Value<'v>, PathErr> {
        let mut value = root;
        let mut passed = &self.steps[0..0];
        for step in self.steps {
            value = match (step, value) {
                (PathStep::List(index), Value::List(list)) => list
                    .vec
                    .get_mut(*index)
                    .ok_or(PathErr::some(passed, "List too short", step)),
                (PathStep::Dict(lookup), Value::Dict(dict)) => dict
                    .find_mut(lookup)
                    .map(|k| &mut k.value)
                    .ok_or(PathErr::some(passed, "Dict missing key", step)),
                (_, Value::Text(_)) => Err(PathErr::some(passed, "Text", step)),
                (_, Value::List(_)) => Err(PathErr::some(passed, "List", step)),
                (_, Value::Dict(_)) => Err(PathErr::some(passed, "Dict", step)),
            }?;
            passed = &self.steps[0..passed.len() + 1]
        }
        Ok(value)
    }

    /// resolve this path, if possible, to a [Text]
    pub fn text<'v>(&self, root: &'v Value<'v>) -> Result<&'v Text<'v>, PathErr> {
        match self.value(root)? {
            Value::Text(text) => Ok(text),
            Value::List(_) => Err(PathErr::none(self.steps, "List (not Text)")),
            Value::Dict(_) => Err(PathErr::none(self.steps, "Dict (not Text)")),
        }
    }
    /// resolve this path, if possible, to a mutable [Text]
    pub fn text_mut<'v>(&self, root: &'v mut Value<'v>) -> Result<&'v mut Text<'v>, PathErr> {
        match self.value_mut(root)? {
            Value::Text(text) => Ok(text),
            Value::List(_) => Err(PathErr::none(self.steps, "List (not Text)")),
            Value::Dict(_) => Err(PathErr::none(self.steps, "Dict (not Text)")),
        }
    }

    /// resolve this path, if possible, to a [List]
    pub fn list<'v>(&self, root: &'v Value<'v>) -> Result<&'v List<'v>, PathErr> {
        match self.value(root)? {
            Value::List(list) => Ok(list),
            Value::Dict(_) => Err(PathErr::none(self.steps, "Dict (not List)")),
            Value::Text(_) => Err(PathErr::none(self.steps, "Text (not List)")),
        }
    }
    /// resolve this path, if possible, to a mutable [List]
    pub fn list_mut<'v>(&self, root: &'v mut Value<'v>) -> Result<&'v mut List<'v>, PathErr> {
        match self.value_mut(root)? {
            Value::List(list) => Ok(list),
            Value::Dict(_) => Err(PathErr::none(self.steps, "Dict (not List)")),
            Value::Text(_) => Err(PathErr::none(self.steps, "Text (not List)")),
        }
    }

    /// resolve this path, if possible, to a [Dict]
    pub fn dict<'v>(&self, root: &'v Value<'v>) -> Result<&'v Dict<'v>, PathErr> {
        match self.value(root)? {
            Value::Dict(dict) => Ok(dict),
            Value::List(_) => Err(PathErr::none(self.steps, "List (not Dict)")),
            Value::Text(_) => Err(PathErr::none(self.steps, "Text (not Dict)")),
        }
    }
    /// resolve this path, if possible, to a mutable [Dict]
    pub fn dict_mut<'v>(&self, root: &'v mut Value<'v>) -> Result<&'v mut Dict<'v>, PathErr> {
        match self.value_mut(root)? {
            Value::Dict(dict) => Ok(dict),
            Value::List(_) => Err(PathErr::none(self.steps, "List (not Dict)")),
            Value::Text(_) => Err(PathErr::none(self.steps, "Text (not Dict)")),
        }
    }
}
impl Display for Path {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        for step in self.steps {
            match step {
                PathStep::List(index) => write!(f, "[{}]", index)?,
                PathStep::Dict(lookup) => write!(f, ".{}", lookup)?,
            };
        }
        Ok(())
    }
}
/// build a [Path] from steps
#[macro_export]
macro_rules! path {
    (@step [$n:expr]) => {
        $crate::PathStep::List($n)
    };
    (@step $s:literal) => {
        $crate::PathStep::Dict($s)
    };
    ($($step:tt),+) => {
        $crate::Path::from(&[$($crate::path!(@step $step)),+][..])
    };
}
