#![warn(missing_docs)] //, unused
#![allow(unused)]

//! Text in Nested Dictionaries and Lists - with Important Comments

/// build a [Path] from steps
#[macro_export]
macro_rules! path {
    ($($step:tt),+) => {
        $crate::Path::from(&[$($crate::path!(@step $step)),+][..])
    };
    (@step [$n:expr]) => {
        $crate::PathStep::List($n)
    };
    (@step $s:literal) => {
        $crate::PathStep::Dict($s)
    };
}

#[cfg(test)]
mod tests;

use std::fmt::{self, Display, Formatter, Write};
use std::ops::Deref;
use std::str::FromStr;

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

// ====================================================================================

#[derive(Clone, Debug)]
struct Indented<'a, T> {
    indent: usize,
    value: &'a T,
}

impl<'a,T> Deref for Indented<'a,T> {
    type Target = &'a T;
    fn deref(&self) -> &Self::Target {
        &self.value
    }
}

impl<'a,T> Indented<'a,T> {
    fn write_indent(&self, f: &mut Formatter<'_>) -> fmt::Result {
        for _ in 0..self.indent {
            f.write_char('\t')?;
        }
        Ok(())
    }
    fn from(indent:usize, value:&'a T) -> Self {
        Indented { value, indent }
    }
}

#[derive(Clone, Debug)]
struct Marked<'a, T> {
    marker: &'a str,
    indented: Indented<'a, T>,
}

impl<'a,T> Deref for Marked<'a,T> {
    type Target = Indented<'a,T>;
    fn deref(&self) -> &Self::Target {
        &self.indented
    }
}

impl<'a,T> Marked<'a,T> {
    fn from(indent:usize, marker: &'a str, value:&'a T) -> Self {
        Marked { indented: Indented::from(indent, value), marker }
    }
}

#[derive(Clone, Debug)]
struct Encoded<'a> {
    utf8: &'a str,
    dedent: usize,
}

impl<'a> Encoded<'a> {
    pub fn one_liner(&self) -> bool {
        if self.dedent == usize::MAX {
            debug_assert!(!self.utf8.contains('\n'), "one_liner contains newline");
            true
        } else {
            debug_assert!(self.utf8.contains('\n'), "missing newline in !one_liner");
            false
        }
    }

    fn parse(source: &'a str, indent: usize) -> Self {
        let bytes = source.as_bytes();
        let mut newlines = 0usize;
        let indent = indent + 1;
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
            utf8: &source[..cursor],
            dedent: if newlines == 0 { usize::MAX } else { indent },
        }
    }

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

impl<'a> From<&'a str> for Encoded<'a> {
    fn from(utf8: &'a str) -> Self {
        Encoded {
            utf8: utf8,
            dedent: if utf8.contains('\n') { 0 } else { usize::MAX },
        }
    }
}

// impl<'a> Display for Encoded<'a> {
//     fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
//         Indented::from(0, self).fmt(f)
//     }
// }

impl<'a> Display for Indented<'a, Encoded<'a>> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        if self.indent == self.dedent || self.one_liner() {
            f.write_str(self.utf8)?;
            f.write_char('\n')?;
        } else {
            let mut lines = self.lines();
            if let Some(first) = lines.next() {
                f.write_str(first)?;
                f.write_char('\n')?;
                for line in lines {
                    self.write_indent(f)?;
                    f.write_str(&line[self.dedent..])?;
                    f.write_char('\n')?;
                }
            };
        }
        Ok(())
    }
}

impl<'a> Comment<'a> {
    /// instantiate into [Option::Some].
    pub fn some(utf8: &'a str) -> Option<Self> {
        Some(Comment::from(utf8))
    }
    fn parse(utf8: &'a str, indent: usize, marker: &'a str) -> Option<Comment<'a>> {
        todo!()
    }
}

impl<'a> Display for Comment<'a> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        Marked::from(0, "#", &self.encoded).fmt(f)
    }
}

impl<'a> Display for Marked<'a, Option<Comment<'a>>> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        if let Some(comment) = &self.value {
            self.write_indent(f)?;
            f.write_str(self.marker)?;
            Indented::from(self.indent + 1, &comment.encoded).fmt(f)?;
        }
        Ok(())
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
impl<'a> Display for Value<'a> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        Indented::from(0, self).fmt(f)
    }
}
impl<'a> Display for Indented<'a, Value<'a>> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self.value {
            Value::Text(text) => Indented::from(self.indent,text).fmt(f),
            Value::List(list) => Indented::from(self.indent,list).fmt(f),
            Value::Dict(dict) => Indented::from(self.indent,dict).fmt(f),
        }
    }
}
impl<'a> Display for Marked<'a, Value<'a>> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self.value {
            Value::Text(text) => Marked::from(self.indent,self.marker,text).fmt(f),
            Value::List(list) => Marked::from(self.indent,self.marker,list).fmt(f),
            Value::Dict(dict) => Marked::from(self.indent,self.marker,dict).fmt(f),
        }
    }
}

