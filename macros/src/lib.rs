#![allow(missing_docs)]

//! The `indent` macro defined here is re-exported from and documented in
//! [the main `tindalwic` crate](https://docs.rs/tindalwic).
//! You could depend on and import from this macros crate directly,
//! but the simpler `use tindalwic::indent` is suggested.

use heck::{ToSnakeCase, ToUpperCamelCase};
use proc_macro::TokenStream as CompilerStream;
use proc_macro2::extra::DelimSpan;
use proc_macro2::{Delimiter, Group, Punct, Span, TokenStream};
use quote::{ToTokens, quote};
use std::collections::{HashMap, HashSet};
use std::fmt::Display;
use syn::buffer::{Cursor, TokenBuffer};
use syn::parse::{Parse, Parser};
use syn::spanned::Spanned as _;
use syn::{Error, parse_macro_input, parse_str};
use syn::{Fields, Generics, Ident, ItemEnum, ItemStruct, LitStr, Type};
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
    fn new<M: Display>(&mut self, span: Span, message: M) {
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
    fn and(self, original: CompilerStream) -> CompilerStream {
        match self.any {
            None => original,
            Some(error) => {
                let mut stream: CompilerStream = error.into_compile_error().into();
                stream.extend(original);
                stream
            }
        }
    }
    fn bail(self) -> CompilerStream {
        match self.any {
            None => CompilerStream::new(),
            Some(error) => error.into_compile_error().into(),
        }
    }
}

