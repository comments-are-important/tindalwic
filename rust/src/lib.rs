//! ALACS file format and tools
//!
//! Data structures for representing text with comments.
//! Values can be Text, List, or Dict - augmented with comment metadata.
//! Users can manipulate these like standard collections while comments are preserved.
//!
//! All structures borrow from a source buffer via lifetime `'a`.

use std::collections::HashMap;
use std::fmt;

// =============================================================================
// UTF8 - a list of byte slices representing lines
// =============================================================================

/// A list of byte slices, each representing a line of UTF-8 text.
/// Lines are joined with newlines when converted to bytes or string.
/// Borrows from an external buffer.
#[derive(Clone, Default, PartialEq, Eq, Hash)]
pub struct Utf8<'a> {
    lines: Vec<&'a [u8]>,
}

impl<'a> Utf8<'a> {
    /// Create empty.
    pub fn new() -> Self {
        Self { lines: Vec::new() }
    }

    /// Create from a single line.
    pub fn from_line(line: &'a [u8]) -> Self {
        Self { lines: vec![line] }
    }

    /// Create from multiple lines.
    pub fn from_lines(lines: impl IntoIterator<Item = &'a [u8]>) -> Self {
        Self {
            lines: lines.into_iter().collect(),
        }
    }

    /// Number of lines.
    pub fn len(&self) -> usize {
        self.lines.len()
    }

    /// True if no lines.
    pub fn is_empty(&self) -> bool {
        self.lines.is_empty()
    }

    /// Clear all lines.
    pub fn clear(&mut self) {
        self.lines.clear();
    }

    /// Append a line.
    pub fn push(&mut self, line: &'a [u8]) {
        self.lines.push(line);
    }

    /// Get a line by index.
    pub fn get(&self, index: usize) -> Option<&'a [u8]> {
        self.lines.get(index).copied()
    }

    /// Iterate over lines.
    pub fn iter(&self) -> impl Iterator<Item = &'a [u8]> + '_ {
        self.lines.iter().copied()
    }

    /// Join all lines with newlines into a single byte vector.
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut result = Vec::new();
        for (i, line) in self.lines.iter().enumerate() {
            if i > 0 {
                result.push(b'\n');
            }
            result.extend_from_slice(line);
        }
        result
    }

    /// Decode all lines as UTF-8 string, joined with newlines.
    pub fn to_string(&self) -> String {
        String::from_utf8_lossy(&self.to_bytes()).into_owned()
    }

    /// Normalize: split any lines containing embedded newlines.
    /// Also clears if the only content is a single empty line.
    pub fn normalize(&mut self) {
        // If single empty line, clear entirely
        if self.lines.len() == 1 && self.lines[0].is_empty() {
            self.clear();
            return;
        }

        // Split any lines containing newlines
        let mut i = 0;
        while i < self.lines.len() {
            let bytes = self.lines[i];
            if bytes.iter().any(|&b| b == b'\n') {
                // This line contains a newline - split it
                let line = self.lines.remove(i);
                let mut start = 0;
                for (j, &b) in line.iter().enumerate() {
                    if b == b'\n' {
                        self.lines.insert(i, &line[start..j]);
                        i += 1;
                        start = j + 1;
                    }
                }
                // Insert the remainder
                self.lines.insert(i, &line[start..]);
            }
            i += 1;
        }
    }
}

impl fmt::Debug for Utf8<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Utf8({:?})", self.to_bytes())
    }
}

// =============================================================================
// Comment - just a Utf8 with semantic meaning
// =============================================================================

/// A comment block: one or more lines of comment text.
pub type Comment<'a> = Utf8<'a>;

// =============================================================================
// Value - the enum of possible value types
// =============================================================================

