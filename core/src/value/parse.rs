//! everything related to converting bytes into a File

use core::str::Split;

use crate::{CommentMarker, Dict, Entries, Entry, File, Item, Items, List, Text, Value};

// there are some lines/branches here that are impossible to get coverage for,
// and the mechanisms for suppressing the report are inadequate ... until:
//  + https://github.com/rust-lang/rust/issues/84605
//  + https://github.com/rust-lang/rust/issues/15701

/// parsing problems
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ParseError {
    /// a problem in the Tindalwic
    Syntax {
        /// the first line (inclusive ala Range::start)
        start: usize,
        /// one past last line (exclusive ala Range::end)
        end: usize,
        /// English description of the problem
        message: &'static str,
    },
    /// ran out of room in the storage
    Memory(
        /// English description of the problem
        &'static str,
    ),
}
impl core::error::Error for ParseError {}
impl ParseError {
    /// make a Syntax error with an arbitrary span of lines.
    pub fn new(start: usize, end: usize, message: &'static str) -> Self {
        ParseError::Syntax {
            start,
            end,
            message,
        }
    }
    /// make a Syntax error for a single line.
    pub fn at(line: usize, message: &'static str) -> Self {
        ParseError::new(line, line + 1, message)
    }
}

/// used by parser to create items
pub trait Build<'a> {
    /// push an item for a future .finish_items to use.
    fn push_item(&mut self, item: Item<'a>) -> Result<(), &'static str>;
    /// create an [Items] from the `count` most recently pushed items.
    fn finish_items(&mut self, count: usize) -> Result<Items<'a>, &'static str>;
    /// push an entry for a future .finish_entries to use.
    fn push_entry(&mut self, entry: Entry<'a>) -> Result<(), &'static str>;
    /// create an [Entries] from the `count` most recently pushed entries.
    fn finish_entries(&mut self, count: usize) -> Result<Entries<'a>, &'static str>;
    /// push an [Item::Text] (no metadata) for a future .finish_items to use.
    fn text_item(&mut self, value: &'a str) -> Result<(), &'static str> {
        self.push_item(value.into())
    }
    /// push an [Item::List] (no metadata) for a future .finish_items to use.
    fn list_item(&mut self, count: usize) -> Result<(), &'static str> {
        let items = self.finish_items(count)?;
        self.push_item(items.into())
    }
    /// push an [Item::Dict] (no metadata) for a future .finish_items to use.
    fn dict_item(&mut self, count: usize) -> Result<(), &'static str> {
        let entries = self.finish_entries(count)?;
        self.push_item(entries.into())
    }
    /// push a `key` -> [Item::Text] association (no metadata) for a future .finish_entries to use.
    fn text_entry(&mut self, key: &'a str, value: &'a str) -> Result<(), &'static str> {
        self.associate(key, value.into())
    }
    /// push a `key` -> [Item::List] association (no metadata) for a future .finish_entries to use.
    fn list_entry(&mut self, key: &'a str, count: usize) -> Result<(), &'static str> {
        let items = self.finish_items(count)?;
        self.associate(key, items.into())
    }
    /// push a `key` -> [Item::Dict] association (no metadata) for a future .finish_entries to use.
    fn dict_entry(&mut self, key: &'a str, count: usize) -> Result<(), &'static str> {
        let entries = self.finish_entries(count)?;
        self.associate(key, entries.into())
    }
    /// push a `key` -> `item` association (no metadata) for a future .finish_entries to use
    fn associate(&mut self, key: &'a str, item: Item<'a>) -> Result<(), &'static str> {
        let mut entry = Entry::default();
        entry.name.key = key.into();
        entry.item = item;
        self.push_entry(entry)
    }
    /// default is an Err because intern needs alloc
    #[allow(unused_variables)]
    fn intern(&mut self, value: &'_ str) -> Result<&'a str, &'static str> {
        Err("intern not supported")
    }
}