/// it is a bit awkward to write snippets of tindalwic in Rust source code.
// sub-span isn't ready yet https://github.com/rust-lang/rust/issues/54725
#[proc_macro]
pub fn indent(input: CompilerStream) -> CompilerStream {
    let literal = parse_macro_input!(input as LitStr);
    let mut errors = Errors::default();
    let value = literal.value();
    let mut all: Vec<_> = value.lines().map(|it| Line::from(it)).collect();
    if all.len() < 3 {
        errors.span(literal, "not enough lines for macro to work with");
        return errors.bail();
    }
    let last = all.len() - 1;
    if all[0].content.len() != 0 || !all[last].content.is_empty() {
        errors.span(literal, "the quotes must go on their own lines");
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

struct Entity {
    category: &'static str,
    facets: &'static [(&'static str, &'static str)],
}
static ENTITIES: phf::OrderedMap<&'static str, Entity> = phf::phf_ordered_map! {
    "File" => Entity {
        category: "struct",
        facets: &[
            ("shebang", "Comment"),
            ("prolog", "Comment"),
            ("groups", "Groups"),
        ],
    },
    "Groups" => Entity {
        category: "array",
        facets: &[
            ("items", "Group"),
        ],
    },
    "Group" => Entity {
        category: "struct",
        facets: &[
            ("heading", "Comment"),
            ("members", "Members"),
        ],
    },
    "Members" => Entity {
        category: "array",
        facets: &[
            ("items", "Member"),
        ],
    },
    "Member" => Entity {
        category: "struct",
        facets: &[
            ("comment", "Comment"),
            ("name", "Lines"),
            ("longer", "bool"),
            ("data", "Data"),
        ],
    },
    "Data" => Entity {
        category: "enum",
        facets: &[
            ("text", "Text"),
            ("list", "List"),
        ],
    },
    "List" => Entity {
        category: "struct",
        facets: &[
            ("prolog", "Comment"),
            ("groups", "Groups"),
            ("epilog", "Comment"),
        ],
    },
    "Text" => Entity {
        category: "struct",
        facets: &[
            ("lines", "Lines"),
            ("epilog", "Comment"),
            ("verbose", "bool"),
        ],
    },
    "Comment" => Entity {
        category: "option",
        facets: &[
            ("inner", "Lines"),
        ]
    },
    "Lines" => Entity {
        category: "opaque",
        facets: &[
            ("slice", "str"),
            ("indent", "usize"),
        ],
    },
};

#[cfg(test)]
mod entities {
    use super::{ENTITIES, ToSnakeCase, ToUpperCamelCase};

    #[test]
    fn correct_categories_and_cases() {
        fn nope(message: &'static str, key: &'static str, detail: &'static str) -> bool {
            eprintln!("{message} @ {key}: {detail}");
            false
        }
        for (&key, entity) in ENTITIES.entries() {
            assert!(match entity.category {
                "option" => {
                    assert_eq!(1, entity.facets.len());
                    assert_eq!("inner", entity.facets[0].0);
                    true
                }
                "enum" => true,
                "struct" => true,
                "array" => true,
                "opaque" => true,
                _ => nope("category", key, entity.category),
            });
            assert_eq!(key, key.to_upper_camel_case());
            for (name, link) in entity.facets.iter().copied() {
                let expected = if entity.category == "enum" {
                    name.to_upper_camel_case()
                } else {
                    name.to_snake_case()
                };
                assert_eq!(name, expected);
                let found = ENTITIES.get(link).is_some();
                let snake = link == link.to_snake_case();
                assert!(match (found, snake) {
                    (false, true) => true,
                    (true, false) => true,
                    _ => nope("link", key, link),
                });
            }
        }
    }
}

struct ParsedType {
    link: &'static str,
    parsed: Type,
}
struct ParsedTypeMap {
    map: HashMap<&'static str, ParsedType>,
}
impl ParsedTypeMap {
    fn new(entity: &Entity, errors: &mut Errors) -> Self {
        let mut map = HashMap::new();
        for (name, link) in entity.facets.iter().copied() {
            let resolved = if link == "str" {
                "&'a str".to_string()
            } else if let Some(found) = ENTITIES.get(link) {
                match found.category {
                    "option" => format!("Option<{}<'a>>", found.facets[0].1),
                    _ => format!("{link}<'a>"),
                }
            } else {
                link.to_string()
            };
            match parse_str::<Type>(&resolved) {
                Err(err) => errors.push(err),
                Ok(parsed) => {
                    map.insert(name, ParsedType { parsed, link });
                }
            }
        }
        ParsedTypeMap { map }
    }
    fn remove(&mut self, ident: &Ident, check: &Type, message: &str, errors: &mut Errors) {
        let key = ident.to_string().to_snake_case();
        match self.map.remove(key.as_str()) {
            None => errors.span(ident, format!("unexpected {message} {key}")),
            Some(found) if check != &found.parsed => {
                let expected = found.parsed.to_token_stream().to_string();
                let actual = check.to_token_stream().to_string();
                errors.span(
                    check,
                    format!(
                        "wrong `{}`, expected {expected}, actual {actual}",
                        found.link
                    ),
                )
            }
            _ => {}
        }
    }
    fn check_exhaustive(&self, ident: &Ident, errors: &mut Errors) {
        if !self.map.is_empty() {
            let missing: Vec<_> = self
                .map
                .iter()
                .map(|(&key, parsed)| format!("{key}: {}", parsed.link))
                .collect();
            errors.span(ident, format!("missing: {:?}", missing));
        }
    }
}
impl Entity {
    fn check_selector(&self, expected: &'static str, errors: &mut Errors) {
        if self.category != expected {
            errors.new(Span::call_site(), format!("expected {expected}"));
        }
    }
    fn check_lifetime(&self, generics: &Generics, errors: &mut Errors) {
        let actual: String = generics
            .to_token_stream()
            .to_string()
            .split_ascii_whitespace()
            .collect();
        if "<'a>" != actual {
            errors.span(generics, format!("expected: <'a> (not {actual})"));
        }
    }
}

#[proc_macro_attribute]
pub fn check_fields(attr: CompilerStream, item: CompilerStream) -> CompilerStream {
    let mut errors = Errors::default();
    if !attr.is_empty() {
        errors.new(TokenStream::from(attr).span(), "unexpected");
    }
    match ItemStruct::parse.parse(item.clone()) {
        Err(err) => errors.push(err),
        Ok(input) => match ENTITIES.get(&input.ident.to_string()) {
            None => errors.span(&input.ident, "unknown struct"),
            Some(truth) => {
                truth.check_selector("struct", &mut errors);
                truth.check_lifetime(&input.generics, &mut errors);
                let mut expected = ParsedTypeMap::new(truth, &mut errors);
                for field in &input.fields {
                    let Some(name) = &field.ident else {
                        errors.span(&field, "impossible unnamed struct field");
                        continue;
                    };
                    expected.remove(name, &field.ty, "field", &mut errors);
                }
                expected.check_exhaustive(&input.ident, &mut errors);
            }
        },
    }
    errors.and(item)
}

#[proc_macro_attribute]
pub fn check_variants(attr: CompilerStream, item: CompilerStream) -> CompilerStream {
    let mut errors = Errors::default();
    if !attr.is_empty() {
        errors.new(TokenStream::from(attr).span(), "unexpected");
    }
    match ItemEnum::parse.parse(item.clone()) {
        Err(err) => errors.push(err),
        Ok(input) => match ENTITIES.get(&input.ident.to_string()) {
            None => errors.span(&input.ident, "unknown enum"),
            Some(truth) => {
                truth.check_selector("enum", &mut errors);
                truth.check_lifetime(&input.generics, &mut errors);
                let mut expected = ParsedTypeMap::new(truth, &mut errors);
                for variant in &input.variants {
                    let name = &variant.ident;
                    match &variant.fields {
                        Fields::Unit => errors.span(&variant.ident, "expected unnamed fields"),
                        Fields::Named(_) => errors.span(&variant.fields, "expected unnamed fields"),
                        Fields::Unnamed(fields) => {
                            match fields.unnamed.first() {
                                None => errors.span(&fields, "expected one field"),
                                Some(first) => {
                                    expected.remove(name, &first.ty, "variant", &mut errors);
                                }
                            }
                            if fields.unnamed.len() > 1 {
                                errors.span(&fields, "expected single field")
                            }
                        }
                    }
                }
                expected.check_exhaustive(&input.ident, &mut errors);
            }
        },
    }
    errors.and(item)
}

#[allow(dead_code)]
fn reveal(cursor: Cursor) -> String {
    match cursor.token_tree() {
        None => "None".to_string(),
        Some((token, _)) => token.to_token_stream().to_string(),
    }
}

type Replace = HashMap<&'static str, String>;

struct Subst<'a> {
    literal: bool,
    marker: Span,
    ident: Ident,
    next: Cursor<'a>,
}
impl<'a> Subst<'a> {
    fn parse(input: Cursor<'a>) -> Option<Self> {
        if let Some((punct, one)) = input.punct() {
            let literal = '~' == punct.as_char();
            if literal || '$' == punct.as_char() {
                if let Some((ident, next)) = one.ident() {
                    let marker = input.span().join(one.span()).unwrap_or(input.span());
                    return Some(Subst {
                        literal,
                        marker,
                        ident,
                        next,
                    });
                }
            }
        }
        None
    }
    fn run(&self, replace: &Replace, output: &mut TokenStream, errors: &mut Errors) -> Cursor<'a> {
        let ident = self.ident.to_string();
        match replace.get(ident.as_str()) {
            Some(subst) => {
                // eprintln!("Subst: {ident} -> {subst}");
                output.extend(if self.literal {
                    LitStr::new(subst, self.marker).to_token_stream()
                } else {
                    parse_str::<Type>(subst).unwrap().to_token_stream()
                    //Type::new(subst, self.marker).to_token_stream()
                });
            }
            None => {
                errors.new(self.marker, "expected a placeholder");
                output.extend(self.ident.to_token_stream());
            }
        }
        self.next
    }
}

