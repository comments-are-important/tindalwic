#![allow(missing_docs)]

//! The macros defined here are re-exported from and documented in
//! [the main tindalwic crate](https://docs.rs/tindalwic).
//! You could depend on and import from this crate,
//! but the simpler `use tindalwic` is suggested.

use proc_macro::TokenStream as RawStream;
use proc_macro2::{Span, TokenStream, TokenTree};
use quote::{ToTokens, TokenStreamExt, quote};
use syn::parse::{Nothing, Parse, ParseStream};
use syn::punctuated::Punctuated;
use syn::token::{Brace, Bracket, Paren};
use syn::{Error, Ident, Result, Token, braced, bracketed, parenthesized, parse_macro_input};

fn into_semis<T, P>(other: Punctuated<T, P>) -> Punctuated<T, Token![;]> {
    let mut semis: Punctuated<T, Token![;]> = other.into_iter().collect();
    semis.push_punct(Default::default());
    semis
}

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
    fn clash<P>(within: &Punctuated<Variable, P>) -> Option<&Variable> {
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

struct Propagate {
    expr: TokenStream,
}
impl Parse for Propagate {
    fn parse(input: ParseStream) -> Result<Self> {
        let mut expr = TokenStream::new();
        while !input.is_empty() && !input.peek(Token![;]) {
            expr.append(input.parse::<TokenTree>()?);
        }
        if expr.is_empty() {
            return Err(input.error("need `?` (or similar) to propagate"));
        }
        Ok(Propagate { expr })
    }
}
impl ToTokens for Propagate {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let Propagate { expr } = self;
        tokens.extend(quote!(#expr));
    }
}

// ====================================================================================

struct Branch {
    list: bool,        // true means `List`, false means `Dict`.
    expr: TokenStream, // unparsed (hopefully produces either `usize` or `Key`)
}
impl ToTokens for Branch {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let expr = &self.expr;
        if self.list {
            tokens.extend(quote!(::tindalwic::internals::Branch::List(#expr)));
        } else {
            tokens.extend(quote!(::tindalwic::internals::Branch::Dict(#expr)));
        }
    }
}

#[allow(unused)]
struct Walk {
    origin: TokenStream,    // where the walk begins - unparsed (Item or File)
    steps: Vec<Branch>,     // the decisions that form a Path
    cell: Variable,         // binding for the Item/Entry Cell the walk ends on
    name: Option<Variable>, // if ends at Entry Cell, then the Name gets a binding
    lands: Option<bool>,    // `None` means `Text`, `Some` => `Branch::list`
    result: Variable,       // binding for the Item enum payload
    err: Propagate,         // the caller must specify `?` (or similar) for result
}
impl Parse for Walk {
    fn parse(input: ParseStream) -> Result<Self> {
        input.parse::<Token![let]>()?;
        let mut binds: Punctuated<Variable, Token![,]>;
        if input.peek(Paren) {
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
        if input.peek(Bracket) {
            list = true;
            let content;
            let delims = bracketed!(content in input).span;
            if content.is_empty() {
                return Err(Error::new(delims.join(), "missing origin inside []"));
            }
            origin = content.parse()?;
        } else if input.peek(Brace) {
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
            if input.peek(Bracket) {
                let content;
                let delims = bracketed!(content in input).span;
                if content.is_empty() {
                    return Err(Error::new(delims.join(), "missing expr inside []"));
                }
                let expr = content.parse()?;
                steps.push(Branch { expr, list });
                list = true;
            } else if input.peek(Brace) {
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
        let err = input.parse()?;
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
            err,
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
        let unwrap = &self.err;
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

type ItemsSemi = Punctuated<Item, Token![;]>;
type EntriesSemi = Punctuated<Entry, Token![;]>;

enum Root {
    List(ItemsSemi),
    Dict(EntriesSemi),
}
impl Parse for Root {
    fn parse(input: ParseStream) -> Result<Self> {
        if input.peek(Bracket) {
            let content;
            bracketed!(content in input); // empty is fine
            let commas = content.parse_terminated(Item::parse, Token![,])?;
            Ok(Root::List(into_semis(commas)))
        } else if input.peek(Brace) {
            let content;
            braced!(content in input); // empty is fine
            let commas = content.parse_terminated(Entry::parse, Token![,])?;
            Ok(Root::Dict(into_semis(commas)))
        } else {
            Err(input.error("root item must be [] or {}"))
        }
    }
}

enum Item {
    Text(TokenStream),
    List(ItemsSemi),
    Dict(EntriesSemi),
    Expr(TokenStream),
}
impl Parse for Item {
    fn parse(input: ParseStream) -> Result<Self> {
        if input.peek(Bracket) {
            let content;
            bracketed!(content in input); // empty is fine
            let commas = content.parse_terminated(Item::parse, Token![,])?;
            Ok(Item::List(into_semis(commas)))
        } else if input.peek(Brace) {
            let content;
            braced!(content in input); // empty is fine
            let commas = content.parse_terminated(Entry::parse, Token![,])?;
            Ok(Item::Dict(into_semis(commas)))
        } else if input.peek(Paren) {
            let content;
            let delims = parenthesized!(content in input).span;
            if content.is_empty() {
                return Err(Error::new(delims.join(), "missing expr inside ()"));
            }
            Ok(Item::Expr(content.parse()?))
        } else {
            let mut text = TokenStream::new();
            while !input.is_empty() && !input.peek(Token![,]) && !input.peek(Token![;]) {
                text.append(input.parse::<TokenTree>()?);
            }
            if text.is_empty() {
                return Err(input.error("missing text expr"));
            }
            Ok(Item::Text(text))
        }
    }
}

struct Entry {
    key: TokenStream,
    item: Item,
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
            item: input.parse()?,
        })
    }
}

struct Counts {
    items: usize,
    entries: usize,
}
impl Counts {
    fn new() -> Self {
        Counts {
            items: 0,
            entries: 0,
        }
    }
    fn root(mut self, root: &Root) -> Self {
        match root {
            Root::List(list) => self.list(list),
            Root::Dict(dict) => self.dict(dict),
        }
        self
    }
    fn list(&mut self, list: &ItemsSemi) {
        self.items += list.len();
        for child in list {
            self.item(child);
        }
    }
    fn dict(&mut self, dict: &EntriesSemi) {
        self.entries += dict.len();
        for child in dict {
            self.item(&child.item);
        }
    }
    fn item(&mut self, item: &Item) {
        match item {
            Item::List(list) => self.list(list),
            Item::Dict(dict) => self.dict(dict),
            _ => {}
        }
    }
}

/// context for converting Root/Item/Entry to tokens.
/// it would be easier to impl ToTokens for those structs, and then generate
/// a closure around the Root tokens from JSON.to_tokens, like this:
///     let #build: &dyn for<'a> Fn(
///         &'a mut ::tindalwic::internals::Arena<'a>
///     ) -> Option<::tindalwic::#kind<'a>> = &|arena| {
///         #root
///     };
///     let #name = #build(&mut #arena)#err;
/// that would allow use of the fixed literal name "arena", and the #err
/// propagation only happens once. unfortunately when an Item::Expr drags
/// something into a closure the lifetimes won't work (the compiler can't see
/// that the <'a> in the #build signature applies to the Expr identifiers).
struct Arena<'a> {
    arena: &'a Ident,
    err: &'a Propagate,
}
impl<'a> Arena<'a> {
    fn root(&self, name: &'a Variable, root: &'a Root, tokens: &mut TokenStream) {
        let Arena { arena, err } = self;
        match root {
            Root::List(list) => {
                let count = self.list(list, tokens);
                tokens.extend(quote! {
                    let #name = #arena.list(#count)#err;
                });
            }
            Root::Dict(dict) => {
                let count = self.dict(dict, tokens);
                tokens.extend(quote! {
                    let #name = #arena.dict(#count)#err;
                });
            }
        }
    }
    fn list(&self, list: &'a ItemsSemi, tokens: &mut TokenStream) -> usize {
        for child in list {
            self.item(child, tokens);
        }
        list.len()
    }
    fn dict(&self, dict: &'a EntriesSemi, tokens: &mut TokenStream) -> usize {
        for child in dict {
            self.entry(child, tokens);
        }
        dict.len()
    }
    fn item(&self, item: &'a Item, tokens: &mut TokenStream) {
        let Arena { arena, err } = self;
        match item {
            Item::Text(text) => {
                tokens.extend(quote! {
                    #arena.text_item(#text)#err;
                });
            }
            Item::List(list) => {
                let count = self.list(list, tokens);
                tokens.extend(quote! {
                    #arena.list_item(#count)#err;
                });
            }
            Item::Dict(dict) => {
                let count = self.dict(dict, tokens);
                tokens.extend(quote! {
                    #arena.dict_item(#count)#err;
                });
            }
            Item::Expr(expr) => {
                tokens.extend(quote! {
                    #arena.item((#expr).into())#err;
                });
            }
        }
    }
    fn entry(&self, entry: &'a Entry, tokens: &mut TokenStream) {
        let Arena { arena, err } = self;
        let Entry { key, item } = entry;
        match item {
            Item::Text(text) => {
                tokens.extend(quote! {
                    #arena.text_entry(#key, #text)#err;
                });
            }
            Item::List(list) => {
                let count = self.list(list, tokens);
                tokens.extend(quote! {
                    #arena.list_entry(#key, #count)#err;
                });
            }
            Item::Dict(dict) => {
                let count = self.dict(dict, tokens);
                tokens.extend(quote! {
                    #arena.dict_entry(#key, #count)#err;
                });
            }
            Item::Expr(expr) => {
                tokens.extend(quote! {
                    #arena.keyed(#key, (#expr).into())#err;
                });
            }
        }
    }
}

struct JSON {
    name: Variable,
    root: Root,
    err: Propagate,
}
impl Parse for JSON {
    fn parse(input: ParseStream) -> Result<Self> {
        input.parse::<Token![let]>()?;
        let name = input.parse()?;
        input.parse::<Token![=]>()?;
        let root = input.parse()?;
        let err = input.parse()?;
        input.parse::<Token![;]>()?;
        Ok(JSON { name, root, err })
    }
}
impl ToTokens for JSON {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let JSON { name, root, err } = self;
        let ia = name.derive("items");
        let ea = name.derive("entries");
        let arena = name.derive("arena");
        let Counts { items, entries } = Counts::new().root(root);
        tokens.extend(quote! {
            let #ia = ::tindalwic::Item::array::<#items>();
            let #ea = ::tindalwic::Entry::array::<#entries>();
            let mut #arena = ::tindalwic::internals::Arena::wrap(&#ia, &#ea);
        });
        let arena = Arena {
            arena: &arena.ident,
            err: &err,
        };
        arena.root(&name, &root, tokens);
    }
}

#[proc_macro]
pub fn json(input: RawStream) -> RawStream {
    let parse = Punctuated::<JSON, Nothing>::parse_terminated;
    let output = parse_macro_input!(input with parse);
    quote!(#output).into()
}