/// provide a Builder to get access to parsing
pub trait Parse<'a> {
    /// get a builder for the parser to use
    fn builder(&mut self) -> &mut dyn Build<'a>;
    /// call the parser on the provided content, with a callback for errors.
    fn report_errors(
        &mut self,
        content: &'a str,
        report: &'_ mut dyn FnMut(ParseError) -> Reported,
    ) -> Option<File<'a>> {
        Input::parse(content, self.builder(), report)
    }
    /// call the parser on the provided content, give up at first error.
    fn first_error(&mut self, content: &'a str) -> Result<File<'a>, ParseError> {
        let mut first: Option<ParseError> = None;
        self.report_errors(content, &mut |error| {
            first = Some(error);
            Reported::Abort
        })
        .ok_or_else(|| first.expect("error should have been reported"))
    }
    /// call the parser on the provided content, panic if the content isn't legit.
    fn panic_first_error(&mut self, content: &'a str) -> File<'a> {
        self.report_errors(content, &mut |error| panic!("{error}"))
            .expect("panic should have already happened in report")
    }
}

/// the "report" callback provided to the parser should return one of these
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Reported {
    /// tell parser to give up
    Abort,
    /// tell parser to keep going if it can, to find additional errors
    Continue,
}

/// can be advanced through each line in a str slice.
pub struct LineCursor<'a> {
    current: Option<&'a str>,
    pending: Split<'a, char>,
    exhausted: bool,
    /// the line number (starts at 1)
    pub number: usize,
}
impl<'a> LineCursor<'a> {
    /// return a cursor for the given str slice
    pub fn new(slice: &'a str) -> Self {
        let mut pending = slice.split('\n');
        let current = pending.next();
        LineCursor {
            current,
            pending,
            exhausted: current.is_none(),
            number: 1,
        }
    }

    /// move to the next line
    pub fn advance(&mut self) {
        if self.exhausted {
            self.current = None;
        } else {
            self.number += 1;
            self.current = self.pending.next();
            if self.current.is_none() {
                self.exhausted = true
            }
        }
    }

    /// Some if current line is empty or if margin successfully removed
    pub fn dedent(&self, indent: usize) -> Option<&'a str> {
        let Some(line) = self.current else {
            return None;
        };
        if line.is_empty() || indent == 0 {
            return Some(line);
        }
        let bytes = line.as_bytes();
        for offset in 0..indent {
            if bytes[offset] != b'\t' {
                return None;
            }
        }
        Some(&line[indent..])
    }

    /// count trailing empty lines, or Err if a non-empty line remains
    pub fn finish(&mut self) -> Result<usize, &'static str> {
        let start = self.number;
        loop {
            match self.current {
                None => return Ok(self.number - start),
                Some(line) if line.is_empty() => self.advance(),
                _ => return Err("not finished, found a non-empty line"),
            }
        }
    }
}

