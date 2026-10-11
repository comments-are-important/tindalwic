//! code for encoding data into the Tindalwic format.

use crate::parse::ParseError;
use crate::{CommentMarker, Entries, Entry, File, Item, Items, Text, Value};

use core::fmt::{Debug, Display, Formatter, Result, Write};
use core::write;

impl<'a> Debug for Value<'a> {
    fn fmt(&self, out: &mut Formatter<'_>) -> Result {
        out.debug_list().entries(self.lines()).finish()
    }
}
impl<'a> Debug for Items<'a> {
    fn fmt(&self, out: &mut Formatter<'_>) -> Result {
        let mut items = out.debug_struct("Items");
        for item in self {
            items.field("cells", &item);
        }
        items.finish()
    }
}
impl<'a> Debug for Entries<'a> {
    fn fmt(&self, out: &mut Formatter<'_>) -> Result {
        let mut entries = out.debug_struct("Entries");
        for entry in self {
            entries.field("cells", &entry);
        }
        entries.finish()
    }
}

impl Display for ParseError {
    fn fmt(&self, out: &mut Formatter<'_>) -> Result {
        match self {
            ParseError::Memory(message) => write!(out, "0: error: {message}"),
            ParseError::Syntax {
                start,
                end,
                message,
            } => {
                let last = end - 1;
                if *start >= last {
                    write!(out, "{start}: error: {message}")
                } else {
                    write!(out, "{start}: error: (thru line {last}) {message}")
                }
            }
        }
    }
}

/*
pub struct Indented<'a, T> {
    pub thing: &'a T,
    pub indent: usize,
}

impl<'a, T> Indented<'a, T> {
    pub fn new(thing: &'a T, indent: usize) -> Self {
        Self { thing, indent }
    }
}

impl Display for Indented<'_, Node> {
}
*/

/// the string value (without indentation, *not* the encoded form).
impl<'a> Display for Value<'a> {
    fn fmt(&self, out: &mut Formatter<'_>) -> Result {
        if let Some(verbatim) = self.verbatim(0) {
            out.write_str(verbatim)
        } else {
            let mut lines = self.lines();
            if let Some(first) = lines.next() {
                out.write_str(first)?;
                for line in lines {
                    out.write_char('\n')?;
                    out.write_str(line)?;
                }
            }
            Ok(())
        }
    }
}

impl<'a> Display for File<'a> {
    fn fmt(&self, out: &mut Formatter<'_>) -> Result {
        Output {
            out,
            indent: 0,
            empty: true,
        }
        .file(self)
    }
}

