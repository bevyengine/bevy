use proc_macro::TokenStream;
use proc_macro2::{Span, TokenStream as TokenStream2};
use quote::quote;
use syn::{parse_macro_input, parse_quote, DeriveInput, Ident, LitStr, Path, Result};

pub fn derive_event(input: TokenStream) -> TokenStream {
    let mut ast = parse_macro_input!(input as DeriveInput);
    let bevy_ecs_path: Path = crate::bevy_ecs_path();

    ast.generics
        .make_where_clause()
        .predicates
        .push(parse_quote! { Self: Send + Sync + 'static });

    let struct_name = &ast.ident;
    let (impl_generics, type_generics, where_clause) = &ast.generics.split_for_impl();

    TokenStream::from(quote! {
        impl #impl_generics #bevy_ecs_path::event::Event for #struct_name #type_generics #where_clause {
        }
    })
}

pub fn derive_resource(input: TokenStream) -> TokenStream {
    let mut ast = parse_macro_input!(input as DeriveInput);
    let bevy_ecs_path: Path = crate::bevy_ecs_path();

    ast.generics
        .make_where_clause()
        .predicates
        .push(parse_quote! { Self: Send + Sync + 'static });

    let struct_name = &ast.ident;
    let (impl_generics, type_generics, where_clause) = &ast.generics.split_for_impl();
    let mut register_type: Option<proc_macro2::TokenStream> = None;

    #[cfg(feature = "bevy_reflect")]
    if has_reflect_attr(&ast, "Resource") {
        register_type = Some(quote! {
            #[doc(hidden)]
            fn __register_type(registry: &#bevy_ecs_path::private::bevy_reflect::TypeRegistryArc) {
                #bevy_ecs_path::private::bevy_reflect::register_type_shim::<Self>(registry);
            }
        });
    }

    TokenStream::from(quote! {
        impl #impl_generics #bevy_ecs_path::system::Resource for #struct_name #type_generics #where_clause {
            #register_type
        }
    })
}

pub fn derive_component(input: TokenStream) -> TokenStream {
    let mut ast = parse_macro_input!(input as DeriveInput);
    let bevy_ecs_path: Path = crate::bevy_ecs_path();

    let attrs = match parse_component_attr(&ast) {
        Ok(attrs) => attrs,
        Err(e) => return e.into_compile_error().into(),
    };

    let storage = storage_path(&bevy_ecs_path, attrs.storage);

    ast.generics
        .make_where_clause()
        .predicates
        .push(parse_quote! { Self: Send + Sync + 'static });

    let struct_name = &ast.ident;
    let (impl_generics, type_generics, where_clause) = &ast.generics.split_for_impl();
    let mut register_type: Option<proc_macro2::TokenStream> = None;

    #[cfg(feature = "bevy_reflect")]
    if has_reflect_attr(&ast, "Component") {
        register_type = Some(quote! {
            #[doc(hidden)]
            fn __register_type(registry: &#bevy_ecs_path::private::bevy_reflect::TypeRegistryArc) {
                #bevy_ecs_path::private::bevy_reflect::register_type_shim::<Self>(registry);
            }
        });
    }

    TokenStream::from(quote! {
        impl #impl_generics #bevy_ecs_path::component::Component for #struct_name #type_generics #where_clause {
            const STORAGE_TYPE: #bevy_ecs_path::component::StorageType = #storage;
            #register_type
        }
    })
}

pub const COMPONENT: &str = "component";
pub const STORAGE: &str = "storage";

struct Attrs {
    storage: StorageTy,
}

#[derive(Clone, Copy)]
enum StorageTy {
    Table,
    SparseSet,
}

// values for `storage` attribute
const TABLE: &str = "Table";
const SPARSE_SET: &str = "SparseSet";

fn parse_component_attr(ast: &DeriveInput) -> Result<Attrs> {
    let mut attrs = Attrs {
        storage: StorageTy::Table,
    };

    for meta in ast.attrs.iter().filter(|a| a.path().is_ident(COMPONENT)) {
        meta.parse_nested_meta(|nested| {
            if nested.path.is_ident(STORAGE) {
                attrs.storage = match nested.value()?.parse::<LitStr>()?.value() {
                    s if s == TABLE => StorageTy::Table,
                    s if s == SPARSE_SET => StorageTy::SparseSet,
                    s => {
                        return Err(nested.error(format!(
                            "Invalid storage type `{s}`, expected '{TABLE}' or '{SPARSE_SET}'.",
                        )));
                    }
                };
                Ok(())
            } else {
                Err(nested.error("Unsupported attribute"))
            }
        })?;
    }

    Ok(attrs)
}

#[cfg(feature = "bevy_reflect")]
fn has_reflect_attr(ast: &DeriveInput, reflect_trait: &'static str) -> bool {
    use syn::Meta;

    // Generics require the generic parameters to implement Reflect
    // This is unsupported for now.
    if !ast.generics.params.is_empty() {
        return false;
    }

    const REFLECT: &str = "reflect";
    ast.attrs.iter().any(|attr| {
        if !attr.path().is_ident(REFLECT) || !matches!(attr.meta, Meta::List(_)) {
            return false;
        }

        attr.parse_nested_meta(|meta| {
            meta.path.is_ident(reflect_trait)
                .then(|| ())
                .ok_or_else(|| meta.error("missing required reflect attribute"))
        })
        .is_ok()
    })
}

fn storage_path(bevy_ecs_path: &Path, ty: StorageTy) -> TokenStream2 {
    let storage_type = match ty {
        StorageTy::Table => Ident::new("Table", Span::call_site()),
        StorageTy::SparseSet => Ident::new("SparseSet", Span::call_site()),
    };

    quote! { #bevy_ecs_path::component::StorageType::#storage_type }
}
