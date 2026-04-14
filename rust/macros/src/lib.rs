#![deny(unused)]

//! The macros defined here are re-exported from and documented in
//! [the main tindalwic crate](https://docs.rs/tindalwic).

use proc_macro::TokenStream as RawStream;
use proc_macro2::{Span, TokenStream, TokenTree};
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
    name: Ident,
    root: TokenStream,
    path: Vec<Branch>,
    lands: Option<bool>, // None means
    after: TokenStream,
}
impl Parse for Walk {
    fn parse(input: ParseStream) -> Result<Self> {
        input.parse::<Token![let]>()?;
        let name = Ident::parse(input)?;
        input.parse::<Token![=]>()?;
        let root;
        let mut keyed;
        let mut start = input.span();
        if input.peek(token::Bracket) {
            let content;
            bracketed!(content in input);
            if content.is_empty() {
                return Err(Error::new(start, "missing root inside []"));
            }
            root = TokenStream::parse(&content)?;
            keyed = false;
        } else if input.peek(token::Brace) {
            let content;
            braced!(content in input);
            if content.is_empty() {
                return Err(Error::new(start, "missing root inside {}"));
            }
            root = TokenStream::parse(&content)?;
            keyed = true;
        } else {
            return Err(input.error("must start with [root] or {root}"));
        }
        let mut path = Vec::new();
        let mut text = false;
        while !input.is_empty() {
            start = input.span();
            if input.peek(token::Bracket) {
                let content;
                bracketed!(content in input);
                if content.is_empty() {
                    return Err(Error::new(start, "missing expr inside []"));
                }
                let expr = TokenStream::parse(&content)?;
                path.push(Branch { expr, keyed });
                keyed = false;
            } else if input.peek(token::Brace) {
                let content;
                braced!(content in input);
                if content.is_empty() {
                    return Err(Error::new(start, "missing expr inside {}"));
                }
                let expr = TokenStream::parse(&content)?;
                path.push(Branch { expr, keyed });
                keyed = true;
            } else if input.peek(Token![<]) {
                input.parse::<Token![<]>()?;
                let mut expr = TokenStream::new();
                let mut depth: usize = if input.peek(Token![>]) { 0 } else { 1 };
                while depth != 0 {
                    if input.is_empty() {
                        return Err(Error::new(start, "unbalanced <> brackets"));
                    }
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
                text = true;
                break;
            } else {
                break;
            }
        }
        if path.is_empty() {
            return Err(input.error("requires at least one step"));
        }
        let lands = if text { None } else { Some(keyed) };
        let mut after = TokenStream::new();
        while !input.is_empty() && !input.peek(Token![;]) {
            after.append(TokenTree::parse(input)?);
        }
        input.parse::<Token![;]>()?;
        Ok(Walk {
            name,
            root,
            path,
            lands,
            after,
        })
    }
}
impl ToTokens for Walk {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let name = &self.name;
        let array = Ident::new(&format!("__{name}_array"), Span::mixed_site());
        let walk = Ident::new(&format!("__{name}_path"), Span::mixed_site());
        let branches = &self.path;
        let mut method = String::from(match self.lands {
            None => "text",
            Some(false) => "list",
            Some(true) => "dict",
        });
        method.push('_');
        method.push_str(if branches.last().unwrap().keyed {
            "keyed"
        } else {
            "value"
        });
        let method = Ident::new(&method, Span::call_site());
        let root = &self.root;
        let after = &self.after;
        tokens.extend(quote! {
            let #array = [#(#branches),*];
            let #walk = ::tindalwic::Path { branches: &#array };
            #[allow(unused_mut)]
            let mut #name = #walk.#method((#root).to_value()) #after;
        })
    }
}

