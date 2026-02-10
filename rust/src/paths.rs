#![allow(unused)]

use crate::values::{Dict, List, Text, Value};
use std::fmt;

#[derive(Debug, Clone)]
pub struct PathErr {
    good: &'static [Step],
    have: &'static str,
    fail: Option<&'static Step>,
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
    fn some(good: &'static [Step], have: &'static str, fail: &'static Step) -> Self {
        PathErr {
            good,
            have,
            fail: Some(fail),
        }
    }
    fn none(good: &'static [Step], have: &'static str) -> Self {
        PathErr {
            good,
            have,
            fail: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Step {
    List(usize),
    Dict(&'static str),
}

impl From<usize> for Step {
    fn from(value: usize) -> Self {
        Step::List(value)
    }
}
impl From<&'static str> for Step {
    fn from(value: &'static str) -> Self {
        Step::Dict(value)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Path {
    steps: &'static [Step],
}

impl From<&'static [Step]> for Path {
    fn from(steps: &'static [Step]) -> Self {
        if steps.is_empty() {
            panic!("need at least one step")
        }
        Path { steps }
    }
}

impl Path {
    pub fn value<'v>(&self, root: &'v Value<'v>) -> Result<&'v Value<'v>, PathErr> {
        let mut value = root;
        let mut passed = &self.steps[0..0];
        for step in self.steps {
            value = match (step, value) {
                (Step::List(index), Value::List(list)) => list
                    .vec
                    .get(*index)
                    .ok_or(PathErr::some(passed, "List too short", step)),
                (Step::Dict(lookup), Value::Dict(dict)) => dict
                    .map
                    .get(lookup)
                    .ok_or(PathErr::some(passed, "Dict missing key", step)),
                (_, Value::Text(_)) => Err(PathErr::some(passed, "Text", step)),
                (_, Value::List(_)) => Err(PathErr::some(passed, "List", step)),
                (_, Value::Dict(_)) => Err(PathErr::some(passed, "Dict", step)),
            }?;
            passed = &self.steps[0..passed.len() + 1]
        }
        Ok(value)
    }

    pub fn value_mut<'v>(&self, root: &'v mut Value<'v>) -> Result<&'v mut Value<'v>, PathErr> {
        let mut value = root;
        let mut passed = &self.steps[0..0];
        for step in self.steps {
            value = match (step, value) {
                (Step::List(index), Value::List(list)) => list
                    .vec
                    .get_mut(*index)
                    .ok_or(PathErr::some(passed, "List too short", step)),
                (Step::Dict(lookup), Value::Dict(dict)) => dict
                    .map
                    .get_mut(lookup)
                    .ok_or(PathErr::some(passed, "Dict missing key", step)),
                (_, Value::Text(_)) => Err(PathErr::some(passed, "Text", step)),
                (_, Value::List(_)) => Err(PathErr::some(passed, "List", step)),
                (_, Value::Dict(_)) => Err(PathErr::some(passed, "Dict", step)),
            }?;
            passed = &self.steps[0..passed.len() + 1]
        }
        Ok(value)
    }

    pub fn text<'v>(&self, root: &'v Value<'v>) -> Result<&'v Text<'v>, PathErr> {
        match self.value(root)? {
            Value::Text(text) => Ok(text),
            Value::List(_) => Err(PathErr::none(self.steps, "List (not Text)")),
            Value::Dict(_) => Err(PathErr::none(self.steps, "Dict (not Text)")),
        }
    }
    pub fn text_mut<'v>(&self, root: &'v mut Value<'v>) -> Result<&'v mut Text<'v>, PathErr> {
        match self.value_mut(root)? {
            Value::Text(text) => Ok(text),
            Value::List(_) => Err(PathErr::none(self.steps, "List (not Text)")),
            Value::Dict(_) => Err(PathErr::none(self.steps, "Dict (not Text)")),
        }
    }

    pub fn list<'v>(&self, root: &'v Value<'v>) -> Result<&'v List<'v>, PathErr> {
        match self.value(root)? {
            Value::List(list) => Ok(list),
            Value::Dict(_) => Err(PathErr::none(self.steps, "Dict (not List)")),
            Value::Text(_) => Err(PathErr::none(self.steps, "Text (not List)")),
        }
    }
    pub fn list_mut<'v>(&self, root: &'v mut Value<'v>) -> Result<&'v mut List<'v>, PathErr> {
        match self.value_mut(root)? {
            Value::List(list) => Ok(list),
            Value::Dict(_) => Err(PathErr::none(self.steps, "Dict (not List)")),
            Value::Text(_) => Err(PathErr::none(self.steps, "Text (not List)")),
        }
    }

    pub fn dict<'v>(&self, root: &'v Value<'v>) -> Result<&'v Dict<'v>, PathErr> {
        match self.value(root)? {
            Value::Dict(dict) => Ok(dict),
            Value::List(_) => Err(PathErr::none(self.steps, "List (not Dict)")),
            Value::Text(_) => Err(PathErr::none(self.steps, "Text (not Dict)")),
        }
    }
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
                Step::List(index) => write!(f, "[{}]", index),
                Step::Dict(lookup) => write!(f, ".{}", lookup),
            };
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::path;

    #[test]
    fn path_display() {
        assert_eq!(path!("zero", [1], "two").to_string(), ".zero[1].two");
    }

    #[test]
    fn resolve_list() {
        use crate::values::{List, Text};
        let inner = Value::Text(Text::adopt("hello"));
        let list = Value::List(List::adopt(vec![inner]));

        let resolved = path!([0]).text(&list).unwrap();
        assert_eq!(resolved.utf8.to_string(), "hello");
    }

    #[test]
    fn resolve_failure() {
        use crate::values::{List, Text};
        let inner = Value::Text(Text::adopt("hello"));
        let list = Value::List(List::adopt(vec![inner]));

        path!([5]).value(&list).unwrap_err();
    }
}
