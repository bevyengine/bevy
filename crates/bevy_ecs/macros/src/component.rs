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
    DeriveInput, ExprClosure, ExprPath, Ident, LitStr, Path, Result,
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
            type Traversal = ();
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

    let RelatedComponentsReturn {
        register_related: register_suggested,
        register_recursive_related: register_recursive_suggested,
        docs: suggested_component_docs,
    } = related_components(&attrs.suggests, Relatedness::Suggested);
    let RelatedComponentsReturn {
        register_related: register_included,
        register_recursive_related: register_recursive_included,
        docs: included_component_docs,
    } = related_components(&attrs.includes, Relatedness::Included);
    let RelatedComponentsReturn {
        register_related: register_required,
        register_recursive_related: register_recursive_required,
        docs: required_component_docs,
    } = related_components(&attrs.requires, Relatedness::Required);

    let struct_name = &ast.ident;
    let (impl_generics, type_generics, where_clause) = &ast.generics.split_for_impl();

    // This puts `register_required` before `register_recursive_requires` to ensure that the constructors of _all_ top
    // level components are initialized first, giving them precedence over recursively defined constructors for the same component type
    TokenStream::from(quote! {
        #required_component_docs
        #included_component_docs
        #suggested_component_docs
        impl #impl_generics #bevy_ecs_path::component::Component for #struct_name #type_generics #where_clause {
            const STORAGE_TYPE: #bevy_ecs_path::component::StorageType = #storage;
            fn register_related_components(
                requiree: #bevy_ecs_path::component::ComponentId,
                components: &mut #bevy_ecs_path::component::Components,
                storages: &mut #bevy_ecs_path::storage::Storages,
                required_components: &mut #bevy_ecs_path::component::RelatedComponents,
                inheritance_depth: u16,
            ) {
                #(#register_suggested)*
                #(#register_recursive_suggested)*

                #(#register_included)*
                #(#register_recursive_included)*

                #(#register_required)*
                #(#register_recursive_required)*
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

// Because the macro crate cannot access the `bevy_ecs` crate, we need to create our own equivalent.
// This should be kept in sync with the actual `bevy_ecs` crate!
enum Relatedness {
    Suggested,
    Included,
    Required,
}

impl Relatedness {
    /// Returns the token that represents the corresponding `Relatedness` enum variant in `bevy_ecs`.
    fn token(&self) -> TokenStream2 {
        let bevy_ecs_path: Path = crate::bevy_ecs_path();
        match self {
            Relatedness::Suggested => quote! { #bevy_ecs_path::component::Relatedness::Included },
            Relatedness::Included => quote! { #bevy_ecs_path::component::Relatedness::Included },
            Relatedness::Required => quote! { #bevy_ecs_path::component::Relatedness::Required },
        }
    }

    /// Returns the stringified name of this `Relatedness` enum variant for use in docs.
    fn doc_name(&self) -> &'static str {
        match self {
            Relatedness::Suggested => "Suggested",
            Relatedness::Included => "Included",
            Relatedness::Required => "Required",
        }
    }

    /// Returns the doc string fragment that explains the corresponding `Relatedness` enum variant.
    fn doc_explanation(&self) -> &'static str {
        match self {
            Relatedness::Suggested => "This component might work well with the components listed above, unlocking new functionality.",
            Relatedness::Included => "A component's Included Components are inserted whenever it is inserted. Note that this will also insert the included components _of_ the included components, recursively, in depth-first order.",
            Relatedness::Required => "A component's Required Components are inserted whenever it is inserted. Note that this will also insert the required components _of_ the required components, recursively, in depth-first order. Unlike included components, this relationship cannot be removed.",
        }
    }
}

struct RelatedComponentsReturn {
    register_related: Vec<TokenStream2>,
    register_recursive_related: Vec<TokenStream2>,
    docs: Option<TokenStream2>,
}

/// Generates the code needed to add related components to the component's related components list.
fn related_components(
    attribute: &Option<Punctuated<Related, Comma>>,
    relatedness: Relatedness,
) -> RelatedComponentsReturn {
    let mut register_related = Vec::with_capacity(attribute.iter().len());
    let mut register_recursive_related = Vec::with_capacity(attribute.iter().len());

    let relatedness_token = relatedness.token();

    if let Some(related) = attribute {
        for require in related {
            let ident = &require.path;
            register_recursive_related.push(quote! {
                <#ident as Component>::register_related_components(
                    requiree,
                    components,
                    storages,
                    required_components,
                    inheritance_depth + 1,
                );
            });
            match &require.func {
                Some(RequireFunc::Path(func)) => {
                    register_related.push(quote! {
                        components.register_related_components_manual::<Self, #ident>(
                            storages,
                            required_components,
                            || { let x: #ident = #func().into(); x },
                            inheritance_depth,
                            #relatedness_token,
                        );
                    });
                }
                Some(RequireFunc::Closure(func)) => {
                    register_related.push(quote! {
                        components.register_related_components_manual::<Self, #ident>(
                            storages,
                            required_components,
                            || { let x: #ident = (#func)().into(); x },
                            inheritance_depth,
                            #relatedness_token,
                        );
                    });
                }
                None => {
                    register_related.push(quote! {
                        components.register_related_components_manual::<Self, #ident>(
                            storages,
                            required_components,
                            <#ident as Default>::default,
                            inheritance_depth,
                            #relatedness_token,
                        );
                    });
                }
            }
        }
    }

    let docs = attribute.as_ref().map(|r| {
        let paths = r
            .iter()
            .map(|r| format!("[`{}`]", r.path.to_token_stream()))
            .collect::<Vec<_>>()
            .join(", ");
        let doc_name = relatedness.doc_name();
        let doc_explanation = relatedness.doc_explanation();
        let doc = format!("{doc_name}: {paths}. \n\n {doc_explanation}");
        quote! {
            #[doc = #doc]
        }
    });

    RelatedComponentsReturn {
        register_related,
        register_recursive_related,
        docs,
    }
}

pub const COMPONENT: &str = "component";
pub const STORAGE: &str = "storage";

pub const SUGGEST: &str = "suggest";
pub const INCLUDE: &str = "include";
pub const REQUIRE: &str = "require";

pub const ON_ADD: &str = "on_add";
pub const ON_INSERT: &str = "on_insert";
pub const ON_REPLACE: &str = "on_replace";
pub const ON_REMOVE: &str = "on_remove";

struct Attrs {
    storage: StorageTy,
    suggests: Option<Punctuated<Related, Comma>>,
    includes: Option<Punctuated<Related, Comma>>,
    requires: Option<Punctuated<Related, Comma>>,
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

struct Related {
    path: Path,
    func: Option<RequireFunc>,
}

enum RequireFunc {
    Path(Path),
    Closure(ExprClosure),
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
        suggests: None,
        includes: None,
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
        } else if attr.path().is_ident(SUGGEST) {
            let punctuated =
                attr.parse_args_with(Punctuated::<Related, Comma>::parse_terminated)?;
            for suggest in punctuated.iter() {
                if !require_paths.insert(suggest.path.to_token_stream().to_string()) {
                    return Err(syn::Error::new(
                        suggest.path.span(),
                        "Duplicate suggested components are not allowed.",
                    ));
                }
            }
            if let Some(current) = &mut attrs.suggests {
                current.extend(punctuated);
            } else {
                attrs.suggests = Some(punctuated);
            }
        } else if attr.path().is_ident(INCLUDE) {
            let punctuated =
                attr.parse_args_with(Punctuated::<Related, Comma>::parse_terminated)?;
            for include in punctuated.iter() {
                if !require_paths.insert(include.path.to_token_stream().to_string()) {
                    return Err(syn::Error::new(
                        include.path.span(),
                        "Duplicate included components are not allowed.",
                    ));
                }
            }
            if let Some(current) = &mut attrs.includes {
                current.extend(punctuated);
            } else {
                attrs.includes = Some(punctuated);
            }
        } else if attr.path().is_ident(REQUIRE) {
            let punctuated =
                attr.parse_args_with(Punctuated::<Related, Comma>::parse_terminated)?;
            for require in punctuated.iter() {
                if !require_paths.insert(require.path.to_token_stream().to_string()) {
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

impl Parse for Related {
    fn parse(input: syn::parse::ParseStream) -> Result<Self> {
        let path = input.parse::<Path>()?;
        let func = if input.peek(Paren) {
            let content;
            parenthesized!(content in input);
            if let Ok(func) = content.parse::<ExprClosure>() {
                Some(RequireFunc::Closure(func))
            } else {
                let func = content.parse::<Path>()?;
                Some(RequireFunc::Path(func))
            }
        } else {
            None
        };
        Ok(Related { path, func })
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
