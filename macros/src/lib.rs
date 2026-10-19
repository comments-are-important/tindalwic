#![allow(missing_docs)]
#![allow(unused)]

//! The `indent` macro defined here is re-exported from and documented in
//! [the main `tindalwic` crate](https://docs.rs/tindalwic).
//! You could depend on and import from this macros crate directly,
//! but the simpler `use tindalwic::indent` is suggested.

use heck::ToUpperCamelCase;
use proc_macro::TokenStream as RawStream;
use proc_macro2::{Delimiter, Group, Span, TokenStream, TokenTree};
use quote::{ToTokens, quote};
use std::collections::HashMap;
use std::fmt::Display;
use std::iter::once;
use syn::parse::{Parse, ParseStream, Parser};
use syn::punctuated::Punctuated;
use syn::spanned::Spanned as _;
use syn::{Error, Result, parenthesized, parse_macro_input, parse_str};
use syn::{Fields, Ident, ItemEnum, ItemStruct, LitStr, Token, Type, Variant};
use tindalwic_indent::Line;

#[derive(Default)]
struct Errors {
    any: Option<Error>,
}
impl Errors {
    fn push(&mut self, error: Error) {
        match &mut self.any {
            None => self.any = Some(error),
            Some(previous) => previous.combine(error),
        }
    }
    fn new<M: Display>(&mut self, span:Span, message: M) {
        self.push(Error::new(span, message));
    }
    fn span<T: ToTokens, M: Display>(&mut self, tokens: T, message: M) {
        self.push(Error::new_spanned(tokens, message));
    }
    fn or<T: ToTokens>(self, success: T) -> TokenStream {
        match self.any {
            None => quote!(#success),
            Some(error) => error.into_compile_error(),
        }
    }
    fn and<T: ToTokens>(self, success: T) -> TokenStream {
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
    fn bail(self) -> RawStream {
        match self.any {
            None => RawStream::new(),
            Some(error) => error.into_compile_error().into(),
        }
    }
}

/// it is a bit awkward to write snippets of tindalwic in Rust source code.
// sub-span isn't ready yet https://github.com/rust-lang/rust/issues/54725
#[proc_macro]
pub fn indent(stream: RawStream) -> RawStream {
    let input = parse_macro_input!(stream as LitStr);
    let mut errors = Errors::default();
    let value = input.value();
    let mut all: Vec<_> = value.lines().map(|it| Line::from(it)).collect();
    if all.len() < 3 {
        errors.span(input, "not enough lines for macro to work with");
        return errors.bail();
    }
    let last = all.len() - 1;
    if all[0].content.len() != 0 || !all[last].content.is_empty() {
        errors.span(input, "the quotes must go on their own lines");
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
type Facts = phf::Map<&'static str, &'static str>;
static TRUTH: phf::Map<&'static str, Facts> = phf::phf_map! {
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
fn hash_map(map: &'static Facts) -> HashMap<&'static str, &'static str> {
    map.entries()
        .map(|(&k, &v)| (k, v))
        .collect::<HashMap<&'static str, &'static str>>()
}
fn parse_str_eq<T: Parse + PartialEq>(input: &T, truth: &str) -> bool {
    match parse_str::<T>(truth) {
        Err(_) => false,
        Ok(parsed) => input == &parsed,
    }
}

#[proc_macro_attribute]
pub fn check_fields(_attr: RawStream, stream: RawStream) -> RawStream {
    let input = parse_macro_input!(stream as ItemStruct);
    let mut errors = Errors::default();
    match TRUTH.get(&input.ident.to_string()) {
        None => errors.span(&input.ident, "unknown struct"),
        Some(truth) => {
            let mut expected = hash_map(truth);
            match expected.remove("") {
                None if !input.generics.params.is_empty() => {
                    errors.span(&input.generics, "unexpected generics")
                }
                Some(generics) if !parse_str_eq(&input.generics, generics) => {
                    errors.span(&input.generics, format!("expected `{generics}`"))
                }
                _ => {}
            }
            for field in &input.fields {
                let Some(name) = &field.ident else {
                    errors.span(&field, "impossible unnamed struct field");
                    continue;
                };
                let key = name.to_string();
                match expected.remove(key.as_str()) {
                    None => errors.span(&name, "unexpected field"),
                    Some(ty) if !parse_str_eq(&field.ty, ty) => {
                        errors.span(&field.ty, format!("expected `{ty}`"))
                    }
                    _ => {}
                }
            }
            if !expected.is_empty() {
                let missing: Vec<_> = expected
                    .iter()
                    .map(|(&k, &v)| format!("{k}: {v}"))
                    .collect();
                errors.span(&input.ident, format!("missing: {:?}", missing));
            }
        }
    }
    errors.and(input).into()
}

#[proc_macro_attribute]
pub fn check_variants(_attr: RawStream, stream: RawStream) -> RawStream {
    let input = parse_macro_input!(stream as ItemEnum);
    let mut errors = Errors::default();
    match TRUTH.get(&input.ident.to_string()) {
        None => errors.span(&input.ident, "unknown enum"),
        Some(truth) => {
            let mut expected = hash_map(truth);
            match expected.remove("") {
                None if !input.generics.params.is_empty() => {
                    errors.span(&input.generics, "unexpected generics")
                }
                Some(generics) if !parse_str_eq(&input.generics, generics) => {
                    errors.span(&input.generics, format!("expected `{generics}`"))
                }
                _ => {}
            }
            for variant in &input.variants {
                let name = &variant.ident;
                match &variant.fields {
                    Fields::Unit => errors.span(&variant.ident, "expected unnamed fields"),
                    Fields::Named(_) => errors.span(&variant.fields, "expected unnamed fields"),
                    Fields::Unnamed(fields) => {
                        match fields.unnamed.first() {
                            None => errors.span(&fields, "expected one field"),
                            Some(first) => {
                                let key = name.to_string();
                                match expected.remove(key.as_str()) {
                                    None => errors.span(&name, "unexpected variant"),
                                    Some(ty) if !parse_str_eq(&first.ty, ty) => {
                                        errors.span(&first.ty, format!("expected `{ty}`"))
                                    }
                                    _ => {}
                                }
                            }
                        }
                        if fields.unnamed.len() > 1 {
                            errors.span(&fields, "expected single field")
                        }
                    }
                }
            }
            if !expected.is_empty() {
                let missing: Vec<_> = expected
                    .iter()
                    .map(|(&k, &v)| format!("{k}: {v}"))
                    .collect();
                errors.span(&input.ident, format!("missing: {:?}", missing));
            }
        }
    }
    errors.and(input).into()
}

// use std::cell::Cell;
// use std::sync::atomic::{AtomicU64, Ordering};
// static TAG: AtomicU64 = AtomicU64::new(1);
// thread_local! { static STACK: Cell<Vec<u64>> = Cell::new(Vec::new()); }
// fn trace_in(message: String) {
//     STACK.with(|cell| {
//         let mut stack = cell.take();
//         stack.push(TAG.fetch_add(1, Ordering::Relaxed));
//         eprintln!("{:?}  in: {}", stack, message);
//         cell.set(stack);
//     });
// }
// fn trace_msg(message: String) {
//     STACK.with(|cell| {
//         let mut stack = cell.take();
//         eprintln!("{:?} msg: {}", stack, message);
//         cell.set(stack);
//     });
// }
// fn trace_out(message: String) {
//     STACK.with(|cell| {
//         let mut stack = cell.take();
//         eprintln!("{:?} out: {}", stack, message);
//         stack.pop();
//         cell.set(stack);
//     });
// }

struct ExpandEach {
    pattern: String,
    span: Span,
    template: TokenStream,
}
impl Parse for ExpandEach {
    fn parse(input: ParseStream) -> Result<Self> {
        input.parse::<Token![@]>()?;
        let key = input.parse::<Type>()?; // a plain old Ident can also be a Type
        input.parse::<Token![|]>()?;
        Ok(ExpandEach {
            template: input.parse()?,
            span: key.span(),
            pattern: quote!(#key)
                .to_string()
                .chars()
                .filter(|c| !c.is_whitespace())
                .collect(),
        })
    }
}
impl ExpandEach {
    fn expand(self, context: Option<&'static Facts>, errors: &mut Errors) -> TokenStream {
        // trace_in(format!("each {} @{:?}", self.pattern, context));
        if let Some(facts) = context {
            for (&key, &value) in facts {
                if !key.is_empty() {
                    if key == self.pattern || value == self.pattern {
                        let one = ExpandOne { facts, key, value };
                        let expanded = one.expand(self.template.clone(), errors);
                        // trace_out(format!("{} winner: {key} -> {}", self.pattern, expanded.clone().to_string()));
                        return expanded;
                    }
                    // else { trace_msg(format!("{} elided: isn't {key}", self.pattern)) }
                }
            }
            // trace_msg(format!("each falling through no winners"));
        }
        let mut output = TokenStream::new();
        if let Some(facts) = TRUTH.get(&self.pattern) {
            for (&key, &value) in facts.entries() {
                if !key.is_empty() {
                    let one = ExpandOne { facts, key, value };
                    let expanded = one.expand(self.template.clone(), errors);
                    // trace_msg(format!("{} each: {key} -> {}", self.pattern, expanded.clone().to_string()));
                    output.extend(once(Group::new(Delimiter::None, expanded)));
                }
            }
        } else {
            let message = format!(
                "bad key {} expected one of: {:?}",
                self.pattern,
                TRUTH.keys()
            );
            errors.new(self.span, message);
        }
        // trace_out(format!("each {}", output.clone().to_string()));
        output
    }
}
struct ExpandOne {
    facts: &'static Facts,
    key: &'static str,
    value: &'static str,
}
impl ExpandOne {
    fn expand(&self, stream: TokenStream, errors: &mut Errors) -> TokenStream {
        // trace_in(format!("one {}: {}", self.key, self.value));
        let mut output = TokenStream::new();
        let mut iter = stream.clone().into_iter().peekable();
        while let Some(tt) = iter.next() {
            if let TokenTree::Punct(punct) = &tt {
                let marker = punct.as_char();
                let literal = marker == '#';
                if marker == '@' || literal {
                    if let Some(TokenTree::Ident(ident)) = iter.peek() {
                        if let Some(replacement) = self.substitute(literal, ident, errors) {
                            // trace_msg(format!("subst {literal} {ident} -> {}", replacement.clone().to_string()));
                            output.extend(replacement);
                            iter.next(); // advance past the marker, and
                            continue; // peek already handled, so do not fall through
                        }
                    }
                }
            }
            if let TokenTree::Group(group) = &tt {
                if let Some(TokenTree::Punct(punct)) = group.stream().into_iter().next() {
                    if '@' == punct.as_char() {
                        if group.delimiter() != Delimiter::Parenthesis {
                            let message = "expansion requires parens: `(@pattern|...template...)`";
                            errors.new(group.span_open().into(), message);
                        }
                        match syn::parse2::<ExpandEach>(group.stream()) {
                            Err(err) => errors.push(err),
                            Ok(truth) => {
                                if self.key == truth.pattern || self.value == truth.pattern {
                                    output.extend(truth.expand(Some(self.facts), errors));
                                }
                            },
                        }
                        continue;
                    }
                }
                let stream = self.expand(group.stream(), errors);
                let mut expanded = Group::new(group.delimiter(), stream);
                expanded.set_span(group.span());
                output.extend(once(expanded));
                continue;
            }
            output.extend(once(tt));
        }
        // trace_out(format!("one {}", output.clone().to_string()));
        output
    }
    fn substitute(&self, literal: bool, ident: &Ident, errors: &mut Errors) -> Option<TokenStream> {
        let ExpandOne { key, value, .. } = self;
        let upper_camel = key.to_upper_camel_case();
        errors.push(match ident.to_string().as_str() {
            "name" if literal => return Some(quote!(#key)),
            "name" => match Ident::parse.parse_str(key) {
                Ok(ok) => return Some(quote!(#ok)),
                Err(err) => err,
            },
            "Name" if literal => return Some(quote!(#upper_camel)),
            "Name" => match Ident::parse.parse_str(&upper_camel) {
                Ok(ok) => return Some(quote!(#ok)),
                Err(err) => err,
            },
            "type" if literal => return Some(quote!(#value)),
            "type" => match Type::parse.parse_str(value) {
                Ok(ok) => return Some(quote!(#ok)),
                Err(err) => err,
            },
            _ => return None,
        });
        None
    }
}

fn expand_raw(stream: RawStream, errors: &mut Errors) -> RawStream {
    use proc_macro::{Delimiter, Group, TokenTree};
    let mut output = RawStream::new();
    for tt in stream {
        if let TokenTree::Group(group) = &tt {
            if let Some(TokenTree::Punct(punct)) = group.stream().into_iter().next() {
                if '@' == punct.as_char() {
                    if group.delimiter() != Delimiter::Parenthesis {
                        let message = "expansion requires parens: `(@Type|...template...)`";
                        errors.new(group.span_open().into(), message);
                    }
                    match syn::parse::<ExpandEach>(group.stream()) {
                        Err(err) => errors.push(err),
                        Ok(truth) => output.extend(RawStream::from(truth.expand(None, errors))),
                    }
                    continue;
                }
            }
            let stream = expand_raw(group.stream(), errors);
            let mut expanded = Group::new(group.delimiter(), stream);
            expanded.set_span(group.span());
            output.extend(once(expanded));
            continue;
        }
        output.extend(once(tt));
    }
    output
}
#[proc_macro]
pub fn expand_truth(stream: RawStream) -> RawStream {
    let mut errors = Errors::default();
    let output = expand_raw(stream, &mut errors);
    if errors.any.is_some() {
        errors.bail()
    } else {
        output
    }
}

/// macros are not allowed in enum definitions. can't expand_truth! to fill in the
/// variants. use this instead.
#[proc_macro_attribute]
pub fn inject_unit_variants(attr: RawStream, stream: RawStream) -> RawStream {
    let names: Punctuated<Ident, Token![,]> =
        parse_macro_input!(attr with Punctuated::parse_separated_nonempty);
    let mut input = parse_macro_input!(stream as ItemEnum);
    let mut errors = Errors::default();
    for name in names {
        match TRUTH.get(&name.to_string()) {
            None => errors.span(name, "unknown"),
            Some(map) => {
                for (&k, _) in map.entries() {
                    if !k.is_empty() {
                        match Variant::parse.parse_str(&k.to_upper_camel_case()) {
                            Err(err) => errors.push(err),
                            Ok(variant) => input.variants.push(variant),
                        }
                    }
                }
            }
        }
    }
    errors.and(input).into()
}