impl<'a> Text<'a> {
    fn one_liner_in_list(&self) -> bool {
        if !self.encoded.one_liner() {
            return false;
        }
        if self.encoded.utf8.is_empty() {
            return true;
        }
        match self.encoded.utf8.as_bytes()[0] {
            b'\t' | b'#' | b'<' | b'>' | b'[' | b']' | b'{' | b'}' | b'/' | b'=' => false,
            _ => true,
        }
    }
    fn one_liner_in_dict(&self, key: &str) -> bool {
        false
    }
}

// impl<'a> Display for Text<'a> {
//     fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
//         Indented::from(0, self).fmt(f)
//     }
// }
impl<'a> Display for Indented<'a, Text<'a>> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let more =Indented::from(self.indent+1, &self.encoded);
        if !self.one_liner_in_list() {
            self.write_indent(f)?;
            f.write_str("<>\n")?;
        }
        more.write_indent(f)?;
        more.fmt(f)?;
        Marked::from(self.indent, "#",&self.epilog).fmt(f)
    }
}
impl<'a> Display for Marked<'a, Text<'a>> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        self.write_indent(f)?;
        if self.one_liner_in_dict(self.marker) {
            f.write_str(self.marker)?;
            f.write_char('=')?;
            f.write_str(self.encoded.utf8)?;
        } else {
            f.write_char('<')?;
            f.write_str(self.marker)?;
            f.write_str(">\n")?;
            let more =Indented::from(self.indent+1, &self.encoded);
            more.write_indent(f)?;
            more.fmt(f)?;
        }
        Marked::from(self.indent, "#",&self.epilog).fmt(f)
    }
}

// impl<'a> Display for List<'a> {
//     fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
//         Indented::from(0, self).fmt(f)
//     }
// }
impl<'a> Display for Indented<'a, Vec<Value<'a>>> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        for item in self.value {
            Indented::from(self.indent, item).fmt(f)?;
        }
        Ok(())
    }
}
impl<'a> Display for Indented<'a, List<'a>> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        self.write_indent(f)?;
        f.write_str("[]\n")?;
        let more = self.indent + 1;
        Marked::from(more, "#",&self.prolog).fmt(f)?;
        Indented::from(more, &self.vec).fmt(f)?;
        Marked::from(self.indent, "#",&self.epilog).fmt(f)
    }
}
impl<'a> Display for Marked<'a, List<'a>> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        self.write_indent(f)?;
        f.write_char('[')?;
        f.write_str(self.marker)?;
        f.write_str("]\n")?;
        let more = self.indent + 1;
        Marked::from(more, "#",&self.prolog).fmt(f)?;
        Indented::from(more, &self.vec).fmt(f)?;
        Marked::from(self.indent, "#",&self.epilog).fmt(f)
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
    pub fn from(key: &'a str, value: Value<'a>) -> Self {
        Keyed {
            key,
            gap: false,
            before: None,
            value,
        }
    }
}

// impl<'a> Display for Dict<'a> {
//     fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
//         Indented::from(0, self).fmt(f)
//     }
// }
impl<'a> Display for Indented<'a, Vec<Keyed<'a>>> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        for item in self.value {
            if item.gap {
                // TODO be strict? self.write_indent(f)?;
                f.write_char('\n')?;
            }
            Marked::from(self.indent, "//", &item.before).fmt(f)?;
            Marked::from(self.indent,&item.key, &item.value).fmt(f)?;
        }
        Ok(())
    }
}
impl<'a> Display for Indented<'a, Dict<'a>> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        self.write_indent(f)?;
        f.write_str("{}\n")?;
        let more = self.indent+1;
        Marked::from(more, "#",&self.prolog).fmt(f)?;
        Indented::from(more, &self.vec).fmt(f)?;
        Marked::from(self.indent, "#",&self.epilog).fmt(f)
    }
}
impl<'a> Display for Marked<'a, Dict<'a>> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        self.write_indent(f)?;
        f.write_char('{')?;
        f.write_str(self.marker)?;
        f.write_str("}\n")?;
        let more = self.indent+1;
        Marked::from(more, "#",&self.prolog).fmt(f)?;
        Indented::from(more, &self.vec).fmt(f)?;
        Marked::from(self.indent, "#",&self.epilog).fmt(f)
    }
}

impl<'a> Display for File<'a> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        Marked::from(0, "#!",&self.hashbang).fmt(f)?;
        Marked::from(0, "#",&self.prolog).fmt(f)?;
        Indented::from(0, &self.vec).fmt(f)
    }
}

// ####################################################################################

