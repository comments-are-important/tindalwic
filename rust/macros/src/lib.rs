#![allow(missing_docs, unused)]

//! see the documentation in the `tindalwic` crate.

use proc_macro::TokenStream as RawStream;
use proc_macro2::{Span, TokenStream};
use quote::{ToTokens, quote};
use syn::parse::{Parse, ParseStream};
use syn::token::Star;
use syn::*;

#[proc_macro]
pub fn tindalwic_json(input: RawStream) -> RawStream {
    let parsed = parse_macro_input!(input as File);
    quote!(#parsed).into()
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
}

struct UTF8 {
    token: LitStr,
    range: Range,
}
impl Parse for UTF8 {
    fn parse(input: ParseStream) -> Result<Self> {
        let token: LitStr = input.parse()?;
        Ok(UTF8 { token, range: Range{start:0,end:0} })
    }
}

struct Indexed {
    index: usize,
    value: Value,
}
impl Parse for Indexed {
    fn parse(input: ParseStream) -> Result<Self> {
        let value: Value = input.parse()?;
        Ok(Indexed { index: 0, value })
    }
}
impl Indexed {
    fn parse_vec(input: ParseStream) -> Result<Vec<Indexed>> {
        let content;
        bracketed!(content in input);
        let items = content
            .parse_terminated(Indexed::parse, Token![,])?
            .into_iter()
            .collect();
        Ok(items)
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
                if list.is_empty() {
                    tokens.extend(quote!(.lv(#index,0..0)));
                } else {
                    let value = Range::new(list[0].index, 1 + list[list.len() - 1].index);
                    tokens.extend(quote!(.lv(#index,#value)#(#list)*));
                }
            }
            Value::Dict(dict) => {
                if dict.is_empty() {
                    tokens.extend(quote!(.dv(#index,0..0)));
                } else {
                    let value = Range::new(dict[0].index, 1 + dict[dict.len() - 1].index);
                    tokens.extend(quote!(.dv(#index,#value)#(#dict)*));
                }
            }
        }
    }
}

struct Keyed {
    index: usize,
    key: UTF8,
    value: Value,
}
impl Parse for Keyed {
    fn parse(input: ParseStream) -> Result<Self> {
        let key: UTF8 = input.parse()?;
        input.parse::<Token![:]>()?;
        let value: Value = input.parse()?;
        Ok(Keyed {
            index: 0,
            key,
            value,
        })
    }
}
impl Keyed {
    fn parse_vec(input: ParseStream) -> Result<Vec<Keyed>> {
        let content;
        braced!(content in input);
        let items = content
            .parse_terminated(Keyed::parse, Token![,])?
            .into_iter()
            .collect();
        Ok(items)
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
                if list.is_empty() {
                    tokens.extend(quote!(.lk(#index,#key,0..0)));
                } else {
                    let value = Range::new(list[0].index, 1 + list[list.len() - 1].index);
                    tokens.extend(quote!(.lk(#index,#key,#value)#(#list)*));
                }
            }
            Value::Dict(dict) => {
                if dict.is_empty() {
                    tokens.extend(quote!(.dk(#index,#key,0..0)));
                } else {
                    let value = Range::new(dict[0].index, 1 + dict[dict.len() - 1].index);
                    tokens.extend(quote!(.dk(#index,#key,#value)#(#dict)*));
                }
            }
        }
    }
}

enum Value {
    //Name(Ident), // can't do that with current arena
    Text(UTF8),
    List(Vec<Indexed>),
    Dict(Vec<Keyed>),
}
impl Parse for Value {
    fn parse(input: ParseStream) -> Result<Self> {
        if input.peek(LitStr) {
            Ok(Value::Text(UTF8::parse(input)?))
        } else if input.peek(token::Bracket) {
            Ok(Value::List(Indexed::parse_vec(input)?))
        } else if input.peek(token::Brace) {
            Ok(Value::Dict(Keyed::parse_vec(input)?))
        } else {
            Err(input.error("expected string literal, [...], or {...}"))
        }
    }
}

struct File {
    utf8: String,
    list: usize,
    dict: usize,
    name: Ident,
    vec: Vec<Keyed>,
}
impl Parse for File {
    fn parse(input: ParseStream) -> Result<Self> {
        let name: Ident = input.parse()?;
        input.parse::<Token![=]>()?;
        let mut dict = Keyed::parse_vec(input)?;
        let mut file = File {
            name,
            vec: Vec::new(),
            utf8: String::new(),
            list: 0,
            dict: 0,
        };
        file.dict(&mut dict);
        file.vec = dict;
        Ok(file)
    }
}
impl File {
    fn value(&mut self, value: &mut Value) {
        match value {
            Value::Text(text) => self.text(text),
            Value::List(list) => self.list(list),
            Value::Dict(dict) => self.dict(dict),
        }
    }
    fn dict(&mut self, dict: &mut Vec<Keyed>) {
        let start = self.dict;
        self.dict += dict.len();
        for (offset, keyed) in dict.iter_mut().enumerate() {
            keyed.index = start + offset;
            self.text(&mut keyed.key);
            self.value(&mut keyed.value);
        }
    }
    fn list(&mut self, list: &mut Vec<Indexed>) {
        let start = self.list;
        self.list += list.len();
        for (offset, indexed) in list.iter_mut().enumerate() {
            indexed.index = start + offset;
            self.value(&mut indexed.value);
        }
    }
    fn text(&mut self, text: &mut UTF8) {
        let start = self.utf8.len();
        self.utf8.push_str(&text.token.value());
        text.range = Range { start, end: self.utf8.len()};
    }
}
impl ToTokens for File {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let File {
            utf8,
            list,
            dict,
            name,
            vec,
        } = self;
        tokens.extend(quote! {
            let #name = ::tindalwic::Arena {
                utf8_bytes: #utf8,
                value_cells: ::core::array::from_fn::<_,#list,_>(::tindalwic::Value::blank),
                keyed_cells: ::core::array::from_fn::<_,#dict,_>(::tindalwic::Keyed::blank),
                file: ::core::cell::Cell::new(::tindalwic::File::new()),
            };
            #name #(#vec)* .f(0..#dict);
        });
    }
}
