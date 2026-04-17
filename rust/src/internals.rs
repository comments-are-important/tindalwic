#![doc(hidden)] // only public so macro generated code can access.

use super::*;

use core::ops::{Deref, DerefMut, Range};

macro_rules! cell_helpers {
    ($Name:ident, $field:ident, $NameInList:ident, $NameInDict:ident) => {
        pub struct $NameInList<'a> {
            pub $field: $Name<'a>,
            cell: &'a Cell<Value<'a>>,
        }
        impl<'a> Deref for $NameInList<'a> {
            type Target = $Name<'a>;
            fn deref(&self) -> &Self::Target {
                &self.$field
            }
        }
        impl<'a> DerefMut for $NameInList<'a> {
            fn deref_mut(&mut self) -> &mut Self::Target {
                &mut self.$field
            }
        }
        impl<'a> $NameInList<'a> {
            pub fn from(cell: &'a Cell<Value<'a>>) -> Option<Self> {
                if let Value::$Name($field) = cell.get() {
                    Some($NameInList { $field, cell })
                } else {
                    None
                }
            }
            pub fn __set(&mut self, new: Option<Value<'a>>) {
                self.cell.set(match new {
                    None => Value::$Name(self.$field),
                    Some(value) => value,
                });
            }
        }
        pub struct $NameInDict<'a> {
            pub key: &'a str,
            pub gap: bool,
            pub before: Option<Comment<'a>>,
            pub $field: $Name<'a>,
            cell: &'a Cell<Keyed<'a>>,
        }
        impl<'a> Deref for $NameInDict<'a> {
            type Target = $Name<'a>;
            fn deref(&self) -> &Self::Target {
                &self.$field
            }
        }
        impl<'a> DerefMut for $NameInDict<'a> {
            fn deref_mut(&mut self) -> &mut Self::Target {
                &mut self.$field
            }
        }
        impl<'a> From<&$NameInDict<'a>> for Value<'a> {
            fn from(value: &$NameInDict<'a>) -> Self {
                Value::from(value.$field)
            }
        }
        impl<'a> $NameInDict<'a> {
            pub fn from(cell: &'a Cell<Keyed<'a>>) -> Option<Self> {
                let keyed = cell.get();
                if let Value::$Name($field) = keyed.value {
                    Some($NameInDict {
                        key: keyed.key,
                        gap: keyed.gap,
                        before: keyed.before,
                        $field,
                        cell,
                    })
                } else {
                    None
                }
            }
            pub fn __set(&mut self, new: Option<Value<'a>>) {
                self.cell.set(Keyed {
                    key: self.key,
                    gap: self.gap,
                    before: self.before,
                    value: match new {
                        None => Value::$Name(self.$field),
                        Some(value) => value,
                    },
                });
            }
        }
    };
}

cell_helpers! {Text,text,TextInList,TextInDict}
cell_helpers! {List,list,ListInList,ListInDict}
cell_helpers! {Dict,dict,DictInList,DictInDict}

pub struct Arena<'a> {
    value_cells: &'a [Cell<Value<'a>>],
    keyed_cells: &'a [Cell<Keyed<'a>>],
    value_next: usize,
    keyed_next: usize,
}
impl<'a> Arena<'a> {
    pub fn new(value_cells: &'a [Cell<Value<'a>>], keyed_cells: &'a [Cell<Keyed<'a>>]) -> Self {
        Arena {
            value_cells,
            keyed_cells,
            value_next: 0,
            keyed_next: 0,
        }
    }
    pub fn value_in_list(&mut self, value: Value<'a>) {
        self.value_cells[self.value_next].set(value);
        self.value_next += 1;
    }
    pub fn text_in_list(&mut self, utf8: &'a str) {
        self.value_in_list(Value::Text(Text::wrap(utf8)));
    }
    pub fn list_in_list(&mut self, list: Range<usize>) {
        self.value_in_list(Value::List(List::wrap(&self.value_cells[list])));
    }
    pub fn dict_in_list(&mut self, dict: Range<usize>) {
        self.value_in_list(Value::Dict(Dict::wrap(&self.keyed_cells[dict])));
    }
    pub fn value_in_dict(&mut self, key: &'a str, value: Value<'a>) {
        self.keyed_cells[self.keyed_next].set(Keyed::from(key, value));
        self.keyed_next += 1;
    }
    pub fn text_in_dict(&mut self, key: &'a str, utf8: &'a str) {
        self.value_in_dict(key, Value::Text(Text::wrap(utf8)));
    }
    pub fn list_in_dict(&mut self, key: &'a str, list: Range<usize>) {
        self.value_in_dict(key, Value::List(List::wrap(&self.value_cells[list])));
    }
    pub fn dict_in_dict(&mut self, key: &'a str, dict: Range<usize>) {
        self.value_in_dict(key, Value::Dict(Dict::wrap(&self.keyed_cells[dict])));
    }
    pub fn end(&self) -> &'a Cell<Value<'a>> {
        &self.value_cells[self.value_next - 1]
    }
    pub fn value(&self) -> Value<'a> {
        self.end().get()
    }
    pub fn text(&self) -> Option<Text<'a>> {
        if let Value::Text(text) = self.end().get() {
            Some(text)
        } else {
            None
        }
    }
    pub fn list(&self) -> Option<List<'a>> {
        if let Value::List(list) = self.end().get() {
            Some(list)
        } else {
            None
        }
    }
    pub fn dict(&self) -> Option<Dict<'a>> {
        if let Value::Dict(dict) = self.end().get() {
            Some(dict)
        } else {
            None
        }
    }
}

