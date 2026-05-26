#![no_std]

//! Text in Nested Dictionaries and Lists - with Important Comments

#[doc(inline)]
/// traverse a path from the root down into the data structure.
///
/// the syntax is very close to that of the encoded data.
pub use tindalwic_macros::walk;

#[doc(inline)]
/// build an [tree::Item] using a subset of the JSON syntax.
///
/// this helps to write code snippets that make a structural change to a [tree::File].
/// a typical snippet would:
///  + [walk!] into a [tree::File] to the place to be changed,
///  + use [json!] to build a new [tree::Item],
///  + then use [core::cell::Cell::set] to affect the change.
pub use tindalwic_macros::json;

#[doc(inline)]
pub use tindalwic_macros::arena;

pub mod capped;
pub mod fmt;
pub mod parse;
pub mod tree;
pub mod walk;

#[cfg(feature = "alloc")]
pub mod alloc;
#[cfg(feature = "bumpalo")]
pub mod bumpalo;
#[cfg(feature = "serde")]
pub mod serde;

#[cfg(test)]
#[allow(unused_extern_crates)]
extern crate self as test_rename_of_tindalwic_dependency;
