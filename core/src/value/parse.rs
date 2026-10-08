//! everything related to converting bytes into a File

use core::str::SplitInclusive;

use crate::{Comment, Dict, Entries, Entry, File, Item, Items, List, Name, Text, Value};

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
        self.push_entry(Entry {
            name: key.into(),
            item,
            ..Default::default()
        })
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
        Input::parse(self.builder(), content, report)
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

enum CommentMark {
    Shebang,
    DoubleSlash,
    TripleSlash,
}

struct Input<'a, 'b, 'r> {
    line: usize,
    empties: usize,
    tabs: usize, // indentation on this line, unless gap, then peek from next line
    current: Option<&'a str>,
    good: bool,
    report: &'r mut dyn FnMut(ParseError) -> Reported,
    pending: Option<SplitInclusive<'a, char>>,
    arena: &'b mut dyn Build<'a>,
    utf8: &'a str, // entire tindalwic encoded content
}
impl<'a, 'b, 'r> Input<'a, 'b, 'r> {
    /// None means the arena is too small (or the UTF-8 is way too big).
    pub fn parse(
        arena: &'b mut dyn Build<'a>,
        utf8: &'a str,
        mut report: impl FnMut(ParseError) -> Reported + 'r,
    ) -> Option<File<'a>> {
        let pending = Some(utf8.split_inclusive('\n'));
        let mut input = Input {
            utf8,
            arena,
            line: 0,
            pending,
            good: true,
            report: &mut report,
            current: None,
            tabs: 0,
            empties: 0,
        };
        if input.next(0, true).is_err() {
            return None;
        }
        input.file().map_or_else(|_| None, |f| Some(f))
    }
    fn file(&mut self) -> Result<File<'a>, &'static str> {
        if self.utf8.len() >= usize::MAX {
            // not covered (impossible to get, can't suppress completely).
            // MAX is a sentinel (in Value::indent), so it can't also be a length.
            // paranoid: no str can be this big (assuming usize correctly implemented),
            // but it is simple and cheap to be explicit about the contract.
            self.report(ParseError::Memory("way too big"))?;
            return Err("parse.file: can't even start");
        }
        let hashbang = self.comment(0, false, CommentMark::Shebang)?.value;
        let prolog = self.comment(0, true, CommentMark::DoubleSlash)?;
        let entries = self.entries(0)?;
        if self.current.is_some() {
            self.report(ParseError::at(
                self.line,
                "input was not completely consumed",
            ))?;
        }
        let _cell = core::cell::Cell::new("hi");
        // TODO do something with empties at EOF
        if !self.good {
            Err("parse.file: something was reported")
        } else {
            Ok(File {
                hashbang,
                prolog,
                entries,
            })
        }
    }

    /// actual ParseError is consumed by the reporting.
    /// returns an Err(()) to unwind the call stack.
    fn report(&mut self, err: ParseError) -> Result<(), &'static str> {
        self.good = false;
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

    fn advance(&mut self) {
        let Some(mut iter) = self.pending.take() else {
            self.current = None;
            return;
        };
        self.current = iter.next();
        self.line += 1;
        self.tabs = match self.current {
            None => 0,
            Some(line) => {
                self.pending = Some(iter);
                line.len() - line.trim_start_matches('\t').len()
            }
        }
    }

    fn after_indent(&self, indent: usize) -> Option<&'a str> {
        let Some(line) = self.current else {
            return None;
        };
        if self.tabs < indent {
            None
        } else {
            Some(&line[indent..].trim_end_matches('\n'))
        }
    }

    fn claim_empties(&mut self) -> usize {
        let result = self.empties;
        self.empties = 0;
        result
    }

    /// done with current line, so advance past excessively indented lines.
    /// pass indent==usize::MAX to avoid skipping the excess (still skips empties).
    /// return the line just previous to what becomes current.
    fn next(
        &mut self,
        indent: usize,
        report_excess: bool,
    ) -> Result<Option<&'a str>, &'static str> {
        self.empties = 0;
        let mut previous = None; // current can't be "previous"
        self.advance();
        let first = self.line;
        let mut excess = false;
        loop {
            while self.current == Some("\n") {
                self.empties += 1;
                // leave previous, exclude last clump of empties from stretch
                self.advance();
            }
            if self.current.is_none() {
                break;
            }
            if self.tabs <= indent {
                break;
            }
            excess = true;
            self.empties = 0; // only the last clump of consecutive empties matters
            previous = self.current;
            self.advance();
        }
        if report_excess && excess {
            self.report(ParseError::new(first, self.line, "excess indentation"))?;
        }
        Ok(previous)
    }

    /// current line has been recognized as beginning of a Comment or Text that might
    /// continue, so stretch a portion of it out to include the whole thing.
    fn stretch(&mut self, indent: usize, from: &'a str) -> Result<Value<'a>, &'static str> {
        let start = Value::from(from);
        return match self.next(indent, false)? {
            None => Ok(start),
            Some(previous) => {
                Ok(start.stretched(indent + 1, previous.trim_end_matches('\n'), self.utf8)?)
            }
        };
    }

    /// use this whenever a comment is allowed.
    fn comment(
        &mut self,
        indent: usize,
        gap: bool,
        mark: CommentMark,
    ) -> Result<Comment<'a>, &'static str> {
        let line = self.after_indent(indent).and_then(|line| match mark {
            CommentMark::Shebang => line.strip_prefix("#!"),
            CommentMark::DoubleSlash => line.strip_prefix("//"), // TODO reject triple?
            CommentMark::TripleSlash => line.strip_prefix("///"),
        });
        let mut comment = Comment::default();
        if gap {
            comment.gap = self.claim_empties();
        }
        if let Some(from) = line {
            comment.value = Some(self.stretch(indent, from)?);
        }
        return Ok(comment);
    }

    /// current line has been recognized as beginning a Text, from a `<>` context on
    /// the previous line, or from shortcut syntax. `from` is where text begins.
    /// lenient - one-liners can stretch.
    fn text(&mut self, indent: usize, from: &'a str) -> Result<Item<'a>, &'static str> {
        let value = self.stretch(indent, from)?;
        let epilog = self.comment(indent, false, CommentMark::DoubleSlash)?.value;
        Ok(Item::Text(Text {
            value,
            longer: false,
            epilog,
        }))
    }
    fn text_block(&mut self, indent: usize) -> Result<Item<'a>, &'static str> {
        let value = self.block(indent)?;
        let epilog = self.comment(indent, false, CommentMark::DoubleSlash)?.value;
        Ok(Item::Text(Text {
            value,
            longer: true,
            epilog,
        }))
    }
    /// a block (optionally) follows current line (at indent+1).
    /// always need some value, use end of current if no block follows
    fn block(&mut self, indent: usize) -> Result<Value<'a>, &'static str> {
        self.advance();
        let Some(first) = self.after_indent(indent + 1) else {
            return Ok(Value::default());
        };
        self.stretch(indent, first)
    }

    /// previous line opened a list context, so parse all the lines in it.
    fn list(&mut self, indent: usize) -> Result<Item<'a>, &'static str> {
        Ok(Item::List(List {
            prolog: self.comment(indent + 1, true, CommentMark::DoubleSlash)?,
            items: self.items(indent + 1)?,
            epilog: self.comment(indent, true, CommentMark::DoubleSlash)?,
        }))
    }

    fn one_item(&mut self, indent: usize) -> Result<Option<Item<'a>>, &'static str> {
        loop {
            let Some(scan) = self.after_indent(indent) else {
                return Ok(None);
            };
            let line = self.line;
            let message = if scan.is_empty() {
                return Ok(Some(self.text(indent, scan)?));
            } else if scan == "<>" {
                return Ok(Some(self.text_block(indent)?));
            } else if scan.starts_with('<') {
                "malformed `<>` in list"
            } else if scan == "[]" {
                self.next(indent + 1, true)?;
                return Ok(Some(self.list(indent)?));
            } else if scan.starts_with('[') {
                "malformed `[]` in list"
            } else if scan == "{}" {
                self.next(indent + 1, true)?;
                return Ok(Some(self.dict(indent)?));
            } else if scan.starts_with('{') {
                "malformed `{}` in list"
            } else if scan.starts_with("//") {
                "stray comment"
            } else {
                return Ok(Some(self.text(indent, scan)?));
            };
            self.report(ParseError::at(line, message))?;
            self.next(indent, false)?;
        }
    }

    fn items(&mut self, indent: usize) -> Result<Items<'a>, &'static str> {
        let mut count = 0usize;
        loop {
            let Some(item) = self.one_item(indent)? else {
                break;
            };
            if let Err(err) = self.arena.push_item(item) {
                self.report(ParseError::Memory(err))?;
                return Err(err); // memory err should always unwind but to be safe
            }
            count += 1;
        }
        if count == 0 {
            Ok(&[])
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
    fn dict(&mut self, indent: usize) -> Result<Item<'a>, &'static str> {
        Ok(Item::Dict(Dict {
            prolog: self.comment(indent + 1, true, CommentMark::DoubleSlash)?,
            entries: self.entries(indent + 1)?,
            epilog: self.comment(indent, true, CommentMark::DoubleSlash)?,
        }))
    }
    fn one_entry(&mut self, indent: usize) -> Result<Option<Entry<'a>>, &'static str> {
        loop {
            if self.current.is_none() || self.tabs != indent {
                return Ok(None);
            }
            let comment = self.comment(indent, true, CommentMark::TripleSlash)?;
            let Some(scan) = self.after_indent(indent) else {
                if comment.gap > 0 || comment.value.is_some() {
                    self.report(ParseError::at(self.line, "gap/comment but no key"))?;
                }
                return Ok(None);
            };
            let line = self.line;
            let message = if scan.starts_with('<') && scan.ends_with('>') {
                let key = Value::from(&scan[1..scan.len() - 1]);
                let item = self.text_block(indent)?;
                let name = Name {
                    comment,
                    key,
                    longer: true,
                };
                return Ok(Some(Entry {
                    name,
                    item,
                    longer: false,
                }));
            } else if scan.starts_with('<') {
                "malformed `<key>` in dict"
            } else if scan.starts_with('[') && scan.ends_with(']') {
                let key = Value::from(&scan[1..scan.len() - 1]);
                self.next(indent + 1, true)?;
                let item = self.list(indent)?;
                let name = Name {
                    comment,
                    key,
                    longer: true,
                };
                return Ok(Some(Entry {
                    name,
                    item,
                    longer: false,
                }));
            } else if scan.starts_with('[') {
                "malformed `[key]` in dict"
            } else if scan.starts_with('{') && scan.ends_with('}') {
                let key = Value::from(&scan[1..scan.len() - 1]);
                self.next(indent + 1, true)?;
                let item = self.dict(indent)?;
                let name = Name {
                    comment,
                    key,
                    longer: true,
                };
                return Ok(Some(Entry {
                    name,
                    item,
                    longer: false,
                }));
            } else if scan.starts_with('{') {
                "malformed `{key}` in dict"
            } else if scan == "@" {
                let key = self.block(indent)?;
                if let Some(item) = self.one_item(indent)? {
                    let name = Name {
                        comment,
                        key,
                        longer: true,
                    };
                    return Ok(Some(Entry {
                        name,
                        item,
                        longer: true,
                    }));
                }
                "long `@` key needs a value"
            } else if scan.starts_with('@') {
                "`@` has trailing char"
            } else if scan.starts_with("//") {
                "stray comment"
            } else if let Some((before, after)) = scan.split_once('=') {
                let key = Value::from(before);
                let item = self.text(indent, after)?;
                let name = Name {
                    comment,
                    key,
                    longer: false,
                };
                return Ok(Some(Entry {
                    name,
                    item,
                    longer: false,
                }));
            } else {
                "missing `=` in dict"
            };
            self.report(ParseError::at(line, message))?;
            self.next(indent, false)?;
        }
    }

    fn entries(&mut self, indent: usize) -> Result<Entries<'a>, &'static str> {
        let mut count = 0usize;
        loop {
            let Some(entry) = self.one_entry(indent)? else {
                break;
            };
            if let Err(err) = self.arena.push_entry(entry) {
                self.report(ParseError::Memory(err))?;
                return Err(err); // memory err should always unwind but to be safe
            }
            count += 1;
        }
        if count == 0 {
            Ok(&[])
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
