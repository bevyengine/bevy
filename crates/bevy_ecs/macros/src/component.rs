use proc_macro::TokenStream;
use proc_macro2::{Span, TokenStream as TokenStream2};
use quote::{format_ident, quote, ToTokens};
use std::collections::HashSet;
use syn::{
    braced, parenthesized,
    parse::Parse,
    parse_macro_input, parse_quote,
    punctuated::Punctuated,
    spanned::Spanned,
    token::{Brace, Comma, Paren},
    Data, DataEnum, DataStruct, DeriveInput, Expr, ExprCall, ExprPath, Field, Fields, Ident,
    LitStr, Member, Path, Result, Token, Type, Visibility,
};

pub const EVENT: &str = "entity_event";
pub const AUTO_PROPAGATE: &str = "auto_propagate";
pub const TRAVERSAL: &str = "traversal";

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
        impl #impl_generics #bevy_ecs_path::event::Event for #struct_name #type_generics #where_clause {}
    })
}

pub fn derive_entity_event(input: TokenStream) -> TokenStream {
    let mut ast = parse_macro_input!(input as DeriveInput);
    let mut auto_propagate = false;
    let mut traversal: Type = parse_quote!(());
    let bevy_ecs_path: Path = crate::bevy_ecs_path();

    let mut processed_attrs = Vec::new();

    ast.generics
        .make_where_clause()
        .predicates
        .push(parse_quote! { Self: Send + Sync + 'static });

    for attr in ast.attrs.iter().filter(|attr| attr.path().is_ident(EVENT)) {
        if let Err(e) = attr.parse_nested_meta(|meta| match meta.path.get_ident() {
            Some(ident) if processed_attrs.iter().any(|i| ident == i) => {
                Err(meta.error(format!("duplicate attribute: {ident}")))
            }
            Some(ident) if ident == AUTO_PROPAGATE => {
                auto_propagate = true;
                processed_attrs.push(AUTO_PROPAGATE);
                Ok(())
            }
            Some(ident) if ident == TRAVERSAL => {
                traversal = meta.value()?.parse()?;
                processed_attrs.push(TRAVERSAL);
                Ok(())
            }
            Some(ident) => Err(meta.error(format!("unsupported attribute: {ident}"))),
            None => Err(meta.error("expected identifier")),
        }) {
            return e.to_compile_error().into();
        }
    }

    let struct_name = &ast.ident;
    let (impl_generics, type_generics, where_clause) = &ast.generics.split_for_impl();

    TokenStream::from(quote! {
        impl #impl_generics #bevy_ecs_path::event::Event for #struct_name #type_generics #where_clause {}
        impl #impl_generics #bevy_ecs_path::event::EntityEvent for #struct_name #type_generics #where_clause {
            type Traversal = #traversal;
            const AUTO_PROPAGATE: bool = #auto_propagate;
        }
    })
}

pub fn derive_buffered_event(input: TokenStream) -> TokenStream {
    let mut ast = parse_macro_input!(input as DeriveInput);
    let bevy_ecs_path: Path = crate::bevy_ecs_path();

    ast.generics
        .make_where_clause()
        .predicates
        .push(parse_quote! { Self: Send + Sync + 'static });

    let struct_name = &ast.ident;
    let (impl_generics, type_generics, where_clause) = &ast.generics.split_for_impl();

    TokenStream::from(quote! {
        impl #impl_generics #bevy_ecs_path::event::BufferedEvent for #struct_name #type_generics #where_clause {}
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
        impl #impl_generics #bevy_ecs_path::resource::Resource for #struct_name #type_generics #where_clause {
        }
    })
}

/// Component derive syntax is documented on both the macro and the trait.
pub fn derive_component(input: TokenStream) -> TokenStream {
    let mut ast = parse_macro_input!(input as DeriveInput);
    let bevy_ecs_path: Path = crate::bevy_ecs_path();

    let attrs = match parse_component_attr(&ast) {
        Ok(attrs) => attrs,
        Err(e) => return e.into_compile_error().into(),
    };

    let relationship = match derive_relationship(&ast, &attrs, &bevy_ecs_path) {
        Ok(value) => value,
        Err(err) => err.into_compile_error().into(),
    };
    let relationship_target = match derive_relationship_target(&ast, &attrs, &bevy_ecs_path) {
        Ok(value) => value,
        Err(err) => err.into_compile_error().into(),
    };

    let map_entities = map_entities(
        &ast.data,
        &bevy_ecs_path,
        Ident::new("this", Span::call_site()),
        relationship.is_some(),
        relationship_target.is_some(),
        attrs.map_entities
    ).map(|map_entities_impl| quote! {
        fn map_entities<M: #bevy_ecs_path::entity::EntityMapper>(this: &mut Self, mapper: &mut M) {
            use #bevy_ecs_path::entity::MapEntities;
            #map_entities_impl
        }
    });

    let storage = storage_path(&bevy_ecs_path, attrs.storage);

    let on_add_path = attrs
        .on_add
        .map(|path| path.to_token_stream(&bevy_ecs_path));
    let on_remove_path = attrs
        .on_remove
        .map(|path| path.to_token_stream(&bevy_ecs_path));

    let on_insert_path = if relationship.is_some() {
        if attrs.on_insert.is_some() {
            return syn::Error::new(
                ast.span(),
                "Custom on_insert hooks are not supported as relationships already define an on_insert hook",
            )
            .into_compile_error()
            .into();
        }

        Some(quote!(<Self as #bevy_ecs_path::relationship::Relationship>::on_insert))
    } else {
        attrs
            .on_insert
            .map(|path| path.to_token_stream(&bevy_ecs_path))
    };

    let on_replace_path = if relationship.is_some() {
        if attrs.on_replace.is_some() {
            return syn::Error::new(
                ast.span(),
                "Custom on_replace hooks are not supported as Relationships already define an on_replace hook",
            )
            .into_compile_error()
            .into();
        }

        Some(quote!(<Self as #bevy_ecs_path::relationship::Relationship>::on_replace))
    } else if attrs.relationship_target.is_some() {
        if attrs.on_replace.is_some() {
            return syn::Error::new(
                ast.span(),
                "Custom on_replace hooks are not supported as RelationshipTarget already defines an on_replace hook",
            )
            .into_compile_error()
            .into();
        }

        Some(quote!(<Self as #bevy_ecs_path::relationship::RelationshipTarget>::on_replace))
    } else {
        attrs
            .on_replace
            .map(|path| path.to_token_stream(&bevy_ecs_path))
    };

    let on_despawn_path = if attrs
        .relationship_target
        .is_some_and(|target| target.linked_spawn)
    {
        if attrs.on_despawn.is_some() {
            return syn::Error::new(
                ast.span(),
                "Custom on_despawn hooks are not supported as this RelationshipTarget already defines an on_despawn hook, via the 'linked_spawn' attribute",
            )
            .into_compile_error()
            .into();
        }

        Some(quote!(<Self as #bevy_ecs_path::relationship::RelationshipTarget>::on_despawn))
    } else {
        attrs
            .on_despawn
            .map(|path| path.to_token_stream(&bevy_ecs_path))
    };

    let on_add = hook_register_function_call(&bevy_ecs_path, quote! {on_add}, on_add_path);
    let on_insert = hook_register_function_call(&bevy_ecs_path, quote! {on_insert}, on_insert_path);
    let on_replace =
        hook_register_function_call(&bevy_ecs_path, quote! {on_replace}, on_replace_path);
    let on_remove = hook_register_function_call(&bevy_ecs_path, quote! {on_remove}, on_remove_path);
    let on_despawn =
        hook_register_function_call(&bevy_ecs_path, quote! {on_despawn}, on_despawn_path);

    ast.generics
        .make_where_clause()
        .predicates
        .push(parse_quote! { Self: Send + Sync + 'static });

    let requires = &attrs.requires;
    let mut register_required = Vec::with_capacity(attrs.requires.iter().len());
    if let Some(requires) = requires {
        for require in requires {
            let ident = &require.path;
            let constructor = match &require.func {
                Some(func) => quote! { || { let x: #ident = (#func)().into(); x } },
                None => quote! { <#ident as Default>::default },
            };
            register_required.push(quote! {
                required_components.register_required::<#ident>(#constructor);
            });
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
        let doc = format!("**Required Components**: {paths}. \n\n A component's Required Components are inserted whenever it is inserted. Note that this will also insert the required components _of_ the required components, recursively, in depth-first order.");
        quote! {
            #[doc = #doc]
        }
    });

    let mutable_type = (attrs.immutable || relationship.is_some())
        .then_some(quote! { #bevy_ecs_path::component::Immutable })
        .unwrap_or(quote! { #bevy_ecs_path::component::Mutable });

    let clone_behavior = if relationship_target.is_some() || relationship.is_some() {
        quote!(
            use #bevy_ecs_path::relationship::{
                RelationshipCloneBehaviorBase, RelationshipCloneBehaviorViaClone, RelationshipCloneBehaviorViaReflect,
                RelationshipTargetCloneBehaviorViaClone, RelationshipTargetCloneBehaviorViaReflect, RelationshipTargetCloneBehaviorHierarchy
                };
            (&&&&&&&#bevy_ecs_path::relationship::RelationshipCloneBehaviorSpecialization::<Self>::default()).default_clone_behavior()
        )
    } else if let Some(behavior) = attrs.clone_behavior {
        quote!(#bevy_ecs_path::component::ComponentCloneBehavior::#behavior)
    } else {
        quote!(
            use #bevy_ecs_path::component::{DefaultCloneBehaviorBase, DefaultCloneBehaviorViaClone};
            (&&&#bevy_ecs_path::component::DefaultCloneBehaviorSpecialization::<Self>::default()).default_clone_behavior()
        )
    };

    // This puts `register_required` before `register_recursive_requires` to ensure that the constructors of _all_ top
    // level components are initialized first, giving them precedence over recursively defined constructors for the same component type
    TokenStream::from(quote! {
        #required_component_docs
        impl #impl_generics #bevy_ecs_path::component::Component for #struct_name #type_generics #where_clause {
            const STORAGE_TYPE: #bevy_ecs_path::component::StorageType = #storage;
            type Mutability = #mutable_type;
            fn register_required_components(
                _requiree: #bevy_ecs_path::component::ComponentId,
                required_components: &mut #bevy_ecs_path::component::RequiredComponentsRegistrator,
            ) {
                #(#register_required)*
            }

            #on_add
            #on_insert
            #on_replace
            #on_remove
            #on_despawn

            fn clone_behavior() -> #bevy_ecs_path::component::ComponentCloneBehavior {
                #clone_behavior
            }

            #map_entities
        }

        #relationship

        #relationship_target
    })
}

const ENTITIES: &str = "entities";

pub(crate) fn map_entities(
    data: &Data,
    bevy_ecs_path: &Path,
    self_ident: Ident,
    is_relationship: bool,
    is_relationship_target: bool,
    map_entities_attr: Option<MapEntitiesAttributeKind>,
) -> Option<TokenStream2> {
    if let Some(map_entities_override) = map_entities_attr {
        let map_entities_tokens = map_entities_override.to_token_stream(bevy_ecs_path);
        return Some(quote!(
            #map_entities_tokens(#self_ident, mapper)
        ));
    }

    match data {
        Data::Struct(DataStruct { fields, .. }) => {
            let mut map = Vec::with_capacity(fields.len());

            let relationship = if is_relationship || is_relationship_target {
                relationship_field(fields, "MapEntities", fields.span()).ok()
            } else {
                None
            };
            fields
                .iter()
                .enumerate()
                .filter(|(_, field)| {
                    field.attrs.iter().any(|a| a.path().is_ident(ENTITIES))
                        || relationship.is_some_and(|relationship| relationship == *field)
                })
                .for_each(|(index, field)| {
                    let field_member = field
                        .ident
                        .clone()
                        .map_or(Member::from(index), Member::Named);

                    map.push(quote!(#self_ident.#field_member.map_entities(mapper);));
                });
            if map.is_empty() {
                return None;
            };
            Some(quote!(
                #(#map)*
            ))
        }
        Data::Enum(DataEnum { variants, .. }) => {
            let mut map = Vec::with_capacity(variants.len());

            for variant in variants.iter() {
                let field_members = variant
                    .fields
                    .iter()
                    .enumerate()
                    .filter(|(_, field)| field.attrs.iter().any(|a| a.path().is_ident(ENTITIES)))
                    .map(|(index, field)| {
                        field
                            .ident
                            .clone()
                            .map_or(Member::from(index), Member::Named)
                    })
                    .collect::<Vec<_>>();

                let ident = &variant.ident;
                let field_idents = field_members
                    .iter()
                    .map(|member| format_ident!("__self_{}", member))
                    .collect::<Vec<_>>();

                map.push(
                    quote!(Self::#ident {#(#field_members: #field_idents,)* ..} => {
                        #(#field_idents.map_entities(mapper);)*
                    }),
                );
            }

            if map.is_empty() {
                return None;
            };

            Some(quote!(
                match #self_ident {
                    #(#map,)*
                    _ => {}
                }
            ))
        }
        Data::Union(_) => None,
    }
}

pub const COMPONENT: &str = "component";
pub const STORAGE: &str = "storage";
pub const REQUIRE: &str = "require";
pub const RELATIONSHIP: &str = "relationship";
pub const RELATIONSHIP_TARGET: &str = "relationship_target";

pub const ON_ADD: &str = "on_add";
pub const ON_INSERT: &str = "on_insert";
pub const ON_REPLACE: &str = "on_replace";
pub const ON_REMOVE: &str = "on_remove";
pub const ON_DESPAWN: &str = "on_despawn";
pub const MAP_ENTITIES: &str = "map_entities";

pub const IMMUTABLE: &str = "immutable";
pub const CLONE_BEHAVIOR: &str = "clone_behavior";

/// All allowed attribute value expression kinds for component hooks.
/// This doesn't simply use general expressions because of conflicting needs:
/// - we want to be able to use `Self` & generic parameters in paths
/// - call expressions producing a closure need to be wrapped in a function
///   to turn them into function pointers, which prevents access to the outer generic params
#[derive(Debug)]
enum HookAttributeKind {
    /// expressions like function or struct names
    ///
    /// structs will throw compile errors on the code generation so this is safe
    Path(ExprPath),
    /// function call like expressions
    Call(ExprCall),
}

impl HookAttributeKind {
    fn from_expr(value: Expr) -> Result<Self> {
        match value {
            Expr::Path(path) => Ok(HookAttributeKind::Path(path)),
            Expr::Call(call) => Ok(HookAttributeKind::Call(call)),
            // throw meaningful error on all other expressions
            _ => Err(syn::Error::new(
                value.span(),
                [
                    "Not supported in this position, please use one of the following:",
                    "- path to function",
                    "- call to function yielding closure",
                ]
                .join("\n"),
            )),
        }
    }

    fn to_token_stream(&self, bevy_ecs_path: &Path) -> TokenStream2 {
        match self {
            HookAttributeKind::Path(path) => path.to_token_stream(),
            HookAttributeKind::Call(call) => {
                quote!({
                    fn _internal_hook(world: #bevy_ecs_path::world::DeferredWorld, ctx: #bevy_ecs_path::lifecycle::HookContext) {
                        (#call)(world, ctx)
                    }
                    _internal_hook
                })
            }
        }
    }
}

impl Parse for HookAttributeKind {
    fn parse(input: syn::parse::ParseStream) -> Result<Self> {
        input.parse::<Expr>().and_then(Self::from_expr)
    }
}

#[derive(Debug)]
pub(super) enum MapEntitiesAttributeKind {
    /// expressions like function or struct names
    ///
    /// structs will throw compile errors on the code generation so this is safe
    Path(ExprPath),
    /// When no value is specified
    Default,
}

impl MapEntitiesAttributeKind {
    fn from_expr(value: Expr) -> Result<Self> {
        match value {
            Expr::Path(path) => Ok(Self::Path(path)),
            // throw meaningful error on all other expressions
            _ => Err(syn::Error::new(
                value.span(),
                [
                    "Not supported in this position, please use one of the following:",
                    "- path to function",
                    "- nothing to default to MapEntities implementation",
                ]
                .join("\n"),
            )),
        }
    }

    fn to_token_stream(&self, bevy_ecs_path: &Path) -> TokenStream2 {
        match self {
            MapEntitiesAttributeKind::Path(path) => path.to_token_stream(),
            MapEntitiesAttributeKind::Default => {
                quote!(
                   <Self as #bevy_ecs_path::entity::MapEntities>::map_entities
                )
            }
        }
    }
}

impl Parse for MapEntitiesAttributeKind {
    fn parse(input: syn::parse::ParseStream) -> Result<Self> {
        if input.peek(Token![=]) {
            input.parse::<Token![=]>()?;
            input.parse::<Expr>().and_then(Self::from_expr)
        } else {
            Ok(Self::Default)
        }
    }
}

struct Attrs {
    storage: StorageTy,
    requires: Option<Punctuated<Require, Comma>>,
    on_add: Option<HookAttributeKind>,
    on_insert: Option<HookAttributeKind>,
    on_replace: Option<HookAttributeKind>,
    on_remove: Option<HookAttributeKind>,
    on_despawn: Option<HookAttributeKind>,
    relationship: Option<Relationship>,
    relationship_target: Option<RelationshipTarget>,
    immutable: bool,
    clone_behavior: Option<Expr>,
    map_entities: Option<MapEntitiesAttributeKind>,
}

#[derive(Clone, Copy)]
enum StorageTy {
    Table,
    SparseSet,
}

struct Require {
    path: Path,
    func: Option<TokenStream2>,
}

struct Relationship {
    relationship_target: Type,
}

struct RelationshipTarget {
    relationship: Type,
    linked_spawn: bool,
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
        on_despawn: None,
        requires: None,
        relationship: None,
        relationship_target: None,
        immutable: false,
        clone_behavior: None,
        map_entities: None,
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
                    attrs.on_add = Some(nested.value()?.parse::<HookAttributeKind>()?);
                    Ok(())
                } else if nested.path.is_ident(ON_INSERT) {
                    attrs.on_insert = Some(nested.value()?.parse::<HookAttributeKind>()?);
                    Ok(())
                } else if nested.path.is_ident(ON_REPLACE) {
                    attrs.on_replace = Some(nested.value()?.parse::<HookAttributeKind>()?);
                    Ok(())
                } else if nested.path.is_ident(ON_REMOVE) {
                    attrs.on_remove = Some(nested.value()?.parse::<HookAttributeKind>()?);
                    Ok(())
                } else if nested.path.is_ident(ON_DESPAWN) {
                    attrs.on_despawn = Some(nested.value()?.parse::<HookAttributeKind>()?);
                    Ok(())
                } else if nested.path.is_ident(IMMUTABLE) {
                    attrs.immutable = true;
                    Ok(())
                } else if nested.path.is_ident(CLONE_BEHAVIOR) {
                    attrs.clone_behavior = Some(nested.value()?.parse()?);
                    Ok(())
                } else if nested.path.is_ident(MAP_ENTITIES) {
                    attrs.map_entities = Some(nested.input.parse::<MapEntitiesAttributeKind>()?);
                    Ok(())
                } else {
                    Err(nested.error("Unsupported attribute"))
                }
            })?;
        } else if attr.path().is_ident(REQUIRE) {
            let punctuated =
                attr.parse_args_with(Punctuated::<Require, Comma>::parse_terminated)?;
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
        } else if attr.path().is_ident(RELATIONSHIP) {
            let relationship = attr.parse_args::<Relationship>()?;
            attrs.relationship = Some(relationship);
        } else if attr.path().is_ident(RELATIONSHIP_TARGET) {
            let relationship_target = attr.parse_args::<RelationshipTarget>()?;
            attrs.relationship_target = Some(relationship_target);
        }
    }

    if attrs.relationship_target.is_some() && attrs.clone_behavior.is_some() {
        return Err(syn::Error::new(
                attrs.clone_behavior.span(),
                "A Relationship Target already has its own clone behavior, please remove `clone_behavior = ...`",
            ));
    }

    Ok(attrs)
}

impl Parse for Require {
    fn parse(input: syn::parse::ParseStream) -> Result<Self> {
        let mut path = input.parse::<Path>()?;
        let mut last_segment_is_lower = false;
        let mut is_constructor_call = false;

        // Use the case of the type name to check if it's an enum
        // This doesn't match everything that can be an enum according to the rust spec
        // but it matches what clippy is OK with
        let is_enum = {
            let mut first_chars = path
                .segments
                .iter()
                .rev()
                .filter_map(|s| s.ident.to_string().chars().next());
            if let Some(last) = first_chars.next() {
                if last.is_uppercase() {
                    if let Some(last) = first_chars.next() {
                        last.is_uppercase()
                    } else {
                        false
                    }
                } else {
                    last_segment_is_lower = true;
                    false
                }
            } else {
                false
            }
        };

        let func = if input.peek(Token![=]) {
            // If there is an '=', then this is a "function style" require
            input.parse::<Token![=]>()?;
            let expr: Expr = input.parse()?;
            Some(quote!(|| #expr ))
        } else if input.peek(Brace) {
            // This is a "value style" named-struct-like require
            let content;
            braced!(content in input);
            let content = content.parse::<TokenStream2>()?;
            Some(quote!(|| #path { #content }))
        } else if input.peek(Paren) {
            // This is a "value style" tuple-struct-like require
            let content;
            parenthesized!(content in input);
            let content = content.parse::<TokenStream2>()?;
            is_constructor_call = last_segment_is_lower;
            Some(quote!(|| #path (#content)))
        } else if is_enum {
            // if this is an enum, then it is an inline enum component declaration
            Some(quote!(|| #path))
        } else {
            // if this isn't any of the above, then it is a component ident, which will use Default
            None
        };
        if is_enum || is_constructor_call {
            path.segments.pop();
            path.segments.pop_punct();
        }
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
    bevy_ecs_path: &Path,
    hook: TokenStream2,
    function: Option<TokenStream2>,
) -> Option<TokenStream2> {
    function.map(|meta| {
        quote! {
            fn #hook() -> ::core::option::Option<#bevy_ecs_path::lifecycle::ComponentHook> {
                ::core::option::Option::Some(#meta)
            }
        }
    })
}

mod kw {
    syn::custom_keyword!(relationship_target);
    syn::custom_keyword!(relationship);
    syn::custom_keyword!(linked_spawn);
}

impl Parse for Relationship {
    fn parse(input: syn::parse::ParseStream) -> Result<Self> {
        input.parse::<kw::relationship_target>()?;
        input.parse::<Token![=]>()?;
        Ok(Relationship {
            relationship_target: input.parse::<Type>()?,
        })
    }
}

impl Parse for RelationshipTarget {
    fn parse(input: syn::parse::ParseStream) -> Result<Self> {
        let mut relationship: Option<Type> = None;
        let mut linked_spawn: bool = false;

        while !input.is_empty() {
            let lookahead = input.lookahead1();
            if lookahead.peek(kw::linked_spawn) {
                input.parse::<kw::linked_spawn>()?;
                linked_spawn = true;
            } else if lookahead.peek(kw::relationship) {
                input.parse::<kw::relationship>()?;
                input.parse::<Token![=]>()?;
                relationship = Some(input.parse()?);
            } else {
                return Err(lookahead.error());
            }
            if !input.is_empty() {
                input.parse::<Token![,]>()?;
            }
        }
        Ok(RelationshipTarget {
            relationship: relationship.ok_or_else(|| {
                syn::Error::new(input.span(), "Missing `relationship = X` attribute")
            })?,
            linked_spawn,
        })
    }
}

fn derive_relationship(
    ast: &DeriveInput,
    attrs: &Attrs,
    bevy_ecs_path: &Path,
) -> Result<Option<TokenStream2>> {
    let Some(relationship) = &attrs.relationship else {
        return Ok(None);
    };
    let Data::Struct(DataStruct {
        fields,
        struct_token,
        ..
    }) = &ast.data
    else {
        return Err(syn::Error::new(
            ast.span(),
            "Relationship can only be derived for structs.",
        ));
    };
    let field = relationship_field(fields, "Relationship", struct_token.span())?;

    let relationship_member = field.ident.clone().map_or(Member::from(0), Member::Named);
    let members = fields
        .members()
        .filter(|member| member != &relationship_member);

    let struct_name = &ast.ident;
    let (impl_generics, type_generics, where_clause) = &ast.generics.split_for_impl();

    let relationship_target = &relationship.relationship_target;

    Ok(Some(quote! {
        impl #impl_generics #bevy_ecs_path::relationship::Relationship for #struct_name #type_generics #where_clause {
            type RelationshipTarget = #relationship_target;

            #[inline(always)]
            fn get(&self) -> #bevy_ecs_path::entity::Entity {
                self.#relationship_member
            }

            #[inline]
            fn from(entity: #bevy_ecs_path::entity::Entity) -> Self {
                Self {
                    #(#members: core::default::Default::default(),)*
                    #relationship_member: entity
                }
            }

            #[inline]
            fn set_risky(&mut self, entity: Entity) {
                self.#relationship_member = entity;
            }
        }
    }))
}

fn derive_relationship_target(
    ast: &DeriveInput,
    attrs: &Attrs,
    bevy_ecs_path: &Path,
) -> Result<Option<TokenStream2>> {
    let Some(relationship_target) = &attrs.relationship_target else {
        return Ok(None);
    };

    let Data::Struct(DataStruct {
        fields,
        struct_token,
        ..
    }) = &ast.data
    else {
        return Err(syn::Error::new(
            ast.span(),
            "RelationshipTarget can only be derived for structs.",
        ));
    };
    let field = relationship_field(fields, "RelationshipTarget", struct_token.span())?;

    if field.vis != Visibility::Inherited {
        return Err(syn::Error::new(field.span(), "The collection in RelationshipTarget must be private to prevent users from directly mutating it, which could invalidate the correctness of relationships."));
    }
    let collection = &field.ty;
    let relationship_member = field.ident.clone().map_or(Member::from(0), Member::Named);

    let members = fields
        .members()
        .filter(|member| member != &relationship_member);

    let relationship = &relationship_target.relationship;
    let struct_name = &ast.ident;
    let (impl_generics, type_generics, where_clause) = &ast.generics.split_for_impl();
    let linked_spawn = relationship_target.linked_spawn;
    Ok(Some(quote! {
        impl #impl_generics #bevy_ecs_path::relationship::RelationshipTarget for #struct_name #type_generics #where_clause {
            const LINKED_SPAWN: bool = #linked_spawn;
            type Relationship = #relationship;
            type Collection = #collection;

            #[inline]
            fn collection(&self) -> &Self::Collection {
                &self.#relationship_member
            }

            #[inline]
            fn collection_mut_risky(&mut self) -> &mut Self::Collection {
                &mut self.#relationship_member
            }

            #[inline]
            fn from_collection_risky(collection: Self::Collection) -> Self {
                Self {
                    #(#members: core::default::Default::default(),)*
                    #relationship_member: collection
                }
            }
        }
    }))
}

/// Returns the field with the `#[relationship]` attribute, the only field if unnamed,
/// or the only field in a [`Fields::Named`] with one field, otherwise `Err`.
fn relationship_field<'a>(
    fields: &'a Fields,
    derive: &'static str,
    span: Span,
) -> Result<&'a Field> {
    match fields {
        Fields::Named(fields) if fields.named.len() == 1 => Ok(fields.named.first().unwrap()),
        Fields::Named(fields) => fields.named.iter().find(|field| {
            field
                .attrs
                .iter()
                .any(|attr| attr.path().is_ident(RELATIONSHIP))
        }).ok_or(syn::Error::new(
            span,
            format!("{derive} derive expected named structs with a single field or with a field annotated with #[relationship].")
        )),
        Fields::Unnamed(fields) if fields.unnamed.len() == 1 => Ok(fields.unnamed.first().unwrap()),
        Fields::Unnamed(fields) => fields.unnamed.iter().find(|field| {
                field
                    .attrs
                    .iter()
                    .any(|attr| attr.path().is_ident(RELATIONSHIP))
            })
            .ok_or(syn::Error::new(
                span,
                format!("{derive} derive expected unnamed structs with one field or with a field annotated with #[relationship]."),
            )),
        Fields::Unit => Err(syn::Error::new(
            span,
            format!("{derive} derive expected named or unnamed struct, found unit struct."),
        )),
    }
}
