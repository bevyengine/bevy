use proc_macro::TokenStream;
use proc_macro2::{Span, TokenStream as TokenStream2};
use quote::{quote, ToTokens};
use std::collections::HashSet;
use syn::{
    parenthesized,
    parse::Parse,
    parse_macro_input, parse_quote,
    punctuated::Punctuated,
    spanned::Spanned,
    token::{Comma, Paren},
    DeriveInput, ExprPath, Ident, LitStr, Path, Result,
};

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
            type Traversal = #bevy_ecs_path::traversal::TraverseNone;
            const AUTO_PROPAGATE: bool = false;
        }

        impl #impl_generics #bevy_ecs_path::component::Component for #struct_name #type_generics #where_clause {
            const STORAGE_TYPE: #bevy_ecs_path::component::StorageType = #bevy_ecs_path::component::StorageType::SparseSet;
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

    TokenStream::from(quote! {
        impl #impl_generics #bevy_ecs_path::system::Resource for #struct_name #type_generics #where_clause {
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

    let on_add = hook_register_function_call(quote! {on_add}, attrs.on_add);
    let on_insert = hook_register_function_call(quote! {on_insert}, attrs.on_insert);
    let on_replace = hook_register_function_call(quote! {on_replace}, attrs.on_replace);
    let on_remove = hook_register_function_call(quote! {on_remove}, attrs.on_remove);

    ast.generics
        .make_where_clause()
        .predicates
        .push(parse_quote! { Self: Send + Sync + 'static });

    let requires = &attrs.requires;
    let mut register_required = Vec::with_capacity(attrs.requires.iter().len());
    let mut register_recursive_requires = Vec::with_capacity(attrs.requires.iter().len());
    if let Some(requires) = requires {
        for require in requires {
            let ident = &require.path;
            register_recursive_requires.push(quote! {
                <#ident as Component>::register_required_components(components, storages, required_components);
            });
            if let Some(func) = &require.func {
                register_required.push(quote! {
                    required_components.register(components, storages, || { let x: #ident = #func().into(); x });
                });
            } else {
                register_required.push(quote! {
                    required_components.register(components, storages, <#ident as Default>::default);
                });
            }
        }
    }
    let struct_name = &ast.ident;
    let (impl_generics, type_generics, where_clause) = &ast.generics.split_for_impl();

    let required_component_docs = attrs.requires.map(|r| {
        let paths = r
            .iter()
            .map(|r| format!("[`{}`]", r.path.to_token_stream()))
            .collect::<Vec<_>>()
            .join(", ");
        let doc = format!("Required Components: {paths}. \n\n A component's Required Components are inserted whenever it is inserted. Note that this will also insert the required components _of_ the required components, recursively.");
        quote! {
            #[doc = #doc]
        }
    });

    // This puts `register_required` before `register_recursive_requires` to ensure that the constructors of _all_ top
    // level components are initialized first, giving them precedence over recursively defined constructors for the same component type
    TokenStream::from(quote! {
        #required_component_docs
        impl #impl_generics #bevy_ecs_path::component::Component for #struct_name #type_generics #where_clause {
            const STORAGE_TYPE: #bevy_ecs_path::component::StorageType = #storage;
            fn register_required_components(
                components: &mut #bevy_ecs_path::component::Components,
                storages: &mut #bevy_ecs_path::storage::Storages,
                required_components: &mut #bevy_ecs_path::component::RequiredComponents
            ) {
                #(#register_required)*
                #(#register_recursive_requires)*
            }

            #[allow(unused_variables)]
            fn register_component_hooks(hooks: &mut #bevy_ecs_path::component::ComponentHooks) {
                #on_add
                #on_insert
                #on_replace
                #on_remove
            }
        }
    })
}

pub const COMPONENT: &str = "component";
pub const STORAGE: &str = "storage";
pub const REQUIRE: &str = "require";

pub const ON_ADD: &str = "on_add";
pub const ON_INSERT: &str = "on_insert";
pub const ON_REPLACE: &str = "on_replace";
pub const ON_REMOVE: &str = "on_remove";

struct Attrs {
    storage: StorageTy,
    requires: Option<Punctuated<Require, Comma>>,
    on_add: Option<ExprPath>,
    on_insert: Option<ExprPath>,
    on_replace: Option<ExprPath>,
    on_remove: Option<ExprPath>,
}

#[derive(Clone, Copy)]
enum StorageTy {
    Table,
    SparseSet,
}

struct Require {
    path: Path,
    func: Option<Path>,
}

// values for `storage` attribute
const TABLE: &str = "Table";
const SPARSE_SET: &str = "SparseSet";

fn parse_component_attr(ast: &DeriveInput) -> Result<Attrs> {
    let mut attrs = Attrs {
        storage: StorageTy::Table,
        on_add: None,
        on_insert: None,
        on_replace: None,
        on_remove: None,
        requires: None,
    };

    let mut require_paths = HashSet::new();
    for attr in ast.attrs.iter() {
        if attr.path().is_ident(COMPONENT) {
            attr.parse_nested_meta(|nested| {
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
                } else if nested.path.is_ident(ON_ADD) {
                    attrs.on_add = Some(nested.value()?.parse::<ExprPath>()?);
                    Ok(())
                } else if nested.path.is_ident(ON_INSERT) {
                    attrs.on_insert = Some(nested.value()?.parse::<ExprPath>()?);
                    Ok(())
                } else if nested.path.is_ident(ON_REPLACE) {
                    attrs.on_replace = Some(nested.value()?.parse::<ExprPath>()?);
                    Ok(())
                } else if nested.path.is_ident(ON_REMOVE) {
                    attrs.on_remove = Some(nested.value()?.parse::<ExprPath>()?);
                    Ok(())
                } else {
                    Err(nested.error("Unsupported attribute"))
                }
            })?;
        } else if attr.path().is_ident(REQUIRE) {
            let punctuated =
                attr.parse_args_with(Punctuated::<Require, Comma>::parse_terminated)?;
            for require in punctuated.iter() {
                if !require_paths.insert(require.path.clone()) {
                    return Err(syn::Error::new(
                        require.path.span(),
                        "Duplicate required components are not allowed.",
                    ));
                }
            }
            if let Some(current) = &mut attrs.requires {
                current.extend(punctuated);
            } else {
                attrs.requires = Some(punctuated);
            }
        }
    }

    Ok(attrs)
}

impl Parse for Require {
    fn parse(input: syn::parse::ParseStream) -> Result<Self> {
        let path = input.parse::<Path>()?;
        let func = if input.peek(Paren) {
            let content;
            parenthesized!(content in input);
            let func = content.parse::<Path>()?;
            Some(func)
        } else {
            None
        };
        Ok(Require { path, func })
    }
}

fn storage_path(bevy_ecs_path: &Path, ty: StorageTy) -> TokenStream2 {
    let storage_type = match ty {
        StorageTy::Table => Ident::new("Table", Span::call_site()),
        StorageTy::SparseSet => Ident::new("SparseSet", Span::call_site()),
    };

    quote! { #bevy_ecs_path::component::StorageType::#storage_type }
}

fn hook_register_function_call(
    hook: TokenStream2,
    function: Option<ExprPath>,
) -> Option<TokenStream2> {
    function.map(|meta| quote! { hooks. #hook (#meta); })
}
