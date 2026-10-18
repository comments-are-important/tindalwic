//! the only primitive type

#![allow(unused)]

use super::Text;
use core::fmt::{Display, Formatter, Result as FmtResult, Write as _};
use core::str::Split;

/// # One or more lines of text, possibly containing embedded margins.
///
/// There is no quoting/escaping mechanism, the encoding is just the UTF-8. This allows
/// the parser to instantiate text using zero-copy slices from an encoded file. The
/// methods strip indentation as necessary. Encoding is particularly cheap when the
/// embedded margins already match what is needed.
#[derive(Clone, Copy, Debug, Eq)]
#[tindalwic_macros::check_fields]
pub struct UTF8<'a> {
    slice: &'a str,
    indent: usize, // usize::MAX => single line
}

impl<'a> UTF8<'a> {
    pub(crate) fn parsed(slice: &'a str, indent: usize) -> Self {
        UTF8 { slice, indent }
    }
    /// Returns `true` if `self` has a length of zero bytes.
    pub fn is_empty(&self) -> bool {
        self.slice.is_empty()
    }
    /// the format sometimes allows shorter encoding for single line values
    pub fn only_line(&self) -> Option<&'a str> {
        if self.indent == usize::MAX {
            Some(self.slice)
        } else {
            None
        }
    }
    /// if the value was captured at this indent, then it can be used as is.
    pub fn verbatim(&self, indent: usize) -> Option<&'a str> {
        let only = self.only_line();
        if only.is_some() {
            only
        } else if indent == self.indent {
            Some(self.slice)
        } else {
            None
        }
    }
}

/// strip margin as it goes
pub struct LineIterator<'a> {
    lines: Split<'a, char>,
    margin: usize,
    first: bool,
}

impl<'a> Iterator for LineIterator<'a> {
    type Item = &'a str;
    fn next(&mut self) -> Option<Self::Item> {
        if self.first {
            self.first = false;
            self.lines.next()
        } else if let Some(next) = self.lines.next() {
            if next.is_empty() {
                Some(next)
            } else {
                Some(&next[self.margin..])
            }
        } else {
            None
        }
    }
}

impl<'a> IntoIterator for UTF8<'a> {
    type Item = &'a str;
    type IntoIter = LineIterator<'a>;
    /// the lines without margin/eol, always at least one
    fn into_iter(self) -> Self::IntoIter {
        let mut iter = LineIterator {
            lines: self.slice.split('\n'),
            margin: self.indent,
            first: true,
        };
        if iter.margin == usize::MAX {
            iter.margin = 0
        }
        iter
    }
}

impl<'a> Default for UTF8<'a> {
    fn default() -> Self {
        UTF8 {
            slice: "",
            indent: usize::MAX,
        }
    }
}

impl<'a> PartialEq for UTF8<'a> {
    fn eq(&self, other: &Self) -> bool {
        if self.indent == other.indent {
            self.slice == other.slice
        } else {
            self.into_iter().eq(other.into_iter())
        }
    }
}

impl<'a> From<&'a str> for UTF8<'a> {
    fn from(value: &'a str) -> Self {
        UTF8 {
            slice: value,
            indent: if value.contains('\n') { 0 } else { usize::MAX },
        }
    }
}

impl<'a> core::hash::Hash for UTF8<'a> {
    fn hash<H: core::hash::Hasher>(&self, state: &mut H) {
        let mut lines = self.into_iter();
        let first = lines.next().expect("lines is never empty");
        first.hash(state);
        for line in lines {
            b'\n'.hash(state);
            line.hash(state);
        }
    }
}

impl<'a, T> From<T> for Text<'a>
where
    UTF8<'a>: From<T>,
{
    fn from(value: T) -> Self {
        Text {
            lines: value.into(),
            ..Default::default()
        }
    }
}
