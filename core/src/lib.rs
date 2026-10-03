#![no_std]

//! Text in Nested Dictionaries and Lists - with Important Comments

use core::cell::Cell;

#[doc(inline)]
/// build a [walk::Path]
pub use tindalwic_macros::path;

#[doc(inline)]
/// build an [Item] using a subset of the JSON syntax.
///
/// this helps to write code snippets that make a structural change to a [File].
/// a typical snippet would:
///  + [path!].walk([File].cells) to the place to be changed,
///  + use [json!] to build a new [Item],
///  + then use [core::cell::Cell::set] to affect the change.
pub use tindalwic_macros::json;

#[doc(inline)]
pub use tindalwic_macros::arena;

pub mod capped;
pub mod fmt;
pub mod walk;

#[cfg(feature = "alloc")]
pub mod alloc;
#[cfg(feature = "bumpalo")]
pub mod bumpalo;

mod value;

/// converting bytes into a File
pub mod parse {
    pub use super::value::parse::{Build, Parse, ParseError, Reported};
}

/// the semver plus the git fingerprint
pub const VERSION: &str = env!("TINDALWIC_VERSION");

// ====================================================================================

pub use value::Value;
impl<'a> Value<'a> {
    /// linear `O(n)` scan.
    // TODO: add link to `alloc` map view, say it "offers `O(1)`."
    pub fn find_linearly_in(self, cells: Entries<'_>) -> Option<usize> {
        cells.iter().position(|cell| cell.get().name.key == self)
    }
}

// ====================================================================================

/// Metadata about an [Item], [Entry] or [File].
///
/// A serialized [Comment] will start with one of three possible markers, depending
/// on its position:
///  + `#!` for [File::hashbang],
///  + `//` for [Name::comment].
///  + `#` for the various `prolog` and `epilog` fields,
///
/// The content is UTF-8 Github Flavored Markdown.
///
/// A field within the [Item] or File will hold the Comment, there is no mechanism to
/// navigate from a Comment to the thing it describes.
///
/// # Examples
///
/// ```
/// # #[cfg(feature="alloc")]
/// # {
/// use tindalwic::*;
/// let comment = Comment {
///     value: "with ~strikethrough~ extension".into(),
/// };
///
/// let html = markdown::to_html_with_options(&comment.value.joined(), &markdown::Options::gfm())
///     .expect(
///         "should never error, according to:
///      <https://docs.rs/markdown/latest/markdown/fn.to_html_with_options.html#errors>",
///     );
///
/// assert_eq!(html, "<p>with <del>strikethrough</del> extension</p>");
/// # }
/// ```
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Comment<'a> {
    /// the string value
    pub value: Value<'a>,
}
impl<'a> Comment<'a> {
    /// helper for setting one of the fields.
    pub fn some(value: &'a str) -> Option<Comment<'a>> {
        Some(Comment {
            value: value.into(),
        })
    }
}
impl<'a> From<&'a str> for Comment<'a> {
    fn from(value: &'a str) -> Self {
        Comment {
            value: value.into(),
        }
    }
}

// ------------------------------------------------------------------------------------

/// the key in an association.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Name<'a> {
    /// a key can have a blank line before it (above its comment)
    pub gap: bool,
    /// a key can have a comment before it (below its blank line).
    pub comment: Option<Comment<'a>>,
    /// the string value
    pub key: Value<'a>,
}
impl<'a> From<&'a str> for Name<'a> {
    fn from(value: &'a str) -> Self {
        Name {
            key: value.into(),
            ..Default::default()
        }
    }
}

/// an association (from name to item).
///
/// at the lowest level, these are stored in an array.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Entry<'a> {
    /// the key half
    pub name: Name<'a>,
    /// the value half
    pub item: Item<'a>,
}
impl<'a> Default for Entry<'a> {
    fn default() -> Self {
        Entry {
            name: Name::default(),
            item: Item::default(),
        }
    }
}
impl<'a> Entry<'a> {
    /// Make a fixed-size array of cells on the stack.
    pub fn array<const N: usize>() -> [Cell<Entry<'a>>; N] {
        ::core::array::from_fn::<_, N, _>(|_| Cell::default())
    }
}

// ------------------------------------------------------------------------------------

/// the slice type for [Item::Dict::cells]
pub type Entries<'a> = &'a [Cell<Entry<'a>>];
/// the slice type for [Item::List::cells]
pub type Items<'a> = &'a [Cell<Item<'a>>];

// ------------------------------------------------------------------------------------

/// the three Item variants
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Item<'a> {
    /// a [Value]
    Text {
        /// the string value
        value: Value<'a>,
        /// A Text can have a Comment after it.
        epilog: Option<Comment<'a>>,
    },
    /// a linear array of [Item]
    List {
        /// A List can have an introductory Comment.
        prolog: Option<Comment<'a>>,
        /// The contents of the Item::List.
        cells: Items<'a>,
        /// A List can have a Comment after it.
        epilog: Option<Comment<'a>>,
    },
    /// an associative array of [Entry]
    Dict {
        /// A Dict can have an introductory Comment.
        prolog: Option<Comment<'a>>,
        /// The contents of the Item::Dict.
        cells: Entries<'a>,
        /// A Dict can have a Comment after it.
        epilog: Option<Comment<'a>>,
    },
}
impl<'a> Default for Item<'a> {
    fn default() -> Self {
        Item::Text {
            value: Value::default(),
            epilog: None,
        }
    }
}
impl<'a> Item<'a> {
    /// Make a fixed-size array of cells on the stack.
    pub fn array<const N: usize>() -> [Cell<Item<'a>>; N] {
        ::core::array::from_fn::<_, N, _>(|_| Cell::default())
    }
    /// wrap a value (no epilog) into an Item::Text
    pub fn text(value: Value<'a>) -> Self {
        Item::Text {
            value,
            epilog: None,
        }
    }
    /// wrap an array of cells of items into an Item::List
    pub fn list(cells: Items<'a>) -> Self {
        Item::List {
            prolog: None,
            cells,
            epilog: None,
        }
    }
    /// wrap an array of cells of entries into an Item::Dict
    pub fn dict(cells: Entries<'a>) -> Self {
        Item::Dict {
            prolog: None,
            cells,
            epilog: None,
        }
    }
}

// ------------------------------------------------------------------------------------

/// the outermost context.
///
/// similar to a [Item::Dict], but with different comments.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct File<'a> {
    /// A File can start with a Unix `#!` Comment.
    pub hashbang: Option<Comment<'a>>,
    /// A File can have an introductory Comment.
    pub prolog: Option<Comment<'a>>,
    /// The contents of the Item::File.
    pub cells: Entries<'a>,
}
impl<'a> File<'a> {
    /// make an [Item::Dict] from self.prolog and self.cells
    pub fn embed_without_hashbang(&self) -> Item<'a> {
        Item::Dict {
            prolog: self.prolog,
            cells: self.cells,
            epilog: None,
        }
    }
    /// take prolog and cells from an [Item::Dict] to make a new File.
    ///
    /// None if the item is not a dictionary.
    pub fn try_from_dict_without_epilog(dict: &Item<'a>) -> Option<Self> {
        match dict {
            Item::Dict { prolog, cells, .. } => Some(File {
                hashbang: None,
                prolog: *prolog,
                cells,
            }),
            _ => None,
        }
    }
}

// ====================================================================================
