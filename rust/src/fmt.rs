use super::*;

use core::fmt::{Display, Formatter, Result, Write};

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
    fn encoded<'a>(&mut self, encoded: &Encoded<'a>) -> Result {
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
    fn some_comment<'a>(&mut self, marker: &'a str, comment: &Comment<'a>) -> Result {
        self.indent()?;
        self.out.write_str(marker)?;
        self.indent += 1;
        self.encoded(&comment.encoded)?;
        self.indent -= 1;
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
    fn text_in_dict<'a>(&mut self, key: &'a str, text: &Text<'a>) -> Result {
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
    fn list_in_list<'a>(&mut self, list: &List<'a>) -> Result {
        self.comment("#", &list.prolog)?;
        self.indent()?;
        self.out.write_str("[]\n")?;
        self.indent += 1;
        for value in list.cells {
            self.value_in_list(value)?;
        }
        self.indent -= 1;
        self.comment("#", &list.epilog)
    }
    fn list_in_dict<'a>(&mut self, key: &'a str, list: &List<'a>) -> Result {
        self.indent()?;
        self.out.write_char('[')?;
        self.out.write_str(key)?;
        self.out.write_str("]\n")?;
        self.indent += 1;
        for value in list.cells {
            self.value_in_list(value)?;
        }
        self.indent -= 1;
        self.comment("#", &list.epilog)
    }
    fn dict_in_list<'a>(&mut self, dict: &Dict<'a>) -> Result {
        self.indent()?;
        self.out.write_str("{}\n")?;
        self.indent += 1;
        for keyed in dict.cells {
            self.value_in_dict(keyed)?;
        }
        self.indent -= 1;
        self.comment("#", &dict.epilog)
    }
    fn dict_in_dict<'a>(&mut self, key: &'a str, dict: &Dict<'a>) -> Result {
        self.indent()?;
        self.out.write_char('{')?;
        self.out.write_str(key)?;
        self.out.write_str("}\n")?;
        self.indent += 1;
        for keyed in dict.cells {
            self.value_in_dict(keyed)?;
        }
        self.indent -= 1;
        self.comment("#", &dict.epilog)
    }
    fn value_in_list<'a>(&mut self, cell: &Cell<Value<'a>>) -> Result {
        let value = cell.get();
        match value {
            Value::Text(text) => self.text_in_list(&text),
            Value::List(list) => self.list_in_list(&list),
            Value::Dict(dict) => self.dict_in_list(&dict),
        }
    }
    fn value_in_dict<'a>(&mut self, cell: &Cell<Keyed<'a>>) -> Result {
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
    fn file<'a>(&mut self, file: &File<'a>) -> Result {
        self.comment("#!", &file.hashbang)?;
        self.comment("#", &file.prolog)?;
        for keyed in file.cells {
            self.value_in_dict(&keyed)?;
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

impl<'a> Display for Value<'a> {
    fn fmt(&self, out: &mut Formatter<'_>) -> Result {
        let mut out = Output { out, indent: 0 };
        match self {
            Value::Text(text) => out.text_in_list(text),
            Value::List(list) => out.list_in_list(list),
            Value::Dict(dict) => out.dict_in_list(dict),
        }
    }
}

impl<'a> Display for File<'a> {
    fn fmt(&self, out: &mut Formatter<'_>) -> Result {
        Output { out, indent: 0 }.file(self)
    }
}
