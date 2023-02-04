use bevy_macro_utils::{get_lit_str, Symbol};
use proc_macro::TokenStream;
use proc_macro2::{Punct, Spacing, Span, TokenStream as TokenStream2};
use quote::{quote, TokenStreamExt, ToTokens};
use syn::{parse_macro_input, parse_quote, DeriveInput, Error, Ident, Path, Result};
use syn::spanned::Spanned;

pub fn derive_resource(input: TokenStream) -> TokenStream {
    let mut ast = parse_macro_input!(input as DeriveInput);
    let bevy_ecs_path: Path = crate::bevy_ecs_path();

    let attrs = match parse_resource_attrs(&ast) {
        Ok(attrs) => attrs,
        Err(e) => return e.into_compile_error().into(),
    };

    let change_detection_mode = attrs.change_detection_mode;
    let change_detection_token = if matches!(change_detection_mode, ChangeDetectionMode::DerefMut) {
        quote!()
    } else {
        quote!(const CHANGE_DETECTION_MODE: ChangeDetectionMode = #change_detection_mode;)
    };

    ast.generics
        .make_where_clause()
        .predicates
        .push(parse_quote! { Self: Send + Sync + 'static });

    let struct_name = &ast.ident;
    let (impl_generics, type_generics, where_clause) = &ast.generics.split_for_impl();

    TokenStream::from(quote! {
        impl #impl_generics #bevy_ecs_path::system::Resource for #struct_name #type_generics #where_clause {
            #change_detection_token
        }
    })
}

pub fn derive_component(input: TokenStream) -> TokenStream {
    let mut ast = parse_macro_input!(input as DeriveInput);
    let bevy_ecs_path: Path = crate::bevy_ecs_path();

    let attrs = match parse_component_attrs(&ast) {
        Ok(attrs) => attrs,
        Err(e) => return e.into_compile_error().into(),
    };

    let change_detection_mode = attrs.change_detection_mode;
    let change_detection_token = if matches!(change_detection_mode, ChangeDetectionMode::DerefMut) {
        quote!()
    } else {
        quote!(const CHANGE_DETECTION_MODE: ChangeDetectionMode = #change_detection_mode;)
    };
    let component_mut = if matches!(change_detection_mode, ChangeDetectionMode::Disabled) {
        quote! { &'a mut Self }
    } else {
        quote! { #bevy_ecs_path::change_detection::Mut<'a, Self> }
    };
    let storage = storage_path(&bevy_ecs_path, attrs.storage);

    ast.generics
        .make_where_clause()
        .predicates
        .push(parse_quote! { Self: Send + Sync + 'static });

    let struct_name = &ast.ident;
    let (impl_generics, type_generics, where_clause) = &ast.generics.split_for_impl();

    TokenStream::from(quote! {
        impl #impl_generics #bevy_ecs_path::component::Component for #struct_name #type_generics #where_clause {
            #change_detection_token
            type WriteFetch<'a> = #component_mut;
            type Storage = #storage;
            fn shrink<'wlong: 'wshort, 'wshort>(item: Self::WriteFetch<'wlong>) -> Self::WriteFetch<'wshort> {
                item
            }
        }
    })
}

pub const COMPONENT: Symbol = Symbol("component");
pub const RESOURCE: Symbol = Symbol("resource");
pub const CHANGE_DETECTION_MODE: Symbol = Symbol("change_detection_mode");
pub const STORAGE: Symbol = Symbol("storage");

/// Defines the behaviour of change detection
enum ChangeDetectionMode {
    /// Trigger change detection if the object is mutably dereferenced
    DerefMut,
    /// Trigger change detection if the new and old values are not equal according to PartialEq
    PartialEq,
    /// Trigger change detection if the new and old values are not equal according to PartialEq
    Eq,
    /// Disable change detection
    Disabled,
}

impl ToTokens for ChangeDetectionMode {
    fn to_tokens(&self, tokens: &mut TokenStream2) {
        // "ChangeDetectionMode".to_tokens(tokens);
        tokens.append(Ident::new("ChangeDetectionMode", tokens.span()));
        tokens.append(Punct::new(':', Spacing::Joint));
        tokens.append(Punct::new(':', Spacing::Alone));
        match self {
            Self::DerefMut => tokens.append(Ident::new("DerefMut", tokens.span())),
            Self::PartialEq => tokens.append(Ident::new("PartialEq", tokens.span())),
            Self::Eq => tokens.append(Ident::new("Eq", tokens.span())),
            Self::Disabled => tokens.append(Ident::new("Disabled", tokens.span())),
        }
    }
}

struct ComponentAttrs {
    change_detection_mode: ChangeDetectionMode,
    storage: StorageTy,
}

struct ResourceAttrs {
    change_detection_mode: ChangeDetectionMode,
}

#[derive(Clone, Copy)]
enum StorageTy {
    Table,
    SparseSet,
}

// values for `storage` attribute
const TABLE: &str = "Table";
const SPARSE_SET: &str = "SparseSet";

fn parse_component_attrs(ast: &DeriveInput) -> Result<ComponentAttrs> {
    let meta_items = bevy_macro_utils::parse_attrs(ast, COMPONENT)?;

    let mut attrs = ComponentAttrs {
        change_detection_mode: ChangeDetectionMode::DerefMut,
        storage: StorageTy::Table,
    };

    for meta in meta_items {
        use syn::{
            Meta::NameValue,
            NestedMeta::{Lit, Meta},
        };
        match meta {
            Meta(NameValue(m)) if m.path == STORAGE => {
                attrs.storage = match get_lit_str(STORAGE, &m.lit)?.value().as_str() {
                    TABLE => StorageTy::Table,
                    SPARSE_SET => StorageTy::SparseSet,
                    s => {
                        return Err(Error::new_spanned(
                            m.lit,
                            format!(
                                "Invalid storage type `{s}`, expected '{TABLE}' or '{SPARSE_SET}'.",
                            ),
                        ))
                    }
                };
            }
            Meta(NameValue(m)) if m.path == CHANGE_DETECTION_MODE => {
                attrs.change_detection_mode = match m.lit {
                    syn::Lit::Str(value) => match value.value().as_str() {
                        "PartialEq" => ChangeDetectionMode::PartialEq,
                        "Eq" => ChangeDetectionMode::Eq,
                        "Disabled" => ChangeDetectionMode::Disabled,
                        _ => {
                            return Err(Error::new_spanned(
                                value,
                                "Change detection mode must be a string among ['PartialEq', 'Eq', 'Disabled'].",
                            ))
                        }
                    }
                    s => {
                        return Err(Error::new_spanned(
                            s,
                            "Change detection mode must be a string among ['PartialEq', 'Eq', 'Disabled'].",
                        ))
                    }
                };
            }
            Meta(meta_item) => {
                return Err(Error::new_spanned(
                    meta_item.path(),
                    format!(
                        "unknown component attribute `{}`",
                        meta_item.path().into_token_stream()
                    ),
                ));
            }
            Lit(lit) => {
                return Err(Error::new_spanned(
                    lit,
                    "unexpected literal in component attribute",
                ))
            }
        }
    }

    Ok(attrs)
}

fn parse_resource_attrs(ast: &DeriveInput) -> Result<ResourceAttrs> {
    let meta_items = bevy_macro_utils::parse_attrs(ast, RESOURCE)?;

    let mut attrs = ResourceAttrs {
        change_detection_mode: ChangeDetectionMode::DerefMut,
    };

    for meta in meta_items {
        use syn::{
            Meta::NameValue,
            NestedMeta::{Lit, Meta},
        };
        match meta {
            Meta(NameValue(m)) if m.path == CHANGE_DETECTION_MODE => {
                attrs.change_detection_mode = match m.lit {
                    syn::Lit::Str(value) => match value.value().as_str() {
                        "PartialEq" => ChangeDetectionMode::PartialEq,
                        "Eq" => ChangeDetectionMode::Eq,
                        "Disabled" => ChangeDetectionMode::Disabled,
                        _ => {
                            return Err(Error::new_spanned(
                                value,
                                "Change detection mode must be a string among ['PartialEq', 'Eq', 'Disabled'].",
                            ))
                        }
                    }
                    s => {
                        return Err(Error::new_spanned(
                            s,
                            "Change detection mode must be a string among ['PartialEq', 'Eq', 'Disabled'].",
                        ))
                    }
                };
            }
            Meta(meta_item) => {
                return Err(Error::new_spanned(
                    meta_item.path(),
                    format!(
                        "unknown resource attribute `{}`",
                        meta_item.path().into_token_stream()
                    ),
                ));
            }
            Lit(lit) => {
                return Err(Error::new_spanned(
                    lit,
                    "unexpected literal in resource attribute",
                ))
            }
        }
    }

    Ok(attrs)
}

fn storage_path(bevy_ecs_path: &Path, ty: StorageTy) -> TokenStream2 {
    let typename = match ty {
        StorageTy::Table => Ident::new("TableStorage", Span::call_site()),
        StorageTy::SparseSet => Ident::new("SparseStorage", Span::call_site()),
    };

    quote! { #bevy_ecs_path::component::#typename }
}
