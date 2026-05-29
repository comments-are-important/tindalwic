//! code for encoding data into the Tindalwic format.

use crate::Value;
use crate::parse::{MemoryError, ParseError, SyntaxError};
use crate::{Comment, Dict, Entry, File, Item, List, Text};

use core::cell::Cell;
use core::fmt::{Display, Formatter, Result, Write};

impl Display for ParseError {
    fn fmt(&self, out: &mut Formatter<'_>) -> Result {
        match self {
            ParseError::Memory(err) => err.fmt(out),
            ParseError::Syntax(err) => err.fmt(out),
        }
    }
}
impl Display for MemoryError {
    fn fmt(&self, out: &mut Formatter<'_>) -> Result {
        out.write_str(self.message)
    }
}
impl Display for SyntaxError {
    fn fmt(&self, out: &mut Formatter<'_>) -> Result {
        if self.start + 1 == self.end {
            write!(out, "{}: {}", self.start, self.message)
        } else {
            write!(out, "{}-{}: {}", self.start, self.end - 1, self.message)
        }
    }
}

struct Output<'o, 'f> {
    out: &'o mut Formatter<'f>,
    indent: usize,
}
impl<'o, 'f> Output<'o, 'f> {
    fn indent(&mut self) -> Result {
        for _ in 0..self.indent {
            self.out.write_char('\t')?;
        }
        Ok(())
    }
    fn encoded<'a>(&mut self, encoded: &Value<'a>) -> Result {
        if let Some(slice) = encoded.verbatim(self.indent) {
            self.out.write_str(slice)?;
            self.out.write_char('\n')?;
        } else {
            let mut lines = encoded.lines();
            if let Some(first) = lines.next() {
                self.out.write_str(first)?;
                self.out.write_char('\n')?;
                for line in lines {
                    self.indent()?;
                    self.out.write_str(line)?;
                    self.out.write_char('\n')?;
                }
            } else {
                self.out.write_char('\n')?;
            }
        }
        Ok(())
    }
    fn some_comment<'a>(&mut self, marker: &'a str, comment: &Comment<'a>) -> Result {
        self.indent()?;
        self.out.write_str(marker)?;
        if comment.value.is_empty() {
            self.out.write_char('\n')?;
        } else {
            self.indent += 1;
            if marker == "#" && (comment.value.starts_with('!') || comment.value.starts_with('\n'))
            {
                self.out.write_char('\n')?;
                self.indent()?;
            }
            self.encoded(&comment.value)?;
            self.indent -= 1;
        }
        Ok(())
    }
    fn comment<'a>(&mut self, marker: &'a str, option: &Option<Comment<'a>>) -> Result {
        if let Some(comment) = option {
            self.some_comment(marker, comment)?;
        }
        Ok(())
    }
    fn one_liner_in_list<'a>(text: &Text<'a>) -> Option<&'a str> {
        let only = text.value.only_line()?;
        if text.value.is_empty() {
            Some(only)
        } else if matches!(
            only.as_bytes()[0],
            b'\t' | b'#' | b'<' | b'>' | b'[' | b']' | b'{' | b'}' | b'/' | b'='
        ) {
            None
        } else {
            Some(only)
        }
    }
    fn text_in_list<'a>(&mut self, text: &Text<'a>) -> Result {
        self.indent()?;
        if let Some(slice) = Output::one_liner_in_list(text) {
            self.out.write_str(slice)?;
            self.out.write_char('\n')?;
        } else {
            self.out.write_str("<>\n")?;
            self.indent += 1;
            self.indent()?;
            self.encoded(&text.value)?;
            self.indent -= 1;
        }
        self.comment("#", &text.epilog)
    }
    fn one_liner_in_dict<'a>(text: &Text<'a>, key: &'_ str) -> Option<&'a str> {
        let only = text.value.only_line()?;
        if key.is_empty() {
            Some(only)
        } else if key.contains('=') {
            None
        } else if matches!(
            key.as_bytes()[0],
            b'\t' | b'#' | b'<' | b'>' | b'[' | b']' | b'{' | b'}' | b'/' | b'='
        ) {
            None
        } else {
            Some(only)
        }
    }
    fn text_in_dict<'a>(&mut self, key: &Value<'a>, text: &Text<'a>) -> Result {
        let first = key.lines().next().unwrap_or(""); // TODO key.one_liner
        self.indent()?;
        if let Some(slice) = Output::one_liner_in_dict(text, first) {
            self.out.write_str(first)?;
            self.out.write_char('=')?;
            self.out.write_str(slice)?;
            self.out.write_char('\n')?;
        } else {
            self.out.write_char('<')?;
            self.out.write_str(first)?;
            self.out.write_str(">\n")?;
            self.indent += 1;
            self.indent()?;
            self.encoded(&text.value)?;
            self.indent -= 1;
        }
        self.comment("#", &text.epilog)
    }
    fn list_in_list<'a>(&mut self, list: &List<'a>) -> Result {
        self.indent()?;
        self.out.write_str("[]\n")?;
        self.indent += 1;
        self.comment("#", &list.prolog)?;
        for cell in list.cells {
            self.item_in_list(cell)?;
        }
        self.indent -= 1;
        self.comment("#", &list.epilog)
    }
    fn list_in_dict<'a>(&mut self, key: &Value<'a>, list: &List<'a>) -> Result {
        let first = key.lines().next().unwrap_or(""); // TODO key.one_liner
        self.indent()?;
        self.out.write_char('[')?;
        self.out.write_str(first)?;
        self.out.write_str("]\n")?;
        self.indent += 1;
        self.comment("#", &list.prolog)?;
        for cell in list.cells {
            self.item_in_list(cell)?;
        }
        self.indent -= 1;
        self.comment("#", &list.epilog)
    }
    fn dict_in_list<'a>(&mut self, dict: &Dict<'a>) -> Result {
        self.indent()?;
        self.out.write_str("{}\n")?;
        self.indent += 1;
        self.comment("#", &dict.prolog)?;
        for cell in dict.cells {
            self.entry_in_dict(cell)?;
        }
        self.indent -= 1;
        self.comment("#", &dict.epilog)
    }
    fn dict_in_dict<'a>(&mut self, key: &Value<'a>, dict: &Dict<'a>) -> Result {
        let first = key.lines().next().unwrap_or(""); // TODO key.one_liner
        self.indent()?;
        self.out.write_char('{')?;
        self.out.write_str(first)?;
        self.out.write_str("}\n")?;
        self.indent += 1;
        self.comment("#", &dict.prolog)?;
        for cell in dict.cells {
            self.entry_in_dict(cell)?;
        }
        self.indent -= 1;
        self.comment("#", &dict.epilog)
    }
    fn item_in_list<'a>(&mut self, cell: &Cell<Item<'a>>) -> Result {
        let item = cell.get();
        match item {
            Item::Text(text) => self.text_in_list(&text),
            Item::List(list) => self.list_in_list(&list),
            Item::Dict(dict) => self.dict_in_list(&dict),
        }
    }
    fn entry_in_dict<'a>(&mut self, cell: &Cell<Entry<'a>>) -> Result {
        let entry = cell.get();
        if entry.gap {
            // TODO be strict? f.write_indent(self.indent)?;
            self.out.write_char('\n')?;
        }
        self.comment("//", &entry.before)?;
        match &entry.item {
            Item::Text(text) => self.text_in_dict(&entry.key, text),
            Item::List(list) => self.list_in_dict(&entry.key, list),
            Item::Dict(dict) => self.dict_in_dict(&entry.key, dict),
        }
    }
    fn file<'a>(&mut self, file: &File<'a>) -> Result {
        self.comment("#!", &file.hashbang)?;
        self.comment("#", &file.prolog)?;
        for cell in file.cells {
            self.entry_in_dict(&cell)?;
        }
        Ok(())
    }
}

