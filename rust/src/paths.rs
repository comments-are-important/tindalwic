use crate::values::{Dict, List, Text, Value};
use std::fmt;

/// an [Err] [Result] for path resolution
#[derive(Debug, Clone)]
pub struct PathErr {
    good: &'static [PathStep],
    have: &'static str,
    fail: Option<&'static PathStep>,
}

impl fmt::Display for PathErr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.fail {
            None => {
                write!(
                    f,
                    "Path `{}` leads to {}.",
                    Path::from(self.good),
                    self.have
                )
            }
            Some(fail) => {
                write!(
                    f,
                    "Path `{}` leads to {}, can't {:?}.",
                    Path::from(self.good),
                    self.have,
                    fail
                )
            }
        }
    }
}
impl PathErr {
    fn some(good: &'static [PathStep], have: &'static str, fail: &'static PathStep) -> Self {
        PathErr {
            good,
            have,
            fail: Some(fail),
        }
    }
    fn none(good: &'static [PathStep], have: &'static str) -> Self {
        PathErr {
            good,
            have,
            fail: None,
        }
    }
}

/// a single step in a [Path]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PathStep {
    /// an index into a linear array
    List(usize),
    /// the key into an associative array
    Dict(&'static str),
}

impl From<usize> for PathStep {
    fn from(value: usize) -> Self {
        PathStep::List(value)
    }
}
impl From<&'static str> for PathStep {
    fn from(value: &'static str) -> Self {
        PathStep::Dict(value)
    }
}

/// one or more [Step]s
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Path {
    steps: &'static [PathStep],
}

impl From<&'static [PathStep]> for Path {
    fn from(steps: &'static [PathStep]) -> Self {
        if steps.is_empty() {
            panic!("need at least one step")
        }
        Path { steps }
    }
}

