#![deny(unused)]

//! The macros defined here are re-exported from and documented in
//! [the main tindalwic crate](https://docs.rs/tindalwic).

use proc_macro::TokenStream as RawStream;
use proc_macro2::{Span, TokenStream, TokenTree};
use quote::{ToTokens, TokenStreamExt, quote};
use syn::parse::{Parse, ParseStream};
use syn::punctuated::Punctuated;
use syn::{
    Error, Ident, Result, Token, braced, bracketed, parenthesized, parse_macro_input, token,
};

// ====================================================================================

struct Branch {
    keyed: bool, // true means `Dict`, false means `List`.
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
    mutable: Option<Token![mut]>,
    name: Ident,
    root: TokenStream,
    path: Vec<Branch>,
    lands: Option<bool>, // `None` means `Text`, `Some` is per `Branch::keyed`.
    after: TokenStream,
}
impl Parse for Walk {
    fn parse(input: ParseStream) -> Result<Self> {
        input.parse::<Token![let]>()?;
        let mutable: Option<Token![mut]> = input.parse()?;
        let name = input.parse()?;
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
            root = content.parse()?;
            keyed = false;
        } else if input.peek(token::Brace) {
            let content;
            braced!(content in input);
            if content.is_empty() {
                return Err(Error::new(start, "missing root inside {}"));
            }
            root = content.parse()?;
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
                let expr = content.parse()?;
                path.push(Branch { expr, keyed });
                keyed = false;
            } else if input.peek(token::Brace) {
                let content;
                braced!(content in input);
                if content.is_empty() {
                    return Err(Error::new(start, "missing expr inside {}"));
                }
                let expr = content.parse()?;
                path.push(Branch { expr, keyed });
                keyed = true;
            } else if input.peek(Token![<]) {
                input.parse::<Token![<]>()?;
                let mut expr = TokenStream::new();
                if !input.peek(Token![>]) {
                    let mut depth = 1usize;
                    while depth != 0 {
                        if input.is_empty() {
                            return Err(Error::new(start, "unbalanced <> brackets"));
                        }
                        expr.append(input.parse::<TokenTree>()?);
                        if input.peek(Token![<]) {
                            depth += 1;
                        } else if input.peek(Token![>]) {
                            depth -= 1;
                        }
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
            after.append(input.parse::<TokenTree>()?);
        }
        input.parse::<Token![;]>()?;
        Ok(Walk {
            mutable,
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
        let mutable = &self.mutable;
        let name = &self.name;
        let array = Ident::new(&format!("__{name}_array"), Span::mixed_site());
        let path = Ident::new(&format!("__{name}_path"), Span::mixed_site());
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
            let #path = ::tindalwic::Path { branches: &#array };
            let #mutable #name = #path.#method((#root).to_value()) #after;
        })
    }
}

#[proc_macro]
pub fn walk(input: RawStream) -> RawStream {
    let parse = Punctuated::<Walk, syn::parse::Nothing>::parse_terminated;
    let output = parse_macro_input!(input with parse);
    quote!(#output).into()
}

struct Save {
    ident: Ident,
    value: Option<TokenStream>,
}
impl Parse for Save {
    fn parse(input: ParseStream) -> Result<Self> {
        Ok(Save {
            ident: input.parse()?,
            value: if input.is_empty() {
                None
            } else {
                input.parse::<Token![,]>()?;
                Some(input.parse()?)
            },
        })
    }
}
impl ToTokens for Save {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let Save{ident,value} = self;
        match value {
            None => {
                tokens.extend(quote!{
                    #ident.__set(None);
                    drop(#ident)
                });
            },
            Some(expr) => {
                tokens.extend(quote!{
                    #ident.__set(Some(#expr));
                    drop(#ident)
                });
            },
        }
    }
}
#[proc_macro]
pub fn set(input: RawStream) -> RawStream {
    let output = parse_macro_input!(input as Save);
    quote!(#output).into()
}

// ====================================================================================

struct Range {
    start: usize,
    end: usize,
}
impl Range {
    fn list(list: &Punctuated<Indexed, Token![,]>) -> Self {
        Range {
            start: list.first().map_or(0, |it| it.index),
            end: list.last().map_or(0, |it| 1 + it.index),
        }
    }
    fn dict(dict: &Punctuated<Keyed, Token![,]>) -> Self {
        Range {
            start: dict.first().map_or(0, |it| it.index),
            end: dict.last().map_or(0, |it| 1 + it.index),
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
                tokens.extend(quote!(text_in_list(#text)));
            }
            Value::List(list) => {
                let range = Range::list(list);
                tokens.extend(quote!(list_in_list(#range)));
            }
            Value::Dict(dict) => {
                let range = Range::dict(dict);
                tokens.extend(quote!(dict_in_list(#range)));
            }
            Value::Expr(expr) => {
                tokens.extend(quote!(value_in_list((#expr).to_value())));
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
            key.append(input.parse::<TokenTree>()?);
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
                tokens.extend(quote!(text_in_dict(#key,#text)));
            }
            Value::List(list) => {
                let range = Range::list(list);
                tokens.extend(quote!(list_in_dict(#key,#range)));
            }
            Value::Dict(dict) => {
                let range = Range::dict(dict);
                tokens.extend(quote!(dict_in_dict(#key,#range)));
            }
            Value::Expr(expr) => {
                tokens.extend(quote!(value_in_dict(#key,(#expr).to_value())));
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
                text.append(input.parse::<TokenTree>()?);
            }
            if text.is_empty() {
                return Err(input.error("missing expr for a value"));
            }
            Ok(Value::Text(text))
        }
    }
}

/// a tree of nested `Value`s needs to be organized into a sequence of calls to the
/// methods on Arena that build the indicated structure. the children of a parent node
/// must be adjacent within the Arena arrays, which means the method calls building the
/// children must be made in order with no other calls between.
struct Organize {
    name: Ident,
    value_index: usize,
    keyed_index: usize,
    build: TokenStream,
}
impl Organize {
    fn new(name: &Ident) -> Self {
        Organize {
            name: Ident::new(&format!("__{name}_mutable"), Span::mixed_site()),
            value_index: 0,
            keyed_index: 0,
            build: TokenStream::new(),
        }
    }
    /// assign indexes to and tokenize all children (if any) of the provided `value`.
    /// can't organize `value` itself at this recursion level because its index is
    /// impossible to know (it will eventually be organized by its container).
    fn value(&mut self, value: &mut Value) {
        match value {
            Value::List(items) => self.list(items),
            Value::Dict(items) => self.dict(items),
            _ => {}
        }
    }
    fn list(&mut self, children: &mut Punctuated<Indexed, Token![,]>) {
        for indexed in children.iter_mut() {
            self.value(&mut indexed.value); // recursively organize grandchildren.
        }
        // second loop does not recurse, it just keeps children adjacent.
        let name = &self.name;
        for indexed in children.iter_mut() {
            indexed.index = self.value_index;
            self.value_index += 1;
            self.build.extend(quote!(#name.#indexed;));
        }
    }
    fn dict(&mut self, children: &mut Punctuated<Keyed, Token![,]>) {
        for keyed in children.iter_mut() {
            self.value(&mut keyed.value); // recursively organize grandchildren.
        }
        // second loop does not recurse, it just keeps children adjacent.
        let name = &self.name;
        for keyed in children.iter_mut() {
            keyed.index = self.keyed_index;
            self.keyed_index += 1;
            self.build.extend(quote!(#name.#keyed;));
        }
    }
    fn root(&mut self, root: &mut Value) {
        self.value(root); // organizes children, but not `root` itself.
        self.value_index += 1; // `root` needs a place within `value_cells`.
        let name = &self.name;
        match root {
            Value::Text(root) => {
                self.build.extend(quote!(#name.text_in_list(#root);));
            }
            Value::List(root) => {
                let range = Range::list(root);
                self.build.extend(quote!(#name.list_in_list(#range);))
            }
            Value::Dict(root) => {
                let range = Range::dict(root);
                self.build.extend(quote!(#name.dict_in_list(#range);))
            }
            Value::Expr(expr) => {
                self.build
                    .extend(quote!(#name.value_in_list((#expr).to_value());));
            }
        }
    }
}

struct JSON {
    name: Ident,
    root: Value,
    make: Organize,
    after: TokenStream,
}
impl Parse for JSON {
    fn parse(input: ParseStream) -> Result<Self> {
        input.parse::<Token![let]>()?;
        let name = input.parse()?;
        input.parse::<Token![=]>()?;
        let root = input.parse()?;
        let mut after = TokenStream::new();
        while !input.is_empty() && !input.peek(Token![;]) {
            after.append(input.parse::<TokenTree>()?);
        }
        input.parse::<Token![;]>()?;
        let make = Organize::new(&name);
        let mut arena = JSON {
            name,
            root,
            make,
            after,
        };
        arena.make.root(&mut arena.root);
        Ok(arena)
    }
}
impl ToTokens for JSON {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let name = &self.name;
        let arena = &self.make.name;
        let value_array = Ident::new(&format!("__{name}_value"), Span::mixed_site());
        let keyed_array = Ident::new(&format!("__{name}_keyed"), Span::mixed_site());
        let value_size = self.make.value_index;
        let keyed_size = self.make.keyed_index;
        let build = &self.make.build;
        let unpack = match &self.root {
            Value::Text(_) => quote!(.text().unwrap()),
            Value::List(_) => quote!(.list().unwrap()),
            Value::Dict(_) => quote!(.dict().unwrap()),
            Value::Expr(_) => quote!(.value()),
        };
        let after = &self.after;
        tokens.extend(quote! {
            let #value_array = ::tindalwic::Value::array::<#value_size>();
            let #keyed_array = ::tindalwic::Keyed::array::<#keyed_size>();
            let mut #arena = ::tindalwic::Arena::new(&#value_array, &#keyed_array);
            #build
            let #name = #arena #unpack #after;
        });
    }
}

#[proc_macro]
pub fn json(input: RawStream) -> RawStream {
    let parse = Punctuated::<JSON, syn::parse::Nothing>::parse_terminated;
    let output = parse_macro_input!(input with parse);
    quote!(#output).into()
}
