//! all this stuff is enabled by the "alloc" feature.

extern crate alloc;

use crate::Value;
use crate::{Comment, Text};
use alloc::string::String;

impl<'a> Value<'a> {
    /// Allocates a [String], filled with the UTF-8 copied from `self`.
    pub fn joined(&self) -> String {
        if let Some(slice) = self.shortcut(0) {
            String::from(slice)
        } else {
            let mut result = String::new(); //with_capacity(self.len());
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
        self.value.joined()
    }
}

impl<'a> Text<'a> {
    /// Allocates a [String], filled with the UTF-8 copied from `self`.
    pub fn joined(&self) -> String {
        self.value.joined()
    }
}

/// companion to parse::Builder allowing arena to intern values
#[allow(dead_code)] // used in bumpalo feature
pub(crate) trait Intern<'a> {
    /// intern a str slice
    fn str(&self, value: &'_ str) -> &'a str;
}
