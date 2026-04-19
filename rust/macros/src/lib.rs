#![allow(missing_docs)]

//! The macros defined here are re-exported from and documented in
//! [the main tindalwic crate](https://docs.rs/tindalwic).
//! You could depend on and import from this crate,
//! but the simpler `use tindalwic` is suggested.

use proc_macro::TokenStream as RawStream;
use proc_macro2::{Span, TokenStream, TokenTree};
use quote::{ToTokens, TokenStreamExt, quote};
use syn::parse::{Parse, ParseStream};
use syn::punctuated::Punctuated;
use syn::{
    Error, Ident, Result, Token, braced, bracketed, parenthesized, parse_macro_input, token,
};

struct Variable {
    mutable: Option<Token![mut]>,
    ident: Ident,
}
impl Parse for Variable {
    fn parse(input: ParseStream) -> Result<Self> {
        Ok(Variable {
            mutable: input.parse()?,
            ident: input.parse()?,
        })
    }
}
impl Variable {
    fn new(name: &str) -> Self {
        Variable {
            mutable: None,
            ident: Ident::new(name, Span::call_site()),
        }
    }
    fn hidden(name: &str) -> Self {
        Variable {
            mutable: None,
            ident: Ident::new(name, Span::mixed_site()),
        }
    }
    fn derive(&self, suffix: &'static str) -> Self {
        let name = self.ident.to_string();
        Variable::hidden(&format!("__{name}_{suffix}"))
    }
    fn clash(within: &Punctuated<Variable, Token![,]>) -> Option<&Variable> {
        for earlier in 0..within.len() - 1 {
            if let Some(already) = within.get(earlier) {
                let already = already.ident.to_string();
                for later in earlier + 1..within.len() {
                    if let Some(later) = within.get(later) {
                        if later.ident.to_string() == already {
                            return Some(later);
                        }
                    }
                }
            }
        }
        None
    }
}
impl ToTokens for Variable {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let Variable { mutable, ident } = self;
        tokens.extend(quote!(#mutable #ident));
    }
}

// ====================================================================================

struct Branch {
    list: bool,        // true means `List`, false means `Dict`.
    expr: TokenStream, // unparsed (hopefully produces either `usize` or `Key`)
}
impl ToTokens for Branch {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let kind = Variable::new(if self.list { "List" } else { "Dict" }).ident;
        let expr = &self.expr;
        tokens.extend(quote!(::tindalwic::internals::Branch::#kind(#expr)));
    }
}

#[allow(unused)]
struct Walk {
    origin: TokenStream,    // where the walk begins - unparsed (Item or File)
    steps: Vec<Branch>,     // the decisions that form a Path
    cell: Variable,         // binding for the Item/Entry Cell the walk ends on
    name: Option<Variable>, // if ends at Entry Cell, then the Name gets a binding
    lands: Option<bool>,    // `None` means `Text`, `Some` -> `Branch::list`
    result: Variable,       // binding for the Item enum payload
    unwrap: TokenStream,    // the caller must specify `?` (or similar) for Path Result
}
impl Parse for Walk {
    fn parse(input: ParseStream) -> Result<Self> {
        input.parse::<Token![let]>()?;
        let mut binds: Punctuated<Variable, Token![,]>;
        if input.peek(token::Paren) {
            let content;
            let delims = parenthesized!(content in input).span;
            binds = content.parse_terminated(Variable::parse, Token![,])?;
            if binds.is_empty() {
                return Err(Error::new(delims.join(), "missing binding inside ()"));
            } else if binds.len() == 1 {
                return Err(Error::new(delims.join(), "remove unnecessary parens"));
            } else if let Some(clash) = Variable::clash(&binds) {
                return Err(Error::new(clash.ident.span(), "duplicate binding"));
            }
        } else {
            binds = Punctuated::new();
            binds.push(input.parse()?);
        }
        input.parse::<Token![=]>()?;
        let mut list = false;
        let origin;
        if input.peek(token::Bracket) {
            list = true;
            let content;
            let delims = bracketed!(content in input).span;
            if content.is_empty() {
                return Err(Error::new(delims.join(), "missing origin inside []"));
            }
            origin = content.parse()?;
        } else if input.peek(token::Brace) {
            let content;
            let delims = braced!(content in input).span;
            if content.is_empty() {
                return Err(Error::new(delims.join(), "missing origin inside {}"));
            }
            origin = content.parse()?;
        } else {
            return Err(input.error("must start with [origin] or {origin}"));
        }
        let mut text = false;
        let mut steps = Vec::new();
        while !input.is_empty() {
            if input.peek(token::Bracket) {
                let content;
                let delims = bracketed!(content in input).span;
                if content.is_empty() {
                    return Err(Error::new(delims.join(), "missing expr inside []"));
                }
                let expr = content.parse()?;
                steps.push(Branch { expr, list });
                list = true;
            } else if input.peek(token::Brace) {
                let content;
                let delims = braced!(content in input).span;
                if content.is_empty() {
                    return Err(Error::new(delims.join(), "missing expr inside {}"));
                }
                let expr = content.parse()?;
                steps.push(Branch { expr, list });
                list = false;
            } else if input.peek(Token![<]) {
                let open = input.parse::<Token![<]>()?;
                let mut expr = TokenStream::new();
                if !input.peek(Token![>]) {
                    let mut depth = 1usize;
                    while depth != 0 {
                        if input.is_empty() {
                            let span = open.span.join(input.span()).unwrap_or(open.span);
                            return Err(Error::new(span, "unbalanced <> brackets"));
                        }
                        expr.append(input.parse::<TokenTree>()?);
                        if input.peek(Token![<]) {
                            depth += 1;
                        } else if input.peek(Token![>]) {
                            depth -= 1;
                        }
                    }
                }
                let close = input.parse::<Token![>]>()?;
                if expr.is_empty() {
                    let span = open.span.join(close.span).unwrap_or(open.span);
                    return Err(Error::new(span, "missing expr inside <>"));
                }
                steps.push(Branch { expr, list });
                text = true;
                break;
            } else {
                break;
            }
        }
        if steps.is_empty() {
            return Err(input.error("requires at least one step"));
        }
        let mut unwrap = TokenStream::new();
        while !input.is_empty() && !input.peek(Token![;]) {
            unwrap.append(input.parse::<TokenTree>()?);
        }
        if unwrap.is_empty() {
            return Err(input.error("need `?` (or similar) to unwrap Result"));
        }
        input.parse::<Token![;]>()?;
        let mut variables = binds.into_iter();
        let Some(result) = variables.next() else {
            panic!("logic error: zero variables");
        };
        let name = if list {
            None
        } else {
            variables.next().or_else(|| Some(result.derive("name")))
        };
        let cell = variables.next().unwrap_or_else(|| result.derive("cell"));
        let walk = Walk {
            origin,
            steps,
            cell,
            unwrap,
            name,
            lands: if text { None } else { Some(list) },
            result,
        };
        if let Some(excess) = variables.next() {
            return Err(Error::new(excess.ident.span(), "too many bindings"));
        }
        Ok(walk)
    }
}
impl ToTokens for Walk {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let result = &self.result;
        let branches = result.derive("branches");
        let steps = &self.steps;
        let path = result.derive("path");
        let cell = &self.cell;
        let method = Variable::new(match self.name {
            None => "item_cell",
            Some(_) => "entry_cell",
        });
        let origin = &self.origin;
        let unwrap = &self.unwrap;
        tokens.extend(quote! {
            let #branches = [#(#steps),*];
            let #path = ::tindalwic::internals::Path::wrap(&#branches);
            let #cell = #path.#method((#origin).into())#unwrap;
        });
        let item = result.derive("item");
        if let Some(name) = &self.name {
            tokens.extend(quote! {
                let Entry { name: #name, item: #item } = #cell.get();
            });
        } else {
            tokens.extend(quote! {
                let #item = #cell.get();
            });
        }
        let method = Variable::new(match self.lands {
            None => "text",
            Some(true) => "list",
            Some(false) => "dict",
        });
        tokens.extend(quote! {
            let #result = #path.#method(&#item)#unwrap;
        });
    }
}

#[proc_macro]
pub fn walk(input: RawStream) -> RawStream {
    let parse = Punctuated::<Walk, syn::parse::Nothing>::parse_terminated;
    let output = parse_macro_input!(input with parse);
    quote!(#output).into()
}

// ====================================================================================

struct Range {
    start: usize,
    end: usize,
}
impl Range {
    fn list(list: &Punctuated<Item, Token![,]>) -> Self {
        Range {
            start: list.first().map_or(0, |it| it.index),
            end: list.last().map_or(0, |it| 1 + it.index),
        }
    }
    fn dict(dict: &Punctuated<Entry, Token![,]>) -> Self {
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

struct Item {
    value: Node,
    index: usize,
}
impl Parse for Item {
    fn parse(input: ParseStream) -> Result<Self> {
        Ok(Item {
            value: input.parse()?,
            index: usize::MAX,
        })
    }
}
impl ToTokens for Item {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        match &self.value {
            Node::Text(text) => {
                tokens.extend(quote!(text_item(#text)));
            }
            Node::List(list) => {
                let range = Range::list(list);
                tokens.extend(quote!(list_item(#range)));
            }
            Node::Dict(dict) => {
                let range = Range::dict(dict);
                tokens.extend(quote!(dict_item(#range)));
            }
            Node::Expr(expr) => {
                tokens.extend(quote!(item(#expr)));
            }
        }
    }
}

struct Entry {
    key: TokenStream,
    value: Node,
    index: usize,
}
impl Parse for Entry {
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
        Ok(Entry {
            key,
            value: input.parse()?,
            index: usize::MAX,
        })
    }
}
impl ToTokens for Entry {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let key = &self.key;
        match &self.value {
            Node::Text(text) => {
                tokens.extend(quote!(text_entry(#key,#text)));
            }
            Node::List(list) => {
                let range = Range::list(list);
                tokens.extend(quote!(list_entry(#key,#range)));
            }
            Node::Dict(dict) => {
                let range = Range::dict(dict);
                tokens.extend(quote!(dict_entry(#key,#range)));
            }
            Node::Expr(expr) => {
                tokens.extend(quote!(entry(#key,#expr)));
            }
        }
    }
}

enum Node {
    Text(TokenStream),
    List(Punctuated<Item, Token![,]>),
    Dict(Punctuated<Entry, Token![,]>),
    Expr(TokenStream),
}
impl Parse for Node {
    fn parse(input: ParseStream) -> Result<Self> {
        if input.peek(token::Bracket) {
            let content;
            bracketed!(content in input); // empty is fine
            Ok(Node::List(
                content.parse_terminated(Item::parse, Token![,])?,
            ))
        } else if input.peek(token::Brace) {
            let content;
            braced!(content in input); // empty is fine
            Ok(Node::Dict(
                content.parse_terminated(Entry::parse, Token![,])?,
            ))
        } else if input.peek(token::Paren) {
            let content;
            let delims = parenthesized!(content in input).span;
            if content.is_empty() {
                return Err(Error::new(delims.join(), "missing expr inside ()"));
            }
            Ok(Node::Expr(content.parse()?))
        } else {
            let mut text = TokenStream::new();
            while !input.is_empty() && !input.peek(Token![,]) && !input.peek(Token![;]) {
                text.append(input.parse::<TokenTree>()?);
            }
            if text.is_empty() {
                return Err(input.error("missing expr for a value"));
            }
            Ok(Node::Text(text))
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
            name: Ident::new(&format!("__{name}_arena"), Span::mixed_site()),
            value_index: 0,
            keyed_index: 0,
            build: TokenStream::new(),
        }
    }
    /// assign indexes to and tokenize all children (if any) of the provided `value`.
    /// can't organize `value` itself at this recursion level because its index is
    /// impossible to know (it will eventually be organized by its container).
    fn value(&mut self, value: &mut Node) {
        match value {
            Node::List(items) => self.list(items),
            Node::Dict(entries) => self.dict(entries),
            _ => {}
        }
    }
    fn list(&mut self, children: &mut Punctuated<Item, Token![,]>) {
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
    fn dict(&mut self, children: &mut Punctuated<Entry, Token![,]>) {
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
}

struct JSON {
    name: Ident,
    root: Node,
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
        arena.make.value(&mut arena.root);
        Ok(arena)
    }
}
impl ToTokens for JSON {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let name = &self.name;
        let arena = &self.make.name;
        let item_count = self.make.value_index;
        let entry_count = self.make.keyed_index;
        let item_array = Ident::new(&format!("__{name}_items"), Span::mixed_site());
        let entry_array = Ident::new(&format!("__{name}_entries"), Span::mixed_site());
        let build = &self.make.build;
        let bind = quote! {
            let #item_array = ::tindalwic::Item::array::<#item_count>();
            let #entry_array = ::tindalwic::Entry::array::<#entry_count>();
            let mut #arena = ::tindalwic::internals::Arena::wrap(&#item_array, &#entry_array);
            #build
        };
        let after = &self.after;
        tokens.extend(match &self.root {
            Node::Text(text) => quote! {
                let #name = Text::wrap(#text)#after;
            },
            Node::List(list) => {
                let range = Range::list(list);
                quote! {
                    #bind
                    let #name = #arena.list(#range)#after;
                }
            }
            Node::Dict(dict) => {
                let range = Range::dict(dict);
                quote! {
                    #bind
                    let #name = #arena.dict(#range)#after;
                }
            }
            Node::Expr(_) => todo!(),
        });
    }
}

#[proc_macro]
pub fn json(input: RawStream) -> RawStream {
    let parse = Punctuated::<JSON, syn::parse::Nothing>::parse_terminated;
    let output = parse_macro_input!(input with parse);
    quote!(#output).into()
}
