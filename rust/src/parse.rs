use core::usize;

use super::internals::Arena;
use super::*;

pub(crate) struct Input<'a, F> {
    utf8: &'a str, // entire tindalwic encoded content
    line: usize,   // the number of the current line
    start: usize,  // start of current line, `MAX` means finished
    first: usize,  // first non-tab byte of current line
    assign: usize, // the `=` on current line, `MAX` means none
    end: usize,    // the newline ending current line, or `utf8.len()`
    report: F,
}
impl<'a, F> Input<'a, F>
where
    F: FnMut(&(usize, &'static str)),
{
    fn tabs(&self) -> usize {
        self.first - self.start
    }

    /// None means the UTF-8 is too big.
    pub(crate) fn parse(arena: &mut Arena<'a>, utf8: &'a str, report: F) -> Option<File<'a>> {
        if utf8.len() >= usize::MAX - 2 {
            // make sure sentinel can't show up in data
            return None;
        }
        let mut input = Input {
            utf8,
            line: 1,
            start: 0,
            first: 0,
            assign: 0,
            end: usize::MAX,
            report,
        };
        input.next(0);
        let hashbang = input.comment(0, b"#!");
        let dict = input.entries(0, arena)?;
        Some(File {
            cells: dict.cells,
            hashbang,
            prolog: dict.prolog,
        })
    }

    /// examine UTF-8 starting at offset `start` and set: `first`, `assign`, `end`.
    /// if `start` was already past `utf8.len()` set all 4 fields to `MAX` and return
    /// false, else leave `start` as is and return true. never sets any other field.
    fn scan(&mut self) -> bool {
        let bytes = self.utf8.as_bytes();
        let limit = bytes.len();
        let mut offset = self.start;
        if offset >= limit {
            self.start = usize::MAX;
            self.first = usize::MAX;
            self.assign = usize::MAX;
            self.end = usize::MAX;
            return false;
        }
        while offset < limit && bytes[offset] == b'\t' {
            offset += 1;
        }
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
        assert!(
            offset != usize::MAX,
            "impossible: previously checked content size"
        );
        self.end = offset;
        true
    }

    /// current line has been recognized as beginning of a Comment or Text that might
    /// continue, so stretch it out to include the whole thing by changing `end`.
    fn stretch(&mut self, mut indent: usize, from: usize) -> UTF8<'a> {
        if !self.stretch_once(indent) {
            indent = usize::MAX;
        } else {
            while self.stretch_once(indent) {}
        }
        return UTF8 {
            slice: &self.utf8[from..self.end],
            dedent: indent,
        };
    }
    fn stretch_once(&mut self, mut indent: usize) -> bool {
        let bytes = self.utf8.as_bytes();
        let limit = bytes.len();
        let mut offset = self.end;
        if offset >= limit {
            return false;
        }
        assert!(bytes[offset] == b'\n', "impossible: not at newline");
        offset += 1;
        while offset < limit && bytes[offset] == b'\t' {
            offset += 1;
            if indent > 0 {
                indent -= 1;
            }
        }
        if indent != 0 {
            return false;
        }
        while offset < limit && bytes[offset] != b'\n' {
            offset += 1;
        }
        self.end = offset;
        true
    }

    /// done with current line, so advance, skipping excessively indented lines.
    /// pass usize::MAX to never skip. return false if finished with entire UTF-8.
    fn next(&mut self, excess: usize) -> bool {
        if self.start == usize::MAX {
            return false;
        }
        self.line += 1;
        self.start = self.end.wrapping_add(1);
        if !self.scan() {
            return false;
        }
        if self.tabs() <= excess {
            return true;
        }
        let begin = self.line;
        self.line += 1;
        self.start = self.end.wrapping_add(1);
        while self.scan() && self.tabs() > excess {
            self.line += 1;
            self.start = self.end.wrapping_add(1);
        }
        (self.report)(&(begin, "excess indentation")); // TODO mention self.line-1
        self.start != usize::MAX
    }

    /// use this whenever a comment is allowed, returns None if current line does not
    /// have exactly the provided indent and prefix.
    fn comment(&mut self, indent: usize, prefix: &'static [u8]) -> Option<Comment<'a>> {
        if self.start == usize::MAX || self.tabs() != indent {
            return None;
        }
        let bytes = self.utf8.as_bytes();
        let limit = bytes.len();
        let from = self.first + prefix.len();
        if from > limit || &bytes[self.first..from] != prefix {
            return None;
        }
        let utf8 = self.stretch(indent + 1, from);
        self.next(usize::MAX); // stretch means excess is impossible
        Some(Comment { utf8 })
    }

    /// current line has been recognized as beginning a Text, from a `<>` context on
    /// the previous line, or from shortcut syntax. `from` says where text begins.
    /// lenient - one-liners can stretch.
    fn text(&mut self, indent: usize, from: usize) -> Text<'a> {
        let utf8 = self.stretch(indent + 1, from);
        self.next(usize::MAX); // stretch means excess is impossible
        let epilog = self.comment(indent, b"#");
        Text { utf8, epilog }
    }

    /// previous line opened a list context, so parse all the items in it.
    /// None means insufficient space in Arena.
    fn items(&mut self, indent: usize, arena: &mut Arena<'a>) -> Option<List<'a>> {
        let bytes = self.utf8.as_bytes();
        let prolog = self.comment(indent, b"#");
        let mut count = 0usize;
        while self.start != usize::MAX {
            let mut item: Option<Item<'a>> = None;
            let tabs = self.tabs();
            if (tabs == 0 || tabs == indent) && self.first >= self.end {
                // empty or indentation-only is shortcut for empty text
                item = Some(self.text(indent, self.end).into());
            } else if tabs == indent {
                let len = self.end - self.first;
                match bytes[self.first] {
                    b'#' => {
                        (self.report)(&(self.line, "stray `#` comment"));
                        self.comment(indent, b"#"); // read and throw away
                    }
                    b'/' => {
                        (self.report)(&(
                            self.line,
                            if len < 2 || bytes[self.first + 1] != b'/' {
                                "malformed // comment"
                            } else {
                                "no // comments in lists"
                            },
                        ));
                        self.comment(indent, b"/"); // read and throw away
                    }
                    b'<' => {
                        if len != 2 || bytes[self.end - 1] != b'>' {
                            (self.report)(&(self.line, "malformed `<>` in list"));
                            self.next(indent);
                        } else {
                            let end = self.end;
                            item = Some(
                                if !self.stretch_once(indent + 1) {
                                    // zero lines in this text, take empty slice from this line
                                    self.text(indent, end)
                                } else {
                                    // first line of stretched text can have excess indent
                                    self.text(indent, end + indent + 1)
                                }
                                .into(),
                            );
                        }
                    }
                    b'[' => {
                        if len != 2 || bytes[self.end - 1] != b']' {
                            (self.report)(&(self.line, "malformed `[]` in list"));
                            self.next(indent);
                        } else {
                            item = Some(self.items(indent + 1, arena)?.into());
                        }
                    }
                    b'{' => {
                        if len != 2 || bytes[self.end - 1] != b'}' {
                            (self.report)(&(self.line, "malformed `{}` in list"));
                            self.next(indent);
                        } else {
                            item = Some(self.entries(indent + 1, arena)?.into());
                        }
                    }
                    _ => {
                        item = Some(self.text(indent, self.start + indent).into());
                    }
                }
            }
            if let Some(item) = item {
                arena.item(item)?;
                count += 1;
            }
        }
        let mut list = arena.list(count)?;
        if indent > 0 {
            list.epilog = self.comment(indent - 1, b"#");
        }
        list.prolog = prolog;
        Some(list)
    }

    /// previous line opened a dict context, so parse all the entries in it.
    /// None means insufficient space in Arena.
    fn entries(&mut self, indent: usize, arena: &mut Arena<'a>) -> Option<Dict<'a>> {
        let bytes = self.utf8.as_bytes();
        let prolog = self.comment(indent, b"#");
        let mut count = 0usize;
        while self.start != usize::MAX {
            let mut item: Option<Item<'a>> = None;
            let tabs = self.tabs();
            let gap = if (tabs == 0 || tabs == indent) && self.first >= self.end {
                self.next(indent);
                true
            } else {
                false
            };
            let before = self.comment(indent, b"//");
            if self.start == usize::MAX || tabs != indent {
                // report if gap or before
                break;
            }
            let mut key: &'a str = "";
            let len = self.end - self.first;
            match bytes[self.first] {
                b'#' => {
                    (self.report)(&(self.line, "stray `#` comment"));
                    self.comment(indent, b"#"); // read and throw away
                }
                b'/' => {
                    (self.report)(&(
                        self.line,
                        if len < 2 || bytes[self.first + 1] != b'/' {
                            "malformed // comment"
                        } else {
                            "stray `//` comment"
                        },
                    ));
                    self.comment(indent, b"/"); // read and throw away
                }
                b'<' => {
                    if len < 2 || bytes[self.end - 1] != b'>' {
                        (self.report)(&(self.line, "malformed `<key>` in dict"));
                        self.next(indent);
                    } else {
                        key = &self.utf8[self.first + 1..self.end - 1];
                        let end = self.end;
                        item = Some(
                            if !self.stretch_once(indent + 1) {
                                // zero lines in this text, take empty slice from this line
                                self.text(indent, end)
                            } else {
                                // first line of stretched text can have excess indent
                                self.text(indent, end + 1 + indent + 1)
                            }
                            .into(),
                        );
                    }
                }
                b'[' => {
                    if len < 2 || bytes[self.end - 1] != b']' {
                        (self.report)(&(self.line, "malformed `[key]` in dict"));
                        self.next(indent);
                    } else {
                        key = &self.utf8[self.first + 1..self.end - 1];
                        self.next(indent + 1);
                        item = Some(self.items(indent + 1, arena)?.into());
                    }
                }
                b'{' => {
                    if len < 2 || bytes[self.end - 1] != b'}' {
                        (self.report)(&(self.line, "malformed `{key}` in dict"));
                        self.next(indent);
                    } else {
                        key = &self.utf8[self.first + 1..self.end - 1];
                        self.next(indent + 1);
                        item = Some(self.entries(indent + 1, arena)?.into());
                    }
                }
                b'\t' => {
                    (self.report)(&(self.line, "excess indentation?"));
                    self.next(indent);
                }
                _ => {
                    if self.assign == usize::MAX {
                        (self.report)(&(self.line, "missing `=` in dict"))
                    } else {
                        key = &self.utf8[self.first..self.assign];
                        item = Some(self.text(indent, self.assign + 1).into());
                    }
                }
            }
            if let Some(item) = item {
                arena.entry(Entry {
                    name: Name { key, gap, before },
                    item,
                })?;
                count += 1;
            } // else report if gap or before or key
        }
        let mut dict = arena.dict(count)?;
        dict.prolog = prolog;
        if indent > 0 {
            dict.epilog = self.comment(indent - 1, b"#");
        }
        Some(dict)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty() {
        let items = Item::array::<0>();
        let entries = Entry::array::<0>();
        let mut arena = internals::Arena::wrap(&items, &entries);
        let file =
            parse::Input::parse(&mut arena, "", |(line, message)| panic!("{line}:{message}"));
        match file {
            None => panic!("got None"),
            Some(file) => {
                assert!(file.is_empty())
            }
        }
    }
    #[test]
    fn assign() {
        let items = Item::array::<0>();
        let entries = Entry::array::<1>();
        let mut arena = internals::Arena::wrap(&items, &entries);
        let file = parse::Input::parse(&mut arena, "k=v", |(line, message)| {
            panic!("{line}:{message}")
        });
        match file {
            None => panic!("got None"),
            Some(file) => {
                assert!(file.hashbang.is_none());
                assert!(file.prolog.is_none());
                assert_eq!(file.cells.len(), 1);
                let entry = file.find("k").unwrap();
                let Item::Text(text) = entry.get().item else {
                    panic!("not text?");
                };
                let mut lines = text.lines();
                assert_eq!(lines.next(), Some("v"));
                assert_eq!(lines.next(), None);
            }
        }
    }
    #[test]
    fn sublist() {
        let items = Item::array::<1>();
        let entries = Entry::array::<1>();
        let mut arena = internals::Arena::wrap(&items, &entries);
        let file = parse::Input::parse(&mut arena, "[k]\n\tv", |(line, message)| {
            panic!("{line}:{message}")
        });
        match file {
            None => panic!("got None"),
            Some(file) => {
                assert!(file.hashbang.is_none());
                assert!(file.prolog.is_none());
                assert_eq!(file.cells.len(), 1);
                let entry = file.find("k").unwrap();
                let Item::List(list) = entry.get().item else {
                    panic!("not list?");
                };
                assert_eq!(list.cells.len(), 1);
                let Item::Text(text) = list.cells[0].get() else {
                    panic!("not text?");
                };
                let mut lines = text.lines();
                assert_eq!(lines.next(), Some("v"));
                assert_eq!(lines.next(), None);
            }
        }
    }
    #[test]
    fn subdict() {
        let items = Item::array::<0>();
        let entries = Entry::array::<2>();
        let mut arena = internals::Arena::wrap(&items, &entries);
        let file = parse::Input::parse(&mut arena, "{z}\n\t<k>\n\t\tv", |(line, message)| {
            panic!("{line}:{message}")
        });
        walk! {
            let v = (file.unwrap()){"z"}<"k">.unwrap();
        }
        let mut lines = v.lines();
        assert_eq!(lines.next(), Some("v"));
        assert_eq!(lines.next(), None);
    }
}
