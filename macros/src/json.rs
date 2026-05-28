use crate::*;

enum Root {
    List(Punctuated<Item, Token![,]>),
    Dict(Punctuated<Entry, Token![,]>),
}
impl Parse for Root {
    fn parse(input: ParseStream) -> Result<Self> {
        if let Some(stream) = Group::optional_bracketed(input)? {
            Ok(Root::List(stream.punctuated()?))
        } else if let Some(stream) = Group::optional_braced(input)? {
            Ok(Root::Dict(stream.punctuated()?))
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

pub(super) struct JSONs {
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
///         &'a mut ::tindalwic::capped::Arena<'a>
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