/// A value that can appear in an ALACS structure.
/// All values can have a trailing comment.
#[derive(Clone)]
pub enum Value<'a> {
    Text(Text<'a>),
    List(List<'a>),
    Dict(Dict<'a>),
}

impl<'a> Value<'a> {
    /// Get the comment after this value, if any.
    pub fn comment_after(&self) -> Option<&Comment<'a>> {
        match self {
            Value::Text(t) => t.comment_after.as_ref(),
            Value::List(l) => l.comment_after.as_ref(),
            Value::Dict(d) => d.comment_after.as_ref(),
        }
    }

    /// Set the comment after this value.
    pub fn set_comment_after(&mut self, comment: Option<Comment<'a>>) {
        match self {
            Value::Text(t) => t.comment_after = comment,
            Value::List(l) => l.comment_after = comment,
            Value::Dict(d) => d.comment_after = comment,
        }
    }
}

impl fmt::Debug for Value<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Value::Text(t) => t.fmt(f),
            Value::List(l) => l.fmt(f),
            Value::Dict(d) => d.fmt(f),
        }
    }
}

// =============================================================================
// Text - UTF8 lines with optional trailing comment
// =============================================================================

/// Text value: UTF-8 lines with an optional trailing comment.
#[derive(Clone, Default)]
pub struct Text<'a> {
    pub content: Utf8<'a>,
    pub comment_after: Option<Comment<'a>>,
}

impl<'a> Text<'a> {
    /// Create empty text.
    pub fn new() -> Self {
        Self {
            content: Utf8::new(),
            comment_after: None,
        }
    }

    /// Create from a single line.
    pub fn from_line(line: &'a [u8]) -> Self {
        Self {
            content: Utf8::from_line(line),
            comment_after: None,
        }
    }

    /// Get the text as a string.
    pub fn to_string(&self) -> String {
        self.content.to_string()
    }

    /// Normalize the content.
    pub fn normalize(&mut self) {
        self.content.normalize();
    }
}

impl fmt::Debug for Text<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Text({:?}", self.content)?;
        if let Some(ref c) = self.comment_after {
            write!(f, ",after={:?}", c)?;
        }
        write!(f, ")")
    }
}

impl<'a> From<Text<'a>> for Value<'a> {
    fn from(t: Text<'a>) -> Self {
        Value::Text(t)
    }
}

// =============================================================================
// List - a sequence of Values with optional comments
// =============================================================================

/// A list of values with optional intro and trailing comments.
#[derive(Clone, Default)]
pub struct List<'a> {
    pub items: Vec<Value<'a>>,
    pub comment_intro: Option<Comment<'a>>,
    pub comment_after: Option<Comment<'a>>,
}

impl<'a> List<'a> {
    /// Create empty list.
    pub fn new() -> Self {
        Self {
            items: Vec::new(),
            comment_intro: None,
            comment_after: None,
        }
    }

    /// Number of items.
    pub fn len(&self) -> usize {
        self.items.len()
    }

    /// True if empty.
    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    /// Append a value.
    pub fn push(&mut self, value: Value<'a>) {
        self.items.push(value);
    }

    /// Get item by index.
    pub fn get(&self, index: usize) -> Option<&Value<'a>> {
        self.items.get(index)
    }

    /// Get mutable item by index.
    pub fn get_mut(&mut self, index: usize) -> Option<&mut Value<'a>> {
        self.items.get_mut(index)
    }

    /// Iterate over items.
    pub fn iter(&self) -> impl Iterator<Item = &Value<'a>> {
        self.items.iter()
    }

    /// Iterate mutably over items.
    pub fn iter_mut(&mut self) -> impl Iterator<Item = &mut Value<'a>> {
        self.items.iter_mut()
    }
}

impl fmt::Debug for List<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "List(")?;
        for (i, item) in self.items.iter().enumerate() {
            if i > 0 {
                write!(f, ",")?;
            }
            write!(f, "{:?}", item)?;
        }
        if let Some(ref c) = self.comment_intro {
            write!(f, ",intro={:?}", c)?;
        }
        if let Some(ref c) = self.comment_after {
            write!(f, ",after={:?}", c)?;
        }
        write!(f, ")")
    }
}

