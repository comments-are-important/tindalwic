use super::*;

pub(super) struct SerDe {
    ser: Ident,
    de: Ident,
    value: TokenStream,
    expecting: TokenStream,
    deserialize: TokenStream,
    serialize: TokenStream,
    visitors: TokenStream,
}
impl Parse for SerDe {
    fn parse(input: ParseStream) -> Result<Self> {
        let ii: ItemImpl = input.parse()?;
        let kind = SerDe::validate_parse(&ii)?;
        let kind_str = kind.to_string();
        let ser = Ident::new(&format!("{kind_str}Ser"), Span::call_site());
        let de = Ident::new(&format!("{kind_str}De"), Span::call_site());
        let value = match &kind_str[..] {
            "Comment" => quote!(Option<Comment<'a>>),
            "Items" => quote!(&'a [Cell<Item<'a>>]),
            "Entries" => quote!(&'a [Cell<Entry<'a>>]),
            "Text" => quote!((Value<'a>, Option<Comment<'a>>)),
            "List" => quote!((
                Option<Comment<'a>>,
                &'a [Cell<Item<'a>>],
                Option<Comment<'a>>
            )),
            "Dict" => quote!((
                Option<Comment<'a>>,
                &'a [Cell<Entry<'a>>],
                Option<Comment<'a>>
            )),
            _ => quote!(#kind<'a>),
        };
        let mut expecting = None;
        let mut deserialize = None;
        for attr in &ii.attrs {
            let message = "only: #[expecting=\"...\"] or #[deserialize_*]";
            match &attr.meta {
                Meta::List(list) => {
                    return Err(Error::new_spanned(list, message));
                }
                Meta::NameValue(assign) => {
                    if !assign.path.is_ident("expecting") {
                        return Err(Error::new_spanned(&assign.path, message));
                    }
                    if expecting.is_some() {
                        return Err(Error::new_spanned(&assign.path, "too many expecting"));
                    }
                    expecting = Some(assign.value.to_token_stream());
                }
                Meta::Path(path) => {
                    let Some(ident) = path.get_ident() else {
                        return Err(Error::new_spanned(path, message));
                    };
                    if !ident.to_string().starts_with("deserialize_") {
                        return Err(Error::new_spanned(path, message));
                    }
                    if deserialize.is_some() {
                        return Err(Error::new_spanned(path, "too many deserialize"));
                    }
                    deserialize = Some(match &ident.to_string()[..] {
                        "deserialize_enum" | "deserialize_struct" => match &kind_str[..] {
                            "Item" => quote!(#ident("Item",&["Text","List","Dict"],self)),
                            "Text" => quote!(#ident("Text",&["value","epilog"],self)),
                            "List" => quote!(#ident("List",&["prolog","items","epilog"],self)),
                            "Dict" => quote!(#ident("Dict",&["prolog","entries","epilog"],self)),
                            "Entry" => quote!(#ident("Entry",&["gap","before","key","item"],self)),
                            "File" => quote!(#ident("File",&["hashbang","prolog","entries"],self)),
                            _ => quote!(#ident(self)),
                        },
                        _ => quote! {
                            #ident(self)
                        },
                    });
                }
            }
        }
        let Some(expecting) = expecting else {
            return Err(Error::new_spanned(
                ii.impl_token,
                "need: #[expecting=\"...\"]",
            ));
        };
        let Some(deserialize) = deserialize else {
            return Err(Error::new_spanned(ii.impl_token, "need: #[deserialize_*]"));
        };
        let mut serialize = None;
        let mut visitors = TokenStream::new();
        for item in &ii.items {
            let ImplItem::Fn(f) = item else {
                return Err(Error::new_spanned(item, "not allowed"));
            };
            match &f.sig.ident.to_string()[..] {
                "serialize" => {
                    if serialize.is_some() {
                        return Err(Error::new_spanned(f, "duplicate"));
                    }
                    let body = SerDe::validate_func(&f)?.stmts;
                    serialize = Some(quote!(#(#body)*));
                }
                name if name.starts_with("visit_") => {
                    let ident = &f.sig.ident;
                    let signature = SerDe::visitor_sig(&f);
                    let body = SerDe::validate_func(&f)?.stmts;
                    visitors.extend(quote! {
                        fn #ident #signature {
                            #[allow(unused)]
                            let #de(arena, _) = self;
                            #(#body)*
                        }
                    });
                }
                _ => {
                    return Err(Error::new_spanned(
                        &f.sig.ident,
                        "allowed fns: serialize, visit_*",
                    ));
                }
            }
        }
        let Some(serialize) = serialize else {
            let missing = ii.brace_token.span.close();
            return Err(Error::new(missing, "need: fn serialize() {...}"));
        };
        return Ok(SerDe {
            ser,
            de,
            value,
            expecting,
            deserialize,
            serialize,
            visitors,
        });
    }
}
impl SerDe {
    fn validate_parse(t: &ItemImpl) -> Result<Ident> {
        if let Some(token) = &t.defaultness {
            Err(Error::new_spanned(token, "default not allowed"))
        } else if let Some(token) = &t.unsafety {
            Err(Error::new_spanned(token, "unsafe not allowed"))
        } else if let Some(token) = &t.generics.lt_token {
            Err(Error::new_spanned(token, "generics not allowed"))
        } else if let Some(clause) = &t.generics.where_clause {
            Err(Error::new_spanned(clause, "where clause not allowed"))
        } else if let Some(tr) = &t.trait_ {
            Err(Error::new_spanned(tr.2, "trait not allowed"))
        } else {
            let message = "must be identifier";
            let Type::Path(path) = t.self_ty.as_ref() else {
                return Err(Error::new_spanned(t.self_ty.clone(), message));
            };
            if let Some(qual) = &path.qself {
                return Err(Error::new_spanned(qual.lt_token, message));
            }
            let Some(ident) = path.path.get_ident() else {
                return Err(Error::new_spanned(path, message));
            };
            Ok(ident.clone())
        }
    }
    fn validate_func(f: &ImplItemFn) -> Result<Block> {
        if let Some(first) = &f.attrs.first() {
            Err(Error::new_spanned(first, "attributes not allowed"))
        } else if !matches!(&f.vis, Visibility::Inherited) {
            Err(Error::new_spanned(&f.vis, "visibility not allowed"))
        } else if let Some(token) = &f.defaultness {
            Err(Error::new_spanned(token, "default not allowed"))
        } else if let Some(token) = &f.sig.constness {
            Err(Error::new_spanned(token, "const not allowed"))
        } else if let Some(token) = &f.sig.asyncness {
            Err(Error::new_spanned(token, "async not allowed"))
        } else if let Some(token) = &f.sig.unsafety {
            Err(Error::new_spanned(token, "unsafe not allowed"))
        } else if let Some(abi) = &f.sig.abi {
            Err(Error::new_spanned(abi, "ABI not allowed"))
        } else if let Some(token) = &f.sig.generics.lt_token {
            Err(Error::new_spanned(token, "generics not allowed"))
        } else if let Some(clause) = &f.sig.generics.where_clause {
            Err(Error::new_spanned(clause, "where clause not allowed"))
        } else if !f.sig.inputs.is_empty() {
            return Err(Error::new_spanned(&f.sig.inputs, "params not allowed"));
        } else if let Some(variadic) = &f.sig.variadic {
            Err(Error::new_spanned(variadic, "variadic not allowed"))
        } else if !matches!(&f.sig.output, ReturnType::Default) {
            Err(Error::new_spanned(&f.sig.output, "return type not allowed"))
        } else {
            Ok(f.block.clone())
        }
    }
    fn visitor_sig(visitor: &ImplItemFn) -> TokenStream {
        let name = &visitor.sig.ident;
        match &name.to_string()[..] {
            "visit_str" => quote! {
                <E: ::serde::de::Error>(self,v:&str)->Result<Self::Value,E>
            },
            "visit_borrowed_str" => quote! {
                <E: ::serde::de::Error>(self,v:&'de str)->Result<Self::Value,E>
            },
            "visit_string" => quote! {
                <E: ::serde::de::Error>(self,v: ::alloc::string::String)->Result<Self::Value,E>
            },
            "visit_bytes" => quote! {
                <E: ::serde::de::Error>(self,v:&[u8])->Result<Self::Value,E>
            },
            "visit_borrowed_bytes" => quote! {
                <E: ::serde::de::Error>(self,v:&'de [u8])->Result<Self::Value,E>
            },
            "visit_byte_buf" => quote! {
                <E: ::serde::de::Error>(self,v: ::alloc::vec::Vec<u8>)->Result<Self::Value,E>
            },
            "visit_none" => quote! {
                <E: ::serde::de::Error>(self)->Result<Self::Value,E>
            },
            "visit_some" => quote! {
                <D: ::serde::de::Deserializer<'de>>(self,d:D)->Result<Self::Value,D::Error>
            },
            "visit_unit" => quote! {
                <E: ::serde::de::Error>(self)->Result<Self::Value,E>
            },
            "visit_newtype_struct" => quote! {
                <D: ::serde::de::Deserializer<'de>>(self,d:D)->Result<Self::Value,D::Error>
            },
            "visit_seq" => quote! {
                <A: ::serde::de::SeqAccess<'de>>(self,mut seq:A)->Result<Self::Value,A::Error>
            },
            "visit_map" => quote! {
                <A: ::serde::de::MapAccess<'de>>(self,mut map:A)->Result<Self::Value,A::Error>
            },
            "visit_enum" => quote! {
                <A: ::serde::de::EnumAccess<'de>>(self,data:A)->Result<Self::Value,A::Error>
            },
            other => {
                assert!(
                    other.starts_with("visit_"),
                    "caller asked for sig of non-visitor"
                );
                let kind = Ident::new(&other[6..], name.span());
                quote! {
                    <E: ::serde::de::Error>(self,v:#kind)->Result<Self::Value,E>
                }
            }
        }
    }
}
impl ToTokens for SerDe {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let SerDe {
            ser,
            de,
            value,
            expecting,
            serialize,
            deserialize,
            visitors,
        } = self;
        let tindalwic = tindalwic();
        tokens.extend(quote! {
            struct #ser <'a>(#value);
            impl <'a> #ser <'a> { const EXPECTING: &'static str = #expecting; }
            impl <'a> ::serde::ser::Serialize for #ser <'a> {
                fn serialize<S: ::serde::ser::Serializer>(&self,s:S)->Result<S::Ok,S::Error> {
                    let #ser(this) = self;
                    #serialize
                }
            }
            struct #de<'a, 'ib, IB: #tindalwic::alloc::Intern<'a> + #tindalwic::parse::Builder<'a>>(&'ib mut IB, ::core::marker::PhantomData<&'a()>);
            impl<'a, 'ib, IB: #tindalwic::alloc::Intern<'a> + #tindalwic::parse::Builder<'a>> #de<'a,'ib, IB> {
                fn of(arena:&'ib mut IB) -> Self { #de(arena, ::core::marker::PhantomData) }
            }
            impl<'de, 'a, 'ib, IB: #tindalwic::alloc::Intern<'a> + #tindalwic::parse::Builder<'a>> ::serde::de::DeserializeSeed<'de> for #de<'a, 'ib, IB> {
                type Value = #value ;
                fn deserialize<D: ::serde::de::Deserializer<'de>>(self,d:D)->Result<Self::Value,D::Error>{
                    d.#deserialize
                }
            }
            impl<'de, 'a,'ib, IB: #tindalwic::alloc::Intern<'a> + #tindalwic::parse::Builder<'a>> ::serde::de::Visitor<'de> for #de<'a, 'ib, IB> {
                type Value = #value ;
                fn expecting(&self, out: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
                    out.write_str(#ser::EXPECTING)
                }
                #visitors
            }
        });
    }
}
