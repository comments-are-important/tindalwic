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
    tabs: usize,   // indentation on this line, unless gap, then peek from next line
    report: F,
}
impl<'a, F> Input<'a, F>
where
    F: FnMut(&(usize, &'static str)),
{
    /// None means the arena is too small (or the UTF-8 is way too big).
    pub(crate) fn parse(arena: &mut Arena<'a>, utf8: &'a str, report: F) -> Option<File<'a>> {
        if utf8.len() >= usize::MAX {
            // MAX is a sentinel, so it also can't be a len. the wrap-around that could
            // maybe happen without this check will almost certainly never actually
            // occur because the arena will fill up before that.
            return None;
        }
        let mut input = Input {
            utf8,
            line: 0,
            start: 0,
            first: 0,
            assign: 0,
            end: usize::MAX, // will wrap to 0 inside `next`
            tabs: 0,
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

    /// done with current line, so advance, skipping excessively indented lines.
    /// usize::MAX prevents skipping. return false if finished with entire UTF-8.
    /// use `stretch` instead for Comment and Text (where no line is excessive).
    fn next(&mut self, indent: usize) -> bool {
        if self.start == usize::MAX {
            return false;
        }
        self.line += 1;
        self.start = self.end.wrapping_add(1);
        if !self.scan() {
            return false;
        }
        if self.tabs <= indent {
            return true;
        }
        let begin = self.line;
        self.line += 1;
        self.start = self.end + 1;
        while self.scan() && self.tabs > indent {
            self.line += 1;
            self.start = self.end + 1;
        }
        (self.report)(&(begin, "excess indentation")); // TODO begin..self.line-1
        return self.start != usize::MAX;
    }

    /// helper for `next` to update state by examining a line of UTF-8.
    /// assumes caller has correctly set `self.start` (out-of-bounds is fine).
    fn scan(&mut self) -> bool {
        let bytes = self.utf8.as_bytes();
        let limit = bytes.len();
        let mut offset = self.start;
        if offset >= limit {
            self.start = usize::MAX;
            self.first = usize::MAX;
            self.assign = usize::MAX;
            self.tabs = 0;
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
        self.end = offset; // never MAX because `parse` checked length
        if self.start != self.end {
            self.tabs = self.first - self.start;
            return true;
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
            (self.report)(&(begin, "consecutive empty lines")); // TODO begin..self.line-1
            self.start = offset - 1;
            self.first = offset - 1;
            self.end = offset - 1;
        }
        while offset < limit && bytes[offset] == b'\t' {
            offset += 1;
        }
        self.tabs = offset - 1 - self.end;
        return true;
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
        self.end = offset; // never MAX because `parse` checked length
        true
    }

    /// use this whenever a comment is allowed, returns None if current line does not
    /// have exactly the provided indent and prefix.
    fn comment(&mut self, indent: usize, prefix: &'static [u8]) -> Option<Comment<'a>> {
        if self.start == usize::MAX || self.tabs != indent {
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
            if self.start == self.end || self.tabs != indent {
                break;
            } else if self.first >= self.end {
                // indentation-only is the shortcut for empty text
                // TODO too easily confused with gaps (require explicit `<>`)?
                item = Some(self.text(indent, self.end).into());
            } else {
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
                            self.next(indent + 1);
                            item = Some(self.items(indent + 1, arena)?.into());
                        }
                    }
                    b'{' => {
                        if len != 2 || bytes[self.end - 1] != b'}' {
                            (self.report)(&(self.line, "malformed `{}` in list"));
                            self.next(indent);
                        } else {
                            self.next(indent + 1);
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
            let gap = self.tabs == indent && self.first == self.end;
            if gap {
                self.next(indent);
            }
            let before = self.comment(indent, b"//");
            if self.start == usize::MAX || self.tabs != indent {
                if gap || before.is_some() {
                    (self.report)(&(self.line, "gap/before but no key"));
                }
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
            } else if gap || before.is_some() {
                (self.report)(&(self.line, "gap/before but no item"));
            }
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

    fn bail(args: &(usize, &str)) {
        let (line, message) = args;
        panic!("{line}: {message}");
    }

    macro_rules! assert_lines_eq {
        // checking this gets repetitive without Vec
        ($text:ident, $($line:literal),*) => {
            let mut it = $text.lines();
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
        let file = Input::parse(&mut arena, "", bail).unwrap();
        assert!(!arena.completed().is_some());
        assert!(!file.has_content());
    }

    #[test]
    fn key_eq_value() {
        arena! {
            $crate = crate;
            let mut arena = <1dict>;
        }
        let file = Input::parse(&mut arena, "k=v", bail).unwrap();
        assert!(arena.completed().is_some());
        assert!(file.hashbang.is_none());
        assert!(file.prolog.is_none());
        assert_eq!(file.cells.len(), 1);
        let entry = file.get("k").unwrap();
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
        let file = Input::parse(&mut arena, "[k]\n\t1\n\t2\n\t3", bail).unwrap();
        assert!(arena.completed().is_some());
        assert_eq!(file.cells.len(), 1);
        let entry = file.get("k").unwrap();
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
        let file = Input::parse(&mut arena, "{z}\n\t<k>\n\t\tv", bail).unwrap();
        assert!(arena.completed().is_some());
        walk! { $crate = crate;
            let v = (&file){"z"}<"k">.unwrap();
        }
        assert_lines_eq!(v, "v");
    }
}