#[derive(Debug)]
pub enum Branch<'a> {
    List(usize),
    Dict(&'a str),
}
#[derive(Debug)]
pub struct Error<'a> {
    pub failed: &'a [Branch<'a>],
    pub message: &'static str,
}
#[derive(Debug)]
pub struct Path<'a> {
    pub branches: &'a [Branch<'a>],
}
impl<'a> Path<'a> {
    pub fn new(branches: &'a [Branch<'a>]) -> Self {
        Path { branches }
    }
    pub fn error_full(&'a self, message: &'static str) -> Error<'a> {
        Error {
            failed: &self.branches[..],
            message,
        }
    }
    pub fn error(&'a self, bad: usize, message: &'static str) -> Error<'a> {
        Error {
            failed: &self.branches[..=bad],
            message,
        }
    }
    pub fn text_value(&'a self, from: Value<'a>) -> Result<TextInList<'a>, Error<'a>> {
        TextInList::from(self.value(from)?).ok_or(self.error_full("path does not end at text"))
    }
    pub fn list_value(&'a self, from: Value<'a>) -> Result<ListInList<'a>, Error<'a>> {
        ListInList::from(self.value(from)?).ok_or(self.error_full("path does not end at list"))
    }
    pub fn dict_value(&'a self, from: Value<'a>) -> Result<DictInList<'a>, Error<'a>> {
        DictInList::from(self.value(from)?).ok_or(self.error_full("path does not end at dict"))
    }
    fn value(&'a self, mut from: Value<'a>) -> Result<&'a Cell<Value<'a>>, Error<'a>> {
        if self.branches.is_empty() {
            return Err(self.error_full("empty path can't be resolved"));
        }
        for (step, branch) in self.branches.iter().enumerate() {
            match &from {
                Value::Text(_text) => {
                    return Err(self.error(step, "path ended prematurely by a text value"));
                }
                Value::List(list) => match branch {
                    Branch::List(at) => match list.cells.get(*at) {
                        None => return Err(self.error(step, "index out of bounds")),
                        Some(found) => {
                            if step + 1 == self.branches.len() {
                                return Ok(found);
                            }
                            from = found.get();
                        }
                    },
                    Branch::Dict(_) => {
                        return Err(self.error(step, "path expected dict but found list"));
                    }
                },
                Value::Dict(dict) => match branch {
                    Branch::Dict(key) => {
                        match dict.find(key) {
                            None => return Err(self.error(step, "key not found")),
                            Some(found) => {
                                from = found.get().value;
                            }
                        };
                    }
                    Branch::List(_) => {
                        return Err(self.error(step, "path expected list but found dict"));
                    }
                },
            }
        }
        Err(self.error_full("path did not end at a value inside a list"))
    }
    pub fn text_keyed(&'a self, from: Value<'a>) -> Result<TextInDict<'a>, Error<'a>> {
        TextInDict::from(self.keyed(from)?).ok_or(self.error_full("path does not end at text"))
    }
    pub fn list_keyed(&'a self, from: Value<'a>) -> Result<ListInDict<'a>, Error<'a>> {
        ListInDict::from(self.keyed(from)?).ok_or(self.error_full("path does not end at list"))
    }
    pub fn dict_keyed(&'a self, from: Value<'a>) -> Result<DictInDict<'a>, Error<'a>> {
        DictInDict::from(self.keyed(from)?).ok_or(self.error_full("path does not end at dict"))
    }
    fn keyed(&'a self, mut from: Value<'a>) -> Result<&'a Cell<Keyed<'a>>, Error<'a>> {
        if self.branches.is_empty() {
            return Err(self.error_full("empty path can't be resolved"));
        }
        for (step, branch) in self.branches.iter().enumerate() {
            match &from {
                Value::Text(_text) => {
                    return Err(self.error(step, "path ended prematurely by a text value"));
                }
                Value::List(list) => match branch {
                    Branch::List(at) => match list.cells.get(*at) {
                        None => return Err(self.error(step, "index out of bounds")),
                        Some(found) => {
                            from = found.get();
                        }
                    },
                    Branch::Dict(_) => {
                        return Err(self.error(step, "path expected dict but found list"));
                    }
                },
                Value::Dict(dict) => match branch {
                    Branch::Dict(key) => {
                        match dict.find(key) {
                            None => return Err(self.error(step, "key not found")),
                            Some(found) => {
                                if step + 1 == self.branches.len() {
                                    return Ok(found);
                                }
                                from = found.get().value;
                            }
                        };
                    }
                    Branch::List(_) => {
                        return Err(self.error(step, "path expected list but found dict"));
                    }
                },
            }
        }
        Err(self.error_full("path did not end at a value inside a dict"))
    }
}