struct Input<'a, 'b, 'r> {
    lines: LineCursor<'a>,
    arena: &'b mut dyn Build<'a>,
    report: &'r mut dyn FnMut(ParseError) -> Reported,
    source: &'a str,
}
impl<'a, 'b, 'r> Input<'a, 'b, 'r> {
    /// None means the arena is too small (or the UTF-8 is way too big).
    pub fn parse(
        source: &'a str,
        arena: &'b mut dyn Build<'a>,
        mut report: impl FnMut(ParseError) -> Reported + 'r,
    ) -> Option<File<'a>> {
        let mut input = Input {
            lines: LineCursor::new(source),
            arena,
            report: &mut report,
            source,
        };
        input.file().map_or_else(|_| None, |f| Some(f))
    }

    /// finish a complete file
    pub fn file(&mut self) -> Result<File<'a>, &'static str> {
        let mut file = File::default();
        file.shebang = self.comment(0, CommentMarker::Shebang)?;
        file.prolog = self.comment(0, CommentMarker::Prolog)?;
        file.entries = self.entries(0)?;
        // file.trailing = self.lines.finish()?;
        let end = self.lines.number;
        if 0 != self.lines.finish()? {
            self.report(ParseError::new(
                end,
                self.lines.number,
                "blank lines at EOF",
            ))?;
        }
        Ok(file)
    }

    /// actual ParseError is consumed by the reporting,
    /// so returns a vague Err() to unwind the call stack.
    fn report(&mut self, err: ParseError) -> Result<(), &'static str> {
        match (self.report)(err) {
            Reported::Abort => Err("parse.report: caller wants to abort"),
            Reported::Continue => {
                if let ParseError::Memory(_) = err {
                    Err("parse.report: ran out of memory")
                } else {
                    Ok(())
                }
            }
        }
    }

    /// done with current line, skip past excessively indented lines until exhausted or
    /// find one with given indent or less.
    /// pass usize::MAX to prevent skipping (exactly one advance will be done).
    /// return true if any lines were skipped.
    fn next(&mut self, indent: usize) -> Result<bool, &'static str> {
        self.lines.advance();
        if indent == usize::MAX || self.lines.dedent(indent + 1).is_none() {
            return Ok(false);
        }
        loop {
            self.lines.advance();
            if self.lines.dedent(indent + 1).is_none() {
                return Ok(true);
            }
        }
    }
    fn excess(&mut self, indent: usize) -> Result<(), &'static str> {
        let first = self.lines.number + 1;
        if self.next(indent)? {
            self.report(ParseError::new(
                first,
                self.lines.number,
                "excess indentation",
            ))?;
        }
        Ok(())
    }

    /// current line has been recognized as beginning of a Value that might
    /// flow, so stretch a portion of it out to include the whole thing.
    fn flow(&mut self, indent: usize, from: &'a str) -> Result<Value<'a>, &'static str> {
        if !self.next(indent)? {
            return Ok(from.into());
        }
        let base = self.source.as_ptr() as usize;
        let limit = base + self.source.len();
        let first = from.as_ptr() as usize;
        if first < base || limit < first {
            return Err("flow `from` did not come from self.source");
        }
        let last = match self.lines.dedent(0) {
            None => limit,
            Some(line) => line.as_ptr() as usize - 1,
        };
        if last < base || limit < last {
            return Err("flow current line isn't from self.source");
        }
        if last < first {
            return Err("flow `from` comes after current line");
        }
        let slice = &self.source[first - base..last - base];
        Ok(Value {
            slice,
            indent: indent + 1,
        })
    }

    /// parse a comment if one starts on this line.
    pub fn comment(
        &mut self,
        indent: usize,
        mark: CommentMarker,
    ) -> Result<Option<Value<'a>>, &'static str> {
        if let Some(from) = self
            .lines
            .dedent(indent)
            .and_then(|line| line.strip_prefix(mark.token()))
        {
            Ok(Some(self.flow(indent, from)?))
        } else {
            Ok(None)
        }
    }

    /// current line has been recognized as beginning a Text, from a `<>` context on
    /// the previous line, or from shortcut syntax. `from` is where text begins.
    /// lenient because one-liners can stretch.
    pub fn lenient(&mut self, indent: usize, from: &'a str) -> Result<Text<'a>, &'static str> {
        let value = self.flow(indent, from)?;
        let epilog = self.comment(indent, CommentMarker::Epilog)?;
        Ok(Text {
            value,
            longer: false,
            epilog,
        })
    }

    /// parse an indented block of text
    pub fn text(&mut self, indent: usize) -> Result<Text<'a>, &'static str> {
        let value = self.block(indent)?;
        let epilog = self.comment(indent, CommentMarker::Epilog)?;
        Ok(Text {
            value,
            longer: true,
            epilog,
        })
    }
    /// a block (optionally) follows current line (at indent+1).
    fn block(&mut self, indent: usize) -> Result<Value<'a>, &'static str> {
        self.lines.advance();
        let Some(first) = self.lines.dedent(indent + 1) else {
            return Ok(Value::default());
        };
        self.flow(indent, first)
    }

    /// previous line opened a list context, so parse all the lines in it.
    pub fn list(&mut self, indent: usize) -> Result<List<'a>, &'static str> {
        let mut list = List::default();
        list.prolog = self.comment(indent + 1, CommentMarker::Prolog)?;
        list.items = self.items(indent + 1)?;
        list.epilog = self.comment(indent, CommentMarker::Epilog)?;
        Ok(list)
    }

    /// attempt to parse one item from a list context
    pub fn item(&mut self, indent: usize) -> Result<Option<Item<'a>>, &'static str> {
        loop {
            let Some(scan) = self.lines.dedent(indent) else {
                return Ok(None);
            };
            let line = self.lines.number;
            let message = if scan.is_empty() {
                "gotta use `<>` for empty items"
            } else if scan == "<>" {
                return Ok(Some(Item::Text(self.text(indent)?)));
            } else if scan.starts_with('<') {
                "malformed `<>` in list"
            } else if scan == "[]" {
                self.excess(indent + 1)?;
                return Ok(Some(Item::List(self.list(indent)?)));
            } else if scan.starts_with('[') {
                "malformed `[]` in list"
            } else if scan == "{}" {
                self.excess(indent + 1)?;
                return Ok(Some(Item::Dict(self.dict(indent)?)));
            } else if scan.starts_with('{') {
                "malformed `{}` in list"
            } else if scan.starts_with(&CommentMarker::FIRST_CHAR) {
                "stray comment"
            } else {
                return Ok(Some(Item::Text(self.lenient(indent, scan)?)));
            };
            self.report(ParseError::at(line, message))?;
            self.excess(indent)?;
        }
    }

    /// parse any items inside a list context
    pub fn items(&mut self, indent: usize) -> Result<Items<'a>, &'static str> {
        let mut count = 0usize;
        loop {
            let Some(item) = self.item(indent)? else {
                break;
            };
            if let Err(err) = self.arena.push_item(item) {
                self.report(ParseError::Memory(err))?;
                return Err(err); // memory err should always unwind but to be safe
            }
            count += 1;
        }
        if count == 0 {
            Ok(Items::default())
        } else {
            match self.arena.finish_items(count) {
                Ok(cells) => Ok(cells),
                Err(err) => {
                    // seems like .inspect_err should work except for...
                    self.report(ParseError::Memory(err))?; // this `?`
                    Err(err) // memory err should always unwind but to be safe
                }
            }
        }
    }

    /// previous line opened a dict context, so parse all the lines in it.
    pub fn dict(&mut self, indent: usize) -> Result<Dict<'a>, &'static str> {
        let mut dict = Dict::default();
        dict.prolog = self.comment(indent + 1, CommentMarker::Prolog)?;
        dict.entries = self.entries(indent + 1)?;
        dict.epilog = self.comment(indent, CommentMarker::Epilog)?;
        Ok(dict)
    }
    // attempt to parse one entry
    pub fn entry(&mut self, indent: usize) -> Result<Option<Entry<'a>>, &'static str> {
        loop {
            let comment = self.comment(indent, CommentMarker::NameTag)?;
            let line = self.lines.number;
            let Some(scan) = self.lines.dedent(indent) else {
                if comment.is_some() {
                    self.report(ParseError::at(line, "comment but no key"))?;
                }
                return Ok(None);
            };
            let message = if scan.starts_with('<') && scan.ends_with('>') {
                let mut entry = Entry::default();
                entry.name.key = Value::from(&scan[1..scan.len() - 1]);
                entry.item = Item::Text(self.text(indent)?);
                entry.name.comment = comment;
                entry.name.longer = true;
                return Ok(Some(entry));
            } else if scan.starts_with('<') {
                "malformed `<key>` in dict"
            } else if scan.starts_with('[') && scan.ends_with(']') {
                let mut entry = Entry::default();
                entry.name.key = Value::from(&scan[1..scan.len() - 1]);
                self.excess(indent + 1)?;
                entry.item = Item::List(self.list(indent)?);
                entry.name.comment = comment;
                entry.name.longer = true;
                return Ok(Some(entry));
            } else if scan.starts_with('[') {
                "malformed `[key]` in dict"
            } else if scan.starts_with('{') && scan.ends_with('}') {
                let mut entry = Entry::default();
                entry.name.key = Value::from(&scan[1..scan.len() - 1]);
                self.excess(indent + 1)?;
                entry.item = Item::Dict(self.dict(indent)?);
                entry.name.comment = comment;
                entry.name.longer = true;
                return Ok(Some(entry));
            } else if scan.starts_with('{') {
                "malformed `{key}` in dict"
            } else if scan == "@" {
                let mut entry = Entry::default();
                entry.longer = true;
                entry.name.key = self.block(indent)?;
                if let Some(item) = self.item(indent)? {
                    entry.item = item;
                    entry.name.comment = comment;
                    return Ok(Some(entry));
                }
                "long `@` key needs a value"
            } else if scan.starts_with('@') {
                "`@` has trailing char"
            } else if scan.starts_with(&CommentMarker::FIRST_CHAR) {
                "stray comment"
            } else if let Some((before, after)) = scan.split_once('=') {
                let mut entry = Entry::default();
                entry.name.key = Value::from(before);
                entry.item = Item::Text(self.lenient(indent, after)?);
                entry.name.comment = comment;
                return Ok(Some(entry));
            } else {
                "missing `=` in dict"
            };
            self.report(ParseError::at(line, message))?;
            self.excess(indent)?;
        }
    }

    /// parse any entries inside a dict context
    pub fn entries(&mut self, indent: usize) -> Result<Entries<'a>, &'static str> {
        let mut count = 0usize;
        loop {
            let Some(entry) = self.entry(indent)? else {
                break;
            };
            if let Err(err) = self.arena.push_entry(entry) {
                self.report(ParseError::Memory(err))?;
                return Err(err); // memory err should always unwind but to be safe
            }
            count += 1;
        }
        if count == 0 {
            Ok(Entries::default())
        } else {
            match self.arena.finish_entries(count) {
                Ok(cells) => Ok(cells),
                Err(err) => {
                    // seems like .inspect_err should work except for...
                    self.report(ParseError::Memory(err))?; // this `?`
                    Err(err) // memory err should always unwind but to be safe
                }
            }
        }
    }
}
