#![deny(unused)]

//! The macros defined here are re-exported from and documented in
//! [the main tindalwic crate](https://docs.rs/tindalwic).

use proc_macro::TokenStream as RawStream;
use proc_macro2::{TokenStream, TokenTree};
use quote::{ToTokens, TokenStreamExt, quote};
use syn::parse::{Parse, ParseStream};
use syn::punctuated::Punctuated;
use syn::*;

// ====================================================================================

struct Branch {
    keyed: bool, // true means Dict, false means List
    expr: TokenStream,
}
impl ToTokens for Branch {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let expr = &self.expr;
        if self.keyed {
            tokens.extend(quote!(::tindalwic::Branch::Dict(#expr)));
        } else {
            tokens.extend(quote!(::tindalwic::Branch::List(#expr)));
        }
    }
}

struct Walk {
    root: TokenStream,
    path: Vec<Branch>,
    lands: Option<bool>, // None means Text
}
impl Parse for Walk {
    fn parse(input: ParseStream) -> Result<Self> {
        let root;
        let mut keyed;
        if input.peek(token::Bracket) {
            let content;
            bracketed!(content in input);
            root = TokenStream::parse(&content)?;
            if root.is_empty() {
                return Err(input.error("missing root inside []"));
            }
            keyed = false;
        } else if input.peek(token::Brace) {
            let content;
            braced!(content in input);
            root = TokenStream::parse(&content)?;
            if root.is_empty() {
                return Err(input.error("missing root inside {}"));
            }
            keyed = true;
        } else {
            return Err(input.error("must start with [root] or {root}"));
        }
        let mut path = Vec::new();
        while !input.is_empty() {
            let start = input.span();
            if input.peek(token::Bracket) {
                let content;
                bracketed!(content in input);
                let expr = TokenStream::parse(&content)?;
                if expr.is_empty() {
                    return Err(Error::new(start, "missing expr inside []"));
                }
                path.push(Branch { expr, keyed });
                keyed = false;
            } else if input.peek(token::Brace) {
                let content;
                braced!(content in input);
                let expr = TokenStream::parse(&content)?;
                if expr.is_empty() {
                    return Err(Error::new(start, "missing expr inside {}"));
                }
                path.push(Branch { expr, keyed });
                keyed = true;
            } else if input.peek(Token![<]) {
                input.parse::<Token![<]>()?;
                let mut expr = TokenStream::new();
                let mut depth: usize = if input.peek(Token![>]) { 0 } else { 1 };
                while depth != 0 {
                    expr.append(TokenTree::parse(input)?);
                    if input.peek(Token![<]) {
                        depth += 1;
                    } else if input.peek(Token![>]) {
                        depth -= 1;
                    }
                }
                input.parse::<Token![>]>()?;
                if expr.is_empty() {
                    return Err(Error::new(start, "missing expr inside <>"));
                }
                path.push(Branch { expr, keyed });
                let lands = None;
                return Ok(Walk { root, path, lands });
            } else {
                break;
            }
        }
        if path.is_empty() {
            return Err(input.error("requires at least one step"));
        }
        let lands = Some(keyed);
        Ok(Walk { root, path, lands })
    }
}

#[proc_macro]
pub fn walk(input: RawStream) -> RawStream {
    let Walk { root, path, lands } = parse_macro_input!(input as Walk);
    if path.last().unwrap().keyed {
        match lands {
            None => quote!((#root).to_value().text_keyed(&[#(#path),*])),
            Some(false) => quote!((#root).to_value().list_keyed(&[#(#path),*])),
            Some(true) => quote!((#root).to_value().dict_keyed(&[#(#path),*])),
        }
    } else {
        match lands {
            None => quote!((#root).to_value().text_value(&[#(#path),*])),
            Some(false) => quote!((#root).to_value().list_value(&[#(#path),*])),
            Some(true) => quote!((#root).to_value().dict_value(&[#(#path),*])),
        }
    }
    .into()
}

// ====================================================================================

struct Range {
    start: usize,
    end: usize,
}
impl ToTokens for Range {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let Range { start, end } = self;
        tokens.extend(quote!(#start..#end));
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
        Ok(UTF8 {
            token: input.parse()?,
            range: Range::new(0, 0),
        })
    }
}

struct Indexed {
    value: Value,
    index: usize,
}
impl Parse for Indexed {
    fn parse(input: ParseStream) -> Result<Self> {
        Ok(Indexed {
            value: input.parse()?,
            index: usize::MAX,
        })
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
        match &self.value {
            Value::Text(text) => {
                let range = &text.range;
                tokens.extend(quote!(.tv(#range)));
            }
            Value::List(list) => {
                let range = Range::list(list);
                tokens.extend(quote!(.lv(#range)));
            }
            Value::Dict(dict) => {
                let range = Range::dict(dict);
                tokens.extend(quote!(.dv(#range)));
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
        Ok(Keyed {
            key,
            value: input.parse()?,
            index: usize::MAX,
        })
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
        let key = &self.key.range;
        match &self.value {
            Value::Text(text) => {
                let range = &text.range;
                tokens.extend(quote!(.tk(#key,#range)));
            }
            Value::List(list) => {
                let range = Range::list(list);
                tokens.extend(quote!(.lk(#key,#range)));
            }
            Value::Dict(dict) => {
                let range = Range::dict(dict);
                tokens.extend(quote!(.dk(#key,#range)));
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

struct Root {
    name: Ident,
    root: Value,
}
impl Parse for Root {
    fn parse(input: ParseStream) -> Result<Self> {
        let name: Ident = input.parse()?;
        input.parse::<Token![=]>()?;
        Ok(Root {
            name,
            root: Value::parse(input)?,
        })
    }
}

struct Arena {
    utf8: String,
    list: usize,
    dict: usize,
    make: TokenStream,
}
impl Arena {
    fn new(value: &mut Value) -> Self {
        let mut arena = Arena {
            utf8: String::new(),
            list: 0,
            dict: 0,
            make: TokenStream::new(),
        };
        arena.value(value);
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
        for indexed in list.iter_mut() {
            self.value(&mut indexed.value);
        }
        for indexed in list.iter_mut() {
            indexed.index = self.list;
            self.list += 1;
            indexed.to_tokens(&mut self.make);
        }
    }
    fn dict(&mut self, dict: &mut Punctuated<Keyed, Token![,]>) {
        for keyed in dict.iter_mut() {
            self.text(&mut keyed.key);
            self.value(&mut keyed.value);
        }
        for keyed in dict.iter_mut() {
            keyed.index = self.dict;
            self.dict += 1;
            keyed.to_tokens(&mut self.make)
        }
    }
}
impl ToTokens for Arena {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let utf8 = &self.utf8;
        let list = &self.list;
        let dict = &self.dict;
        tokens.extend(quote! {
            ::tindalwic::Arena {
                utf8_bytes: #utf8,
                value_cells: &::tindalwic::Value::array::<{1+#list}>(),
                keyed_cells: &::tindalwic::Keyed::array::<#dict>(),
                value_next: 0,
                keyed_next: 0,
            }
        });
    }
}

#[proc_macro]
pub fn json(input: RawStream) -> RawStream {
    let Root { name, mut root } = parse_macro_input!(input as Root);
    let arena = Arena::new(&mut root);
    let make = &arena.make;
    match &root {
        Value::Text(root) => {
            let range = &root.range;
            quote!(let mut #name = #arena; #name #make .tv(#range);)
        }
        Value::List(root) => {
            let range = Range::list(root);
            quote!(let mut #name = #arena; #name #make .lv(#range);)
        }
        Value::Dict(root) => {
            let range = Range::dict(root);
            quote!(let mut #name = #arena; #name #make .dv(#range);)
        }
    }
    .into()
}