struct Output<'o, 'f> {
    out: &'o mut Formatter<'f>,
    indent: usize,
    empty: bool,
}
impl<'o, 'f> Output<'o, 'f> {
    fn indent(&mut self) -> Result {
        if self.empty {
            self.empty = false;
        } else {
            self.out.write_char('\n')?;
        }
        for _ in 0..self.indent {
            self.out.write_char('\t')?;
        }
        Ok(())
    }
    const fn special_first(byte: u8) -> bool {
        matches!(
            byte,
            b'\t' | b'#' | b'%' | b'<' | b'>' | b'@' | b'[' | b']' | b'{' | b'}' | b'='
        )
    }
    fn string<'a>(&mut self, value: &Value<'a>) -> Result {
        if let Some(slice) = value.verbatim(self.indent) {
            self.out.write_str(slice)?;
        } else {
            let mut lines = value.lines();
            if let Some(first) = lines.next() {
                self.out.write_str(first)?;
                for line in lines {
                    self.indent()?;
                    self.out.write_str(line)?;
                }
            }
        }
        Ok(())
    }
    fn prolog<'a>(&mut self, comment: &Option<Value<'a>>) -> Result {
        self.comment(CommentMarker::Prolog.token(), comment)
    }
    fn epilog<'a>(&mut self, comment: &Option<Value<'a>>) -> Result {
        self.comment(CommentMarker::Epilog.token(), comment)
    }
    fn comment<'a>(&mut self, marker: &'a str, comment: &Option<Value<'a>>) -> Result {
        self.maybe(marker, &comment)?;
        Ok(())
    }
    fn maybe<'a>(&mut self, marker: &'a str, option: &Option<Value<'a>>) -> Result {
        if let Some(value) = option {
            self.indent()?;
            self.out.write_str(marker)?;
            self.indent += 1;
            self.string(&value)?;
            self.indent -= 1;
        }
        Ok(())
    }

    fn one_liner_in_list<'a>(text: &Text<'a>) -> Option<&'a str> {
        let value = text.value.only_line()?;
        if text.longer || value.is_empty() || Output::special_first(value.as_bytes()[0]) {
            None
        } else {
            Some(value)
        }
    }

    fn one_line_key<'a>(entry: &Entry<'a>) -> Option<&'a str> {
        if entry.longer {
            return None;
        }
        entry.name.key.only_line()
    }

    fn one_liner_in_dict<'a>(entry: &Entry<'a>, text: &Text<'a>) -> Option<(&'a str, &'a str)> {
        let value = text.value.only_line()?;
        let key = entry.name.key.only_line()?;
        if entry.longer || entry.name.longer || text.longer {
            None
        } else if key.is_empty() {
            Some((key, value))
        } else if Output::special_first(key.as_bytes()[0]) || key.contains('=') {
            None
        } else {
            Some((key, value))
        }
    }

    fn item_in_list<'a>(&mut self, item: &Item<'a>) -> Result {
        match item {
            Item::Text(text) => {
                self.indent()?;
                if let Some(slice) = Output::one_liner_in_list(&text) {
                    self.out.write_str(slice)?;
                } else {
                    self.out.write_str("<>")?;
                    self.indent += 1;
                    self.indent()?;
                    self.string(&text.value)?;
                    self.indent -= 1;
                }
                self.comment(CommentMarker::Epilog.token(), &text.epilog)
            }
            Item::List(list) => {
                self.indent()?;
                self.out.write_str("[]")?;
                self.indent += 1;
                self.prolog(&list.prolog)?;
                for item in &list.items {
                    self.item_in_list(&item)?;
                }
                self.indent -= 1;
                self.epilog(&list.epilog)
            }
            Item::Dict(dict) => {
                self.indent()?;
                self.out.write_str("{}")?;
                self.indent += 1;
                self.prolog(&dict.prolog)?;
                for entry in &dict.entries {
                    self.entry_in_dict(&entry)?;
                }
                self.indent -= 1;
                self.epilog(&dict.epilog)
            }
        }
    }
    fn entry_in_dict<'a>(&mut self, entry: &Entry<'a>) -> Result {
        self.comment(CommentMarker::NameTag.token(), &entry.name.comment)?;
        match &entry.item {
            Item::Text(text) => {
                self.indent()?;
                if let Some((key, slice)) = Output::one_liner_in_dict(&entry, &text) {
                    self.out.write_str(key)?;
                    self.out.write_char('=')?;
                    self.out.write_str(slice)?;
                } else if let Some(key) = Output::one_line_key(&entry) {
                    self.out.write_char('<')?;
                    self.out.write_str(key)?;
                    self.out.write_str(">")?;
                    self.indent += 1;
                    self.indent()?;
                    self.string(&text.value)?;
                    self.indent -= 1;
                } else {
                    self.out.write_char('@')?;
                    self.indent += 1;
                    self.empty = false;
                    self.indent()?;
                    self.string(&entry.name.key)?;
                    self.indent -= 1;
                    self.indent()?;
                    self.out.write_str("<>")?;
                    self.indent += 1;
                    self.indent()?;
                    self.string(&text.value)?;
                    self.indent -= 1;
                }
                self.comment(CommentMarker::Epilog.token(), &text.epilog)
            }
            Item::List(list) => {
                self.indent()?;
                if let Some(only) = entry.name.key.only_line() {
                    self.out.write_char('[')?;
                    self.out.write_str(only)?;
                    self.out.write_str("]")?;
                } else {
                    self.out.write_char('@')?;
                    self.indent += 1;
                    self.empty = false;
                    self.indent()?;
                    self.string(&entry.name.key)?;
                    self.indent -= 1;
                    self.indent()?;
                    self.out.write_str("[]")?;
                }
                self.indent += 1;
                self.prolog(&list.prolog)?;
                for item in &list.items {
                    self.item_in_list(&item)?;
                }
                self.indent -= 1;
                self.epilog(&list.epilog)
            }
            Item::Dict(dict) => {
                self.indent()?;
                if let Some(only) = entry.name.key.only_line() {
                    self.out.write_char('{')?;
                    self.out.write_str(only)?;
                    self.out.write_str("}")?;
                } else {
                    self.out.write_char('@')?;
                    self.indent += 1;
                    self.empty = false;
                    self.indent()?;
                    self.string(&entry.name.key)?;
                    self.indent -= 1;
                    self.indent()?;
                    self.out.write_str("{}")?;
                }
                self.indent += 1;
                self.prolog(&dict.prolog)?;
                for entry in &dict.entries {
                    self.entry_in_dict(&entry)?;
                }
                self.indent -= 1;
                self.epilog(&dict.epilog)
            }
        }
    }
    fn file<'a>(&mut self, file: &File<'a>) -> Result {
        self.maybe(CommentMarker::Shebang.token(), &file.shebang)?;
        self.prolog(&file.prolog)?;
        for entry in &file.entries {
            self.entry_in_dict(&entry)?;
        }
        for _ in 0..file.trailing {
            self.out.write_char('\n')?;
        }
        Ok(())
    }
}
