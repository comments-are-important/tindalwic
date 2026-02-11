use crate::comments::Comment;
use crate::values::Value;
use indexmap::IndexMap;

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct Key<'a> {
    pub key: &'a str,
    pub gap: bool,
    pub before: Option<Comment<'a>>,
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct Map<'a> {
    map: IndexMap<&'a str, (Key<'a>, Value<'a>)>,
}
impl<'a> Map<'a> {
    pub fn new() -> Self {
        Map {
            map: IndexMap::new(),
            //pairs: Vec::new(),
        }
    }
    pub fn len(&self) -> usize {
        self.map.len()
    }
    pub fn get(&self, key: &str) -> Option<&(Key<'a>, Value<'a>)> {
        self.map.get(key)
    }
    pub fn get_mut(&mut self, key: &str) -> Option<&mut (Key<'a>, Value<'a>)> {
        self.map.get_mut(key)
    }
    pub fn put(&mut self, key:Key<'a>, value:Value<'a>) {
        let k = key.key;
        self.map.insert(k, (key,value));
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
