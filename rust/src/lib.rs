//! ALACS file format and tools
//!
//! Data structures for representing text with comments.
//! Values can be Text, List, or Dict - augmented with comment metadata.
//! Users can manipulate these like standard collections while comments are preserved.

use std::collections::HashMap;
use std::fmt;
use std::ops::Range;
use std::rc::Rc;

// =============================================================================
// Bytes - zero-copy slice into a shared buffer
// =============================================================================

/// A reference to a slice of bytes within a shared buffer.
/// Enables zero-copy parsing by sharing a single allocation.
#[derive(Clone, PartialEq, Eq, Hash)]
pub struct Bytes {
    buffer: Rc<[u8]>,
    range: Range<usize>,
}

impl Bytes {
    /// Create from owned data, taking ownership of the entire buffer.
    pub fn new(data: impl Into<Rc<[u8]>>) -> Self {
        let buffer: Rc<[u8]> = data.into();
        let range = 0..buffer.len();
        Self { buffer, range }
    }

    /// Create an empty Bytes.
    pub fn empty() -> Self {
        Self {
            buffer: Rc::from([]),
            range: 0..0,
        }
    }

    /// View the bytes as a slice.
    pub fn as_bytes(&self) -> &[u8] {
        &self.buffer[self.range.clone()]
    }

    /// Create a sub-slice sharing the same underlying buffer.
    pub fn slice(&self, range: Range<usize>) -> Self {
        let start = self.range.start + range.start;
        let end = self.range.start + range.end;
        debug_assert!(end <= self.range.end);
        Self {
            buffer: Rc::clone(&self.buffer),
            range: start..end,
        }
    }

    /// Length in bytes.
    pub fn len(&self) -> usize {
        self.range.len()
    }

    /// True if empty.
    pub fn is_empty(&self) -> bool {
        self.range.is_empty()
    }

    /// Decode as UTF-8 string.
    pub fn to_string_lossy(&self) -> String {
        String::from_utf8_lossy(self.as_bytes()).into_owned()
    }
}

impl Default for Bytes {
    fn default() -> Self {
        Self::empty()
    }
}

impl fmt::Debug for Bytes {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Bytes({:?})", self.as_bytes())
    }
}

impl From<&[u8]> for Bytes {
    fn from(data: &[u8]) -> Self {
        Self::new(data.to_vec())
    }
}

impl From<Vec<u8>> for Bytes {
    fn from(data: Vec<u8>) -> Self {
        Self::new(data)
    }
}

impl From<&str> for Bytes {
    fn from(s: &str) -> Self {
        Self::new(s.as_bytes().to_vec())
    }
}

impl From<String> for Bytes {
    fn from(s: String) -> Self {
        Self::new(s.into_bytes())
    }
}

// =============================================================================
// UTF8 - a list of byte slices representing lines
// =============================================================================

/// A list of byte slices, each representing a line of UTF-8 text.
/// Lines are joined with newlines when converted to bytes or string.
#[derive(Clone, Default, PartialEq, Eq, Hash)]
pub struct Utf8 {
    lines: Vec<Bytes>,
}

impl Utf8 {
    /// Create empty.
    pub fn new() -> Self {
        Self { lines: Vec::new() }
    }

    /// Create from a single line.
    pub fn from_line(line: Bytes) -> Self {
        Self { lines: vec![line] }
    }

    /// Create from multiple lines.
    pub fn from_lines(lines: impl IntoIterator<Item = Bytes>) -> Self {
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
    pub fn push(&mut self, line: Bytes) {
        self.lines.push(line);
    }

    /// Get a line by index.
    pub fn get(&self, index: usize) -> Option<&Bytes> {
        self.lines.get(index)
    }

    /// Iterate over lines.
    pub fn iter(&self) -> impl Iterator<Item = &Bytes> {
        self.lines.iter()
    }

    /// Join all lines with newlines into a single byte vector.
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut result = Vec::new();
        for (i, line) in self.lines.iter().enumerate() {
            if i > 0 {
                result.push(b'\n');
            }
            result.extend_from_slice(line.as_bytes());
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
            let bytes = self.lines[i].as_bytes();
            if bytes.iter().any(|&b| b == b'\n') {
                // This line contains a newline - split it
                let line = self.lines.remove(i);
                let mut start = 0;
                let bytes = line.as_bytes();
                for (j, &b) in bytes.iter().enumerate() {
                    if b == b'\n' {
                        self.lines.insert(i, line.slice(start..j));
                        i += 1;
                        start = j + 1;
                    }
                }
                // Insert the remainder
                self.lines.insert(i, line.slice(start..bytes.len()));
            }
            i += 1;
        }
    }
}

