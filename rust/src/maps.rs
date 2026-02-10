use crate::comments::Comment;
use crate::values::Value;
use indexmap::IndexMap;

#[derive(Debug)]
pub struct Key<'a> {
    pub key: &'a str,
    pub gap: bool,
    pub before: Option<Comment<'a>>,
}

#[derive(Debug)]
pub struct Map<'a> {
    pub map: IndexMap<&'a str, (Key<'a>, Value<'a>)>,
}
impl<'a> Map<'a> {
    pub fn new() -> Self {
        Map {
            map: IndexMap::new(),
        }
    }
    pub fn len(&self) -> usize {
        self.map.len()
    }
    pub fn get(&self, key: &str) -> Option<&Value<'a>> {
        match self.map.get(key) {
            None => None,
            Some((_, value)) => Some(value),
        }
    }
    pub fn get_mut(&mut self, key: &str) -> Option<&mut Value<'a>> {
        match self.map.get_mut(key) {
            None => None,
            Some((_, value)) => Some(value),
        }
    }
}

// // =============================================================================
// // Key - a string key with optional preceding blank line and comment
// // =============================================================================

// /// A dictionary key with optional formatting metadata.
// #[derive(Clone)]
// pub struct Key<'a> {
//     pub name: &'a str,
//     pub blank_line_before: bool,
//     pub comment_before: Option<Comment<'a>>,
// }

// impl<'a> Key<'a> {
//     /// Create a key from a string slice.
//     /// Panics if the name contains a newline.
//     pub fn new(name: &'a str) -> Self {
//         if name.contains('\n') {
//             panic!("newline in key");
//         }
//         Self {
//             name,
//             blank_line_before: false,
//             comment_before: None,
//         }
//     }

//     /// Get the key name.
//     pub fn as_str(&self) -> &'a str {
//         self.name
//     }
// }

// impl fmt::Debug for Key<'_> {
//     fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
//         write!(f, "Key({:?})", self.name)
//     }
// }

// impl fmt::Display for Key<'_> {
//     fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
//         write!(f, "{}", self.name)
//     }
// }