impl Path {
    /// resolve this path, if possible, to a [Value]
    pub fn value<'v>(&self, root: &'v Value<'v>) -> Result<&'v Value<'v>, PathErr> {
        let mut value = root;
        let mut passed = &self.steps[0..0];
        for step in self.steps {
            value = match (step, value) {
                (PathStep::List(index), Value::List(list)) => list
                    .vec
                    .get(*index)
                    .ok_or(PathErr::some(passed, "List too short", step)),
                (PathStep::Dict(lookup), Value::Dict(dict)) => dict
                    .find(lookup)
                    .map(|k| &k.value)
                    .ok_or(PathErr::some(passed, "Dict missing key", step)),
                (_, Value::Text(_)) => Err(PathErr::some(passed, "Text", step)),
                (_, Value::List(_)) => Err(PathErr::some(passed, "List", step)),
                (_, Value::Dict(_)) => Err(PathErr::some(passed, "Dict", step)),
            }?;
            passed = &self.steps[0..passed.len() + 1]
        }
        Ok(value)
    }

    /// resolve this path, if possible, to a mutable [Value]
    pub fn value_mut<'v>(&self, root: &'v mut Value<'v>) -> Result<&'v mut Value<'v>, PathErr> {
        let mut value = root;
        let mut passed = &self.steps[0..0];
        for step in self.steps {
            value = match (step, value) {
                (PathStep::List(index), Value::List(list)) => list
                    .vec
                    .get_mut(*index)
                    .ok_or(PathErr::some(passed, "List too short", step)),
                (PathStep::Dict(lookup), Value::Dict(dict)) => dict
                    .find_mut(lookup)
                    .map(|k| &mut k.value)
                    .ok_or(PathErr::some(passed, "Dict missing key", step)),
                (_, Value::Text(_)) => Err(PathErr::some(passed, "Text", step)),
                (_, Value::List(_)) => Err(PathErr::some(passed, "List", step)),
                (_, Value::Dict(_)) => Err(PathErr::some(passed, "Dict", step)),
            }?;
            passed = &self.steps[0..passed.len() + 1]
        }
        Ok(value)
    }

    /// resolve this path, if possible, to a [Text]
    pub fn text<'v>(&self, root: &'v Value<'v>) -> Result<&'v Text<'v>, PathErr> {
        match self.value(root)? {
            Value::Text(text) => Ok(text),
            Value::List(_) => Err(PathErr::none(self.steps, "List (not Text)")),
            Value::Dict(_) => Err(PathErr::none(self.steps, "Dict (not Text)")),
        }
    }
    /// resolve this path, if possible, to a mutable [Text]
    pub fn text_mut<'v>(&self, root: &'v mut Value<'v>) -> Result<&'v mut Text<'v>, PathErr> {
        match self.value_mut(root)? {
            Value::Text(text) => Ok(text),
            Value::List(_) => Err(PathErr::none(self.steps, "List (not Text)")),
            Value::Dict(_) => Err(PathErr::none(self.steps, "Dict (not Text)")),
        }
    }

    /// resolve this path, if possible, to a [List]
    pub fn list<'v>(&self, root: &'v Value<'v>) -> Result<&'v List<'v>, PathErr> {
        match self.value(root)? {
            Value::List(list) => Ok(list),
            Value::Dict(_) => Err(PathErr::none(self.steps, "Dict (not List)")),
            Value::Text(_) => Err(PathErr::none(self.steps, "Text (not List)")),
        }
    }
    /// resolve this path, if possible, to a mutable [List]
    pub fn list_mut<'v>(&self, root: &'v mut Value<'v>) -> Result<&'v mut List<'v>, PathErr> {
        match self.value_mut(root)? {
            Value::List(list) => Ok(list),
            Value::Dict(_) => Err(PathErr::none(self.steps, "Dict (not List)")),
            Value::Text(_) => Err(PathErr::none(self.steps, "Text (not List)")),
        }
    }

    /// resolve this path, if possible, to a [Dict]
    pub fn dict<'v>(&self, root: &'v Value<'v>) -> Result<&'v Dict<'v>, PathErr> {
        match self.value(root)? {
            Value::Dict(dict) => Ok(dict),
            Value::List(_) => Err(PathErr::none(self.steps, "List (not Dict)")),
            Value::Text(_) => Err(PathErr::none(self.steps, "Text (not Dict)")),
        }
    }
    /// resolve this path, if possible, to a mutable [Dict]
    pub fn dict_mut<'v>(&self, root: &'v mut Value<'v>) -> Result<&'v mut Dict<'v>, PathErr> {
        match self.value_mut(root)? {
            Value::Dict(dict) => Ok(dict),
            Value::List(_) => Err(PathErr::none(self.steps, "List (not Dict)")),
            Value::Text(_) => Err(PathErr::none(self.steps, "Text (not Dict)")),
        }
    }
}

impl fmt::Display for Path {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for step in self.steps {
            match step {
                PathStep::List(index) => write!(f, "[{}]", index)?,
                PathStep::Dict(lookup) => write!(f, ".{}", lookup)?,
            };
        }
        Ok(())
    }
}

/// build a [Path] from steps
#[macro_export]
macro_rules! path {
    ($($step:tt),+) => {
        $crate::Path::from(&[$($crate::path!(@step $step)),+][..])
    };
    (@step [$n:expr]) => {
        $crate::PathStep::List($n)
    };
    (@step $s:literal) => {
        $crate::PathStep::Dict($s)
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn path_display() {
        assert_eq!(path!("zero", [1], "two").to_string(), ".zero[1].two");
    }

    #[test]
    fn resolve_list() {
        use crate::values::{List, Text};
        let inner = Value::Text(Text::from("hello"));
        let list = Value::List(List::from(vec![inner]));

        let resolved = path!([0]).text(&list).unwrap();
        assert_eq!(resolved.to_string(), "hello");
    }

    #[test]
    fn resolve_failure() {
        use crate::values::{List, Text};
        let inner = Value::Text(Text::from("hello"));
        let list = Value::List(List::from(vec![inner]));

        path!([5]).value(&list).unwrap_err();
    }
}