struct Paste<'a> {
    before: Ident,
    tilde: Span,
    after: Ident,
    next: Cursor<'a>,
}
impl<'a> Paste<'a> {
    fn parse(input: Cursor<'a>) -> Option<Self> {
        if let Some((before, one)) = input.ident() {
            if let Some((punct, two)) = one.punct() {
                if '$' == punct.as_char() {
                    if let Some((after, next)) = two.ident() {
                        let tilde = input.span().join(two.span()).unwrap_or(one.span());
                        return Some(Paste {
                            before,
                            tilde,
                            after,
                            next,
                        });
                    }
                }
            }
        }
        None
    }
    fn run(&self, replace: &Replace, output: &mut TokenStream, errors: &mut Errors) -> Cursor<'a> {
        let before = self.before.to_string();
        let after = self.after.to_string();
        let mut subbed = false;
        let mut pasted = String::new();
        for part in [&before, &after] {
            match replace.get(part.as_str()) {
                Some(subst) => {
                    pasted.push_str(subst);
                    subbed = true;
                }
                None => pasted.push_str(&part),
            }
        }
        if !subbed {
            errors.new(self.tilde, "expected a placeholder");
        }
        // eprintln!("Paste: {before}~{after} -> {pasted}");
        output.extend(Ident::new(&pasted, self.tilde).to_token_stream());
        self.next
    }
}

