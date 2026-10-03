pub mod parse;

/// All primitive values in Tindalwic are string slice references, not owned.
///
///  + [Comment::value](super::Comment::value)
///  + [Text::value](super::Item::Text::value)
///  + [Name::key](super::Name::key)
///
/// They often contain embedded indentation because the parser is zero-copy from
/// the encoded data. The methods here will strip indentation as necessary.
/// Apps that modify only a few values do not have to pay for any processing of
/// unmodified values that are already appropriately indented.
#[derive(Clone, Copy, Debug)]
pub struct Value<'a> {
    slice: &'a str,
    indent: usize, // usize::MAX => single line
}
impl<'a> PartialEq for Value<'a> {
    fn eq(&self, other: &Self) -> bool {
        if self.indent == other.indent {
            self.slice == other.slice
        } else {
            self.lines().eq(other.lines())
        }
    }
}
impl<'a> Value<'a> {
    /// number of bytes (see [str::len]).
    /// includes any indentation TAB chars
    pub fn len(&self) -> usize {
        self.slice.len()
    }
    /// `true` when zero `len` (see [str::is_empty]).
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
    /// Returned iterator produces one sub-slice for each line.
    ///
    /// Always produces at least one line. Omits indentation and newline chars.
    pub fn lines(&self) -> impl Iterator<Item = &'a str> {
        // that return type is tricky to satisfy: having two branches here (one
        // optimized for absent indentation) causes E0308 incompatible types:
        //   "distinct uses of `impl Trait` result in different opaque types"
        // attempting to hide them behind closures does not help either:
        //   "no two closures, even if identical, have the same type"
        let d = if self.only_line().is_some() {
            0
        } else {
            self.indent
        };
        self.slice.split('\n').enumerate().map(move |(i, s)| {
            if i == 0 || d == 0 || s.is_empty() {
                s
            } else {
                &s[d..]
            }
        })
    }
    fn stretched(
        &self,
        indent: usize,
        concat: &'a str,
        source: &'a str,
    ) -> Result<Self, &'static str> {
        if indent == usize::MAX {
            return Err("value.stretched: sentinel value passed as indent");
        }
        if self.indent != usize::MAX && self.indent != indent {
            return Err("value.stretched: incompatible indents");
        }
        let base = source.as_ptr() as usize;
        let first = self.slice.as_ptr() as usize;
        if first < base || base + source.len() <= first {
            return Err("value.stretched: self did not come from that source");
        }
        let second = concat.as_ptr() as usize;
        if second < base || base + source.len() <= second {
            return Err("value.stretched: concat isn't from that source");
        }
        if second < first + self.slice.len() {
            return Err("value.stretched: concat must follow this value");
        }
        let slice = &source[first - base..second - base + concat.len()];
        Ok(Value { slice, indent })
    }
}
impl<'a> Default for Value<'a> {
    fn default() -> Self {
        Value {
            slice: "",
            indent: usize::MAX,
        }
    }
}
impl<'a> From<&'a str> for Value<'a> {
    fn from(value: &'a str) -> Self {
        Value {
            slice: value,
            indent: if value.contains('\n') { 0 } else { usize::MAX },
        }
    }
}
impl<'a> Eq for Value<'a> {}
impl<'a> core::hash::Hash for Value<'a> {
    fn hash<H: core::hash::Hasher>(&self, state: &mut H) {
        let mut lines = self.lines();
        let first = lines.next().expect("lines is never empty");
        first.hash(state);
        for line in self.lines() {
            b'\n'.hash(state);
            line.hash(state);
        }
    }
}
