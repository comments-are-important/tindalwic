#![allow(missing_docs, unused)]

//! see the documentation in the `tindalwic` crate.

use proc_macro::TokenStream as RawStream;
use proc_macro2::{Span, TokenStream};
use quote::{ToTokens, quote};
use syn::parse::{Parse, ParseStream};
use syn::punctuated::Punctuated;
use syn::*;

// ====================================================================================

enum StepKind {
    List,
    Dict,
    Text,
}

struct Step {
    kind: StepKind,
    tokens: TokenStream,
}
impl Parse for Step {
    fn parse(input: ParseStream) -> Result<Self> {
        if input.peek(Token![<]) {
            input.parse::<Token![<]>()?;
            let mut tokens = TokenStream::new();
            while !input.peek(Token![>]) {
                let tt: proc_macro2::TokenTree = input.parse()?;
                tokens.extend(core::iter::once(tt));
            }
            input.parse::<Token![>]>()?;
            Ok(Step { kind: StepKind::Text, tokens })
        } else if input.peek(token::Bracket) {
            let content;
            bracketed!(content in input);
            let tokens: TokenStream = content.parse()?;
            Ok(Step { kind: StepKind::List, tokens })
        } else if input.peek(token::Brace) {
            let content;
            braced!(content in input);
            let tokens: TokenStream = content.parse()?;
            Ok(Step { kind: StepKind::Dict, tokens })
        } else {
            Err(input.error("each step must be enclosed in <>, [] or {}"))
        }
    }
}

struct Walk {
    name: Ident,
    steps: Vec<Step>,
    cell_ident: Ident,
    value_ident: Ident,
    body: TokenStream,
}
impl Parse for Walk {
    fn parse(input: ParseStream) -> Result<Self> {
        let name: Ident = input.parse()?;
        let mut steps = Vec::new();
        while input.peek(Token![<]) || input.peek(token::Bracket) || input.peek(token::Brace) {
            steps.push(input.parse::<Step>()?);
        }
        if steps.is_empty() {
            return Err(input.error("requires at least one step"));
        }
        if let StepKind::Text = steps.last().unwrap().kind {
            // ok
        } else {
            // Text can only be last, but List/Dict can be last too
        }
        for step in &steps[..steps.len() - 1] {
            if let StepKind::Text = step.kind {
                return Err(input.error("<> step can only appear last"));
            }
        }
        input.parse::<Token![|]>()?;
        let cell_ident: Ident = input.parse()?;
        input.parse::<Token![,]>()?;
        let value_ident: Ident = input.parse()?;
        input.parse::<Token![|]>()?;
        let body: TokenStream = input.parse()?;
        Ok(Walk { name, steps, cell_ident, value_ident, body })
    }
}

/// Build a path string like `[a]{0}<b>` for error messages.
fn path_string(steps: &[Step], up_to: usize) -> String {
    let mut s = String::new();
    for step in &steps[..=up_to] {
        let t = step.tokens.to_string();
        match step.kind {
            StepKind::List => { s.push('['); s.push_str(&t); s.push(']'); }
            StepKind::Dict => { s.push('{'); s.push_str(&t); s.push('}'); }
            StepKind::Text => { s.push('<'); s.push_str(&t); s.push('>'); }
        }
    }
    s
}

#[proc_macro]
pub fn tindalwic_walk(input: RawStream) -> RawStream {
    let Walk { name, steps, cell_ident, value_ident, body } = parse_macro_input!(input as Walk);

    // Build the nested if-let chain from inside out (last step first).
    // At every step we know:
    //   - what we're currently inside (dict at depth 0, then determined by previous step's kind)
    //   - what we expect to find (this step's kind)

    let last = steps.len() - 1;

    // Start with the innermost (last step) happy path.
    let value_arm = match steps[last].kind {
        StepKind::Text => quote! { ::tindalwic::Value::Text(#value_ident) },
        StepKind::List => quote! { ::tindalwic::Value::List(#value_ident) },
        StepKind::Dict => quote! { ::tindalwic::Value::Dict(#value_ident) },
    };
    let type_name = match steps[last].kind {
        StepKind::Text => "text",
        StepKind::List => "list",
        StepKind::Dict => "dict",
    };

    // The innermost code: match the value variant, bind both idents, run body.
    let path_so_far = path_string(&steps, last);
    let not_type_err = format!("{path_so_far} is not {type_name}");

    // What is the "current container" at the last step?
    // depth 0 = dict (file root), after that determined by previous step's kind.
    let in_dict = if last == 0 {
        true
    } else {
        matches!(steps[last - 1].kind, StepKind::Dict)
    };

    let it = Ident::new("__it", Span::mixed_site());
    let last_tokens = &steps[last].tokens;
    let mut inner = if in_dict {
        // dict lookup
        let not_found_err = format!("{path_so_far} not found");
        quote! {
            if let Some(#it) = #it.find(#last_tokens) {
                let mut #cell_ident = #it.get();
                if let #value_arm = &mut #cell_ident.value {
                    #body;
                    #it.set(#cell_ident);
                    Ok(())
                } else {
                    Err(#not_type_err)
                }
            } else {
                Err(#not_found_err)
            }
        }
    } else {
        // list index
        let not_found_err = format!("{path_so_far} index out of bounds");
        quote! {
            if let Some(#it) = #it.list.get(#last_tokens) {
                let mut #cell_ident = #it.get();
                if let #value_arm = &mut #cell_ident {
                    #body;
                    #it.set(#cell_ident);
                    Ok(())
                } else {
                    Err(#not_type_err)
                }
            } else {
                Err(#not_found_err)
            }
        }
    };

    // Now wrap each preceding step from inside out.
    for i in (0..last).rev() {
        let step_tokens = &steps[i].tokens;
        let path_here = path_string(&steps, i);

        let expected_arm = match steps[i].kind {
            StepKind::List => quote! { ::tindalwic::Value::List(#it) },
            StepKind::Dict => quote! { ::tindalwic::Value::Dict(#it) },
            StepKind::Text => unreachable!(), // validated during parse
        };
        let type_name_here = match steps[i].kind {
            StepKind::List => "list",
            StepKind::Dict => "dict",
            StepKind::Text => unreachable!(),
        };
        let not_type_err_here = format!("{path_here} is not {type_name_here}");

        let in_dict_here = if i == 0 {
            true
        } else {
            matches!(steps[i - 1].kind, StepKind::Dict)
        };

        inner = if in_dict_here {
            let not_found_err_here = format!("{path_here} not found");
            quote! {
                if let Some(#it) = #it.find(#step_tokens) {
                    if let #expected_arm = #it.get().value {
                        #inner
                    } else {
                        Err(#not_type_err_here)
                    }
                } else {
                    Err(#not_found_err_here)
                }
            }
        } else {
            let not_found_err_here = format!("{path_here} index out of bounds");
            quote! {
                if let Some(#it) = #it.list.get(#step_tokens) {
                    if let #expected_arm = #it.get() {
                        #inner
                    } else {
                        Err(#not_type_err_here)
                    }
                } else {
                    Err(#not_found_err_here)
                }
            }
        };
    }

    // Wrap in a closure so `#it` starts as the file.
    let expanded = quote! {
        (|| -> Result<(), &'static str> {
            let #it = #name.file.get();
            #inner
        })()
    };

    expanded.into()
}

// ====================================================================================

#[proc_macro]
pub fn tindalwic_json(input: RawStream) -> RawStream {
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