impl fmt::Debug for Utf8 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Utf8({:?})", self.to_bytes())
    }
}

// =============================================================================
// Comment - just a Utf8 with semantic meaning
// =============================================================================

/// A comment block: one or more lines of comment text.
pub type Comment = Utf8;

// =============================================================================
// Value - the trait shared by Text, List, and Dict
// =============================================================================

/// A value that can appear in an ALACS structure.
/// All values can have a trailing comment.
#[derive(Clone)]
pub enum Value {
    Text(Text),
    List(List),
    Dict(Dict),
}

impl Value {
    /// Get the comment after this value, if any.
    pub fn comment_after(&self) -> Option<&Comment> {
        match self {
            Value::Text(t) => t.comment_after.as_ref(),
            Value::List(l) => l.comment_after.as_ref(),
            Value::Dict(d) => d.comment_after.as_ref(),
        }
    }

    /// Set the comment after this value.
    pub fn set_comment_after(&mut self, comment: Option<Comment>) {
        match self {
            Value::Text(t) => t.comment_after = comment,
            Value::List(l) => l.comment_after = comment,
            Value::Dict(d) => d.comment_after = comment,
        }
    }
}

impl fmt::Debug for Value {
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
pub struct Text {
    pub content: Utf8,
    pub comment_after: Option<Comment>,
}

impl Text {
    /// Create empty text.
    pub fn new() -> Self {
        Self {
            content: Utf8::new(),
            comment_after: None,
        }
    }

    /// Create from a single line.
    pub fn from_line(line: Bytes) -> Self {
        Self {
            content: Utf8::from_line(line),
            comment_after: None,
        }
    }