/// Serialize using the "#" marker (ignoring any actual position).
///
/// # Examples
///
/// ```
/// use tindalwic::*;
/// fn check(gfm: &str) {
///     let expected = format!("#{}\n", gfm.replace("\n", "\n\t"));
///     let comment = Comment { value: gfm.into() };
///     assert_eq!(comment.to_string(), expected);
/// }
/// check("one-liner");
/// check("two\nlines");
/// check(
///     "# heading
/// paragraph
///     blockquote
/// ",
/// );
/// ```
impl<'a> Display for Comment<'a> {
    fn fmt(&self, out: &mut Formatter<'_>) -> Result {
        Output { out, indent: 0 }.some_comment("#", self)
    }
}

impl<'a> Display for Text<'a> {
    fn fmt(&self, out: &mut Formatter<'_>) -> Result {
        Output { out, indent: 0 }.text_in_list(self)
    }
}

impl<'a> Display for List<'a> {
    fn fmt(&self, out: &mut Formatter<'_>) -> Result {
        Output { out, indent: 0 }.list_in_list(self)
    }
}

impl<'a> Display for Dict<'a> {
    fn fmt(&self, out: &mut Formatter<'_>) -> Result {
        Output { out, indent: 0 }.dict_in_list(self)
    }
}

impl<'a> Display for Item<'a> {
    fn fmt(&self, out: &mut Formatter<'_>) -> Result {
        let mut out = Output { out, indent: 0 };
        match self {
            Item::Text(text) => out.text_in_list(text),
            Item::List(list) => out.list_in_list(list),
            Item::Dict(dict) => out.dict_in_list(dict),
        }
    }
}

impl<'a> Display for File<'a> {
    fn fmt(&self, out: &mut Formatter<'_>) -> Result {
        Output { out, indent: 0 }.file(self)
    }
}

// TODO file is good, but others include a superfluous introductory first line
