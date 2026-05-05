use super::*;

use core::fmt::{Display, Formatter, Result, Write};
use core::write;

impl Display for ParseError {
    fn fmt(&self, out: &mut Formatter<'_>) -> Result {
        if self.lines.start + 1 == self.lines.end {
            write!(out, "{}: {}", self.lines.start, self.message)
        } else {
            write!(
                out,
                "{}-{}: {}",
                self.lines.start,
                self.lines.end - 1,
                self.message
            )
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
    fn encoded<'a>(&mut self, encoded: &UTF8<'a>) -> Result {
        if self.indent == encoded.dedent || encoded.one_liner() {
            self.out.write_str(encoded.slice)?;
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
    fn some_comment<'a>(&mut self, marker: &'a str, comment: &Comment<'a>) -> Result {
        self.indent()?;
        self.out.write_str(marker)?;
        if comment.utf8.slice.is_empty() {
            self.out.write_char('\n')?;
        } else {
            self.indent += 1;
            if marker == "#" && comment.utf8.slice.starts_with('!') {
                self.out.write_char('\n')?;
                self.indent()?;
            }
            self.encoded(&comment.utf8)?;
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
    fn text_in_list<'a>(&mut self, text: &Text<'a>) -> Result {
        self.indent()?;
        if text.one_liner_in_list() {
            self.out.write_str(text.utf8.slice)?;
            self.out.write_char('\n')?;
        } else {
            self.out.write_str("<>\n")?;
            self.indent += 1;
            self.indent()?;
            self.encoded(&text.utf8)?;
            self.indent -= 1;
        }
        self.comment("#", &text.epilog)
    }
    fn text_in_dict<'a>(&mut self, key: &'a str, text: &Text<'a>) -> Result {
        self.indent()?;
        if text.one_liner_in_dict(key) {
            self.out.write_str(key)?;
            self.out.write_char('=')?;
            self.out.write_str(text.utf8.slice)?;
            self.out.write_char('\n')?;
        } else {
            self.out.write_char('<')?;
            self.out.write_str(key)?;
            self.out.write_str(">\n")?;
            self.indent += 1;
            self.indent()?;
            self.encoded(&text.utf8)?;
            self.indent -= 1;
        }
        self.comment("#", &text.epilog)
    }
    fn list_in_list<'a, 'store>(&mut self, list: &List<'a, 'store>) -> Result {
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
    fn list_in_dict<'a, 'store>(&mut self, key: &'a str, list: &List<'a, 'store>) -> Result {
        self.indent()?;
        self.out.write_char('[')?;
        self.out.write_str(key)?;
        self.out.write_str("]\n")?;
        self.indent += 1;
        self.comment("#", &list.prolog)?;
        for cell in list.cells {
            self.item_in_list(cell)?;
        }
        self.indent -= 1;
        self.comment("#", &list.epilog)
    }
    fn dict_in_list<'a, 'store>(&mut self, dict: &Dict<'a, 'store>) -> Result {
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
    fn dict_in_dict<'a, 'store>(&mut self, key: &'a str, dict: &Dict<'a, 'store>) -> Result {
        self.indent()?;
        self.out.write_char('{')?;
        self.out.write_str(key)?;
        self.out.write_str("}\n")?;
        self.indent += 1;
        self.comment("#", &dict.prolog)?;
        for cell in dict.cells {
            self.entry_in_dict(cell)?;
        }
        self.indent -= 1;
        self.comment("#", &dict.epilog)
    }
    fn item_in_list<'a, 'store>(&mut self, cell: &Cell<Item<'a, 'store>>) -> Result {
        let item = cell.get();
        match item {
            Item::Text(text) => self.text_in_list(&text),
            Item::List(list) => self.list_in_list(&list),
            Item::Dict(dict) => self.dict_in_list(&dict),
        }
    }
    fn entry_in_dict<'a, 'store>(&mut self, cell: &Cell<Entry<'a, 'store>>) -> Result {
        let entry = cell.get();
        if entry.name.gap {
            // TODO be strict? f.write_indent(self.indent)?;
            self.out.write_char('\n')?;
        }
        self.comment("//", &entry.name.before)?;
        match &entry.item {
            Item::Text(text) => self.text_in_dict(entry.name.key, text),
            Item::List(list) => self.list_in_dict(entry.name.key, list),
            Item::Dict(dict) => self.dict_in_dict(entry.name.key, dict),
        }
    }
    fn file<'a, 'store>(&mut self, file: &File<'a, 'store>) -> Result {
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
    fn fmt(&self, out: &mut Formatter<'_>) -> Result {
        Output { out, indent: 0 }.some_comment("#", self)
    }
}

impl<'a> Display for Text<'a> {
    fn fmt(&self, out: &mut Formatter<'_>) -> Result {
        Output { out, indent: 0 }.text_in_list(self)
    }
}

impl<'a, 'store> Display for List<'a, 'store> {
    fn fmt(&self, out: &mut Formatter<'_>) -> Result {
        Output { out, indent: 0 }.list_in_list(self)
    }
}

impl<'a, 'store> Display for Dict<'a, 'store> {
    fn fmt(&self, out: &mut Formatter<'_>) -> Result {
        Output { out, indent: 0 }.dict_in_list(self)
    }
}

impl<'a, 'store> Display for Item<'a, 'store> {
    fn fmt(&self, out: &mut Formatter<'_>) -> Result {
        let mut out = Output { out, indent: 0 };
        match self {
            Item::Text(text) => out.text_in_list(text),
            Item::List(list) => out.list_in_list(list),
            Item::Dict(dict) => out.dict_in_list(dict),
        }
    }
}

impl<'a, 'store> Display for File<'a, 'store> {
    fn fmt(&self, out: &mut Formatter<'_>) -> Result {
        Output { out, indent: 0 }.file(self)
    }
}

// TODO file is good, but others include a superfluous introductory first line
