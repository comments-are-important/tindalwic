use super::*;

/// Using a bool field instead of two-variant enum to make parsing easier.
struct Branch {
    entry: bool,        // true means `Entry`, false means `Item`.
    index: TokenStream, // unparsed (hopefully produces either `usize` or `Key`)
}
impl ToTokens for Branch {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let tindalwic = tindalwic();
        let index = &self.index;
        if self.entry {
            tokens.extend(quote!(#tindalwic::walk::Branch::Entry((#index).into())));
        } else {
            tokens.extend(quote!(#tindalwic::walk::Branch::Item(#index)));
        }
    }
}

pub(super) struct Path {
    steps: Vec<Branch>, // Entry/Item
    lands: Ident,       // Text/List/Dict
}
impl Parse for Path {
    fn parse(input: ParseStream) -> Result<Self> {
        let mut steps = Vec::new();
        while !input.is_empty() {
            if let Some(stream) = Group::optional_bracketed(input)? {
                let expr = stream.not_empty("missing expr inside []")?;
                steps.push(Branch {
                    index: expr,
                    entry: false,
                });
            } else if let Some(stream) = Group::optional_braced(input)? {
                let expr = stream.not_empty("missing expr inside {}")?;
                steps.push(Branch {
                    index: expr,
                    entry: true,
                });
            } else {
                break;
            }
        }
        if steps.is_empty() {
            return Err(input.error("requires at least one step"));
        }
        let lands = input.parse()?;
        Ok(Path { steps, lands })
    }
}
impl ToTokens for Path {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let Path { steps, lands } = self;
        let tindalwic = tindalwic();
        let entry = steps.last().expect("checked in parse").entry;
        tokens.extend(quote! {
            #tindalwic::walk::Path::<#entry>::new(&[
                #(#steps),*, #tindalwic::walk::Branch::#lands
            ])
        });
    }
}
