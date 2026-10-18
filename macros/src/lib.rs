#![allow(missing_docs)]

//! The `indent` macro defined here is re-exported from and documented in
//! [the main `tindalwic` crate](https://docs.rs/tindalwic).
//! You could depend on and import from this macros crate directly,
//! but the simpler `use tindalwic::indent` is suggested.

use proc_macro::TokenStream as RawStream;
use std::collections::HashMap;
use syn::{Fields, ItemEnum, ItemStruct, LitStr};
use syn::{parse::Parse, parse_macro_input};
use tindalwic_indent::Line;

mod errors {
    use proc_macro2::TokenStream;
    use quote::{ToTokens, quote};
    use std::fmt::Display;
    use syn::Error;
    #[derive(Default)]
    pub struct Errors {
        any: Option<Error>,
    }
    impl Errors {
        pub fn push(&mut self, error: Error) {
            match &mut self.any {
                None => self.any = Some(error),
                Some(previous) => previous.combine(error),
            }
        }
        pub fn err<T: ToTokens, M: Display>(&mut self, tokens: T, message: M) {
            self.push(Error::new_spanned(tokens, message));
        }
        #[inline]
        pub fn or<T: ToTokens>(self, success: T) -> TokenStream {
            match self.any {
                None => quote!(#success),
                Some(error) => error.into_compile_error(),
            }
        }
        pub fn and<T: ToTokens>(self, success: T) -> TokenStream {
            match self.any {
                None => quote!(#success),
                Some(error) => {
                    let mut stream = TokenStream::new();
                    stream.extend(error.into_compile_error());
                    stream.extend(quote!(#success));
                    stream
                }
            }
        }
        pub fn bail(self) -> super::RawStream {
            match self.any {
                None => TokenStream::new().into(),
                Some(error) => error.into_compile_error().into(),
            }
        }
    }
}

/// it is a bit awkward to write snippets of tindalwic in Rust source code.
// sub-span isn't ready yet https://github.com/rust-lang/rust/issues/54725
#[proc_macro]
pub fn indent(stream: RawStream) -> RawStream {
    let input = parse_macro_input!(stream as LitStr);
    let mut errors = errors::Errors::default();
    let value = input.value();
    let mut all: Vec<_> = value.lines().map(|it| Line::from(it)).collect();
    if all.len() < 3 {
        errors.err(input, "not enough lines for macro to work with");
        return errors.bail();
    }
    let last = all.len() - 1;
    if all[0].content.len() != 0 || !all[last].content.is_empty() {
        errors.err(input, "the quotes must go on their own lines");
        return errors.bail();
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
    errors.or(converted).into()
}

// ====================================================================================
// perhaps this is a bit of an overreaction, but forgetting to update all the places
// that need to know about precious fields to keep them in sync and only getting
// inscrutable runtime weirdness is super annoying. so making it compile-time...

static TRUTH: phf::Map<&'static str, phf::Map<&'static str, &'static str>> = phf::phf_map! {
    "File" => phf::phf_map!{ ""=>"<'a>",
        "shebang" => "Option<UTF8<'a>>",
        "prolog" => "Option<UTF8<'a>>",
        "groups" => "Groups<'a>" },
    "Groups" => phf::phf_map!{ ""=>"<'a>",
        "slice" => "&'a [Cell<Group<'a>>]" },
    "Group" => phf::phf_map!{ ""=>"<'a>",
        "heading" => "Option<UTF8<'a>>",
        "members" => "Members<'a>" },
    "Members" => phf::phf_map!{ ""=>"<'a>",
        "slice" => "&'a [Cell<Member<'a>>]" },
    "Member" => phf::phf_map!{ ""=>"<'a>",
        "comment" => "Option<UTF8<'a>>",
        "name" => "UTF8<'a>",
        "longer" => "bool",
        "data" => "Data<'a>" },
    "Data" => phf::phf_map!{ ""=>"<'a>",
        "Text" => "Text<'a>",
        "List" => "List<'a>" },
    "List" => phf::phf_map!{ ""=>"<'a>",
        "prolog" => "Option<UTF8<'a>>",
        "groups" => "Groups<'a>",
        "epilog" => "Option<UTF8<'a>>" },
    "Text" => phf::phf_map!{ ""=>"<'a>",
        "lines" => "UTF8<'a>",
        "epilog" => "Option<UTF8<'a>>",
        "verbose" => "bool" },
    "UTF8" => phf::phf_map!{ ""=>"<'a>",
        "slice" => "&'a str",
        "indent" => "usize" },
};
/// naive conversion gives a HashMap<&&str,&&str>
fn hash_map(
    map: &'static phf::Map<&'static str, &'static str>,
) -> HashMap<&'static str, &'static str> {
    map.entries()
        .map(|(&k, &v)| (k, v))
        .collect::<HashMap<&'static str, &'static str>>()
}
fn parse_str_eq<T: Parse + PartialEq>(input: &T, truth: &str) -> bool {
    match syn::parse_str::<T>(truth) {
        Err(_) => false,
        Ok(parsed) => input == &parsed,
    }
}

#[proc_macro_attribute]
pub fn check_fields(_attr: RawStream, stream: RawStream) -> RawStream {
    let input = parse_macro_input!(stream as ItemStruct);
    let mut errors = errors::Errors::default();
    match TRUTH.get(&input.ident.to_string()) {
        None => errors.err(&input.ident, "unknown struct"),
        Some(truth) => {
            let mut expected = hash_map(truth);
            match expected.remove("") {
                None if !input.generics.params.is_empty() => {
                    errors.err(&input.generics, "unexpected generics")
                }
                Some(generics) if !parse_str_eq(&input.generics, generics) => {
                    errors.err(&input.generics, format!("expected `{generics}`"))
                }
                _ => {}
            }
            for field in &input.fields {
                let Some(name) = &field.ident else {
                    errors.err(&field, "impossible unnamed struct field");
                    continue;
                };
                let key = name.to_string();
                match expected.remove(key.as_str()) {
                    None => errors.err(&name, "unexpected field"),
                    Some(ty) if !parse_str_eq(&field.ty, ty) => {
                        errors.err(&field.ty, format!("expected `{ty}`"))
                    }
                    _ => {}
                }
            }
            if !expected.is_empty() {
                let missing: Vec<_> = expected
                    .iter()
                    .map(|(&k, &v)| format!("{k}: {v}"))
                    .collect();
                errors.err(&input.ident, format!("missing: {:?}", missing));
            }
        }
    }
    errors.and(input).into()
}

#[proc_macro_attribute]
pub fn check_variants(_attr: RawStream, stream: RawStream) -> RawStream {
    let input = parse_macro_input!(stream as ItemEnum);
    let mut errors = errors::Errors::default();
    match TRUTH.get(&input.ident.to_string()) {
        None => errors.err(&input.ident, "unknown enum"),
        Some(truth) => {
            let mut expected = hash_map(truth);
            match expected.remove("") {
                None if !input.generics.params.is_empty() => {
                    errors.err(&input.generics, "unexpected generics")
                }
                Some(generics) if !parse_str_eq(&input.generics, generics) => {
                    errors.err(&input.generics, format!("expected `{generics}`"))
                }
                _ => {}
            }
            for variant in &input.variants {
                let name = &variant.ident;
                match &variant.fields {
                    Fields::Unit => errors.err(&variant.ident, "expected unnamed fields"),
                    Fields::Named(_) => errors.err(&variant.fields, "expected unnamed fields"),
                    Fields::Unnamed(fields) => {
                        match fields.unnamed.first() {
                            None => errors.err(&fields, "expected one field"),
                            Some(first) => {
                                let key = name.to_string();
                                match expected.remove(key.as_str()) {
                                    None => errors.err(&name, "unexpected variant"),
                                    Some(ty) if !parse_str_eq(&first.ty, ty) => {
                                        errors.err(&first.ty, format!("expected `{ty}`"))
                                    }
                                    _ => {}
                                }
                            }
                        }
                        if fields.unnamed.len() > 1 {
                            errors.err(&fields, "expected single field")
                        }
                    }
                }
            }
            if !expected.is_empty() {
                let missing: Vec<_> = expected
                    .iter()
                    .map(|(&k, &v)| format!("{k}: {v}"))
                    .collect();
                errors.err(&input.ident, format!("missing: {:?}", missing));
            }
        }
    }
    errors.and(input).into()
}
