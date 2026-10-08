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
///  + [path!].walk([File].entries) to the place to be changed,
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
    #[doc(inline)]
    pub use super::value::parse::{Build, Parse, ParseError, Reported};
}

/// the semver plus the git fingerprint
pub const VERSION: &str = env!("TINDALWIC_VERSION");

// ====================================================================================

#[doc(inline)]
pub use value::Value;
impl<'a> Value<'a> {
    /// linear `O(n)` scan.
    // TODO: add link to `alloc` map view, say it "offers `O(1)`."
    pub fn find_linearly_in(self, entries: Entries<'_>) -> Option<usize> {
        entries.iter().position(|cell| cell.get().name.key == self)
    }
}

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
/// let comment: Comment = "with ~strikethrough~ extension".into();
///
/// let html =
///     markdown::to_html_with_options(&comment.value.unwrap().joined(), &markdown::Options::gfm())
///         .expect(
///             "should never error, according to:
///      <https://docs.rs/markdown/latest/markdown/fn.to_html_with_options.html#errors>",
///         );
///
/// assert_eq!(html, "<p>with <del>strikethrough</del> extension</p>");
/// # }
/// ```
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Comment<'a> {
    /// number of empty lines preceding
    pub gap: usize,
    /// the string value
    pub value: Option<Value<'a>>,
}
impl<'a, T> From<T> for Comment<'a>
where
    Value<'a>: From<T>,
{
    fn from(value: T) -> Self {
        Comment {
            value: Some(value.into()),
            ..Default::default()
        }
    }
}

/// a [Value]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Text<'a> {
    /// the string value
    pub value: Value<'a>,
    /// does Text.value prefer a longer encoding? (i.e. `<>`)
    longer: bool,
    /// A Text can have a Comment after it - but gap is impossible.
    pub epilog: Option<Value<'a>>,
}
impl<'a, T> From<T> for Text<'a>
where
    Value<'a>: From<T>,
{
    fn from(value: T) -> Self {
        Text {
            value: value.into(),
            ..Default::default()
        }
    }
}

/// the slice type for [List::items]
pub type Items<'a> = &'a [Cell<Item<'a>>];
/// a linear array of [Item]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct List<'a> {
    /// A List can have an introductory Comment.
    pub prolog: Comment<'a>,
    /// The contents of the Item::List.
    pub items: Items<'a>,
    /// A List can have a Comment after it.
    pub epilog: Comment<'a>,
}
impl<'a> From<Items<'a>> for List<'a> {
    fn from(value: Items<'a>) -> Self {
        List {
            items: value,
            ..Default::default()
        }
    }
}

/// the slice type for [Dict::entries]
pub type Entries<'a> = &'a [Cell<Entry<'a>>];
/// an associative array of [Entry]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Dict<'a> {
    /// A Dict can have an introductory Comment.
    pub prolog: Comment<'a>,
    /// The contents of the Item::Dict.
    pub entries: Entries<'a>,
    /// A Dict can have a Comment after it.
    pub epilog: Comment<'a>,
}
impl<'a> From<Entries<'a>> for Dict<'a> {
    fn from(value: Entries<'a>) -> Self {
        Dict {
            entries: value,
            ..Default::default()
        }
    }
}

// ------------------------------------------------------------------------------------

/// the key in an association.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Name<'a> {
    /// a key can have a comment before it (below its blank line).
    pub comment: Comment<'a>,
    /// the string value
    pub key: Value<'a>,
    /// does Name.key prefer a longer encoding? (i.e. brackets instead of `=`)
    longer: bool,
}
impl<'a, T> From<T> for Name<'a>
where
    Value<'a>: From<T>,
{
    fn from(value: T) -> Self {
        Name {
            key: value.into(),
            ..Default::default()
        }
    }
}

/// an association (from name to item).
///
/// at the lowest level, these are stored in an array.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Entry<'a> {
    /// the key half
    pub name: Name<'a>,
    /// does Entry.name prefer a longer encoding? (i.e. `@`)
    longer: bool,
    /// the value half
    pub item: Item<'a>,
}
impl<'a> Entry<'a> {
    /// Make a fixed-size array of cells on the stack.
    pub fn array<const N: usize>() -> [Cell<Entry<'a>>; N] {
        ::core::array::from_fn::<_, N, _>(|_| Cell::default())
    }
}

// ------------------------------------------------------------------------------------

/// the three Item variants
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Item<'a> {
    /// a [Value]
    Text(Text<'a>),
    /// a linear array of [Item]
    List(List<'a>),
    /// an associative array of [Entry]
    Dict(Dict<'a>),
}
impl<'a> Default for Item<'a> {
    fn default() -> Self {
        Item::Text(Text::default())
    }
}
impl<'a> Item<'a> {
    /// Make a fixed-size array of cells on the stack.
    pub fn array<const N: usize>() -> [Cell<Item<'a>>; N] {
        ::core::array::from_fn::<_, N, _>(|_| Cell::default())
    }
}
impl<'a, T> From<T> for Item<'a>
where
    Text<'a>: From<T>,
{
    fn from(value: T) -> Self {
        Item::Text(Text::from(value))
    }
}
impl<'a> From<Items<'a>> for Item<'a> {
    fn from(value: Items<'a>) -> Self {
        Item::List(List::from(value))
    }
}
impl<'a> From<Entries<'a>> for Item<'a> {
    fn from(value: Entries<'a>) -> Self {
        Item::Dict(Dict::from(value))
    }
}

// ------------------------------------------------------------------------------------

/// the outermost context.
///
/// similar to a [Item::Dict], but with different comments.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct File<'a> {
    /// A File can start with a Unix `#!` Comment.
    pub hashbang: Option<Value<'a>>,
    /// A File can have an introductory Comment.
    pub prolog: Comment<'a>,
    /// The contents of the Item::File.
    pub entries: Entries<'a>,
}
impl<'a> File<'a> {
    /// make an [Item::Dict] from self.prolog and self.entries
    pub fn embed_without_hashbang(&self) -> Item<'a> {
        Item::Dict(Dict {
            prolog: self.prolog,
            entries: self.entries,
            ..Default::default()
        })
    }
    /// take prolog and entries from an [Dict] to make a new File.
    pub fn from_dict_without_epilog(dict: &Dict<'a>) -> Self {
        File {
            prolog: dict.prolog,
            entries: dict.entries,
            ..Default::default()
        }
    }
    /// take prolog and entries from an [Item::Dict] to make a new File.
    ///
    /// None if the item is not a dictionary.
    pub fn try_from_dict_without_epilog(dict: &Item<'a>) -> Option<Self> {
        match dict {
            Item::Dict(dict) => Some(File::from_dict_without_epilog(dict)),
            _ => None,
        }
    }
}
impl<'a> From<Entries<'a>> for File<'a> {
    fn from(value: Entries<'a>) -> Self {
        File {
            entries: value,
            ..Default::default()
        }
    }
}

// ====================================================================================
