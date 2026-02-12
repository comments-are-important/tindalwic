use crate::encoded::Encoded;

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
/// let comment = tindalwic::Comment::adopt("with ~strikethrough~ extension");
///
/// let html = markdown::to_html_with_options(&comment.unwrap().gfm.to_string(), &markdown::Options::gfm())
///   .expect("should never error, according to:
///      https://docs.rs/markdown/latest/markdown/fn.to_html_with_options.html#errors");
///
/// assert_eq!(html, "<p>with <del>strikethrough</del> extension</p>");
/// ```

#[derive(Debug, Ord, PartialOrd, PartialEq, Eq, Clone, Copy)]
pub struct Comment<'a> {
    /// the encoded content (Github Flavored Markdown)
    pub gfm: Encoded<'a>,
}

impl<'a> Comment<'a> {
    /// wrap a reference to content into a Comment
    pub fn adopt(gfm: &'a str) -> Option<Self> {
        Some(Comment {
            gfm: Encoded::adopt(gfm),
        })
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
            Some(Comment {
                gfm: Encoded::parse(&source[indent + 1..], indent),
            })
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
            Some(Comment {
                gfm: Encoded::parse(&source[indent + 2..], indent),
            })
        }
    }

    /// write the encoding of this Comment into the given String.
    pub(crate) fn encode(&self, indent: usize, marker: &'static str, into: &mut String) {
        self.gfm.encode(indent, marker, into);
    }
}
