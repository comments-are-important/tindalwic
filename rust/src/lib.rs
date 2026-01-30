#![warn(missing_docs)] //, unused
#![allow(unused)]

//! Text in Nested Dicts and Lists - with Important Comments

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

use std::fmt;

/// an [Err] [Result] for path resolution
#[derive(Clone, Debug)]
pub struct PathErr {
    good: &'static [PathStep],
    have: &'static str,
    fail: Option<&'static PathStep>,
}

impl fmt::Display for PathErr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
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

impl fmt::Display for Path {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
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

fn lines<'a>(encoded: &'a str, dedent: usize) -> impl Iterator<Item = &'a str> {
    // that return type is very tricky to satisfy: having two branches here (one
    // optimized for absent indentation) causes E0308 incompatible types:
    //   "distinct uses of `impl Trait` result in different opaque types"
    // attempting to hide them behind closures does not help either:
    //   "no two closures, even if identical, have the same type"
    let d = if dedent == usize::MAX { 0 } else { dedent };
    encoded
        .split('\n')
        .enumerate()
        .map(move |(i, s)| if i == 0 || d == 0 { s } else { &s[d..] })
}

fn to_string<'a>(encoded: &'a str, dedent: usize) -> String {
    if dedent == 0 || dedent == usize::MAX {
        return String::from(encoded);
    }
    let mut string = String::new();
    for line in lines(encoded, dedent) {
        string.push_str(line);
        string.push('\n');
    }
    if string.len() != 0 {
        string.truncate(string.len() - 1);
    }
    string
}

fn parse<'a>(source: &'a str, indent: usize) -> (&'a str, usize) {
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
    (
        &source[..cursor],
        if newlines == 0 { usize::MAX } else { indent },
    )
}