impl<'a> From<List<'a>> for Value<'a> {
    fn from(l: List<'a>) -> Self {
        Value::List(l)
    }
}

// =============================================================================
// Key - a string key with optional preceding blank line and comment
// =============================================================================

/// A dictionary key with optional formatting metadata.
#[derive(Clone)]
pub struct Key<'a> {
    pub name: &'a str,
    pub blank_line_before: bool,
    pub comment_before: Option<Comment<'a>>,
}

impl<'a> Key<'a> {
    /// Create a key from a string slice.
    /// Panics if the name contains a newline.
    pub fn new(name: &'a str) -> Self {
        if name.contains('\n') {
            panic!("newline in key");
        }
        Self {
            name,
            blank_line_before: false,
            comment_before: None,
        }
    }

    /// Get the key name.
    pub fn as_str(&self) -> &'a str {
        self.name
    }
}

impl fmt::Debug for Key<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Key({:?})", self.name)
    }
}

impl fmt::Display for Key<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.name)
    }
}

// =============================================================================
// Dict - a map of Key to Value with optional comments
// =============================================================================

/// A dictionary mapping keys to values, with optional intro and trailing comments.
/// Note: uses std HashMap which does not preserve insertion order.
#[derive(Clone, Default)]
pub struct Dict<'a> {
    pub entries: HashMap<&'a str, (Key<'a>, Value<'a>)>,
    pub comment_intro: Option<Comment<'a>>,
    pub comment_after: Option<Comment<'a>>,
}

impl<'a> Dict<'a> {
    /// Create empty dict.
    pub fn new() -> Self {
        Self {
            entries: HashMap::new(),
            comment_intro: None,
            comment_after: None,
        }
    }

    /// Number of entries.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// True if empty.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Insert a key-value pair.
    pub fn insert(&mut self, key: Key<'a>, value: Value<'a>) {
        self.entries.insert(key.name, (key, value));
    }

    /// Get value by key name.
    pub fn get(&self, key: &str) -> Option<&Value<'a>> {
        self.entries.get(key).map(|(_, v)| v)
    }

    /// Get mutable value by key name.
    pub fn get_mut(&mut self, key: &str) -> Option<&mut Value<'a>> {
        self.entries.get_mut(key).map(|(_, v)| v)
    }

    /// Get key and value by key name.
    pub fn get_entry(&self, key: &str) -> Option<(&Key<'a>, &Value<'a>)> {
        self.entries.get(key).map(|(k, v)| (k, v))
    }

    /// Iterate over (key, value) pairs.
    pub fn iter(&self) -> impl Iterator<Item = (&Key<'a>, &Value<'a>)> {
        self.entries.values().map(|(k, v)| (k, v))
    }

    /// Iterate mutably over values (keys are immutable for HashMap consistency).
    pub fn values_mut(&mut self) -> impl Iterator<Item = &mut Value<'a>> {
        self.entries.values_mut().map(|(_, v)| v)
    }
}

impl fmt::Debug for Dict<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Dict(")?;
        if let Some(ref c) = self.comment_intro {
            write!(f, "intro={:?},", c)?;
        }
        if let Some(ref c) = self.comment_after {
            write!(f, "after={:?},", c)?;
        }
        for (key, value) in self.iter() {
            write!(f, "{}={:?},", key.name, value)?;
        }
        write!(f, ")")
    }
}

impl<'a> From<Dict<'a>> for Value<'a> {
    fn from(d: Dict<'a>) -> Self {
        Value::Dict(d)
    }
}

// =============================================================================
// File - top-level document with optional hashbang
// =============================================================================

/// A top-level ALACS file, which is a Dict with an optional hashbang.
#[derive(Clone, Default)]
pub struct File<'a> {
    pub entries: HashMap<&'a str, (Key<'a>, Value<'a>)>,
    pub hashbang: Option<Comment<'a>>,
    pub comment_intro: Option<Comment<'a>>,
}

