//! code for encoding data into the Tindalwic format.

use crate::parse::ParseError;
use crate::{CommentMarker, Data, File, Group, Lines, Member, Text};

use core::fmt::{Display, Formatter, Result, Write as _};
use core::write;

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

struct Output<'o, 'f> {
    out: &'o mut Formatter<'f>,
    indent: usize,
    empty: bool,
}
impl<'o, 'f> Output<'o, 'f> {
    fn new(out: &'o mut Formatter<'f>) -> Self {
        Output {
            out,
            indent: 0,
            empty: true,
        }
    }
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
    fn utf8<'a>(&mut self, value: &Lines<'a>) -> Result {
        if let Some(slice) = value.verbatim(self.indent) {
            self.out.write_str(slice)?;
        } else {
            let mut lines = value.into_iter();
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
    fn comment<'a>(&mut self, marker: CommentMarker, option: &Option<Lines<'a>>) -> Result {
        if let Some(value) = option {
            self.indent()?;
            self.out.write_str(marker.token())?;
            self.indent += 1;
            self.utf8(&value)?;
            self.indent -= 1;
        }
        Ok(())
    }
    // fn one_liner_in_list<'a>(text: &Text<'a>) -> Option<&'a str> {
    //     let value = text.lines.only_line()?;
    //     if text.verbose || value.is_empty() || Output::special_first(value.as_bytes()[0]) {
    //         None
    //     } else {
    //         Some(value)
    //     }
    // }
    fn one_line_key<'a>(member: &Member<'a>) -> Option<&'a str> {
        if member.longer {
            return None;
        }
        member.name.only_line()
    }
    fn one_liner_in_dict<'a>(member: &Member<'a>, text: &Text<'a>) -> Option<(&'a str, &'a str)> {
        let value = text.lines.only_line()?;
        let key = member.name.only_line()?;
        if member.longer || text.verbose {
            None
        } else if key.is_empty() {
            Some((key, value))
        } else if Output::special_first(key.as_bytes()[0]) || key.contains('=') {
            None
        } else {
            Some((key, value))
        }
    }
    fn text<'a>(&mut self, text: &Text<'a>) -> Result {
        self.indent += 1;
        self.indent()?;
        self.utf8(&text.lines)?;
        self.indent -= 1;
        self.comment(CommentMarker::Epilog, &text.epilog)
    }
    fn member<'a>(&mut self, member: &Member<'a>) -> Result {
        self.comment(CommentMarker::Group, &member.comment)?;
        match &member.data {
            Data::Text(text) => {
                self.indent()?;
                if let Some((key, slice)) = Output::one_liner_in_dict(&member, &text) {
                    if member.name.is_empty() {
                        self.out.write_str(slice)?;
                    } else {
                        write!(self.out, "{key}={slice}")?;
                    }
                } else if let Some(key) = Output::one_line_key(&member) {
                    write!(self.out, "<{key}>")?;
                    self.text(text)?;
                } else {
                    self.out.write_char('@')?;
                    self.indent += 1;
                    self.empty = false;
                    self.indent()?;
                    self.utf8(&member.name)?;
                    self.indent -= 1;
                    self.indent()?;
                    self.out.write_str("<>")?;
                    self.text(text)?;
                }
            }
            Data::List(list) => {
                self.indent()?;
                if let Some(only) = member.name.only_line() {
                    write!(self.out, "[{only}]")?;
                } else {
                    self.out.write_char('@')?;
                    self.indent += 1;
                    self.empty = false;
                    self.indent()?;
                    self.utf8(&member.name)?;
                    self.indent -= 1;
                    self.indent()?;
                    self.out.write_str("{}")?;
                }
                self.indent += 1;
                self.comment(CommentMarker::Prolog, &list.prolog)?;
                for group in list.groups {
                    self.group(&group)?;
                }
                self.indent -= 1;
                self.comment(CommentMarker::Epilog, &list.epilog)?;
            }
        }
        Ok(())
    }
    fn group<'a>(&mut self, group: &Group<'a>) -> Result {
        self.comment(CommentMarker::Group, &group.heading)?;
        for member in group.members {
            self.member(&member)?;
        }
        Ok(())
    }
    fn file<'a>(&mut self, file: &File<'a>) -> Result {
        self.comment(CommentMarker::Shebang, &file.shebang)?;
        self.comment(CommentMarker::Prolog, &file.prolog)?;
        for group in file.groups {
            self.group(&group)?;
        }
        Ok(())
    }
}

impl<'a> Display for File<'a> {
    fn fmt(&self, out: &mut Formatter<'_>) -> Result {
        Output::new(out).file(self)
    }
}
impl<'a> Display for Group<'a> {
    fn fmt(&self, out: &mut Formatter<'_>) -> Result {
        Output::new(out).group(self)
    }
}
impl<'a> Display for Member<'a> {
    fn fmt(&self, out: &mut Formatter<'_>) -> Result {
        Output::new(out).member(self)
    }
}
impl<'a> Display for Text<'a> {
    fn fmt(&self, out: &mut Formatter<'_>) -> Result {
        Output::new(out).text(self)
    }
}
impl<'a> Display for Lines<'a> {
    fn fmt(&self, out: &mut Formatter<'_>) -> Result {
        Output::new(out).utf8(self)
    }
}
impl<'a> Display for Data<'a> {
    fn fmt(&self, out: &mut Formatter<'_>) -> Result {
        let member: Member = (*self).into();
        member.fmt(out)
    }
}