fn encode<'a>(
    encoded: &'a str,
    dedent: usize,
    indent: usize,
    marker: &'static str,
    into: &mut String,
) {
    into.extend(std::iter::repeat_n('\t', indent));
    into.push_str(marker);
    let indent = indent + 1;
    if indent == dedent || dedent == usize::MAX {
        into.push_str(encoded);
        into.push('\n');
    } else {
        let mut lines = lines(encoded, dedent);
        let Some(first) = lines.next() else {
            into.push('\n');
            return;
        };
        into.push_str(first);
        into.push('\n');
        for line in lines {
            into.extend(std::iter::repeat_n('\t', indent));
            into.push_str(&line[dedent..]);
            into.push('\n');
        }
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
    fn encode(&self, indent: usize, keyed: Option<&Keyed<'a>>, into: &mut String) {
        match self {
            Value::Text(text) => text.encode(indent, keyed, into),
            Value::List(list) => list.encode(indent, keyed, into),
            Value::Dict(dict) => dict.encode(indent, keyed, into),
        }
    }
}

impl<'a> Comment<'a> {
    /// instantiate into [Option::Some].
    pub fn some(utf8: &'a str) -> Option<Self> {
        Some(Comment::from(utf8))
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
}

impl<'a> File<'a> {
    /// write the encoding of this File `into` the String (clearing it first).
    pub fn encode(&self, into: &mut String) {
        into.clear();
        if let Some(hashbang) = &self.hashbang {
            hashbang.encode_utf8(0, "#!", into);
        }
        if let Some(prolog) = &self.prolog {
            prolog.encode_utf8(0, "#", into);
        }
        self.encode_keyed(0, into);
    }
    /// return the encoding of this File in a freshly allocated String.
    pub fn tindalwic(&self) -> String {
        let mut bytes = String::new();
        self.encode(&mut bytes);
        bytes
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
/// assert_eq!(html, "<p>with <del>strikethrough</del> extension</p>");
/// ```
#[derive(Clone, Debug)]
pub struct Comment<'a> {
    encoded: &'a str,
    dedent: usize,
}
impl<'a> From<&'a str> for Comment<'a> {
    fn from(utf8: &'a str) -> Self {
        Comment {
            encoded: utf8,
            dedent: if utf8.contains('\n') { 0 } else { usize::MAX },
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
        lines(self.encoded, self.dedent)
    }
    /// Gathers the [Self::lines] into a freshly allocated [String].
    ///
    /// # Examples
    ///
    /// ```
    /// let utf8 = "zero\none\ntwo";
    /// let item = tindalwic::Comment::from(utf8);
    /// assert_eq!(item.to_string(), utf8);
    /// ```
    pub fn to_string(&self) -> String {
        to_string(self.encoded, self.dedent)
    }
    fn parse_utf8(source: &'a str, indent: usize) -> Self {
        let (encoded, dedent) = parse(source, indent);
        Comment {
            encoded: encoded,
            dedent: dedent,
        }
    }
    /// write the encoding of this Comment into the given String.
    fn encode_utf8(&self, indent: usize, marker: &'static str, into: &mut String) {
        encode(self.encoded, self.dedent, indent, marker, into);
    }
}

/// the fields of a [Value::Text]
#[derive(Clone, Debug)]
pub struct Text<'a> {
    encoded: &'a str,
    dedent: usize,
    /// A Text can have a Comment after it.
    pub epilog: Option<Comment<'a>>,
}
impl<'a> Text<'a> {
    /// Sets the epilog Comment.
    pub fn with_epilog(mut self, epilog: &'a str) -> Self {
        self.epilog = Some(Comment::from(epilog));
        self
    }
}
impl<'a> From<&'a str> for Text<'a> {
    fn from(utf8: &'a str) -> Self {
        Text {
            encoded: utf8,
            dedent: if utf8.contains('\n') { 0 } else { usize::MAX },
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
        lines(self.encoded, self.dedent)
    }
    /// Gathers the [Self::lines] into a freshly allocated [String].
    ///
    /// # Examples
    ///
    /// ```
    /// let utf8 = "zero\none\ntwo";
    /// let item = tindalwic::Text::from(utf8);
    /// assert_eq!(item.to_string(), utf8);
    /// ```
    pub fn to_string(&self) -> String {
        to_string(self.encoded, self.dedent)
    }
    fn parse_utf8(source: &'a str, indent: usize) -> Self {
        let (encoded, dedent) = parse(source, indent);
        Text {
            encoded: encoded,
            dedent: dedent,
            epilog: None,
        }
    }
    /// write the encoding of this Text into the given String.
    fn encode_utf8(&self, indent: usize, marker: &'static str, into: &mut String) {
        encode(self.encoded, self.dedent, indent, marker, into);
    }
}

/// the fields of a [Value::List]
#[derive(Clone, Debug)]
pub struct List<'a> {
    /// The contents of the List.
    pub vec: Vec<Value<'a>>,
    /// A List can have an introductory Comment.
    pub prolog: Option<Comment<'a>>,
    /// A List can have a Comment after it.
    pub epilog: Option<Comment<'a>>,
}
impl<'a> List<'a> {
    /// Sets the prolog Comment.
    pub fn with_prolog(mut self, prolog: &'a str) -> Self {
        self.prolog = Some(Comment::from(prolog));
        self
    }
    /// Sets the epilog Comment.
    pub fn with_epilog(mut self, epilog: &'a str) -> Self {
        self.epilog = Some(Comment::from(epilog));
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
    /// The contents of the Dict.
    pub vec: Vec<Keyed<'a>>,
    /// A Dict can have an introductory Comment.
    pub prolog: Option<Comment<'a>>,
    /// A Dict can have a Comment after it.
    pub epilog: Option<Comment<'a>>,
}
impl<'a> Dict<'a> {
    /// Sets the prolog Comment.
    pub fn with_prolog(mut self, prolog: &'a str) -> Self {
        self.prolog = Some(Comment::from(prolog));
        self
    }
    /// Sets the epilog Comment.
    pub fn with_epilog(mut self, epilog: &'a str) -> Self {
        self.epilog = Some(Comment::from(epilog));
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
    fn encode_keyed(&self, indent: usize, into: &mut String) {
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

/// the outermost context.
///
/// very similar to a [Dict], just with different comments.
#[derive(Clone, Debug)]
pub struct File<'a> {
    /// The contents of the File.
    pub vec: Vec<Keyed<'a>>,
    /// A File can start with a Unix `#!` Comment.
    pub hashbang: Option<Comment<'a>>,
    /// A File can have an introductory Comment.
    pub prolog: Option<Comment<'a>>,
}
impl<'a> File<'a> {
    /// Sets the hashbang Comment.
    pub fn with_hashbang(mut self, hashbang: &'a str) -> Self {
        self.hashbang = Some(Comment::from(hashbang));
        self
    }
    /// Sets the prolog Comment.
    pub fn with_prolog(mut self, prolog: &'a str) -> Self {
        self.prolog = Some(Comment::from(prolog));
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
    fn encode_keyed(&self, indent: usize, into: &mut String) {
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