impl<'a> File<'a> {
    /// Create empty file.
    pub fn new() -> Self {
        Self {
            entries: HashMap::new(),
            hashbang: None,
            comment_intro: None,
        }
    }

    /// Number of entries.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// True if empty.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Insert a key-value pair.
    pub fn insert(&mut self, key: Key<'a>, value: Value<'a>) {
        self.entries.insert(key.name, (key, value));
    }

    /// Get value by key name.
    pub fn get(&self, key: &str) -> Option<&Value<'a>> {
        self.entries.get(key).map(|(_, v)| v)
    }

    /// Get mutable value by key name.
    pub fn get_mut(&mut self, key: &str) -> Option<&mut Value<'a>> {
        self.entries.get_mut(key).map(|(_, v)| v)
    }

    /// Get key and value by key name.
    pub fn get_entry(&self, key: &str) -> Option<(&Key<'a>, &Value<'a>)> {
        self.entries.get(key).map(|(k, v)| (k, v))
    }

    /// Iterate over (key, value) pairs.
    pub fn iter(&self) -> impl Iterator<Item = (&Key<'a>, &Value<'a>)> {
        self.entries.values().map(|(k, v)| (k, v))
    }

    /// Iterate mutably over values.
    pub fn values_mut(&mut self) -> impl Iterator<Item = &mut Value<'a>> {
        self.entries.values_mut().map(|(_, v)| v)
    }
}

impl fmt::Debug for File<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "File(")?;
        if let Some(ref c) = self.hashbang {
            write!(f, "hashbang={:?},", c)?;
        }
        if let Some(ref c) = self.comment_intro {
            write!(f, "intro={:?},", c)?;
        }
        for (key, value) in self.iter() {
            write!(f, "{}={:?},", key.name, value)?;
        }
        write!(f, ")")
    }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn utf8_from_line() {
        let data = b"hello world";
        let utf8 = Utf8::from_line(data);
        assert_eq!(utf8.len(), 1);
        assert_eq!(utf8.get(0), Some(&b"hello world"[..]));
    }

    #[test]
    fn utf8_normalize_splits_newlines() {
        let data = b"line1\nline2\nline3";
        let mut utf8 = Utf8::from_line(data);
        utf8.normalize();
        assert_eq!(utf8.len(), 3);
        assert_eq!(utf8.get(0), Some(&b"line1"[..]));
        assert_eq!(utf8.get(1), Some(&b"line2"[..]));
        assert_eq!(utf8.get(2), Some(&b"line3"[..]));
    }

    #[test]
    fn utf8_normalize_clears_single_empty() {
        let data = b"";
        let mut utf8 = Utf8::from_line(data);
        utf8.normalize();
        assert!(utf8.is_empty());
    }

    #[test]
    fn text_to_string() {
        let data = b"hello";
        let text = Text::from_line(data);
        assert_eq!(text.to_string(), "hello");
    }

    #[test]
    fn dict_insert_and_get() {
        let key_name = "name";
        let value_data = b"value";
        let mut dict = Dict::new();
        dict.insert(Key::new(key_name), Text::from_line(value_data).into());
        assert!(dict.get("name").is_some());
    }

    #[test]
    #[should_panic(expected = "newline in key")]
    fn key_rejects_newline() {
        Key::new("bad\nkey");
    }

    #[test]
    fn file_borrows_from_buffer() {
        // Demonstrates the borrowing pattern
        let buffer = b"key=value";
        let key_slice = std::str::from_utf8(&buffer[0..3]).unwrap();
        let value_slice = &buffer[4..9];

        let mut file = File::new();
        file.insert(Key::new(key_slice), Text::from_line(value_slice).into());

        assert_eq!(file.len(), 1);
        // buffer is still accessible here - file borrows from it
        assert_eq!(&buffer[0..3], b"key");
    }
}
