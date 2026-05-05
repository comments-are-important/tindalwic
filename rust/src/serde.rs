use ::serde::ser::{Serialize, SerializeSeq, Serializer};

use super::*;

impl<'a> Serialize for UTF8<'a> {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        if self.dedent == 0 || self.dedent == usize::MAX {
            s.serialize_str(self.slice)
        } else {
            s.serialize_str(&self.joined())
        }
    }
}

pub fn serialize_items<'a, 'store, S: Serializer>(
    cells: &'store [Cell<Item<'a, 'store>>],
    s: S,
) -> Result<S::Ok, S::Error> {
    let mut s = s.serialize_seq(Some(cells.len()))?;
    for cell in cells {
        s.serialize_element(&cell.get())?;
    }
    s.end()
}

pub fn serialize_entries<'a, 'store, S: Serializer>(
    cells: &'store [Cell<Entry<'a, 'store>>],
    s: S,
) -> Result<S::Ok, S::Error> {
    let mut s = s.serialize_seq(Some(cells.len()))?;
    for cell in cells {
        s.serialize_element(&cell.get())?;
    }
    s.end()
}