/// Metadata about a Value or a File.
///
/// The content is UTF-8 Github Flavored Markdown and kept in the encoded form. The
/// fields are private because the encoded form is awkward to work with. An app that
/// ignores the Comments does not have to pay for decoding them: in most cases the
/// Comment content as read is already perfect for writing.
///
/// A field within the Value will hold the Comment, there is no mechanism to navigate
/// from a Comment to the Value it describes.
///
/// The content ownership can be tricky. The caller always provides an immutable
/// string slice to one of the constructors, and the Comment keeps a sub-slice. Zero
/// UTF-8 bytes are moved. But the lifetimes become entangled: the compiler will
/// insist that the caller not drop the source of the string slice without first/also
/// dropping the Comment.
///
/// # Examples
///
/// ```
/// let comment = tindalwic::Comment::from("with ~strikethrough~ extension");
///
/// let html = markdown::to_html_with_options(&comment.to_string(), &markdown::Options::gfm())
///   .expect("should never error, according to:
///      https://docs.rs/markdown/latest/markdown/fn.to_html_with_options.html#errors");
///
/// assert_eq!(html, "<p>with <del>strikethrough</del> extension</p>\n");
/// ```
#[derive(Clone, Debug)]
pub struct Comment<'a> {
    encoded: Encoded<'a>,
}
impl<'a> From<&'a str> for Comment<'a> {
    fn from(utf8: &'a str) -> Self {
        Comment {
            encoded: Encoded::from(utf8),
        }
    }
}
impl<'a> Comment<'a> {
    /// Returns an [Iterator] over the lines (without newline chars).
    ///
    /// This is the most efficient way to access the content. No UTF-8 bytes are moved,
    /// the returned slices simply skip past the indentation TAB chars.
    ///
    /// # Examples
    ///
    /// ```
    /// let expect = ["zero", "one", "two"];
    /// let utf8 = "zero\none\ntwo";
    /// let item = tindalwic::Comment::from(utf8);
    /// for (index, line) in item.lines().enumerate() {
    ///     assert_eq!(line, expect[index]);
    /// }
    /// ```
    pub fn lines(&self) -> impl Iterator<Item = &'a str> {
        self.encoded.lines()
    }
    fn parse_utf8(source: &'a str, indent: usize) -> Self {
        Comment {
            encoded: Encoded::parse(source, indent),
        }
    }
}

/// the fields of a [Value::Text]
#[derive(Clone, Debug)]
pub struct Text<'a> {
    encoded: Encoded<'a>,
    /// A Text can have a Comment after it.
    pub epilog: Option<Comment<'a>>,
}
impl<'a> Text<'a> {
    /// Sets the epilog Comment.
    pub fn with_epilog(mut self, epilog: &'a str) -> Self {
        self.epilog = Comment::some(epilog);
        self
    }
}
impl<'a> From<&'a str> for Text<'a> {
    fn from(utf8: &'a str) -> Self {
        Text {
            encoded: Encoded::from(utf8),
            epilog: None,
        }
    }
}
impl<'a> Text<'a> {
    /// Returns an [Iterator] over the lines (without newline chars).
    ///
    /// This is the most efficient way to access the content. No UTF-8 bytes are moved,
    /// the returned slices simply skip past the indentation TAB chars.
    ///
    /// # Examples
    ///
    /// ```
    /// let expect = ["zero", "one", "two"];
    /// let utf8 = "zero\none\ntwo";
    /// let item = tindalwic::Text::from(utf8);
    /// for (index, line) in item.lines().enumerate() {
    ///     assert_eq!(line, expect[index]);
    /// }
    /// ```
    pub fn lines(&self) -> impl Iterator<Item = &'a str> {
        self.encoded.lines()
    }
    fn parse_utf8(source: &'a str, indent: usize) -> Self {
        Text {
            encoded: Encoded::parse(source, indent),
            epilog: None,
        }
    }
}

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
impl<'a> List<'a> {
    /// Sets the prolog Comment.
    pub fn with_prolog(mut self, prolog: &'a str) -> Self {
        self.prolog = Comment::some(prolog);
        self
    }
    /// Sets the epilog Comment.
    pub fn with_epilog(mut self, epilog: &'a str) -> Self {
        self.epilog = Comment::some(epilog);
        self
    }
}
impl<'a> From<Vec<Value<'a>>> for List<'a> {
    fn from(items: Vec<Value<'a>>) -> Self {
        List {
            vec: items,
            prolog: None,
            epilog: None,
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
impl<'a> Dict<'a> {
    /// Sets the prolog Comment.
    pub fn with_prolog(mut self, prolog: &'a str) -> Self {
        self.prolog = Comment::some(prolog);
        self
    }
    /// Sets the epilog Comment.
    pub fn with_epilog(mut self, epilog: &'a str) -> Self {
        self.epilog = Comment::some(epilog);
        self
    }
}
impl<'a> From<Vec<Keyed<'a>>> for Dict<'a> {
    fn from(items: Vec<Keyed<'a>>) -> Self {
        Dict {
            vec: items,
            prolog: None,
            epilog: None,
        }
    }
}
impl<'a> Dict<'a> {
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
}

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
impl<'a> File<'a> {
    /// Sets the hashbang Comment.
    pub fn with_hashbang(mut self, hashbang: &'a str) -> Self {
        self.hashbang = Comment::some(hashbang);
        self
    }
    /// Sets the prolog Comment.
    pub fn with_prolog(mut self, prolog: &'a str) -> Self {
        self.prolog = Comment::some(prolog);
        self
    }
}
impl<'a> From<Vec<Keyed<'a>>> for File<'a> {
    fn from(items: Vec<Keyed<'a>>) -> Self {
        File {
            vec: items,
            hashbang: None,
            prolog: None,
        }
    }
}
impl<'a> File<'a> {
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
}
