//! all this stuff is enabled by the "alloc" feature.

extern crate alloc;

use crate::{Comment, Text, UTF8};
use alloc::string::String;

impl<'a> UTF8<'a> {
    /// Allocates a [String], filled with the UTF-8 copied from `self`.
    pub(crate) fn joined(&self) -> String {
        if self.dedent == 0 || self.dedent == usize::MAX {
            String::from(self.slice)
        } else {
            let mut result = String::with_capacity(self.slice.len());
            for line in self.lines() {
                result.push_str(line);
                result.push('\n');
            }
            if !result.is_empty() {
                result.truncate(result.len() - 1);
            }
            result
        }
    }
}

impl<'a> Comment<'a> {
    /// Allocates a [String], filled with the UTF-8 copied from `self`.
    pub fn joined(&self) -> String {
        self.utf8.joined()
    }
}

impl<'a> Text<'a> {
    /// Allocates a [String], filled with the UTF-8 copied from `self`.
    pub fn joined(&self) -> String {
        self.utf8.joined()
    }
}

/// companion to parse::Builder allowing arena to intern values
#[allow(dead_code)] // used in bumpalo feature
pub(crate) trait Intern<'a> {
    /// intern a str slice
    fn str(&self, value: &'_ str) -> &'a str;
}
