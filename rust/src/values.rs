use crate::comments::Comment;
use crate::encoded::Encoded;
use crate::maps::Map;

#[derive(Debug, PartialEq, Eq, Clone)]
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
#[derive(Debug, PartialEq, Eq, Clone)]
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
#[derive(Debug, PartialEq, Eq, Clone)]
pub struct Dict<'a> {
    pub map: Map<'a>,
    pub prolog: Option<Comment<'a>>,
    pub epilog: Option<Comment<'a>>,
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum Value<'a> {
    Text(Text<'a>),
    List(List<'a>),
    Dict(Dict<'a>),
}

impl<'a> Value<'a> {
}

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
    // fn duplicate_a_value() {
    //    app might want to clone a value (at some index or under some key)
    //    and add it again (at a different index or under different key)
    //    and that will need impl Clone for Value
    // }
}
