#![allow(missing_docs)]

//! The macros defined here are re-exported from and documented in
//! [the main `tindalwic` crate](https://docs.rs/tindalwic).
//! You could depend on and import from this macros crate directly,
//! but the simpler `use tindalwic` is suggested.
//!
//! Normally these macros emit code containing paths that start with `::tindalwic`.
//! However, if your [Cargo.toml renames the dependency](https://doc.rust-lang.org/cargo/reference/specifying-dependencies.html#renaming-dependencies-in-cargotoml)
//! on `tindalwic` to a different _name_, then inform every macro call by writing, e.g.:
//!     walk! {
//!         $crate = name; // no `::` here
//!         ....
//!     }

use proc_macro::TokenStream as RawStream;
use proc_macro2::{Delimiter, Span, TokenStream, TokenTree};
use quote::{ToTokens, TokenStreamExt, quote};
use std::cell::RefCell;
use syn::parse::{Nothing, Parse, ParseStream, Parser};
use syn::punctuated::Punctuated;
use syn::spanned::Spanned;
use syn::token::{Brace, Bracket, Paren};
use syn::{Block, ImplItem, ItemImpl, Meta, ReturnType, Type};
use syn::{Error, Ident, ImplItemFn, LitInt, Result, Visibility};
use syn::{Token, braced, bracketed, parenthesized, parse_macro_input};