    /// Create from a string (will be encoded as UTF-8).
    pub fn from_str(s: &str) -> Self {
        Self {
            content: Utf8::from_line(Bytes::from(s)),
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

impl fmt::Debug for Text {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Text({:?}", self.content)?;
        if let Some(ref c) = self.comment_after {
            write!(f, ",after={:?}", c)?;
        }
        write!(f, ")")
    }
}

impl From<Text> for Value {
    fn from(t: Text) -> Self {
        Value::Text(t)
    }
}

// =============================================================================
// List - a sequence of Values with optional comments
// =============================================================================

/// A list of values with optional intro and trailing comments.
#[derive(Clone, Default)]
pub struct List {
    pub items: Vec<Value>,
    pub comment_intro: Option<Comment>,
    pub comment_after: Option<Comment>,
}

impl List {
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
    pub fn push(&mut self, value: Value) {
        self.items.push(value);
    }

    /// Get item by index.
    pub fn get(&self, index: usize) -> Option<&Value> {
        self.items.get(index)
    }

    /// Get mutable item by index.
    pub fn get_mut(&mut self, index: usize) -> Option<&mut Value> {
        self.items.get_mut(index)
    }

    /// Iterate over items.
    pub fn iter(&self) -> impl Iterator<Item = &Value> {
        self.items.iter()
    }

    /// Iterate mutably over items.
    pub fn iter_mut(&mut self) -> impl Iterator<Item = &mut Value> {
        self.items.iter_mut()
    }
}

impl fmt::Debug for List {
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

impl From<List> for Value {
    fn from(l: List) -> Self {
        Value::List(l)
    }
}

// =============================================================================
// Key - a string key with optional preceding blank line and comment
// =============================================================================

/// A dictionary key with optional formatting metadata.
#[derive(Clone, PartialEq, Eq, Hash)]
pub struct Key {
    pub name: String,
    pub blank_line_before: bool,
    pub comment_before: Option<Comment>,
}

impl Key {
    /// Create a key from a string.
    pub fn new(name: impl Into<String>) -> Self {
        let name = name.into();
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
    pub fn as_str(&self) -> &str {
        &self.name
    }
}

impl fmt::Debug for Key {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Key({:?})", self.name)
    }
}

impl fmt::Display for Key {
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
pub struct Dict {
    pub entries: HashMap<String, (Key, Value)>,
    pub comment_intro: Option<Comment>,
    pub comment_after: Option<Comment>,
}

impl Dict {
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
    pub fn insert(&mut self, key: Key, value: Value) {
        self.entries.insert(key.name.clone(), (key, value));
    }

    /// Get value by key name.
    pub fn get(&self, key: &str) -> Option<&Value> {
        self.entries.get(key).map(|(_, v)| v)
    }

    /// Get mutable value by key name.
    pub fn get_mut(&mut self, key: &str) -> Option<&mut Value> {
        self.entries.get_mut(key).map(|(_, v)| v)
    }

    /// Get key and value by key name.
    pub fn get_entry(&self, key: &str) -> Option<(&Key, &Value)> {
        self.entries.get(key).map(|(k, v)| (k, v))
    }

    /// Iterate over (key, value) pairs.
    pub fn iter(&self) -> impl Iterator<Item = (&Key, &Value)> {
        self.entries.values().map(|(k, v)| (k, v))
    }

    /// Iterate mutably over values (keys are immutable for HashMap consistency).
    pub fn values_mut(&mut self) -> impl Iterator<Item = &mut Value> {
        self.entries.values_mut().map(|(_, v)| v)
    }
}

impl fmt::Debug for Dict {
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

impl From<Dict> for Value {
    fn from(d: Dict) -> Self {
        Value::Dict(d)
    }
}

// =============================================================================
// File - top-level document with optional hashbang
// =============================================================================

/// A top-level ALACS file, which is a Dict with an optional hashbang.
#[derive(Clone, Default)]
pub struct File {
    pub entries: HashMap<String, (Key, Value)>,
    pub hashbang: Option<Comment>,
    pub comment_intro: Option<Comment>,
}

impl File {
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
    pub fn insert(&mut self, key: Key, value: Value) {
        self.entries.insert(key.name.clone(), (key, value));
    }

    /// Get value by key name.
    pub fn get(&self, key: &str) -> Option<&Value> {
        self.entries.get(key).map(|(_, v)| v)
    }

    /// Get mutable value by key name.
    pub fn get_mut(&mut self, key: &str) -> Option<&mut Value> {
        self.entries.get_mut(key).map(|(_, v)| v)
    }

    /// Get key and value by key name.
    pub fn get_entry(&self, key: &str) -> Option<(&Key, &Value)> {
        self.entries.get(key).map(|(k, v)| (k, v))
    }

    /// Iterate over (key, value) pairs.
    pub fn iter(&self) -> impl Iterator<Item = (&Key, &Value)> {
        self.entries.values().map(|(k, v)| (k, v))
    }

    /// Iterate mutably over values.
    pub fn values_mut(&mut self) -> impl Iterator<Item = &mut Value> {
        self.entries.values_mut().map(|(_, v)| v)
    }
}

impl fmt::Debug for File {
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
    fn bytes_slice_shares_buffer() {
        let original = Bytes::from("hello world");
        let slice = original.slice(0..5);
        assert_eq!(slice.as_bytes(), b"hello");
        // Both share the same underlying Rc
        assert!(Rc::ptr_eq(&original.buffer, &slice.buffer));
    }

    #[test]
    fn utf8_normalize_splits_newlines() {
        let mut utf8 = Utf8::from_line(Bytes::from("line1\nline2\nline3"));
        utf8.normalize();
        assert_eq!(utf8.len(), 3);
        assert_eq!(utf8.get(0).unwrap().as_bytes(), b"line1");
        assert_eq!(utf8.get(1).unwrap().as_bytes(), b"line2");
        assert_eq!(utf8.get(2).unwrap().as_bytes(), b"line3");
    }

    #[test]
    fn utf8_normalize_clears_single_empty() {
        let mut utf8 = Utf8::from_line(Bytes::empty());
        utf8.normalize();
        assert!(utf8.is_empty());
    }

    #[test]
    fn text_to_string() {
        let text = Text::from_str("hello");
        assert_eq!(text.to_string(), "hello");
    }

    #[test]
    fn dict_insert_and_get() {
        let mut dict = Dict::new();
        dict.insert(Key::new("name"), Text::from_str("value").into());
        assert!(dict.get("name").is_some());
    }

    #[test]
    #[should_panic(expected = "newline in key")]
    fn key_rejects_newline() {
        Key::new("bad\nkey");
    }
}
