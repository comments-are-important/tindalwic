import textwrap
import sys
from io import StringIO
from string import Template
from typing import NamedTuple

# gave up on macros - too awkward for what was needed.
# but still want some assurance of consistency, so...

lib = open("src/lib.rs", "r+")
div = f"// {'#' * 84}\n"

while line := lib.readline():
    if line == div:
        pos = lib.tell()
        lib.truncate(pos)
        lib.seek(pos)
        break
else:
    sys.exit("did not find divide marker in lib.rs")


class DOC(NamedTuple):
    string: str
    doc: str


hashbang = DOC("hashbang", r"/// A $name can start with a Unix `#!` Comment.")
prolog = DOC("prolog", r"/// A $name can have an introductory Comment.")
epilog = DOC("epilog", r"/// A $name can have a Comment after it.")


def unquote(block: str) -> str:
    return textwrap.dedent(block).strip("\n")


class Indent:
    def __init__(self, level: int):
        self.level = level
        self.buffer = StringIO()

    def append(self, block: str, *, frame: int = 1, doc: DOC | None = None) -> None:
        values = dict[str, str]()
        for key, value in sys._getframe(frame).f_locals.items():
            match value:
                case str():
                    values[key] = value
                case Indent():
                    values[key] = value.buffer.getvalue()
        if doc:
            values["string"] = doc.string
            values["__doc__"] = doc.doc
        spaces = 4 * self.level
        substituted = Template(unquote(block)).substitute(values)
        while "$" in substituted:
            substituted = Template(substituted).substitute(values)
        indented = textwrap.indent(substituted, " " * spaces)
        if self.buffer.tell() == 0:
            indented = indented[spaces:]
        else:
            self.buffer.write("\n")
        self.buffer.write(indented)


def out(block: str | None) -> None:
    if block is not None:
        template = Indent(0)
        template.append(block, frame=2)
        print(template.buffer.getvalue(), file=lib)


def STRUCT(name: str, *comments: DOC, doc: str, **fields: str | DOC):
    print("", file=lib)
    out(doc)
    declare = Indent(1)
    for field, type in fields.items():
        visible = "" if field.startswith("_") else "pub "
        if not visible:
            field = field.lstrip("_")
        match type:
            case str():
                declare.append(r"$visible$field: $type,")
            case DOC():
                declare.append(
                    r"""
                    $__doc__
                    $visible$field: $string,""",
                    doc=type,
                )
    for comment in comments:
        declare.append(
            r"""
            $__doc__
            pub $string: Option<Comment<'a>>,
            """,
            doc=comment,
        )
    out(r"""
        #[derive(Clone, Debug)]
        pub struct $name<'a> {
            $declare
        }""")
    if comments:
        builders = Indent(1)
        for comment, _ in comments:
            builders.append(r"""
                /// Sets the $comment Comment.
                pub fn with_$comment(mut self, $comment: &'a str) -> Self {
                    self.$comment = Comment::some($comment);
                    self
                }""")
        out(r"""
            impl<'a> $name<'a> {
                $builders
            }""")


def FROM(name: str, param: str, type: str, *comments: DOC, **fields: str | DOC):
    initialize = Indent(3)
    for field, init in fields.items():
        initialize.append(r"$field: $init,")
    for comment in comments:
        initialize.append(r"$string: None,", doc=comment)
    out(r"""
        impl<'a> From<$type> for $name<'a> {
            fn from($param: $type) -> Self {
                $name {
                    $initialize
                }
            }
        }""")


def UTF8(name: str, *comments: DOC, doc: str):
    STRUCT(name, *comments, _encoded="Encoded<'a>", doc=doc)
    FROM(
        name,
        "utf8",
        "&'a str",
        *comments,
        encoded=r"Encoded::from(utf8)",
    )
    initialize = Indent(3)
    initialize.append(r"encoded: Encoded::parse(source, indent),")
    for comment in comments:
        initialize.append(r"$string: None,", doc=comment)
    out(r"""
        impl<'a> $name<'a> {
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
            /// let item = tindalwic::$name::from(utf8);
            /// for (index, line) in item.lines().enumerate() {
            ///     assert_eq!(line, expect[index]);
            /// }
            /// ```
            pub fn lines(&self) -> impl Iterator<Item = &'a str> {
                self.encoded.lines()
            }
            fn parse_utf8(source: &'a str, indent: usize) -> Self {
                $name {
                    $initialize
                }
            }
        }
        """)


def ARRAY(name: str, item: str, comments: tuple[DOC, DOC], doc: str):
    type = f"Vec<{item}<'a>>"
    STRUCT(
        name,
        *comments,
        doc=doc,
        vec=DOC(type, r"/// The contents of the Value::$name."),
    )
    FROM(name, "items", type, *comments, vec="items")


def LIST(name: str, comments: tuple[DOC, DOC], doc: str):
    ARRAY(name, "Value", comments, doc=doc)


def DICT(name: str, comments: tuple[DOC, DOC], doc: str):
    ARRAY(name, "Keyed", comments, doc=doc)
    out(r"""
        impl<'a> $name<'a> {
            /// returns number of entries.
            pub fn len(&self) -> usize {
                self.vec.len()
            }
            /// returns the position of the entry with the given key.
            pub fn position(&self, key: &str) -> Option<usize> {
                self.vec.iter().position(|x| x.key == key)
            }
            /// returns a reference to the entry with the given key.
            pub fn find(&self, key: &str) -> Option<&Keyed<'a>> {
                self.position(key).map(|i| &self.vec[i])
            }
            /// returns a mutable reference to the entry with the given key.
            pub fn find_mut(&mut self, key: &str) -> Option<&mut Keyed<'a>> {
                self.position(key).map(|i| &mut self.vec[i])
            }
            /// append the given entry to the end of the vec.
            pub fn push(&mut self, keyed: Keyed<'a>) {
                self.vec.push(keyed);
            }
        }
        """)


UTF8(
    "Comment",
    doc=r"""
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
        /// assert_eq!(html, "<p>with <del>strikethrough</del> extension</p>\n");
        /// ```
        """,
)
UTF8(
    "Text",
    epilog,
    doc=r"""
        /// the fields of a [Value::$name]
        """,
)
LIST(
    "List",
    (prolog, epilog),
    doc=r"""
        /// the fields of a [Value::$name]
        """,
)
DICT(
    "Dict",
    (prolog, epilog),
    doc=r"""
        /// the fields of a [Value::$name]
        """,
)
DICT(
    "File",
    (hashbang, prolog),
    doc=r"""
        /// the outermost context.
        ///
        /// very similar to a [Value::Dict], just with different comments.
        """,
)

lib.close()
