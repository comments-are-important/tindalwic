use crate::comments::Comment;
use crate::values::Value;
//use indexmap::IndexMap;

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct Keyed<'a> {
    pub key: &'a str,
    pub gap: bool,
    pub before: Option<Comment<'a>>,
    pub value: Value<'a>
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct Map<'a> {
    pub vec: Vec<Keyed<'a>>,
}
impl<'a> Map<'a> {
    pub fn new() -> Self {
        Map {
            vec: Vec::new(),
        }
    }
    pub fn len(&self) -> usize {
        self.vec.len()
    }
    pub fn position(&self, key: &str) -> Option<usize> {
        self.vec.iter().position(|x|x.key == key)
    }
    pub fn find(&self, key: &str) -> Option<&Keyed<'a>> {
        self.position(key).map(|i|&self.vec[i])
    }
    pub fn find_mut(&mut self, key: &str) -> Option<&mut Keyed<'a>> {
        self.position(key).map(|i|&mut self.vec[i])
    }
    pub fn push(&mut self, keyed:Keyed<'a>) {
        self.vec.push(keyed);
    }
    // pub fn swap_values(&mut self, key1: &str, key2: &str) -> bool {
    //     let Some(i) = self.map.get_index_of(key1) else {
    //         return false;
    //     };
    //     let Some(j) = self.map.get_index_of(key2) else {
    //         return false;
    //     };
    //     let (_, a) = self.map.get_index_mut(i).unwrap();
    //     let ptr: *mut (Key<'a>, Value<'a>) = a;
    //     let (_, b) = self.map.get_index_mut(j).unwrap();
    //     // SAFETY: i != j (checked indices are different), so these don't alias
    //     unsafe { std::ptr::swap(ptr, b) };
    //     true
    // }
}
