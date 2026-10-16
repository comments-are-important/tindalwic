//! everything related to converting bytes into a File

use core::str::Split;

use crate::group::{Groups, Members};
use crate::{CommentMarker, Data, File, Group, List, Member, Text, UTF8};

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
    /// push a group for a future .finish_groups to use.
    fn push_group(&mut self, item: Group<'a>) -> Result<(), &'static str>;
    /// create a [Groups] from the `count` most recently pushed groups.
    fn finish_groups(&mut self, count: usize) -> Result<Groups<'a>, &'static str>;
    /// push a member for a future .finish_members to use.
    fn push_member(&mut self, entry: Member<'a>) -> Result<(), &'static str>;
    /// create a [Members] from the `count` most recently pushed members.
    fn finish_members(&mut self, count: usize) -> Result<Members<'a>, &'static str>;
    /// create a List with one group (no heading) from the `count` most recently pushed members.
    fn finish_list(&mut self, count: usize) -> Result<List<'a>, &'static str> {
        if count == 0 {
            return Ok(List::default());
        }
        let members = self.finish_members(count)?;
        self.push_group(members.into())?;
        let groups = self.finish_groups(1)?;
        Ok(groups.into())
    }
    /// create a List of single member and group (no heading).
    fn singleton(&mut self, data: Data<'a>) -> Result<List<'a>, &'static str> {
        self.push_member(data.into())?;
        self.finish_list(1)
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
        let mut input = Input {
            lines: LineCursor::new(content),
            arena: self.builder(),
            report,
        };
        input.file().map_or_else(|_| None, |f| Some(f))
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
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Reported {
    /// tell parser to give up
    Abort,
    /// tell parser to keep going if it can, to find additional errors
    Continue,
}

/// can be advanced through each line of a str.
pub struct LineCursor<'a> {
    source: &'a str,
    current: Option<&'a str>,
    pending: Split<'a, char>,
    exhausted: bool,
    /// the line number (starts at 1)
    pub number: usize,
}
impl<'a> LineCursor<'a> {
    /// return a cursor for the given source
    pub fn new(source: &'a str) -> Self {
        let mut pending = source.split('\n');
        let current = pending.next();
        LineCursor {
            source,
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

    fn stretch(&self, from: &'a str) -> Result<&'a str, &'static str> {
        let base = self.source.as_ptr() as usize;
        let limit = base + self.source.len();
        let first = from.as_ptr() as usize;
        if first < base || limit < first {
            return Err("stretch `from` did not come from self.source");
        }
        let last = match self.current {
            None => limit,
            Some(line) => line.as_ptr() as usize - 1,
        };
        if last < base || limit < last {
            return Err("stretch current line isn't from self.source");
        }
        if last < first {
            return Err("stretch `from` comes after current line");
        }
        Ok(&self.source[first - base..last - base])
    }
}

struct Input<'a, 'b, 'r> {
    lines: LineCursor<'a>,
    arena: &'b mut dyn Build<'a>,
    report: &'r mut dyn FnMut(ParseError) -> Reported,
}
impl<'a, 'b, 'r> Input<'a, 'b, 'r> {
    /// finish a complete file
    fn file(&mut self) -> Result<File<'a>, &'static str> {
        let shebang = self.comment(0, CommentMarker::Shebang)?;
        let list = self.list(0)?;
        let mut file = File::from_list_without_epilog(&list);
        file.shebang = shebang;
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
    fn excess(&mut self, indent: usize) -> Result<bool, &'static str> {
        let first = self.lines.number + 1;
        let skipped = self.next(indent)?;
        if skipped {
            self.report(ParseError::new(
                first,
                self.lines.number,
                "excess indentation",
            ))?;
        }
        Ok(skipped)
    }

    /// current line has been recognized as beginning of a Value that might
    /// flow, so stretch a portion of it out to include the whole thing.
    fn flow(&mut self, indent: usize, from: &'a str) -> Result<UTF8<'a>, &'static str> {
        if !self.next(indent)? {
            return Ok(from.into());
        }
        Ok(UTF8::parsed(self.lines.stretch(from)?, indent + 1))
    }

    /// parse a comment if one starts on this line.
    fn comment(
        &mut self,
        indent: usize,
        mark: CommentMarker,
    ) -> Result<Option<UTF8<'a>>, &'static str> {
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
    fn lenient(&mut self, indent: usize, from: &'a str) -> Result<Text<'a>, &'static str> {
        let lines = self.flow(indent, from)?;
        let epilog = self.comment(indent, CommentMarker::Epilog)?;
        Ok(Text {
            lines,
            verbose: false,
            epilog,
        })
    }

    /// parse an indented block of text
    fn text(&mut self, indent: usize) -> Result<Text<'a>, &'static str> {
        let lines = self.block(indent)?;
        let epilog = self.comment(indent, CommentMarker::Epilog)?;
        Ok(Text {
            lines,
            verbose: true,
            epilog,
        })
    }
    /// a block (optionally) follows current line (at indent+1).
    fn block(&mut self, indent: usize) -> Result<UTF8<'a>, &'static str> {
        self.lines.advance();
        let Some(first) = self.lines.dedent(indent + 1) else {
            return Ok(UTF8::default());
        };
        self.flow(indent, first)
    }

    /// previous line opened a list context, so parse all the lines in it.
    fn list(&mut self, indent: usize) -> Result<List<'a>, &'static str> {
        let mut list = List::default();
        list.prolog = self.comment(indent, CommentMarker::Prolog)?;
        list.groups = self.groups(indent)?;
        if indent != 0 {
            list.epilog = self.comment(indent - 1, CommentMarker::Epilog)?;
        }
        self.excess(indent)?;
        Ok(list)
    }

    /// parse any items inside a list context
    fn groups(&mut self, indent: usize) -> Result<Groups<'a>, &'static str> {
        let mut count = 0usize;
        loop {
            let Some(group) = self.group(indent)? else {
                break;
            };
            if let Err(err) = self.arena.push_group(group) {
                self.report(ParseError::Memory(err))?;
                return Err(err); // memory err should always unwind but to be safe
            }
            count += 1;
        }
        if count == 0 {
            Ok(Groups::default())
        } else {
            match self.arena.finish_groups(count) {
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
    fn group(&mut self, indent: usize) -> Result<Option<Group<'a>>, &'static str> {
        let mut group = Group::default();
        group.heading = self.comment(indent, CommentMarker::Group)?;
        group.members = self.members(indent)?;
        if group.heading.is_some() || !group.members.is_empty() {
            Ok(Some(group))
        } else {
            Ok(None)
        }
    }
    // attempt to parse one entry
    fn member(&mut self, indent: usize) -> Result<Option<Member<'a>>, &'static str> {
        let started = self.lines.number;
        let mut member = Member::default();
        member.comment = self.comment(indent, CommentMarker::NameTag)?;
        let message = match self.lines.dedent(indent) {
            None => "",
            Some(scan) if scan.starts_with('<') && scan.ends_with('>') => {
                member.name = UTF8::from(&scan[1..scan.len() - 1]);
                member.data = Data::Text(self.text(indent)?);
                member.longer = true;
                return Ok(Some(member));
            }
            Some(scan) if scan.starts_with('[') && scan.ends_with(']') => {
                member.name = UTF8::from(&scan[1..scan.len() - 1]);
                member.data = Data::List(self.list(indent + 1)?);
                member.longer = true;
                return Ok(Some(member));
            }
            Some(scan) if scan == "@" => {
                member.longer = true;
                member.name = self.block(indent)?;
                match self.lines.dedent(indent) {
                    None => "long `@` key needs a value",
                    Some(text) if text == "<>" => {
                        member.data = Data::Text(self.text(indent)?);
                        return Ok(Some(member));
                    }
                    Some(list) if list == "[]" => {
                        member.data = Data::List(self.list(indent + 1)?);
                        return Ok(Some(member));
                    }
                    Some(more) if more.is_empty() => "gotta use `<>` for empty items",
                    Some(more) if more.starts_with('<') => "malformed `<>` in list",
                    Some(more) if more.starts_with('[') => "malformed `[]` in list",
                    Some(more) if more.starts_with(&CommentMarker::FIRST_CHAR) => "stray comment",
                    Some(more) => {
                        member.data = Data::Text(self.lenient(indent, more)?);
                        return Ok(Some(member));
                    }
                }
            }
            Some(scan) if scan.starts_with('<') => "malformed `<key>` in dict",
            Some(scan) if scan.starts_with('[') => "malformed `[key]` in dict",
            Some(scan) if scan.starts_with('@') => "`@` has trailing char",
            Some(scan) if scan.starts_with(&CommentMarker::FIRST_CHAR) => "stray comment",
            Some(scan) if scan.starts_with('=') => "",
            Some(scan) => {
                if let Some((before, after)) = scan.split_once('=') {
                    member.name = UTF8::from(before);
                    member.data = Data::Text(self.lenient(indent, after)?);
                    return Ok(Some(member));
                } else {
                    member.data = Data::Text(self.lenient(indent, scan)?);
                    return Ok(Some(member));
                }
            }
        };
        if !message.is_empty() {
            self.report(ParseError::at(started, message))?;
        } else if member.comment.is_some() {
            self.report(ParseError::at(started, "dangling comment"))?;
        }
        Ok(None)
    }

    /// parse any entries inside a dict context
    fn members(&mut self, indent: usize) -> Result<Members<'a>, &'static str> {
        let mut count = 0usize;
        loop {
            match self.member(indent)? {
                Some(member) => {
                    count += 1;
                    if let Err(err) = self.arena.push_member(member) {
                        self.report(ParseError::Memory(err))?;
                        return Err(err); // memory err should always unwind but to be safe
                    }
                }
                None => {
                    self.excess(indent)?;
                    if self.lines.dedent(indent).is_none() {
                        break;
                    }
                }
            }
        }
        if count == 0 {
            Ok(Members::default())
        } else {
            match self.arena.finish_members(count) {
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

// #[cfg(feature = "alloc")]
// impl<'a> FromStr for File<'a> {
//     type Err = ParseError;

//     fn from_str(s: &str) -> Result<Self, Self::Err> {
// TODO need an arena that can drop without killing all the contents???
//         Input::new(s).file()
//     }
// }
// impl<'a> FromStr for Group<'a> {
// }
// impl<'a> FromStr for Member<'a> {
// }
// impl<'a> FromStr for Text<'a> {
// }
// impl<'a> FromStr for UTF8<'a> {
// }