#[proc_macro]
pub fn arena(input: RawStream) -> RawStream {
    let output = parse_macro_input!(input as DollarCrate<Arena>);
    quote!(#output).into()
}

mod json;
#[proc_macro]
pub fn json(input: RawStream) -> RawStream {
    let output = parse_macro_input!(input as DollarCrate<json::JSONs>);
    quote!(#output).into()
}

mod walk;
#[proc_macro]
pub fn walk(input: RawStream) -> RawStream {
    let output = parse_macro_input!(input as DollarCrate<walk::Walks>);
    quote!(#output).into()
}

mod serde;
/// this is too tailored to the way tindalwic implements serde to be useful outside.
/// it has to be a proc_macro, so it has to be over here, so it has to be accessible,
/// but it isn't reexported from the lib crate, and isn't intended for public use.
/// the input syntax is weird, but it helps clarity by hiding boilerplate, and helps
/// prevent bugs via a predictable pattern that works with DeserializeSeed.
#[proc_macro]
pub fn serialize_deserialize_seed_visit(input: RawStream) -> RawStream {
    let output = parse_macro_input!(input as serde::SerDe);
    quote!(#output).into()
}

// ================================================================== dependency rename
// a thread_local is better than spreading the handling all over the place.

thread_local! {
    /// The name used for "tindalwic" crate - if empty, use `crate` keyword.
    static CRATE: RefCell<String> = const { RefCell::new(String::new()) };
}

/// All ToToken impl need to use this instead of `quote!(... ::tindalwic ...)`.
fn tindalwic() -> TokenStream {
    CRATE.with_borrow(|it| {
        if it.is_empty() {
            quote!(crate)
        } else {
            let ident = Ident::new(it, Span::call_site());
            // reconstruct every time to stay safely inside guarantees of Ident API
            // (e.g. they might one day change internals of Ident and/or call_site)
            quote!(::#ident)
        }
    })
}

/// `proc_macro` fns need to opt in to the rename mechanism by wrapping their ASTs.
struct DollarCrate<T>(T);
impl<T: Parse> Parse for DollarCrate<T> {
    fn parse(input: ParseStream) -> Result<Self> {
        parse_and_set_tindalwic_crate_name(input)?;
        Ok(DollarCrate(input.parse()?))
    }
}
impl<T: ToTokens> ToTokens for DollarCrate<T> {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        self.0.to_tokens(tokens);
    }
}

fn parse_and_set_tindalwic_crate_name(input: ParseStream) -> Result<()> {
    if input.peek(Token![$]) {
        input.parse::<Token![$]>()?;
        input.parse::<Token![crate]>()?;
        input.parse::<Token![=]>()?;
        if input.peek(Token![crate]) {
            input.parse::<Token![crate]>()?;
            CRATE.with_borrow_mut(|it| {
                it.clear();
            });
        } else {
            let ident: Ident = input.parse()?;
            CRATE.with_borrow_mut(|it| {
                it.clear();
                it.push_str(&ident.to_string());
            });
        }
        input.parse::<Token![;]>()?;
    } else {
        CRATE.with_borrow_mut(|it| {
            it.clear();
            it.push_str("tindalwic");
        });
    }
    Ok(())
}

// ======================================================================== syn helpers

struct Group(proc_macro2::Group);
impl Group {
    fn not_empty(self, message: &'static str) -> Result<TokenStream> {
        if self.0.stream().is_empty() {
            Err(Error::new(self.0.span(), message))
        } else {
            Ok(self.0.stream())
        }
    }
    fn stream(&self) -> TokenStream {
        self.0.stream()
    }
    fn punctuated<T: Parse, P: Parse>(self) -> Result<Punctuated<T, P>> {
        Ok(Punctuated::<T, P>::parse_terminated.parse2(self.stream())?)
    }
    fn required_braced(input: ParseStream) -> Result<Self> {
        let content;
        let delim = braced!(content in input);
        let mut group = proc_macro2::Group::new(Delimiter::Brace, content.parse()?);
        group.set_span(delim.span.span());
        Ok(Group(group))
    }
    fn optional_braced(input: ParseStream) -> Result<Option<Self>> {
        Ok(if input.peek(Brace) {
            Some(Group::required_braced(input)?)
        } else {
            None
        })
    }
    fn required_bracketed(input: ParseStream) -> Result<Self> {
        let content;
        let delim = bracketed!(content in input);
        let mut group = proc_macro2::Group::new(Delimiter::Bracket, content.parse()?);
        group.set_span(delim.span.span());
        Ok(Group(group))
    }
    fn optional_bracketed(input: ParseStream) -> Result<Option<Self>> {
        Ok(if input.peek(Bracket) {
            Some(Group::required_bracketed(input)?)
        } else {
            None
        })
    }
    fn required_parenthesized(input: ParseStream) -> Result<Self> {
        let content;
        let delim = parenthesized!(content in input);
        let mut group = proc_macro2::Group::new(Delimiter::Parenthesis, content.parse()?);
        group.set_span(delim.span.span());
        Ok(Group(group))
    }
    fn optional_parenthesized(input: ParseStream) -> Result<Option<Self>> {
        Ok(if input.peek(Paren) {
            Some(Group::required_parenthesized(input)?)
        } else {
            None
        })
    }
    fn required_angled(input: ParseStream) -> Result<Self> {
        let open = input.parse::<Token![<]>()?;
        let mut stream = TokenStream::new();
        let mut depth = 1usize;
        while depth != 0 {
            if input.is_empty() {
                let span = open.span.join(input.span()).unwrap_or(open.span);
                return Err(Error::new(span, "unbalanced <> brackets"));
            }
            stream.append(input.parse::<TokenTree>()?);
            if input.peek(Token![<]) {
                depth += 1;
            } else if input.peek(Token![>]) {
                depth -= 1;
            }
        }
        let close = input.parse::<Token![>]>()?;
        let mut group = proc_macro2::Group::new(Delimiter::None, stream);
        group.set_span(open.span.join(close.span).unwrap_or(open.span));
        Ok(Group(group))
    }
    fn optional_angled(input: ParseStream) -> Result<Option<Self>> {
        Ok(if input.peek(Token![<]) {
            Some(Group::required_angled(input)?)
        } else {
            None
        })
    }
}

/// Dual-purpose: parse a simple `let` binding from macro input syntax, also to
/// invent hidden `let` bindings to fix "temporary dropped" compiler complaints.
struct Variable {
    mutable: bool,
    ident: Ident,
}
impl Parse for Variable {
    fn parse(input: ParseStream) -> Result<Self> {
        let mutable = input.peek(Token![mut]);
        if mutable {
            input.parse::<Token![mut]>()?;
        }
        Ok(Variable {
            mutable,
            ident: input.parse()?,
        })
    }
}
impl Variable {
    fn new(name: &str) -> Self {
        Variable {
            mutable: false,
            ident: Ident::new(name, Span::call_site()),
        }
    }
    fn hidden(name: &str) -> Self {
        Variable {
            mutable: false,
            ident: Ident::new(name, Span::mixed_site()),
        }
    }
    fn derive(&self, suffix: &'static str) -> Self {
        let name = self.ident.to_string();
        Variable::hidden(&format!("__{name}_{suffix}"))
    }
}
impl ToTokens for Variable {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let Variable { mutable, ident } = self;
        if *mutable {
            tokens.extend(quote!(mut #ident));
        } else {
            ident.to_tokens(tokens);
        }
    }
}

/// For places in the input syntax where `?` or `.unwrap()` or similar is expected.
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

// ============================================================================= shared

/// Some macros invent hidden `let` bindings for an Arena and its arrays.
/// Provisional `arena!` lets the caller make an exposed Arena instance, which was
/// handy during development of the parse module, but (TODO) should probably be
/// disappeared before first release - assuming that need is addressed elsewhere.
struct Arena {
    name: Variable,
    items: usize,
    entries: usize,
}
impl Parse for Arena {
    fn parse(input: ParseStream) -> Result<Self> {
        input.parse::<Token![let]>()?;
        let mut arena = Arena::new(input.parse()?);
        input.parse::<Token![=]>()?;
        input.parse::<Token![<]>()?;
        for dimension in Punctuated::<LitInt, Token![,]>::parse_separated_nonempty(input)? {
            match dimension.suffix() {
                "list" => arena.items = dimension.base10_parse::<usize>()?,
                "dict" => arena.entries = dimension.base10_parse::<usize>()?,
                _ => {
                    return Err(Error::new_spanned(
                        dimension,
                        "need `list` or `dict` suffix",
                    ));
                }
            }
        }
        if arena.items == 0 && arena.entries == 0 {
            return Err(input.error("need at least one non-zero dimension"));
        }
        if !arena.name.mutable {
            return Err(Error::new_spanned(arena.name.ident, "must specify `mut`"));
        }
        input.parse::<Token![>]>()?;
        input.parse::<Token![;]>()?;
        Ok(arena)
    }
}
impl Arena {
    fn new(mut name: Variable) -> Self {
        name.mutable = true;
        Arena {
            name,
            items: 0,
            entries: 0,
        }
    }
    fn count_list<P>(&mut self, list: &Punctuated<Item, P>) {
        self.items += list.len();
        for item in list {
            self.count_item(item);
        }
    }
    fn count_dict<P>(&mut self, dict: &Punctuated<Entry, P>) {
        self.entries += dict.len();
        for entry in dict {
            self.count_item(&entry.item);
        }
    }
    fn count_item(&mut self, item: &Item) {
        match item {
            Item::List(list) => self.count_list(list),
            Item::Dict(dict) => self.count_dict(dict),
            _ => {}
        }
    }
}
impl ToTokens for Arena {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let Arena {
            name,
            items,
            entries,
        } = self;
        let tindalwic = tindalwic();
        let ia = name.derive("items");
        let ea = name.derive("entries");
        // https://doc.rust-lang.org/reference/items/use-declarations.html#underscore-imports
        tokens.extend(quote! {
            let #ia = #tindalwic::Item::array::<#items>();
            let #ea = #tindalwic::Entry::array::<#entries>();
            let #name = #tindalwic::capped::Arena::wrap(&#ia, &#ea);
        });
    }
}

enum Item {
    Text(TokenStream),
    List(Punctuated<Item, Token![,]>),
    Dict(Punctuated<Entry, Token![,]>),
    Expr(TokenStream),
}
impl Parse for Item {
    fn parse(input: ParseStream) -> Result<Self> {
        if let Some(stream) = Group::optional_bracketed(input)? {
            Ok(Item::List(stream.punctuated()?))
        } else if let Some(stream) = Group::optional_braced(input)? {
            Ok(Item::Dict(stream.punctuated()?))
        } else if let Some(stream) = Group::optional_parenthesized(input)? {
            Ok(Item::Expr(stream.not_empty("missing expr inside ()")?))
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
