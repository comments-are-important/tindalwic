//! macros depend on these, so must be public.
//! but you should probably not use these directly, macros are much easier.

use crate::group::{Groups, Members};
use crate::parse::{Build, Parse};
use crate::{Group, Member};

use core::cell::Cell;

/// push T into stack on low side of array, finish them into high side.
/// aligns to an in-order tree traversal: push on entry, visit kids, finish on exit.
/// all kids finished before visiting next sibling, so siblings are adjacent,
/// finish moves those adjacent cells to the high end of the array.
/// total O(n) moves, zero extra space, caller only needs to track child count.
/// basically two-stacks-in-one-array (Knuth, TAOCP Vol. 1, §2.2.2 p.246),
/// but keep siblings together by transferring, as group, from low to high.
struct LowToHigh<'a, T> {
    cells: &'a [Cell<T>],
    next: usize,
    done: usize,
}
impl<'a, T> LowToHigh<'a, T> {
    fn wrap(cells: &'a [Cell<T>]) -> Self {
        LowToHigh {
            cells,
            next: 0,
            done: cells.len(),
        }
    }
    fn push(&mut self, value: T) -> Option<()> {
        if self.next >= self.done {
            return None;
        }
        let next = self.next;
        self.cells[next].set(value);
        self.next += 1;
        Some(())
    }
    fn finish(&mut self, count: usize) -> Option<&'a [Cell<T>]> {
        if self.next < count {
            return None;
        }
        if self.next == self.done {
            let both = self.next - count;
            self.next = both;
            self.done = both;
            return Some(&self.cells[both..both + count]);
        }
        self.next -= count;
        self.done -= count;
        for offset in (0..count).rev() {
            self.cells[self.next + offset].swap(&self.cells[self.done + offset]);
        }
        Some(&self.cells[self.done..self.done + count])
    }
}

struct StackBuilder<'a> {
    groups: LowToHigh<'a, Group<'a>>,
    members: LowToHigh<'a, Member<'a>>,
}
impl<'a> StackBuilder<'a> {
    pub fn wrap(groups: &'a [Cell<Group<'a>>], members: &'a [Cell<Member<'a>>]) -> Self {
        let groups = LowToHigh::wrap(groups);
        let members = LowToHigh::wrap(members);
        StackBuilder { groups, members }
    }
}
/// a flavor of Arena that uses fixed-size array slices.
/// the arrays can live in the stack.
pub struct Arena<'a> {
    builder: StackBuilder<'a>,
}
impl<'a> Parse<'a> for Arena<'a> {
    fn builder(&mut self) -> &mut dyn Build<'a> {
        &mut self.builder
    }
}
impl<'a> Arena<'a> {
    /// provide the storage
    pub fn wrap(groups: &'a [Cell<Group<'a>>], members: &'a [Cell<Member<'a>>]) -> Self {
        let builder = StackBuilder::wrap(groups, members);
        Arena { builder }
    }
    /// returns count of items that can still fit.
    pub fn item_slots(&self) -> usize {
        self.builder.groups.done - self.builder.groups.next
    }
    /// returns count of entries that can still fit.
    pub fn entry_slots(&self) -> usize {
        self.builder.members.done - self.builder.members.next
    }
    /// the json! macro uses this as a sanity check that no space gets wasted.
    pub fn completed(&self) -> Option<()> {
        if self.builder.groups.done == 0 && self.builder.members.done == 0 {
            Some(())
        } else {
            None
        }
    }
}
impl<'a> Build<'a> for StackBuilder<'a> {
    fn finish_groups(&mut self, count: usize) -> Result<Groups<'a>, &'static str> {
        self.groups
            .finish(count)
            .map(|cells| Groups::new(cells))
            .ok_or_else(|| "not enough items to make that list")
    }
    fn finish_members(&mut self, count: usize) -> Result<Members<'a>, &'static str> {
        self.members
            .finish(count)
            .map(|cells| Members::new(cells))
            .ok_or_else(|| "not enough entries to make that dict")
    }
    fn push_group(&mut self, item: Group<'a>) -> Result<(), &'static str> {
        self.groups.push(item).ok_or_else(|| "no room for item")
    }
    fn push_member(&mut self, entry: Member<'a>) -> Result<(), &'static str> {
        self.members.push(entry).ok_or_else(|| "no room for entry")
    }
}

// ====================================================================================
