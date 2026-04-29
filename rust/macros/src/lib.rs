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
use proc_macro2::{Span, TokenStream, TokenTree};
use quote::{ToTokens, TokenStreamExt, quote};
use std::cell::RefCell;
use syn::parse::{Nothing, Parse, ParseStream};
use syn::punctuated::Punctuated;
use syn::token::{Brace, Bracket, Paren};
use syn::{
    Error, Ident, LitInt, Result, Token, braced, bracketed, parenthesized, parse_macro_input,
};

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

// ==================================================================== general helpers

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
        tokens.extend(quote! {
            let #ia = #tindalwic::Item::array::<#items>();
            let #ea = #tindalwic::Entry::array::<#entries>();
            let #name = #tindalwic::internals::Arena::wrap(&#ia, &#ea);
        });
    }
}

#[proc_macro]
pub fn arena(input: RawStream) -> RawStream {
    let output = parse_macro_input!(input as DollarCrate<Arena>);
    quote!(#output).into()
}

// ============================================================================== walk!

/// Using a bool field instead of two-variant enum to make parsing easier.
struct Branch {
    list: bool,        // true means `List`, false means `Dict`.
    expr: TokenStream, // unparsed (hopefully produces either `usize` or `Key`)
}
impl ToTokens for Branch {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let tindalwic = tindalwic();
        let expr = &self.expr;
        if self.list {
            tokens.extend(quote!(#tindalwic::internals::Branch::List(#expr)));
        } else {
            tokens.extend(quote!(#tindalwic::internals::Branch::Dict(#expr)));
        }
    }
}

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
                return Err(Error::new(delims.join(), "missing list inside []"));
            }
            origin = content.parse()?;
        } else if input.peek(Brace) {
            let content;
            let delims = braced!(content in input).span;
            if content.is_empty() {
                return Err(Error::new(delims.join(), "missing dict inside {}"));
            }
            origin = content.parse()?;
        } else if input.peek(Paren) {
            let content;
            let delims = parenthesized!(content in input).span;
            if content.is_empty() {
                return Err(Error::new(delims.join(), "missing file inside ()"));
            }
            let content: TokenStream = content.parse()?;
            let tindalwic = tindalwic();
            origin = quote!(#tindalwic::Dict::wrap((#content).cells));
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
        let result = variables
            .next()
            .expect("previously checked, count can't be zero, this can't be None");
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
        // derived variables can't clash with each other or `result` by construction,
        // but testing them anyway is cheap and keeps this code straight-line...
        let result_string = walk.result.ident.to_string();
        let cell_string = walk.cell.ident.to_string();
        if cell_string == result_string {
            return Err(Error::new_spanned(&walk.cell.ident, "duplicate binding"));
        } else if let Some(name) = &walk.name {
            let name_string = name.ident.to_string();
            if name_string == result_string {
                return Err(Error::new_spanned(&name.ident, "duplicate binding"));
            } else if cell_string == name_string {
                return Err(Error::new_spanned(&walk.cell.ident, "duplicate binding"));
            }
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
        let tindalwic = tindalwic();
        tokens.extend(quote! {
            let #branches = [#(#steps),*];
            let #path = #tindalwic::internals::Path::wrap(&#branches);
            let #cell = #path.#method(&(#origin).into())#unwrap;
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

struct Walks {
    statements: Punctuated<Walk, Nothing>,
}
impl Parse for Walks {
    fn parse(input: ParseStream) -> Result<Self> {
        let statements = Punctuated::parse_terminated(input)?;
        if statements.is_empty() {
            return Err(input.error("expecting a `let` statement"));
        }
        Ok(Walks { statements })
    }
}
impl ToTokens for Walks {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let Walks { statements } = self;
        for walk in statements {
            walk.to_tokens(tokens);
        }
    }
}

#[proc_macro]
pub fn walk(input: RawStream) -> RawStream {
    let output = parse_macro_input!(input as DollarCrate<Walks>);
    quote!(#output).into()
}

// ====================================================================================

enum Root {
    List(Punctuated<Item, Token![,]>),
    Dict(Punctuated<Entry, Token![,]>),
}
impl Parse for Root {
    fn parse(input: ParseStream) -> Result<Self> {
        if input.peek(Bracket) {
            let content;
            bracketed!(content in input); // empty is fine
            let commas = content.parse_terminated(Item::parse, Token![,])?;
            Ok(Root::List(commas))
        } else if input.peek(Brace) {
            let content;
            braced!(content in input); // empty is fine
            let commas = content.parse_terminated(Entry::parse, Token![,])?;
            Ok(Root::Dict(commas))
        } else {
            Err(input.error("root item must be [] or {}"))
        }
    }
}
impl Root {
    fn count(&self, arena: &mut Arena) {
        // roots live outside Arena, so only count children...
        match self {
            Root::List(list) => arena.count_list(list),
            Root::Dict(dict) => arena.count_dict(dict),
        }
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
        if input.peek(Bracket) {
            let content;
            bracketed!(content in input); // empty is fine
            let commas = content.parse_terminated(Item::parse, Token![,])?;
            Ok(Item::List(commas))
        } else if input.peek(Brace) {
            let content;
            braced!(content in input); // empty is fine
            let commas = content.parse_terminated(Entry::parse, Token![,])?;
            Ok(Item::Dict(commas))
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

struct JSONs {
    arena: Arena,
    statements: Vec<JSON>,
    completed: Option<Propagate>,
}
impl Parse for JSONs {
    fn parse(input: ParseStream) -> Result<Self> {
        let mut statements = Vec::<JSON>::new();
        while input.peek(Token![let]) {
            statements.push(input.parse()?);
        }
        let Some(first) = statements.first() else {
            return Err(input.error("expecting a `let` statement"));
        };
        let mut completed: Option<Propagate> = None;
        if !input.is_empty() {
            let ident: Ident = input.parse()?;
            if ident != "completed" {
                return Err(Error::new_spanned(ident, "only `completed` allowed"));
            }
            completed = Some(input.parse()?);
            input.parse::<Token![;]>()?;
        }
        // the arena name is derived from the name of the first JSON value,
        // even though all the JSONs get built into the single arena instance.
        // the arena `let` bindings are hidden, so they don't really matter, but
        // this little bit of paranoia provides an extra assurance of no clashes.
        let mut arena = Arena::new(first.name.derive("arena"));
        for json in &statements {
            json.root.count(&mut arena);
        }
        Ok(JSONs {
            arena,
            statements,
            completed,
        })
    }
}
/// it would be easier to impl ToTokens for JSON/Root/Item/Entry, and then generate
/// a closure around the builder call tokens from JSON.to_tokens, like this:
///     let #build: &dyn for<'a> Fn(
///         &'a mut ::tindalwic::internals::Arena<'a>
///     ) -> Option<::tindalwic::#kind<'a>> = &|arena| {
///         #root
///     };
///     let #name = #build(&mut #arena)#err;
/// that would allow use of the fixed literal name "arena", and the #err
/// propagation to only happen once. unfortunately when an Item::Expr drags
/// something into a closure the lifetimes won't work. the compiler can't see
/// that the <'a> in the #build signature is the same lifetime as all the
/// bindings that get pulled in to the closure by the Item::Expr expansions.
impl JSONs {
    fn list<P>(&self, list: &Punctuated<Item, P>, err: &Propagate, tokens: &mut TokenStream) {
        let ident = &self.arena.name.ident;
        for item in list {
            match item {
                Item::Text(text) => {
                    tokens.extend(quote! {
                        #ident.text_item(#text)#err;
                    });
                }
                Item::List(list) => {
                    self.list(list, err, tokens);
                    let count = list.len();
                    tokens.extend(quote! {
                        #ident.list_item(#count)#err;
                    });
                }
                Item::Dict(dict) => {
                    self.dict(dict, err, tokens);
                    let count = dict.len();
                    tokens.extend(quote! {
                        #ident.dict_item(#count)#err;
                    });
                }
                Item::Expr(expr) => {
                    tokens.extend(quote! {
                        #ident.item((#expr).into())#err;
                    });
                }
            }
        }
    }
    fn dict<P>(&self, dict: &Punctuated<Entry, P>, err: &Propagate, tokens: &mut TokenStream) {
        let ident = &self.arena.name.ident;
        for entry in dict {
            let Entry { key, item } = entry;
            match item {
                Item::Text(text) => {
                    tokens.extend(quote! {
                        #ident.text_entry(#key, #text)#err;
                    });
                }
                Item::List(list) => {
                    self.list(list, err, tokens);
                    let count = list.len();
                    tokens.extend(quote! {
                        #ident.list_entry(#key, #count)#err;
                    });
                }
                Item::Dict(dict) => {
                    self.dict(dict, err, tokens);
                    let count = dict.len();
                    tokens.extend(quote! {
                        #ident.dict_entry(#key, #count)#err;
                    });
                }
                Item::Expr(expr) => {
                    tokens.extend(quote! {
                        #ident.keyed(#key, (#expr).into())#err;
                    });
                }
            }
        }
    }
}
impl ToTokens for JSONs {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let JSONs {
            arena,
            statements,
            completed,
        } = self;
        tokens.extend(quote!(#arena));
        let ident = &self.arena.name.ident;
        for json in statements {
            let JSON { name, root, err } = json;
            match root {
                Root::List(list) => {
                    self.list(list, err, tokens);
                    let count = list.len();
                    tokens.extend(quote! {
                        let #name = #ident.list(#count)#err;
                    });
                }
                Root::Dict(dict) => {
                    self.dict(dict, err, tokens);
                    let count = dict.len();
                    tokens.extend(quote! {
                        let #name = #ident.dict(#count)#err;
                    });
                }
            }
        }
        if let Some(err) = completed {
            tokens.extend(quote!(#ident.completed()#err));
        }
    }
}

#[proc_macro]
pub fn json(input: RawStream) -> RawStream {
    let output = parse_macro_input!(input as DollarCrate<JSONs>);
    quote!(#output).into()
}
