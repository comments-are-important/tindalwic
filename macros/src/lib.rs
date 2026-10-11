#![allow(missing_docs)]

//! The macros defined here are re-exported from and documented in
//! [the main `tindalwic` crate](https://docs.rs/tindalwic).
//! You could depend on and import from this macros crate directly,
//! but the simpler `use tindalwic` is suggested.

use proc_macro::TokenStream as RawStream;
use quote::quote;
use syn::LitStr;
use syn::parse_macro_input;
use tindalwic_indent::Line;

/// it is a bit awkward to write snippets of tindalwic in Rust source code.
// Literal::subspan isn't ready yet https://github.com/rust-lang/rust/issues/54725
#[proc_macro]
pub fn indent(input: RawStream) -> RawStream {
    let token = parse_macro_input!(input as LitStr);
    let value = token.value();
    let mut all: Vec<_> = value.lines().map(|it| Line::from(it)).collect();
    if all.len() < 3 {
        return syn::Error::new_spanned(token, "not enough lines for macro to work with")
            .to_compile_error()
            .into();
    }
    let last = all.len() - 1;
    if all[0].content.len() != 0 || !all[last].content.is_empty() {
        return syn::Error::new_spanned(token, "the quotes must go on their own lines")
            .to_compile_error()
            .into();
    }
    let lines = &mut all[1..last];
    Line::minimize(lines);
    let maximum = lines.iter().map(|it| it.indentation()).max().unwrap_or(0);
    let tabs = "\t".repeat(maximum);
    for line in lines.iter_mut() {
        line.margin = &tabs[0..line.indentation()];
    }
    let length: usize = lines.iter().map(|it| it.len()).sum();
    let mut converted = String::with_capacity(lines.len() - 1 + length);
    let mut iter = lines.iter();
    if let Some(line) = iter.next() {
        converted.push_str(line.margin);
        converted.push_str(line.content);
    }
    while let Some(line) = iter.next() {
        converted.push('\n');
        converted.push_str(line.margin);
        converted.push_str(line.content);
    }
    quote!(#converted).into()
}
