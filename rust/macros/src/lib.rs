#![allow(missing_docs, unused)]

//! see the documentation in the `tindalwic` crate.

use std::fmt::{Display, Formatter, Result as FmtResult};
use std::iter::once;

use proc_macro::TokenStream as RawStream;
use proc_macro2::{Span, TokenStream, TokenTree};
use quote::{ToTokens, TokenStreamExt, quote};
use syn::parse::{Parse, ParseStream};
use syn::punctuated::Punctuated;
use syn::*;

// ====================================================================================

#[derive(Clone, Copy, PartialEq)]
enum Kind {
    List,
    Dict,
}
struct Step {
    kind: Kind,
    tokens: TokenStream,
}
impl ToTokens for Step {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let it = &self.tokens;
        match self.kind {
            Kind::List => tokens.extend(quote! { ::tindalwic::Branch::List(#it) }),
            Kind::Dict => tokens.extend(quote! { ::tindalwic::Branch::Dict(#it) }),
        };
    }
}

struct Walk {
    file: TokenStream,
    path: Vec<Step>,
    lands: Option<Kind>, // None means Text
}
impl Parse for Walk {
    fn parse(input: ParseStream) -> Result<Self> {
        let mut file = TokenStream::new();
        while !input.is_empty() && !input.peek(Token![,]) {
            file.append(TokenTree::parse(input)?);
        }
        input.parse::<Token![,]>()?;
        let mut path: Vec<Step> = Vec::new();
        let mut kind = Kind::Dict;
        while !input.is_empty() {
            let start = input.span();
            if input.peek(token::Bracket) {
                let content;
                bracketed!(content in input);
                let tokens = TokenStream::parse(&content)?;
                if tokens.is_empty() {
                    return Err(Error::new(start, "missing expr inside []"));
                }
                path.push(Step { tokens, kind });
                kind = Kind::List;
            } else if input.peek(token::Brace) {
                let content;
                braced!(content in input);
                let tokens = TokenStream::parse(&content)?;
                if tokens.is_empty() {
                    return Err(Error::new(start, "missing expr inside {}"));
                }
                path.push(Step { tokens, kind });
                kind = Kind::Dict;
            } else if input.peek(Token![<]) {
                input.parse::<Token![<]>()?;
                let mut tokens = TokenStream::new();
                let mut depth: usize = if input.peek(Token![>]) { 0 } else { 1 };
                while depth != 0 {
                    tokens.append(TokenTree::parse(input)?);
                    if input.peek(Token![<]) {
                        depth += 1;
                    } else if input.peek(Token![>]) {
                        depth -= 1;
                    }
                }
                input.parse::<Token![>]>()?;
                if tokens.is_empty() {
                    return Err(Error::new(start, "missing expr inside <>"));
                }
                path.push(Step { tokens, kind });
                return Ok(Walk {
                    file,
                    path,
                    lands: None,
                });
            } else {
                break;
            }
        }
        if path.is_empty() {
        return Err(input.error("requires at least one step"));
    }
        Ok(Walk {
            file,
            path,
            lands: Some(kind),
        })
    }
}

#[proc_macro]
pub fn walk(input: RawStream) -> RawStream {
    let Walk { file, path, lands } = parse_macro_input!(input as Walk);
    match path.last().unwrap().kind {
        Kind::List =>
            match lands {
                None =>
                    quote! { ::tindalwic::Branch::text_value(&[#(#path),*], &#file)},
                Some(Kind::List) =>
                    quote! { ::tindalwic::Branch::list_value(&[#(#path),*], &#file)},
                Some(Kind::Dict) =>
                    quote! { ::tindalwic::Branch::dict_value(&[#(#path),*], &#file)},
            },
        Kind::Dict =>
            match lands {
                None =>
                    quote! { ::tindalwic::Branch::text_keyed(&[#(#path),*], &#file)},
                Some(Kind::List) =>
                    quote! { ::tindalwic::Branch::list_keyed(&[#(#path),*], &#file)},
                Some(Kind::Dict) =>
                    quote! { ::tindalwic::Branch::dict_keyed(&[#(#path),*], &#file)},
            },
    }.into()
}

// ====================================================================================

#[proc_macro]
pub fn json(input: RawStream) -> RawStream {
    let File { name, mut file } = parse_macro_input!(input as File);
    let Arena { utf8, list, keys } = Arena::new(&mut file);
    let size = file.len();
    let iter = file.iter();
    quote! {
        let mut #name = ::tindalwic::Arena {
            utf8_bytes: #utf8,
            value_cells: ::core::array::from_fn::<_,#list,_>(::tindalwic::Value::blank),
            keyed_cells: ::core::array::from_fn::<_,#keys,_>(::tindalwic::Keyed::blank),
            file: ::core::cell::Cell::new(::tindalwic::File::new()),
        };
        #name #(#iter)* .f(0..#size);
    }
    .into()
}

struct Range {
    start: usize,
    end: usize,
}
impl ToTokens for Range {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let Range { start, end } = self;
        tokens.extend(quote! {#start..#end});
    }
}
impl Range {
    fn new(start: usize, end: usize) -> Self {
        Range { start, end }
    }
    fn list(list: &Punctuated<Indexed, Token![,]>) -> Self {
        if list.is_empty() {
            Range::new(0, 0)
        } else {
            Range::new(list[0].index, 1 + list[list.len() - 1].index)
        }
    }
    fn dict(dict: &Punctuated<Keyed, Token![,]>) -> Self {
        if dict.is_empty() {
            Range::new(0, 0)
        } else {
            Range::new(dict[0].index, 1 + dict[dict.len() - 1].index)
        }
    }
}

struct UTF8 {
    token: LitStr,
    range: Range,
}
impl Parse for UTF8 {
    fn parse(input: ParseStream) -> Result<Self> {
        let token: LitStr = input.parse()?;
        let range = Range::new(0, 0);
        Ok(UTF8 { token, range })
    }
}

struct Indexed {
    value: Value,
    index: usize,
}
impl Parse for Indexed {
    fn parse(input: ParseStream) -> Result<Self> {
        let value: Value = input.parse()?;
        Ok(Indexed { value, index: 0 })
    }
}
impl Indexed {
    fn parse_list(input: ParseStream) -> Result<Punctuated<Indexed, Token![,]>> {
        let content;
        bracketed!(content in input);
        Ok(content.parse_terminated(Indexed::parse, Token![,])?)
    }
}
impl ToTokens for Indexed {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let Indexed { index, value } = self;
        match &self.value {
            Value::Text(text) => {
                let value = &text.range;
                tokens.extend(quote! {.tv(#index,#value)});
            }
            Value::List(list) => {
                let mut value = Range::list(list);
                let mut iter = list.iter();
                tokens.extend(quote!(.lv(#index,#value)#(#iter)*));
            }
            Value::Dict(dict) => {
                let mut value = Range::dict(dict);
                let mut iter = dict.iter();
                tokens.extend(quote!(.dv(#index,#value)#(#iter)*));
            }
        }
    }
}

struct Keyed {
    key: UTF8,
    value: Value,
    index: usize,
}
impl Parse for Keyed {
    fn parse(input: ParseStream) -> Result<Self> {
        let key: UTF8 = input.parse()?;
        input.parse::<Token![:]>()?;
        let value: Value = input.parse()?;
        let index = 0usize;
        Ok(Keyed { key, value, index })
    }
}
impl Keyed {
    fn parse_dict(input: ParseStream) -> Result<Punctuated<Keyed, Token![,]>> {
        let content;
        braced!(content in input);
        Ok(content.parse_terminated(Keyed::parse, Token![,])?)
    }
}
impl ToTokens for Keyed {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let Keyed { index, key, value } = self;
        let key = &key.range;
        match &self.value {
            Value::Text(text) => {
                let value = &text.range;
                tokens.extend(quote! {.tk(#index,#key,#value)});
            }
            Value::List(list) => {
                let mut value = Range::list(list);
                let mut iter = list.iter();
                tokens.extend(quote!(.lk(#index,#key,#value)#(#iter)*));
            }
            Value::Dict(dict) => {
                let mut value = Range::dict(dict);
                let mut iter = dict.iter();
                tokens.extend(quote!(.dk(#index,#key,#value)#(#iter)*));
            }
        }
    }
}

enum Value {
    //Name(Ident), // can't do that with current arena
    Text(UTF8),
    List(Punctuated<Indexed, Token![,]>),
    Dict(Punctuated<Keyed, Token![,]>),
}
impl Parse for Value {
    fn parse(input: ParseStream) -> Result<Self> {
        if input.peek(LitStr) {
            Ok(Value::Text(UTF8::parse(input)?))
        } else if input.peek(token::Bracket) {
            Ok(Value::List(Indexed::parse_list(input)?))
        } else if input.peek(token::Brace) {
            Ok(Value::Dict(Keyed::parse_dict(input)?))
        } else {
            Err(input.error("expected string literal, [...], or {...}"))
        }
    }
}

struct File {
    name: Ident,
    file: Punctuated<Keyed, Token![,]>,
}
impl Parse for File {
    fn parse(input: ParseStream) -> Result<Self> {
        let name: Ident = input.parse()?;
        input.parse::<Token![=]>()?;
        let file = Keyed::parse_dict(input)?;
        Ok(File { name, file })
    }
}

struct Arena {
    utf8: String,
    list: usize,
    keys: usize,
}
impl Arena {
    fn new(dict: &mut Punctuated<Keyed, Token![,]>) -> Self {
        let mut arena = Arena {
            utf8: String::new(),
            list: 0,
            keys: 0,
        };
        arena.dict(dict);
        arena
    }
    fn value(&mut self, value: &mut Value) {
        match value {
            Value::Text(text) => self.text(text),
            Value::List(list) => self.list(list),
            Value::Dict(dict) => self.dict(dict),
        }
    }
    fn text(&mut self, text: &mut UTF8) {
        text.range.start = self.utf8.len();
        self.utf8.push_str(&text.token.value());
        text.range.end = self.utf8.len();
    }
    fn list(&mut self, list: &mut Punctuated<Indexed, Token![,]>) {
        let start = self.list;
        self.list += list.len();
        for (offset, indexed) in list.iter_mut().enumerate() {
            indexed.index = start + offset;
            self.value(&mut indexed.value);
        }
    }
    fn dict(&mut self, dict: &mut Punctuated<Keyed, Token![,]>) {
        let start = self.keys;
        self.keys += dict.len();
        for (offset, keyed) in dict.iter_mut().enumerate() {
            keyed.index = start + offset;
            self.text(&mut keyed.key);
            self.value(&mut keyed.value);
        }
    }
}
