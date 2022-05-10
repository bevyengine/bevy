use proc_macro2::{Ident, Span};
use quote::quote;
use syn::parse::{Parse, ParseStream};
use syn::punctuated::Punctuated;
use syn::token::Comma;
use syn::{Meta, NestedMeta, Path};

#[derive(Clone)]
pub enum TraitImpl {
    NotImplemented,
    Implemented,
    Custom(Ident),
}

impl Default for TraitImpl {
    fn default() -> Self {
        Self::NotImplemented
    }
}

#[derive(Default)]
pub struct ReflectAttrs {
    reflect_hash: TraitImpl,
    pub(crate) reflect_partial_eq: TraitImpl,
    serialize: TraitImpl,
    data: Vec<Ident>,
}

impl ReflectAttrs {
    pub fn from_nested_metas(nested_metas: &Punctuated<NestedMeta, Comma>) -> Self {
        let mut attrs = ReflectAttrs::default();
        for nested_meta in nested_metas.iter() {
            match nested_meta {
                NestedMeta::Lit(_) => {}
                NestedMeta::Meta(meta) => match meta {
                    Meta::Path(path) => {
                        if let Some(segment) = path.segments.iter().next() {
                            let ident = segment.ident.to_string();
                            match ident.as_str() {
                                "PartialEq" => attrs.reflect_partial_eq = TraitImpl::Implemented,
                                "Hash" => attrs.reflect_hash = TraitImpl::Implemented,
                                "Serialize" => attrs.serialize = TraitImpl::Implemented,
                                _ => attrs.data.push(Ident::new(
                                    &format!("Reflect{}", segment.ident),
                                    Span::call_site(),
                                )),
                            }
                        }
                    }
                    Meta::List(list) => {
                        let ident = if let Some(segment) = list.path.segments.iter().next() {
                            segment.ident.to_string()
                        } else {
                            continue;
                        };

                        if let Some(list_nested) = list.nested.iter().next() {
                            match list_nested {
                                NestedMeta::Meta(list_nested_meta) => match list_nested_meta {
                                    Meta::Path(path) => {
                                        if let Some(segment) = path.segments.iter().next() {
                                            match ident.as_str() {
                                                "PartialEq" => {
                                                    attrs.reflect_partial_eq =
                                                        TraitImpl::Custom(segment.ident.clone());
                                                }
                                                "Hash" => {
                                                    attrs.reflect_hash =
                                                        TraitImpl::Custom(segment.ident.clone());
                                                }
                                                "Serialize" => {
                                                    attrs.serialize =
                                                        TraitImpl::Custom(segment.ident.clone());
                                                }
                                                _ => {}
                                            }
                                        }
                                    }
                                    Meta::List(_) => {}
                                    Meta::NameValue(_) => {}
                                },
                                NestedMeta::Lit(_) => {}
                            }
                        }
                    }
                    Meta::NameValue(_) => {}
                },
            }
        }

        attrs
    }

    pub fn data(&self) -> &[Ident] {
        &self.data
    }

    pub fn get_hash_impl(&self, path: &Path) -> proc_macro2::TokenStream {
        match &self.reflect_hash {
            TraitImpl::Implemented => quote! {
                use std::hash::{Hash, Hasher};
                let mut hasher = #path::ReflectHasher::default();
                Hash::hash(&std::any::Any::type_id(self), &mut hasher);
                Hash::hash(self, &mut hasher);
                Some(hasher.finish())
            },
            TraitImpl::Custom(impl_fn) => quote! {
                Some(#impl_fn(self))
            },
            TraitImpl::NotImplemented => quote! {
                None
            },
        }
    }

    pub fn get_partial_eq_impl(&self) -> proc_macro2::TokenStream {
        match &self.reflect_partial_eq {
            TraitImpl::Implemented => quote! {
                let value = value.any();
                if let Some(value) = value.downcast_ref::<Self>() {
                    Some(std::cmp::PartialEq::eq(self, value))
                } else {
                    Some(false)
                }
            },
            TraitImpl::Custom(impl_fn) => quote! {
                Some(#impl_fn(self, value))
            },
            TraitImpl::NotImplemented => quote! {
                None
            },
        }
    }

    pub fn get_serialize_impl(&self, path: &Path) -> proc_macro2::TokenStream {
        match &self.serialize {
            TraitImpl::Implemented => quote! {
                Some(#path::serde::Serializable::Borrowed(self))
            },
            TraitImpl::Custom(impl_fn) => quote! {
                Some(#impl_fn(self))
            },
            TraitImpl::NotImplemented => quote! {
                None
            },
        }
    }
}

impl Parse for ReflectAttrs {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let result = Punctuated::<NestedMeta, Comma>::parse_terminated(input)?;
        Ok(ReflectAttrs::from_nested_metas(&result))
    }
}
