//! all this stuff is enabled by the "alloc" feature.

extern crate alloc;

use crate::Value;
use alloc::string::String;

impl<'a> Value<'a> {
    /// Allocates a [String], filled with the UTF-8 copied from `self`.
    pub fn joined(&self) -> String {
        if let Some(slice) = self.verbatim(0) {
            String::from(slice)
        } else {
            let mut result = String::with_capacity(self.byte_count());
            let mut iter = self.lines();
            if let Some(first) = iter.next() {
                result.push_str(first)
            }
            for more in iter {
                result.push('\n');
                result.push_str(more);
            }
            result
        }
    }
}
