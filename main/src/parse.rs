//! everything related to converting bytes into a File

use core::usize;

use crate::Value;
use crate::{Comment, Dict, Entry, File, Item, List, Text};

/// parsing problems
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ParseError {
    /// a problem in the Tindalwic
    Syntax(SyntaxError),
    /// ran out of room in the storage
    Memory(MemoryError),
}
impl core::error::Error for ParseError {}
impl ParseError {
    /// make a Syntax error with an arbitrary span of lines.
    pub(crate) fn new(start: usize, end: usize, message: &'static str) -> Self {
        ParseError::Syntax(SyntaxError {
            start,
            end,
            message,
        })
    }
    /// make a Syntax error for a single line.
    pub(crate) fn at(line: usize, message: &'static str) -> Self {
        ParseError::new(line, line + 1, message)
    }
    /// make a Memory error
    pub(crate) fn mem(message: &'static str) -> Self {
        ParseError::Memory(MemoryError { message })
    }
}

/// ran out of room in the storage
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MemoryError {
    /// English description of the problem
    pub message: &'static str,
}

/// a problem in the Tindalwic
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SyntaxError {
    /// the first line (inclusive ala Range::start)
    pub start: usize,
    /// one past last line (exclusive ala Range::end)
    pub end: usize,
    /// English description of the problem
    pub message: &'static str,
}

