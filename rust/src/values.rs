use crate::comments::Comment;
use crate::encoded::Encoded;
use crate::maps::Map;

#[derive(Debug)]
pub struct Text<'a> {
    pub utf8: Encoded<'a>,
    pub epilog: Option<Comment<'a>>,
}
impl<'a> Text<'a> {
    pub fn adopt(utf8: &'a str) -> Self {
        Text {
            utf8: Encoded::adopt(utf8),
            epilog: None,
        }
    }
}
#[derive(Debug)]
pub struct List<'a> {
    pub vec: Vec<Value<'a>>,
    pub prolog: Option<Comment<'a>>,
    pub epilog: Option<Comment<'a>>,
}
impl<'a> List<'a> {
    pub fn adopt(list: Vec<Value<'a>>) -> Self {
        List {
            vec: list,
            prolog: None,
            epilog: None,
        }
    }
}
#[derive(Debug)]
pub struct Dict<'a> {
    pub map: Map<'a>,
    pub prolog: Option<Comment<'a>>,
    pub epilog: Option<Comment<'a>>,
}

#[derive(Debug)]
pub enum Value<'a> {
    Text(Text<'a>),
    List(List<'a>),
    Dict(Dict<'a>),
}

impl<'a> Value<'a> {
    pub fn text(&self) -> Option<&Text<'a>> {
        match self {
            Value::Text(text) => Some(text),
            _ => None,
        }
    }
    pub fn text_mut(&mut self) -> Option<&mut Text<'a>> {
        match self {
            Value::Text(text) => Some(text),
            _ => None,
        }
    }
    pub fn at(&self, index: usize) -> Option<&Value<'a>> {
        match self {
            Value::List(list) => list.vec.get(index),
            _ => None,
        }
    }
    pub fn at_mut(&mut self, index: usize) -> Option<&mut Value<'a>> {
        match self {
            Value::List(list) => list.vec.get_mut(index),
            _ => None,
        }
    }
    pub fn get(&self, key: &str) -> Option<&Value<'a>> {
        match self {
            Value::Dict(dict) => dict.map.get(key),
            _ => None,
        }
    }
    pub fn get_mut(&mut self, key: &str) -> Option<&mut Value<'a>> {
        match self {
            Value::Dict(dict) => dict.map.get_mut(key),
            _ => None,
        }
    }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::path;

    #[test]
    fn zzz() {
        let mut hi = String::from("hi");
        let mut text = Text::adopt(&hi);
        text.epilog = Comment::adopt("comment");
        let mut list = Value::List(List::adopt(vec![Value::Text(text)]));
        //hi.clear(); // won't compile
        let result = path!([0]).text_mut(&mut list).unwrap();
        result.epilog = Comment::adopt("changed");
        //assert_eq!(text.epilog.unwrap().gfm.to_string(), "hi");
        hi.clear();
    }
}