#[derive(Clone, Copy)]
struct Wrap {
    delim: Option<Delimiter>,
    span: Option<DelimSpan>,
}
impl Wrap {
    fn outermost() -> Self {
        Wrap {
            delim: Some(Delimiter::None),
            span: None,
        }
    }
    fn some(delim: Delimiter, span: DelimSpan) -> Self {
        Wrap {
            delim: Some(delim),
            span: Some(span),
        }
    }
    fn parse(&mut self, punct: &Punct) -> bool {
        match punct.as_char() {
            '^' => {
                self.delim = None;
                self.span = None;
                true
            }
            '%' => true,
            _ => false,
        }
    }
    fn run(&self, stream: TokenStream) -> TokenStream {
        match self.delim {
            None => stream,
            Some(delim) => {
                let mut wrapped = Group::new(delim, stream);
                if let Some(span) = self.span {
                    wrapped.set_span(span.join());
                }
                wrapped.to_token_stream()
            }
        }
    }
}

#[proc_macro]
pub fn template(input: CompilerStream) -> CompilerStream {
    let buffer = TokenBuffer::new(input);
    let mut errors = Errors::default();
    let output = transcribe(buffer.begin(), Wrap::outermost(), &mut errors);
    return errors.and(output.into());
}

fn transcribe(mut cursor: Cursor, wrap: Wrap, errors: &mut Errors) -> TokenStream {
    if let Some((punct, next)) = cursor.punct() {
        if '?' == punct.as_char() {
            return each(next, wrap, errors);
        }
    }
    let mut output = TokenStream::new();
    while !cursor.eof() {
        cursor = if let Some((group, delim, span, next)) = cursor.any_group() {
            output.extend(transcribe(group, Wrap::some(delim, span), errors));
            next
        } else if let Some((tree, next)) = cursor.token_tree() {
            output.extend(tree.to_token_stream());
            next
        } else {
            // should be unreachable: token_tree should be Some whenever !eof
            errors.new(cursor.span(), "unexpected cursor API behavior");
            break; // failing to advance would loop forever
        }
    }
    wrap.run(output)
}

fn each(mut cursor: Cursor, mut wrap: Wrap, errors: &mut Errors) -> TokenStream {
    let mut requested = HashMap::new();
    let mut explained = false;
    while !cursor.eof() {
        // eprintln!("each ({})", reveal(cursor));
        cursor = if let Some((ident, next)) = cursor.ident() {
            let name = ident.to_string();
            if let Some(each) = ENTITIES.get_entry(&name).map(Each::from_phf) {
                requested.insert(each.key, each);
            } else {
                let mut found = false;
                for each in ENTITIES.entries().map(Each::from_phf) {
                    if each.entity.category == name {
                        found = true;
                        requested.insert(each.key, each);
                    }
                }
                if !found {
                    errors.span(ident, "unknown");
                }
            }
            next
        } else if let Some((punct, next)) = cursor.punct() {
            if ',' != punct.as_char() {
                // commas are ignored, not required
                if wrap.parse(&punct) {
                    cursor = next;
                    break;
                } else {
                    errors.span(punct, "unhandled");
                }
            }
            next
        } else if let Some((_, next)) = cursor.token_tree() {
            errors.new(cursor.span(), "unhandled");
            if !explained {
                let mut choices = HashSet::new();
                for facts in ENTITIES.values() {
                    choices.insert(facts.category);
                }
                for key in ENTITIES.keys() {
                    choices.insert(key);
                }
                let message = format!("allowed before the `^` marker: {:?}", choices);
                errors.new(Span::call_site(), message);
                explained = true;
            }
            next
        } else {
            // should be unreachable: token_tree should be Some whenever !eof
            errors.new(cursor.span(), "unexpected cursor API behavior");
            break; // failing to advance would loop forever
        }
    }
    let mut output = TokenStream::new();
    for each in requested.values() {
        output.extend(each.one(cursor, wrap, errors));
    }
    output // individually wrapped, not here
}

