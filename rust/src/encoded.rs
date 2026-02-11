/// UTF-8 storage for both [crate::comments::Comment] and [crate::values::Value::Text].
///
/// The name "`Encoded`" indicates the ability to store the UTF-8 exactly as read from
/// a Tindalwic [crate::File], including the indentation TAB chars. An app that reads,
/// then modifies only a few of the values, does not have to pay for decoding the whole
/// file - most of the values will already be properly encoded to write back out.
///
/// The content ownership can be tricky. The caller always provides an immutable
/// string slice to one of the constructors, and the Comment keeps a sub-slice. Zero
/// UTF-8 bytes are moved. But the lifetimes become entangled: the compiler will
/// insist that the caller not drop the source of the string slice without first/also
/// dropping the Comment.

#[derive(Debug, Eq, Clone, Copy)]
pub struct Encoded<'a> {
    verbatim: &'a str,
    dedent: usize, // MAX means single-line
}

impl<'a> Ord for Encoded<'a> {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        if self.dedent == other.dedent {
            self.verbatim.cmp(other.verbatim)
        } else {
            self.lines().cmp(other.lines())
        }
    }
}

impl<'a> PartialOrd for Encoded<'a> {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl<'a> PartialEq for Encoded<'a> {
    fn eq(&self, other: &Self) -> bool {
        self.cmp(other) == std::cmp::Ordering::Equal
    }
}

impl<'a> Encoded<'a> {
    /// Returns an [Iterator] over the lines (without newline chars).
    ///
    /// This is the most efficient way to access the content. No UTF-8 bytes are moved,
    /// the returned slices simply skip past the indentation TAB chars.
    ///
    /// # Examples
    ///
    /// ```
    /// let comment = tindalwic::comment("zero\none\ntwo");
    /// let expect = ["zero", "one", "two"];
    /// for (index, line) in comment.unwrap().gfm.lines().enumerate() {
    ///     assert_eq!(line, expect[index]);
    /// }
    /// ```
    pub fn lines(&self) -> impl Iterator<Item = &'a str> {
        // that return type is very tricky to satisfy: having two branches here (one
        // optimized for absent indentation) causes E0308 incompatible types:
        //   "distinct uses of `impl Trait` result in different opaque types"
        // attempting to hide them behind closures does not help either:
        //   "no two closures, even if identical, have the same type"
        let d = if self.dedent == usize::MAX {
            0
        } else {
            self.dedent
        };
        self.verbatim
            .split('\n')
            .enumerate()
            .map(move |(i, s)| if i == 0 || d == 0 { s } else { &s[d..] })
    }

    /// Gathers the [Self::lines] into a freshly allocated [String].
    ///
    /// # Examples
    ///
    /// ```
    /// let utf8 = "zero\none\ntwo";
    /// let comment = tindalwic::comment(utf8);
    /// assert_eq!(comment.unwrap().gfm.to_string(), utf8);
    /// ```
    pub fn to_string(&self) -> String {
        if self.dedent == 0 || self.dedent == usize::MAX {
            return String::from(self.verbatim);
        }
        let mut string = String::new();
        for line in self.lines() {
            string.push_str(line);
            string.push('\n');
        }
        if string.len() != 0 {
            string.truncate(string.len() - 1);
        }
        string
    }

    /// Constructor from a string slice.
    pub(crate) fn adopt(utf8: &'a str) -> Self {
        Encoded {
            verbatim: utf8,
            dedent: if utf8.contains('\n') { 0 } else { usize::MAX },
        }
    }

    pub(crate) fn parse(source: &'a str, indent: usize) -> Self {
        let bytes = source.as_bytes();
        let mut newlines = 0usize;
        let indent = indent + 1;
        let mut cursor = 0;
        'outer: while cursor < bytes.len() {
            if bytes[cursor] != b'\n' {
                cursor += 1;
                continue;
            }
            if cursor + indent >= bytes.len() {
                break;
            }
            for offset in 0..indent {
                if bytes[cursor + 1 + offset] != b'\t' {
                    break 'outer;
                }
            }
            cursor += 1 + indent;
            newlines += 1;
        }
        Encoded {
            verbatim: &source[..cursor],
            dedent: if newlines == 0 { usize::MAX } else { indent },
        }
    }

    /// write the encoding of this Comment into the given String.
    pub(crate) fn build(&self, indent: usize, marker: &'static str, into: &mut String) {
        into.extend(std::iter::repeat_n('\t', indent));
        into.push_str(marker);
        let indent = indent + 1;
        if indent == self.dedent || self.dedent == usize::MAX {
            into.push_str(self.verbatim);
            into.push('\n');
        } else {
            let mut lines = self.lines();
            let Some(first) = lines.next() else {
                into.push('\n');
                return;
            };
            into.push_str(first);
            into.push('\n');
            for line in lines {
                into.extend(std::iter::repeat_n('\t', indent));
                into.push_str(&line[self.dedent..]);
                into.push('\n');
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn visible(string: &str) -> String {
        string.replace("╶─▸", "\t").replace("▁▁▎", "\n")
    }

    struct Expect(String);
    impl Expect {
        fn from(&self, indent: usize, parse: &'static str) -> &Self {
            let parse = visible(parse);
            let vec: Vec<&str> = Encoded::parse(&parse, indent).lines().collect();
            assert_eq!(vec.join("\n"), self.0);
            self
        }
    }

    #[test]
    fn parse() {
        Expect(visible("c")).from(0, "c");

        Expect(visible("a▁▁▎b"))
            .from(0, "a▁▁▎╶─▸b▁▁▎...")
            .from(1, "a▁▁▎╶─▸╶─▸b▁▁▎╶─▸...")
            .from(2, "a▁▁▎╶─▸╶─▸╶─▸b▁▁▎╶─▸╶─▸...");

        Expect(visible("a▁▁▎╶─▸b"))
            .from(0, "a▁▁▎╶─▸╶─▸b▁▁▎...")
            .from(1, "a▁▁▎╶─▸╶─▸╶─▸b▁▁▎╶─▸...");
    }
}
