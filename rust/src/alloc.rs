extern crate alloc;

use alloc::string::String;
use super::*;

impl<'a> UTF8<'a> {
    /// Allocates a [String], filled with the UTF-8 copied from `self`.
    fn joined(&self) -> String {
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