struct Each {
    key: &'static str,
    entity: &'static Entity,
    replace: Replace,
}
impl Each {
    fn from_phf((&key, entity): (&&'static str, &'static Entity)) -> Self {
        let mut replace = Replace::new();
        replace.insert("Each", key.to_upper_camel_case());
        replace.insert("each", key.to_snake_case());
        Each {
            key,
            entity,
            replace,
        }
    }
    fn one(&self, mut cursor: Cursor, mut wrap: Wrap, errors: &mut Errors) -> TokenStream {
        let mut output = TokenStream::new();
        if let Some((punct, next)) = cursor.punct() {
            if wrap.parse(&punct) {
                for pair in self.entity.facets {
                    let mut facet = Facet::from_pair(self, pair);
                    output.extend(facet.one(next, wrap, errors));
                }
                return output; // individually wrapped, not here
            }
        }
        while !cursor.eof() {
            // eprintln!("Each::one ({})", reveal(input));
            cursor = if let Some(paste) = Paste::parse(cursor) {
                paste.run(&self.replace, &mut output, errors)
            } else if let Some(subst) = Subst::parse(cursor) {
                subst.run(&self.replace, &mut output, errors)
            } else if let Some((group, delim, span, next)) = cursor.any_group() {
                output.extend(self.one(group, Wrap::some(delim, span), errors));
                next
            } else if let Some((tree, next)) = cursor.token_tree() {
                output.extend(tree.to_token_stream());
                next
            } else {
                // should be unreachable: token_tree should be Some whenever !eof
                errors.new(cursor.span(), "unexpected cursor API behavior");
                break; // failing to advance would loop forever
            }
        }
        wrap.run(output)
    }
}

struct Facet<'a> {
    each: &'a Each,
    name: &'static str,
    link: &'static str,
    replace: Replace,
    used: usize,
}
impl<'a> Facet<'a> {
    fn from_pair(each: &'a Each, (name, link): &(&'static str, &'static str)) -> Self {
        let mut replace = each.replace.clone();
        replace.insert("name", name.to_snake_case());
        replace.insert("Name", name.to_upper_camel_case());
        replace.insert("link", link.to_snake_case());
        replace.insert("Link", link.to_upper_camel_case());
        Facet {
            each,
            name,
            link,
            replace,
            used: 0,
        }
    }
    fn one(&mut self, mut cursor: Cursor, mut wrap: Wrap, errors: &mut Errors) -> TokenStream {
        if let Some((first, one)) = cursor.punct() {
            if '?' == first.as_char() {
                if let Some((ident, two)) = one.ident() {
                    let pattern = ident.to_string();
                    if pattern == self.name
                        || pattern == self.link
                        || pattern == self.each.key
                        || (self.used == 0 && pattern == "else")
                    {
                        if let Some((after, next)) = two.punct() {
                            if wrap.parse(&after) {
                                if pattern != "else" {
                                    self.used += 1;
                                }
                                cursor = next;
                            } else {
                                errors.span(after, "expected a marker (`^` or `%`)");
                                return TokenStream::new();
                            }
                        } else {
                            errors.new(two.span(), "expected a marker (`^` or `%`)");
                            return TokenStream::new();
                        }
                    } else {
                        return TokenStream::new();
                    }
                } else {
                    errors.new(one.span(), "expected an identifier (to filter)");
                    return TokenStream::new();
                }
            }
        }
        let mut output = TokenStream::new();
        while !cursor.eof() {
            // eprintln!("Fact::one ({})", reveal(input));
            cursor = if let Some(paste) = Paste::parse(cursor) {
                paste.run(&self.replace, &mut output, errors)
            } else if let Some(subst) = Subst::parse(cursor) {
                subst.run(&self.replace, &mut output, errors)
            } else
            // if let Some((literal, next)) = input.literal() {
            //     output.extend(
            //         if let Ok(string) = parse2::<LitStr>(literal.to_token_stream()) {
            //             let sub = self.replaced(each, string.value());
            //             LitStr::new(&sub, literal.span()).to_token_stream()
            //         } else {
            //             literal.to_token_stream()
            //         },
            //     );
            //     input = next;
            //     continue;
            // }
            if let Some((group, delim, span, next)) = cursor.any_group() {
                output.extend(self.one(group, Wrap::some(delim, span), errors));
                next
            } else if let Some((tree, next)) = cursor.token_tree() {
                output.extend(tree.to_token_stream());
                next
            } else {
                // should be unreachable: token_tree should be Some whenever !eof
                errors.new(cursor.span(), "unexpected cursor API behavior");
                break; // failing to advance would loop forever
            }
        }
        wrap.run(output)
    }
}
