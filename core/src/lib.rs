#![no_std]

//! Text in Nested Dictionaries and Lists - with Important Comments

#[doc(inline)]
pub use tindalwic_macros::indent;

pub mod capped;
pub mod fmt;
pub mod group;
pub mod parse;
pub mod text;

#[cfg(feature = "alloc")]
pub mod alloc;
#[cfg(feature = "bumpalo")]
pub mod bumpalo;

/// the semver plus the git fingerprint
pub const VERSION: &str = env!("TINDALWIC_VERSION");

// ====================================================================================

/// the tokens that start a comment
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum CommentMarker {
    /// `#!`
    Shebang,
    /// `%!`
    Prolog,
    /// `%%`
    Epilog,
    /// `%@`
    NameTag,
    /// `=`
    Group,
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
            CommentMarker::Group => "=",
        }
    }
}

use group::{Groups, Members};
use text::Lines;

/// the only primitive data type.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[tindalwic_macros::check_fields]
pub struct Text<'a> {
    /// the data. there are two possible forms for encoding: a single line not
    /// indented, or an indented block.
    pub lines: Lines<'a>,
    /// A Text can have a epilog comment after it,
    pub epilog: Option<Lines<'a>>,
    /// gloss indicating preference to avoid any shortcut encoding
    pub verbose: bool,
}

/// the only composite data type.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[tindalwic_macros::check_fields]
pub struct List<'a> {
    /// A List can have an introductory comment.
    pub prolog: Option<Lines<'a>>,
    /// many lists will have a single group without a header.
    pub groups: Groups<'a>,
    /// A List can have a comment after it.
    pub epilog: Option<Lines<'a>>,
}
impl<'a, T> From<T> for List<'a>
where
    Groups<'a>: From<T>,
{
    fn from(value: T) -> Self {
        List {
            groups: value.into(),
            ..Default::default()
        }
    }
}
impl<'a> List<'a> {
    /// number of members
    pub fn len(&self) -> usize {
        self.groups.len()
    }
    /// flatten groups
    pub fn members(&self) -> impl Iterator<Item = Member<'a>> {
        self.groups.members()
    }
    /// O(N) lookup
    pub fn named<T>(&self, name: T) -> impl Iterator<Item = Member<'a>>
    where
        Lines<'a>: From<T>,
    {
        self.groups.named(name)
    }
}

/// data within a List has metadata. List is the data type, this is structure.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[tindalwic_macros::check_fields]
pub struct Member<'a> {
    /// the name can have a comment before it.
    pub comment: Option<Lines<'a>>,
    /// the name need not be unique, and may be empty.
    pub name: Lines<'a>,
    /// gloss indicating preference to avoid any shortcut encoding
    pub longer: bool,
    /// the Text or List
    pub data: Data<'a>,
}
impl<'a> From<Data<'a>> for Member<'a> {
    fn from(value: Data<'a>) -> Self {
        Member {
            data: value,
            ..Default::default()
        }
    }
}

/// data within a List is grouped. List is the data type, this is structure.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[tindalwic_macros::check_fields]
pub struct Group<'a> {
    /// the metadata
    pub heading: Option<Lines<'a>>,
    /// the data
    pub members: Members<'a>,
}
impl<'a, T> From<T> for Group<'a>
where
    Members<'a>: From<T>,
{
    fn from(value: T) -> Self {
        Group {
            members: value.into(),
            ..Default::default()
        }
    }
}

// ------------------------------------------------------------------------------------

/// the two data types
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[tindalwic_macros::check_variants]
pub enum Data<'a> {
    /// a primitive
    Text(Text<'a>),
    /// a composite
    List(List<'a>),
}
impl<'a> Default for Data<'a> {
    fn default() -> Self {
        Data::Text(Text::default())
    }
}
impl<'a, T> From<T> for Data<'a>
where
    Text<'a>: From<T>,
{
    fn from(value: T) -> Self {
        Data::Text(Text::from(value))
    }
}

// ------------------------------------------------------------------------------------

/// the outermost context.
///
/// basically a [List], but with different comments.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[tindalwic_macros::check_fields]
pub struct File<'a> {
    /// A File can start with a Unix `#!` comment.
    pub shebang: Option<Lines<'a>>,
    /// A File can have an introductory comment.
    pub prolog: Option<Lines<'a>>,
    /// many lists will have a single group without a header.
    pub groups: Groups<'a>,
    // /// number of trailing empty lines
    // pub trailing: usize,
}
impl<'a, T> From<T> for File<'a>
where
    Groups<'a>: From<T>,
{
    fn from(value: T) -> Self {
        File {
            groups: value.into(),
            ..Default::default()
        }
    }
}
impl<'a> File<'a> {
    /// number of members
    pub fn len(&self) -> usize {
        self.groups.len()
    }
    /// flatten groups
    pub fn members(&self) -> impl Iterator<Item = Member<'a>> {
        self.groups.members()
    }
    /// O(N) lookup
    pub fn named<T>(&self, name: T) -> impl Iterator<Item = Member<'a>>
    where
        Lines<'a>: From<T>,
    {
        self.groups.named(name)
    }
    /// make an [List] from self.prolog and self.groups
    pub fn embed_without_shebang(&self) -> List<'a> {
        List {
            prolog: self.prolog,
            groups: self.groups,
            ..Default::default()
        }
    }
    /// use a List prolog and groups to make a new File.
    pub fn from_list_without_epilog(list: &List<'a>) -> Self {
        File {
            prolog: list.prolog,
            groups: list.groups,
            ..Default::default()
        }
    }
    /// use a List prolog and groups to make a new File.
    ///
    /// None if the item is not a List.
    pub fn try_from_list_without_epilog(data: &Data<'a>) -> Option<Self> {
        match data {
            Data::List(list) => Some(File::from_list_without_epilog(list)),
            _ => None,
        }
    }
}

// ====================================================================================