#[proc_macro]
pub fn walk(input: RawStream) -> RawStream {
    let walk = parse_macro_input!(input as Walk);
    quote!(#walk).into()
}

// ====================================================================================

struct Range {
    start: usize,
    end: usize,
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
impl ToTokens for Range {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let Range { start, end } = self;
        tokens.extend(quote!(#start..#end));
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
impl ToTokens for Indexed {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        match &self.value {
            Value::Text(text) => {
                tokens.extend(quote!(.tv(#text)));
            }
            Value::List(list) => {
                let range = Range::list(list);
                tokens.extend(quote!(.lv(#range)));
            }
            Value::Dict(dict) => {
                let range = Range::dict(dict);
                tokens.extend(quote!(.dv(#range)));
            }
            Value::Expr(expr) => {
                tokens.extend(quote!(.vv((#expr).to_value())));
            }
        }
    }
}

struct Keyed {
    key: TokenStream,
    value: Value,
    index: usize,
}
impl Parse for Keyed {
    fn parse(input: ParseStream) -> Result<Self> {
        let mut key = TokenStream::new();
        while !input.peek(Token![:]) {
            if input.is_empty() {
                return Err(input.error("missing expr for the key"));
            }
            key.append(TokenTree::parse(input)?);
        }
        if key.is_empty() {
            return Err(input.error("missing expr for the key"));
        }
        input.parse::<Token![:]>()?;
        Ok(Keyed {
            key,
            value: input.parse()?,
            index: usize::MAX,
        })
    }
}
impl ToTokens for Keyed {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let key = &self.key;
        match &self.value {
            Value::Text(text) => {
                tokens.extend(quote!(.tk(#key,#text)));
            }
            Value::List(list) => {
                let range = Range::list(list);
                tokens.extend(quote!(.lk(#key,#range)));
            }
            Value::Dict(dict) => {
                let range = Range::dict(dict);
                tokens.extend(quote!(.dk(#key,#range)));
            }
            Value::Expr(expr) => {
                tokens.extend(quote!(.vk(#key,(#expr).to_value())));
            }
        }
    }
}

enum Value {
    Text(TokenStream),
    List(Punctuated<Indexed, Token![,]>),
    Dict(Punctuated<Keyed, Token![,]>),
    Expr(TokenStream),
}
impl Parse for Value {
    fn parse(input: ParseStream) -> Result<Self> {
        if input.peek(token::Bracket) {
            let content;
            bracketed!(content in input);
            Ok(Value::List(
                content.parse_terminated(Indexed::parse, Token![,])?,
            ))
        } else if input.peek(token::Brace) {
            let content;
            braced!(content in input);
            Ok(Value::Dict(
                content.parse_terminated(Keyed::parse, Token![,])?,
            ))
        } else if input.peek(token::Paren) {
            let start = input.span();
            let content;
            parenthesized!(content in input);
            if content.is_empty() {
                return Err(Error::new(start, "missing expr inside ()"));
            }
            Ok(Value::Expr(content.parse()?))
        } else {
            let mut text = TokenStream::new();
            while !input.is_empty() && !input.peek(Token![,]) && !input.peek(Token![;]) {
                text.append(TokenTree::parse(input)?);
            }
            if text.is_empty() {
                return Err(input.error("missing expr for a value"));
            }
            Ok(Value::Text(text))
        }
    }
}

struct Organize {
    value_index: usize,
    keyed_index: usize,
    build: TokenStream,
}
impl Organize {
    fn new() -> Self {
        Organize {
            value_index: 0,
            keyed_index: 0,
            build: TokenStream::new(),
        }
    }
    fn value(&mut self, value: &mut Value) {
        match value {
            Value::List(items) => self.list(items),
            Value::Dict(items) => self.dict(items),
            _ => {}
        }
    }
    fn list(&mut self, items: &mut Punctuated<Indexed, Token![,]>) {
        for indexed in items.iter_mut() {
            self.value(&mut indexed.value);
        }
        for indexed in items.iter_mut() {
            indexed.index = self.value_index;
            self.value_index += 1;
            indexed.to_tokens(&mut self.build);
        }
    }
    fn dict(&mut self, items: &mut Punctuated<Keyed, Token![,]>) {
        for keyed in items.iter_mut() {
            self.value(&mut keyed.value);
        }
        for keyed in items.iter_mut() {
            keyed.index = self.keyed_index;
            self.keyed_index += 1;
            keyed.to_tokens(&mut self.build);
        }
    }
    fn root(&mut self, root: &mut Value) {
        self.value(root);
        self.value_index += 1; // so root fits in value_cells
        match root {
            Value::Text(root) => {
                self.build.extend(quote!(.tv(#root);));
            }
            Value::List(root) => {
                let range = Range::list(root);
                self.build.extend(quote!(.lv(#range);))
            }
            Value::Dict(root) => {
                let range = Range::dict(root);
                self.build.extend(quote!(.dv(#range);))
            }
            Value::Expr(expr) => {
                self.build.extend(quote!(.vv((#expr).to_value());));
            }
        }
    }
}

struct Arena {
    name: Ident,
    root: Value,
    make: Organize,
}
impl Parse for Arena {
    fn parse(input: ParseStream) -> Result<Self> {
        input.parse::<Token![let]>()?;
        let name = Ident::parse(input)?;
        input.parse::<Token![=]>()?;
        let root = Value::parse(input)?;
        input.parse::<Token![;]>()?;
        let make = Organize::new();
        let mut arena = Arena { name, root, make };
        arena.make.root(&mut arena.root);
        Ok(arena)
    }
}
impl ToTokens for Arena {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let name = &self.name;
        let value_size = self.make.value_index;
        let keyed_size = self.make.keyed_index;
        let build = &self.make.build;
        tokens.extend(quote! {
            let mut #name = ::tindalwic::Arena {
                value_cells: &::tindalwic::Value::array::<#value_size>(),
                keyed_cells: &::tindalwic::Keyed::array::<#keyed_size>(),
                value_next: 0,
                keyed_next: 0,
            }; #name #build
        });
    }
}

#[proc_macro]
pub fn json(input: RawStream) -> RawStream {
    let input =
        parse_macro_input!(input with Punctuated::<Arena, syn::parse::Nothing>::parse_terminated);
    quote!(#input).into()
}
