macro_rules! encoded_dedent_struct {
    {
        $(#[$struct_meta:meta])*
        $vis:vis struct $name:ident<$lifetime:lifetime> {
            $( $(#[$comment_meta:meta])* $comment:ident ),* $(,)?
        }
    } => {
        $(#[$struct_meta])*
        #[derive(Clone, Debug)]
        pub struct $name<$lifetime> {
            encoded: &$lifetime str,
            dedent: usize, // MAX means single-line
            $( $(#[$comment_meta])* $comment: Option<Comment<$lifetime>>, )*
        }
        impl<$lifetime> From<&$lifetime str> for $name<$lifetime> {
            fn from(utf8: &$lifetime str) -> Self {
                $name {
                    encoded: utf8,
                    dedent: if utf8.contains('\n') { 0 } else { usize::MAX },
                    $( $comment: None, )*
                }
            }
        }
        // impl<$lifetime> Ord for $name<$lifetime> {
        //     fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        //         if self.dedent == other.dedent {
        //             self.encoded.cmp(other.encoded)
        //         } else {
        //             self.lines().cmp(other.lines())
        //         }
        //     }
        // }
        // impl<$lifetime> PartialOrd for $name<$lifetime> {
        //     fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        //         Some(self.cmp(other))
        //     }
        // }
        // impl<$lifetime> PartialEq for $name<$lifetime> {
        //     fn eq(&self, other: &Self) -> bool {
        //         self.cmp(other) == std::cmp::Ordering::Equal
        //     }
        // }
        // impl<$lifetime> Eq for $name<$lifetime> {}
        impl<$lifetime> $name<$lifetime> {

            /// Returns an [Iterator] over the lines (without newline chars).
            ///
            /// This is the most efficient way to access the content. No UTF-8 bytes are moved,
            /// the returned slices simply skip past the indentation TAB chars.
            ///
            /// # Examples
            ///
            /// ```
            /// let expect = ["zero", "one", "two"];
            /// let utf8 = "zero\none\ntwo";
            #[doc = concat!("let item = tindalwic::", stringify!($name), "::from(utf8);")]
            /// for (index, line) in item.lines().enumerate() {
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
                self.encoded
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
            #[doc = concat!("let item = tindalwic::", stringify!($name), "::from(utf8);")]
            /// assert_eq!(item.to_string(), utf8);
            /// ```
            pub fn to_string(&self) -> String {
                if self.dedent == 0 || self.dedent == usize::MAX {
                    return String::from(self.encoded);
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

            pub(crate) fn parse_utf8(source: &'a str, indent: usize) -> Self {
                let bytes = source.as_bytes();
                let mut newlines = 0usize;
                let indent = indent + 1;
                let mut cursor = 0usize;
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
                $name {
                    encoded: &source[..cursor],
                    dedent: if newlines == 0 { usize::MAX } else { indent },
                    $( $comment: None, )*
                }
            }

            /// write the encoding of this Comment into the given String.
            pub(crate) fn encode_utf8(&self, indent: usize, marker: &'static str, into: &mut String) {
                into.extend(std::iter::repeat_n('\t', indent));
                into.push_str(marker);
                let indent = indent + 1;
                if indent == self.dedent || self.dedent == usize::MAX {
                    into.push_str(self.encoded);
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
    };
}

encoded_dedent_struct! {
    /// Metadata about a Value or a File.
    ///
    /// The content is UTF-8 Github Flavored Markdown and kept in the encoded form. The
    /// fields are private because the encoded form is awkward to work with. An app that
    /// ignores the Comments does not have to pay for decoding them: in most cases the
    /// Comment content as read is already perfect for writing.
    ///
    /// A field within the Value will hold the Comment, there is no mechanism to navigate
    /// from a Comment to the Value it describes.
    ///
    /// The content ownership can be tricky. The caller always provides an immutable
    /// string slice to one of the constructors, and the Comment keeps a sub-slice. Zero
    /// UTF-8 bytes are moved. But the lifetimes become entangled: the compiler will
    /// insist that the caller not drop the source of the string slice without first/also
    /// dropping the Comment.
    ///
    /// # Examples
    ///
    /// ```
    /// let comment = tindalwic::Comment::from("with ~strikethrough~ extension");
    ///
    /// let html = markdown::to_html_with_options(&comment.to_string(), &markdown::Options::gfm())
    ///   .expect("should never error, according to:
    ///      https://docs.rs/markdown/latest/markdown/fn.to_html_with_options.html#errors");
    ///
    /// assert_eq!(html, "<p>with <del>strikethrough</del> extension</p>");
    /// ```
    pub struct Comment<'a> {}
}

impl<'a> Comment<'a> {
    /// instantiate into [Option::Some].
    pub fn some(utf8: &'a str) -> Option<Self> {
        Some(Comment::from(utf8))
    }

    /// Attempt to parse a `#` Comment.
    ///
    /// The `source` parameter can start with zero or
    /// more tab ('\t' == 0x09) chars, followed by a `#` char. When that criteria is
    /// met, a Comment will be parsed. It will have at least one line (starting just
    /// after the `#`), possibly more. The parsing stops according to the rules, so the
    /// Comment might not extend all the way to the `source` end.
    pub(crate) fn octothorpe(source: &'a str) -> Option<Self> {
        let indent = source.len() - source.trim_start_matches('\t').len();
        if indent >= source.len() || source.as_bytes()[indent] != b'#' {
            None
        } else {
            Some(Comment::parse_utf8(&source[indent + 1..], indent))
        }
    }

    /// Attempt to parse a `//` Comment.
    ///
    /// The `source` parameter can start with zero or
    /// more tab ('\t' == 0x09) chars, followed by two `/` chars. When that criteria is
    /// met, a Comment will be parsed. It will have at least one line (starting just
    /// after the `//`), possibly more. The parsing stops according to the rules, so the
    /// Comment might not extend all the way to the `source` end.
    pub(crate) fn slash_slash(source: &'a str) -> Option<Self> {
        let indent = source.len() - source.trim_start_matches('\t').len();
        let bytes = source.as_bytes();
        if indent + 1 >= source.len() || bytes[indent] != b'/' || bytes[indent + 1] != b'/' {
            None
        } else {
            Some(Comment::parse_utf8(&source[indent + 2..], indent))
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
            let vec: Vec<&str> = Comment::parse_utf8(&parse, indent).lines().collect();
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
