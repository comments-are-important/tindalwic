#![allow(missing_docs)]

//! The macros defined here are re-exported from and documented in
//! [the main `tindalwic` crate](https://docs.rs/tindalwic).
//! You could depend on and import from this macros crate directly,
//! but the simpler `use tindalwic` is suggested.

use proc_macro::TokenStream as RawStream;
use quote::quote;
use syn::LitStr;
use syn::parse_macro_input;

struct Line<'a> {
    spaces: usize,
    content: &'a str,
}
impl<'a> Line<'a> {
    fn from(value: &'a str) -> Self {
        let content = value.trim_start_matches(' ');
        let spaces = value.len() - content.len();
        Line { spaces, content }
    }
    fn is_empty(&self) -> bool {
        self.spaces == 0 && self.content.is_empty()
    }
    fn is_blank(&self) -> bool {
        self.content.is_empty()
    }
}

/// it is a bit awkward to write snippets of tindalwic in Rust source code.
// Literal::subspan isn't ready yet https://github.com/rust-lang/rust/issues/54725
#[proc_macro]
pub fn indent(input: RawStream) -> RawStream {
    let token = parse_macro_input!(input as LitStr);
    let value = token.value();
    let mut all: Vec<_> = value.lines().map(Line::from).collect();
    if all.len() < 3 {
        return syn::Error::new_spanned(token, "not enough lines for macro to work with")
            .to_compile_error()
            .into();
    }
    let last = all.len() - 1;
    if !all[0].content.is_empty() || !all[last].is_blank() {
        return syn::Error::new_spanned(token, "the quotes must go on their own lines")
            .to_compile_error()
            .into();
    }
    let lines = &mut all[1..last];
    let Some(common) = lines
        .iter()
        .filter(|it| !it.is_empty())
        .map(|it| it.spaces)
        .min()
    else {
        return syn::Error::new_spanned(token, "every line is empty, nothing to work with")
            .to_compile_error()
            .into();
    };
    for line in lines.iter_mut() {
        line.spaces = line.spaces.saturating_sub(common);
    }
    let maximum = lines.iter().map(|it| it.spaces).max().unwrap_or(0);
    let mut tabs = vec![0usize; maximum + 1];
    for line in lines.iter_mut() {
        if line.spaces != 0 {
            tabs[line.spaces] = 1;
        }
    }
    let mut stops = 0usize;
    for tab in tabs.iter_mut() {
        if *tab != 0 {
            stops += 1;
            *tab = stops;
        }
    }
    let mut converted = String::with_capacity(value.len());
    for line in lines.iter_mut() {
        if !converted.is_empty() {
            converted.push('\n');
        }
        if line.spaces != 0 {
            for _ in 0..tabs[line.spaces] {
                converted.push('\t');
            }
        }
        converted.push_str(line.content);
    }
    quote!(#converted).into()
}