/// used by parser to create items
pub(crate) trait Builder<'a> {
    /// push an item into builder memory for future .list call
    fn item(&self, item: Item<'a>) -> Result<(), ParseError>;
    /// create a list from the `count` most recently pushed items
    fn list(&self, count: usize) -> Result<List<'a>, ParseError>;
    /// push an entry into builder memory for future .dict call
    fn entry(&self, entry: Entry<'a>) -> Result<(), ParseError>;
    /// create a dict from the `count` most recently pushed entries
    fn dict(&self, count: usize) -> Result<Dict<'a>, ParseError>;
}

/// the "report" callback provided to the parser should return one of these
pub enum Reported {
    /// tell parser to give up
    Abort,
    /// tell parser to keep going if it can, to find additional errors
    Continue,
}

pub(crate) struct Input<'a, F> {
    utf8: &'a str, // entire tindalwic encoded content
    line: usize,   // the number of the current line
    start: usize,  // start of current line, `MAX` means finished
    first: usize,  // first non-tab byte of current line
    assign: usize, // the `=` on current line, `MAX` means none
    end: usize,    // the newline ending current line, or `utf8.len()`
    tabs: usize,   // indentation on this line, unless gap, then peek from next line
    report: F,
    good: bool,
}
impl<'a, F> Input<'a, F>
where
    F: FnMut(ParseError) -> Reported,
{
    /// None means the arena is too small (or the UTF-8 is way too big).
    pub(crate) fn parse(arena: &dyn Builder<'a>, utf8: &'a str, report: F) -> Option<File<'a>> {
        let mut input = Input {
            utf8,
            line: 0,
            start: 0,
            first: 0,
            assign: 0,
            end: usize::MAX, // will wrap to 0 inside `next`
            tabs: 0,
            report,
            good: true,
        };
        if utf8.len() >= usize::MAX {
            // MAX is a sentinel, so it also can't be a len. the wrap-around will almost
            // certainly never actually occur because the arena is guaranteed to fill up
            // long before that, but it's an easy sanity check...
            input.report(ParseError::mem("way too big"))?;
            return None;
        }
        input.next(0)?;
        let hashbang = input.comment(0, b"#!")?;
        let dict = input.entries(0, arena)?;
        if !input.good {
            None
        } else {
            Some(File {
                cells: dict.cells,
                hashbang,
                prolog: dict.prolog,
            })
        }
    }

    fn report(&mut self, err: ParseError) -> Option<()> {
        self.good = false;
        match (self.report)(err) {
            Reported::Abort => None,
            _ => {
                if let ParseError::Memory(_) = err {
                    None
                } else {
                    Some(())
                }
            }
        }
    }

    /// done with current line, so advance, skipping excessively indented lines.
    /// usize::MAX prevents skipping. return false if finished with entire UTF-8.
    /// use `stretch` instead for Comment and Text (where no line is excessive).
    /// return None if the report signals abort.
    fn next(&mut self, indent: usize) -> Option<bool> {
        if self.start == usize::MAX {
            return Some(false);
        }
        self.line += 1;
        self.start = self.end.wrapping_add(1);
        if !self.scan()? {
            return Some(false);
        }
        if self.tabs <= indent {
            return Some(true);
        }
        let begin = self.line;
        self.line += 1;
        self.start = self.end + 1;
        while self.scan()? && self.tabs > indent {
            self.line += 1;
            self.start = self.end + 1;
        }
        self.report(ParseError::new(begin, self.line, "excess indentation"))?;
        return Some(self.start != usize::MAX);
    }

    /// helper for `next` to update state by examining a line of UTF-8.
    /// assumes caller has correctly set `self.start` (out-of-bounds is fine).
    /// return None if the report signals abort.
    fn scan(&mut self) -> Option<bool> {
        let bytes = self.utf8.as_bytes();
        let limit = bytes.len();
        let mut offset = self.start;
        if offset >= limit {
            self.start = usize::MAX;
            self.first = usize::MAX;
            self.assign = usize::MAX;
            self.tabs = 0;
            return Some(false);
        }
        offset += Value::indentation(bytes, offset, limit);
        self.first = offset;
        self.assign = usize::MAX;
        while offset < limit && bytes[offset] != b'\n' {
            if bytes[offset] == b'=' {
                self.assign = offset;
                while offset < limit && bytes[offset] != b'\n' {
                    offset += 1;
                }
                break;
            }
            offset += 1;
        }
        self.end = offset; // never MAX because `parse` checked length
        if self.start != self.end {
            self.tabs = self.first - self.start;
            return Some(true);
        }
        // found a gap, peek ahead to figure out its virtual indentation
        offset += 1;
        if offset < limit && bytes[offset] == b'\n' {
            let begin = self.line;
            self.line += 1;
            offset += 1;
            while offset < limit && bytes[offset] == b'\n' {
                self.line += 1;
                offset += 1;
            }
            self.report(ParseError::new(begin, self.line, "consecutive empty lines"))?;
            self.start = offset - 1;
            self.first = offset - 1;
            self.end = offset - 1;
        }
        offset += Value::indentation(bytes, offset, limit);
        self.tabs = offset - 1 - self.end;
        return Some(true);
    }

    /// current line has been recognized as beginning of a Comment or Text that might
    /// continue, so stretch it out to include the whole thing by changing `end`.
    /// return None if the report signals abort.
    fn stretch(&mut self, indent: usize, from: usize) -> Option<Value<'a>> {
        let value = Value::parse(indent, &self.utf8[from..]);
        let slice = value
            .shortcut(indent)
            .expect("impossible because just parsed");
        self.end = from + slice.as_bytes().len();
        self.next(usize::MAX)?; // stretch means excess is impossible
        Some(value)
    }
    fn stretch_once(&mut self, indent: usize) -> bool {
        let bytes = self.utf8.as_bytes();
        let limit = bytes.len();
        let mut offset = self.end;
        if offset >= limit {
            return false;
        }
        assert!(bytes[offset] == b'\n', "impossible: not at newline");
        let tabs = Value::indentation(bytes, offset + 1, limit);
        if tabs < indent {
            return false;
        }
        offset += 1 + tabs;
        while offset < limit && bytes[offset] != b'\n' {
            offset += 1;
        }
        self.end = offset; // never MAX because `parse` checked length
        true
    }

    /// use this whenever a comment is allowed, returns None if current line does not
    /// have exactly the provided indent and prefix.
    fn comment(&mut self, indent: usize, prefix: &'static [u8]) -> Option<Option<Comment<'a>>> {
        if self.start == usize::MAX || self.tabs != indent {
            return Some(None);
        }
        let bytes = self.utf8.as_bytes();
        let limit = bytes.len();
        let mut from = self.first + prefix.len();
        if from > limit || &bytes[self.first..from] != prefix {
            return Some(None);
        }
        let more = indent + 1;
        if prefix == [b'#'] && from == self.end && self.stretch_once(more) {
            from += more + 1;
        }
        let value = self.stretch(more, from)?;
        Some(Some(Comment { value }))
    }

    /// current line has been recognized as beginning a Text, from a `<>` context on
    /// the previous line, or from shortcut syntax. `from` says where text begins.
    /// lenient - one-liners can stretch.
    fn text(&mut self, indent: usize, from: usize) -> Option<Text<'a>> {
        let value = self.stretch(indent + 1, from)?;
        let epilog = self.comment(indent, b"#")?;
        Some(Text { value, epilog })
    }

    /// previous line opened a list context, so parse all the items in it.
    /// None means insufficient space in Arena.
    fn items(&mut self, indent: usize, arena: &dyn Builder<'a>) -> Option<List<'a>> {
        let bytes = self.utf8.as_bytes();
        let prolog = self.comment(indent, b"#")?;
        let mut count = 0usize;
        while self.start != usize::MAX {
            let mut item: Option<Item<'a>> = None;
            if self.start == self.end || self.tabs != indent {
                break;
            } else if self.first >= self.end {
                // indentation-only is the shortcut for empty text
                // TODO maybe too easily confused with gaps (require explicit `<>`)?
                item = Some(self.text(indent, self.end)?.into());
            } else {
                let len = self.end - self.first;
                match bytes[self.first] {
                    b'#' => {
                        self.report(ParseError::at(self.line, "stray `#` comment"))?;
                        self.comment(indent, b"#")?; // read and throw away
                    }
                    b'/' => {
                        self.report(ParseError::at(
                            self.line,
                            if len < 2 || bytes[self.first + 1] != b'/' {
                                "malformed // comment"
                            } else {
                                "no // comments in lists"
                            },
                        ))?;
                        self.comment(indent, b"/")?; // read and throw away
                    }
                    b'<' => {
                        if len != 2 || bytes[self.end - 1] != b'>' {
                            self.report(ParseError::at(self.line, "malformed `<>` in list"))?;
                            self.next(indent)?;
                        } else {
                            let end = self.end;
                            item = Some(
                                if !self.stretch_once(indent + 1) {
                                    // zero lines in this text, take empty slice from this line
                                    self.text(indent, end)?
                                } else {
                                    // first line of stretched text can have excess indent
                                    self.text(indent, end + indent + 2)?
                                }
                                .into(),
                            );
                        }
                    }
                    b'[' => {
                        if len != 2 || bytes[self.end - 1] != b']' {
                            self.report(ParseError::at(self.line, "malformed `[]` in list"))?;
                            self.next(indent)?;
                        } else {
                            self.next(indent + 1)?;
                            item = Some(self.items(indent + 1, arena)?.into());
                        }
                    }
                    b'{' => {
                        if len != 2 || bytes[self.end - 1] != b'}' {
                            self.report(ParseError::at(self.line, "malformed `{}` in list"))?;
                            self.next(indent)?;
                        } else {
                            self.next(indent + 1)?;
                            item = Some(self.entries(indent + 1, arena)?.into());
                        }
                    }
                    _ => {
                        item = Some(self.text(indent, self.start + indent)?.into());
                    }
                }
            }
            if let Some(item) = item {
                if let Err(err) = arena.item(item) {
                    self.report(err)?;
                }
                count += 1;
            }
        }
        match arena.list(count) {
            Ok(mut list) => {
                if indent > 0 {
                    list.epilog = self.comment(indent - 1, b"#")?;
                }
                list.prolog = prolog;
                Some(list)
            }
            Err(err) => {
                self.report(err)?;
                None
            }
        }
    }

    /// previous line opened a dict context, so parse all the entries in it.
    /// None means insufficient space in Arena.
    fn entries(&mut self, indent: usize, arena: &dyn Builder<'a>) -> Option<Dict<'a>> {
        let bytes = self.utf8.as_bytes();
        let prolog = self.comment(indent, b"#")?;
        let mut count = 0usize;
        while self.start != usize::MAX {
            let mut item: Option<Item<'a>> = None;
            let gap = self.tabs == indent && self.first == self.end;
            if gap {
                self.next(indent)?;
            }
            let before = self.comment(indent, b"//")?;
            if self.start == usize::MAX || self.tabs != indent {
                if gap || before.is_some() {
                    self.report(ParseError::at(self.line, "gap/before but no key"))?;
                }
                break;
            }
            let mut key: Value<'a> = Value::default();
            let len = self.end - self.first;
            match bytes[self.first] {
                b'#' => {
                    self.report(ParseError::at(self.line, "stray `#` comment"))?;
                    self.comment(indent, b"#")?; // read and throw away
                }
                b'/' => {
                    self.report(ParseError::at(
                        self.line,
                        if len < 2 || bytes[self.first + 1] != b'/' {
                            "malformed // comment"
                        } else {
                            "stray `//` comment"
                        },
                    ))?;
                    self.comment(indent, b"/")?; // read and throw away
                }
                b'<' => {
                    if len < 2 || bytes[self.end - 1] != b'>' {
                        self.report(ParseError::at(self.line, "malformed `<key>` in dict"))?;
                        self.next(indent)?;
                    } else {
                        key = Value::wrap(&self.utf8[self.first + 1..self.end - 1]);
                        let end = self.end;
                        item = Some(
                            if !self.stretch_once(indent + 1) {
                                // zero lines in this text, take empty slice from this line
                                self.text(indent, end)?
                            } else {
                                // first line of stretched text can have excess indent
                                self.text(indent, end + 1 + indent + 1)?
                            }
                            .into(),
                        );
                    }
                }
                b'[' => {
                    if len < 2 || bytes[self.end - 1] != b']' {
                        self.report(ParseError::at(self.line, "malformed `[key]` in dict"))?;
                        self.next(indent)?;
                    } else {
                        key = Value::wrap(&self.utf8[self.first + 1..self.end - 1]);
                        self.next(indent + 1)?;
                        item = Some(self.items(indent + 1, arena)?.into());
                    }
                }
                b'{' => {
                    if len < 2 || bytes[self.end - 1] != b'}' {
                        self.report(ParseError::at(self.line, "malformed `{key}` in dict"))?;
                        self.next(indent)?;
                    } else {
                        key = Value::wrap(&self.utf8[self.first + 1..self.end - 1]);
                        self.next(indent + 1)?;
                        item = Some(self.entries(indent + 1, arena)?.into());
                    }
                }
                b'\t' => {
                    self.report(ParseError::at(self.line, "excess indentation?"))?;
                    self.next(indent)?;
                }
                _ => {
                    if self.assign == usize::MAX {
                        self.report(ParseError::at(self.line, "missing `=` in dict"))?;
                        self.next(indent)?;
                    } else {
                        key = Value::wrap(&self.utf8[self.first..self.assign]);
                        item = Some(self.text(indent, self.assign + 1)?.into());
                    }
                }
            }
            if let Some(item) = item {
                if let Err(err) = arena.entry(Entry {
                    gap,
                    before,
                    key,
                    item,
                }) {
                    self.report(err)?;
                }
                count += 1;
            } else if gap || before.is_some() {
                self.report(ParseError::at(self.line, "gap/before but no item"))?;
            }
        }
        match arena.dict(count) {
            Ok(mut dict) => {
                dict.prolog = prolog;
                if indent > 0 {
                    dict.epilog = self.comment(indent - 1, b"#")?;
                }
                Some(dict)
            }
            Err(err) => {
                self.report(err)?;
                None
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{arena, walk};

    macro_rules! assert_lines_eq {
        // checking this gets repetitive without Vec
        ($text:ident, $($line:literal),*) => {
            let mut it = $text.value.lines();
            $(assert_eq!(it.next(), Some($line));)*
            assert_eq!(it.next(), None);
        };
    }

    #[test]
    fn empty() {
        arena! {
            $crate = crate;
            let mut arena = <10dict,10list>;
        }
        let file = arena.parse_or_panic("");
        assert!(!arena.completed().is_some());
        assert!(file.hashbang.is_none());
        assert!(file.prolog.is_none());
        assert!(file.cells.is_empty());
    }

    #[test]
    fn key_eq_value() {
        arena! {
            $crate = crate;
            let mut arena = <1dict>;
        }
        let file = arena.parse_or_panic("k=v");
        assert!(arena.completed().is_some());
        assert!(file.hashbang.is_none());
        assert!(file.prolog.is_none());
        assert_eq!(file.cells.len(), 1);
        let entry = file.cells[file.position("k").unwrap()].get();
        let Item::Text(text) = entry.item else {
            panic!("not text?");
        };
        assert_lines_eq!(text, "v");
    }
    #[test]
    fn sub_list() {
        arena! {
            $crate = crate;
            let mut arena = <3list,1dict>;
        }
        let file = arena.parse_or_panic("[k]\n\t1\n\t2\n\t3");
        assert!(arena.completed().is_some());
        assert_eq!(file.cells.len(), 1);
        let entry = file.cells[file.position("k").unwrap()].get();
        let Item::List(list) = entry.item else {
            panic!("not list?");
        };
        assert_eq!(list.cells.len(), 3);
        let Item::Text(one) = list.cells[0].get() else {
            panic!("not text?");
        };
        assert_lines_eq!(one, "1");
        let Item::Text(two) = list.cells[1].get() else {
            panic!("not text?");
        };
        assert_lines_eq!(two, "2");
        let Item::Text(three) = list.cells[2].get() else {
            panic!("not text?");
        };
        assert_lines_eq!(three, "3");
    }
    #[test]
    fn sub_dict() {
        arena! {
            $crate = crate;
            let mut arena = <2dict>;
        }
        let file = arena.parse_or_panic("{z}\n\t<k>\n\t\tv");
        assert!(arena.completed().is_some());
        walk! { $crate = crate;
            let v = (&file){"z"}<"k">.unwrap();
        }
        assert_lines_eq!(v, "v");
    }
}
