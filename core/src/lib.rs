#![no_std]

//! Text in Nested Dictionaries and Lists - with Important Comments

#[doc(inline)]
pub use tindalwic_macros::indent;

pub mod capped;
pub mod fmt;
pub mod group;

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

/// the tokens that start a comment
pub enum CommentMarker {
    /// `#!`
    Shebang,
    /// `%!`
    Prolog,
    /// `%%`
    Epilog,
    /// `%@`
    NameTag,
}
impl CommentMarker {
    const FIRST_CHAR: [char; 2] = ['%', '#'];
    /// return the two-ASCII-char-long marker
    pub const fn token(&self) -> &'static str {
        match self {
            CommentMarker::Shebang => "#!",
            CommentMarker::Prolog => "%!",
            CommentMarker::Epilog => "%%",
            CommentMarker::NameTag => "%@",
        }
    }
}

/// a data [Value] and its metadata.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Text<'a> {
    /// the string value. there are two possible forms for encoding: a single line not
    /// indented, or an indented block. the single line form in a dictionary context
    /// will follow a (possibly empty) key and an equals sign: `key=value`. in a list
    /// context it is the entire line, but it must not start with a char (can't be
    /// empty) that is not reserved.
    pub value: Value<'a>,
    /// A Text value can have a epilog comment after it,
    pub epilog: Option<Value<'a>>,
    /// is a longer text encoding preferred? (i.e. `<>` + single line block)
    pub longer: bool,
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

#[doc(inline)]
pub use group::Items;

/// a linear array of [Item]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct List<'a> {
    /// A List can have an introductory comment.
    pub prolog: Option<Value<'a>>,
    /// The contents of the Item::List.
    pub items: Items<'a>,
    /// A List can have a comment after it.
    pub epilog: Option<Value<'a>>,
}
impl<'a, T> From<T> for List<'a>
where
    Items<'a>: From<T>,
{
    fn from(value: T) -> Self {
        List {
            items: value.into(),
            ..Default::default()
        }
    }
}

#[doc(inline)]
pub use group::Entries;
impl<'a> Entries<'a> {
    /// `O(N)` lookup.
    pub fn linear_search<T>(&self, key: T) -> Option<Entry<'a>>
    where
        Value<'a>: From<T>,
    {
        let value: Value<'a> = key.into();
        for entry in self {
            if value == entry.name.key {
                return Some(entry);
            }
        }
        None
    }
}

/// an associative array of [Entry]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Dict<'a> {
    /// A Dict can have an introductory comment.
    pub prolog: Option<Value<'a>>,
    /// The contents of the Item::Dict.
    pub entries: Entries<'a>,
    /// A Dict can have a comment after it.
    pub epilog: Option<Value<'a>>,
}
impl<'a, T> From<T> for Dict<'a>
where
    Entries<'a>: From<T>,
{
    fn from(value: T) -> Self {
        Dict {
            entries: value.into(),
            ..Default::default()
        }
    }
}

// ------------------------------------------------------------------------------------

/// the key in an association.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Name<'a> {
    /// a key can have a comment before it (below its blank line).
    pub comment: Option<Value<'a>>,
    /// the string value
    pub key: Value<'a>,
    /// does Name.key prefer a longer encoding? (i.e. brackets instead of `=`)
    pub longer: bool,
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
    pub longer: bool,
    /// the value half
    pub item: Item<'a>,
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
    /// A File can start with a Unix `#!` comment.
    pub shebang: Option<Value<'a>>,
    /// A File can have an introductory comment.
    pub prolog: Option<Value<'a>>,
    /// The contents of the Item::File.
    pub entries: Entries<'a>,
    // /// number of trailing empty lines
    // pub trailing: usize,
}
impl<'a> File<'a> {
    /// make an [Item::Dict] from self.prolog and self.entries
    pub fn embed_without_shebang(&self) -> Item<'a> {
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
