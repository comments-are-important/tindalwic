use super::*;

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
            tokens.extend(quote!(#tindalwic::walk::Branch::List(#expr)));
        } else {
            tokens.extend(quote!(#tindalwic::walk::Branch::Dict(#expr)));
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
        if let Some(stream) = Group::optional_parenthesized(input)? {
            binds = stream.punctuated()?;
            if binds.is_empty() {
                return Err(Error::new_spanned(binds, "missing binding inside ()"));
            } else if binds.len() == 1 {
                return Err(Error::new_spanned(binds, "remove unnecessary parens"));
            }
        } else {
            binds = Punctuated::new();
            binds.push(input.parse()?);
        }
        input.parse::<Token![=]>()?;
        let mut list = false;
        let origin = if let Some(stream) = Group::optional_bracketed(input)? {
            list = true;
            stream.not_empty("missing list inside []")?
        } else if let Some(stream) = Group::optional_braced(input)? {
            stream.not_empty("missing dict inside {}")?
        } else if let Some(stream) = Group::optional_parenthesized(input)? {
            let stream = stream.not_empty("missing file inside ()")?;
            let tindalwic = tindalwic();
            quote!(#tindalwic::tree::Dict::wrap((#stream).cells))
        } else {
            return Err(input.error("must start with [List], {Dict} or (File)"));
        };
        let mut text = false;
        let mut steps = Vec::new();
        while !input.is_empty() {
            if let Some(stream) = Group::optional_bracketed(input)? {
                let expr = stream.not_empty("missing expr inside []")?;
                steps.push(Branch { expr, list });
                list = true;
            } else if let Some(stream) = Group::optional_braced(input)? {
                let expr = stream.not_empty("missing expr inside {}")?;
                steps.push(Branch { expr, list });
                list = false;
            } else if let Some(stream) = Group::optional_angled(input)? {
                let expr = stream.not_empty("missing expr inside <>")?;
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
            return Err(Error::new_spanned(excess.ident, "too many bindings"));
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
            let #path = #tindalwic::walk::Path::wrap(&#branches);
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

pub(crate) struct Walks {
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
