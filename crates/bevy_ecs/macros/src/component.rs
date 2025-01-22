use proc_macro::{TokenStream, TokenTree};
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
    Data, DataStruct, DeriveInput, ExprClosure, ExprPath, Fields, Ident, LitStr, Path, Result,
    Token, Visibility,
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

    let storage = storage_path(&bevy_ecs_path, attrs.storage);

    let on_add = hook_register_function_call(quote! {on_add}, attrs.on_add);
    let mut on_insert = hook_register_function_call(quote! {on_insert}, attrs.on_insert);
    let mut on_replace = hook_register_function_call(quote! {on_replace}, attrs.on_replace);
    let on_remove: Option<TokenStream2> =
        hook_register_function_call(quote! {on_remove}, attrs.on_remove);
    let mut on_despawn = hook_register_function_call(quote! {on_despawn}, attrs.on_despawn);

    if relationship.is_some() {
        if on_insert.is_some() {
            return syn::Error::new(
                ast.span(),
                "Custom on_insert hooks are not supported as relationships already define an on_insert hook",
            )
            .into_compile_error()
            .into();
        }

        on_insert = Some(
            quote!(hooks.on_insert(<Self as #bevy_ecs_path::relationship::Relationship>::on_insert);),
        );

        if on_replace.is_some() {
            return syn::Error::new(
                ast.span(),
                "Custom on_replace hooks are not supported as Relationships already define an on_replace hook",
            )
            .into_compile_error()
            .into();
        }

        on_replace = Some(
            quote!(hooks.on_replace(<Self as #bevy_ecs_path::relationship::Relationship>::on_replace);),
        );
    }

    if let Some(relationship_target) = &attrs.relationship_target {
        if on_replace.is_some() {
            return syn::Error::new(
                ast.span(),
                "Custom on_replace hooks are not supported as RelationshipTarget already defines an on_replace hook",
            )
            .into_compile_error()
            .into();
        }

        on_replace = Some(
            quote!(hooks.on_replace(<Self as #bevy_ecs_path::relationship::RelationshipTarget>::on_replace);),
        );

        if relationship_target.despawn_descendants {
            if on_despawn.is_some() {
                return syn::Error::new(
                    ast.span(),
                    "Custom on_despawn hooks are not supported as this RelationshipTarget already defines an on_despawn hook, via the despawn_descendants attribute",
                )
                .into_compile_error()
                .into();
            }

            on_despawn = Some(
                quote!(hooks.on_despawn(<Self as #bevy_ecs_path::relationship::RelationshipTarget>::on_despawn);),
            );
        }
    }

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
                <#ident as #bevy_ecs_path::component::Component>::register_required_components(
                    requiree,
                    components,
                    storages,
                    required_components,
                    inheritance_depth + 1,
                    recursion_check_stack
                );
            });
            match &require.func {
                Some(RequireFunc::Path(func)) => {
                    register_required.push(quote! {
                        components.register_required_components_manual::<Self, #ident>(
                            storages,
                            required_components,
                            || { let x: #ident = #func().into(); x },
                            inheritance_depth,
                            recursion_check_stack
                        );
                    });
                }
                Some(RequireFunc::Closure(func)) => {
                    register_required.push(quote! {
                        components.register_required_components_manual::<Self, #ident>(
                            storages,
                            required_components,
                            || { let x: #ident = (#func)().into(); x },
                            inheritance_depth,
                            recursion_check_stack
                        );
                    });
                }
                None => {
                    register_required.push(quote! {
                        components.register_required_components_manual::<Self, #ident>(
                            storages,
                            required_components,
                            <#ident as Default>::default,
                            inheritance_depth,
                            recursion_check_stack
                        );
                    });
                }
            }
        }
    }
    let struct_name = &ast.ident;
    let (impl_generics, type_generics, where_clause) = &ast.generics.split_for_impl();

    let mutable_type = (attrs.immutable || relationship.is_some())
        .then_some(quote! { #bevy_ecs_path::component::Immutable })
        .unwrap_or(quote! { #bevy_ecs_path::component::Mutable });

    let clone_handler = if relationship_target.is_some() {
        quote!(#bevy_ecs_path::component::ComponentCloneHandler::ignore())
    } else {
        quote!(
            use #bevy_ecs_path::component::{ComponentCloneViaClone, ComponentCloneBase};
            (&&&#bevy_ecs_path::component::ComponentCloneSpecializationWrapper::<Self>::default()).get_component_clone_handler()
        )
    };

    // This puts `register_required` before `register_recursive_requires` to ensure that the constructors of _all_ top
    // level components are initialized first, giving them precedence over recursively defined constructors for the same component type
    TokenStream::from(quote! {
        impl #impl_generics #bevy_ecs_path::component::Component for #struct_name #type_generics #where_clause {
            const STORAGE_TYPE: #bevy_ecs_path::component::StorageType = #storage;
            type Mutability = #mutable_type;
            fn register_required_components(
                requiree: #bevy_ecs_path::component::ComponentId,
                components: &mut #bevy_ecs_path::component::Components,
                storages: &mut #bevy_ecs_path::storage::Storages,
                required_components: &mut #bevy_ecs_path::component::RequiredComponents,
                inheritance_depth: u16,
                recursion_check_stack: &mut #bevy_ecs_path::__macro_exports::Vec<#bevy_ecs_path::component::ComponentId>
            ) {
                #bevy_ecs_path::component::enforce_no_required_components_recursion(components, recursion_check_stack);
                let self_id = components.register_component::<Self>(storages);
                recursion_check_stack.push(self_id);
                #(#register_required)*
                #(#register_recursive_requires)*
                recursion_check_stack.pop();
            }

            #[allow(unused_variables)]
            fn register_component_hooks(hooks: &mut #bevy_ecs_path::component::ComponentHooks) {
                #on_add
                #on_insert
                #on_replace
                #on_remove
                #on_despawn
            }

            fn get_component_clone_handler() -> #bevy_ecs_path::component::ComponentCloneHandler {
                #clone_handler
            }
        }

        #relationship

        #relationship_target
    })
}

pub fn document_required_components(attr: TokenStream, item: TokenStream) -> TokenStream {
    let paths = parse_macro_input!(attr with Punctuated::<Require, Comma>::parse_terminated)
        .iter()
        .map(|r| format!("[`{}`]", r.path.to_token_stream()))
        .collect::<Vec<_>>()
        .join(", ");

    let bevy_ecs_path = crate::bevy_ecs_path()
        .to_token_stream()
        .to_string()
        .replace(' ', "");
    let required_components_path = bevy_ecs_path + "::component::Component#required-components";

    // Insert information about required components after any existing doc comments
    let mut out = TokenStream::new();
    let mut end_of_attributes_reached = false;
    for tt in item {
        if !end_of_attributes_reached & matches!(tt, TokenTree::Ident(_)) {
            end_of_attributes_reached = true;
            let doc: TokenStream = format!("#[doc = \"\n\n# Required Components\n{paths} \n\n A component's [required components]({required_components_path}) are inserted whenever it is inserted. Note that this will also insert the required components _of_ the required components, recursively, in depth-first order.\"]").parse().unwrap();
            out.extend(doc);
        }
        out.extend(Some(tt));
    }

    out
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

pub const IMMUTABLE: &str = "immutable";

struct Attrs {
    storage: StorageTy,
    requires: Option<Punctuated<Require, Comma>>,
    on_add: Option<ExprPath>,
    on_insert: Option<ExprPath>,
    on_replace: Option<ExprPath>,
    on_remove: Option<ExprPath>,
    on_despawn: Option<ExprPath>,
    relationship: Option<Relationship>,
    relationship_target: Option<RelationshipTarget>,
    immutable: bool,
}

#[derive(Clone, Copy)]
enum StorageTy {
    Table,
    SparseSet,
}

struct Require {
    path: Path,
    func: Option<RequireFunc>,
}

enum RequireFunc {
    Path(Path),
    Closure(ExprClosure),
}

struct Relationship {
    relationship_target: Ident,
}

struct RelationshipTarget {
    relationship: Ident,
    despawn_descendants: bool,
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
                } else if nested.path.is_ident(ON_DESPAWN) {
                    attrs.on_despawn = Some(nested.value()?.parse::<ExprPath>()?);
                    Ok(())
                } else if nested.path.is_ident(IMMUTABLE) {
                    attrs.immutable = true;
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

    Ok(attrs)
}

impl Parse for Require {
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

impl Parse for Relationship {
    fn parse(input: syn::parse::ParseStream) -> Result<Self> {
        syn::custom_keyword!(relationship_target);
        input.parse::<relationship_target>()?;
        input.parse::<Token![=]>()?;
        Ok(Relationship {
            relationship_target: input.parse::<Ident>()?,
        })
    }
}

impl Parse for RelationshipTarget {
    fn parse(input: syn::parse::ParseStream) -> Result<Self> {
        let mut relationship_ident = None;
        let mut despawn_descendants_exists = false;
        syn::custom_keyword!(relationship);
        syn::custom_keyword!(despawn_descendants);
        let mut done = false;
        loop {
            if input.peek(relationship) {
                input.parse::<relationship>()?;
                input.parse::<Token![=]>()?;
                relationship_ident = Some(input.parse::<Ident>()?);
            } else if input.peek(despawn_descendants) {
                input.parse::<despawn_descendants>()?;
                despawn_descendants_exists = true;
            } else {
                done = true;
            }
            if input.peek(Token![,]) {
                input.parse::<Token![,]>()?;
            }
            if done {
                break;
            }
        }

        let relationship = relationship_ident.ok_or_else(|| syn::Error::new(input.span(), "RelationshipTarget derive must specify a relationship via #[relationship_target(relationship = X)"))?;
        Ok(RelationshipTarget {
            relationship,
            despawn_descendants: despawn_descendants_exists,
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
    const RELATIONSHIP_FORMAT_MESSAGE: &str = "Relationship derives must be a tuple struct with the only element being an EntityTargets type (ex: ChildOf(Entity))";
    if let Data::Struct(DataStruct {
        fields: Fields::Unnamed(unnamed_fields),
        struct_token,
        ..
    }) = &ast.data
    {
        if unnamed_fields.unnamed.len() != 1 {
            return Err(syn::Error::new(ast.span(), RELATIONSHIP_FORMAT_MESSAGE));
        }
        if unnamed_fields.unnamed.first().is_none() {
            return Err(syn::Error::new(
                struct_token.span(),
                RELATIONSHIP_FORMAT_MESSAGE,
            ));
        }
    } else {
        return Err(syn::Error::new(ast.span(), RELATIONSHIP_FORMAT_MESSAGE));
    };

    let struct_name = &ast.ident;
    let (impl_generics, type_generics, where_clause) = &ast.generics.split_for_impl();

    let relationship_target = &relationship.relationship_target;

    Ok(Some(quote! {
        impl #impl_generics #bevy_ecs_path::relationship::Relationship for #struct_name #type_generics #where_clause {
            type RelationshipTarget = #relationship_target;

            #[inline(always)]
            fn get(&self) -> #bevy_ecs_path::entity::Entity {
                self.0
            }

            #[inline]
            fn from(entity: #bevy_ecs_path::entity::Entity) -> Self {
                Self(entity)
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

    const RELATIONSHIP_TARGET_FORMAT_MESSAGE: &str = "RelationshipTarget derives must be a tuple struct with the first element being a private RelationshipSourceCollection (ex: Children(Vec<Entity>))";
    let collection = if let Data::Struct(DataStruct {
        fields: Fields::Unnamed(unnamed_fields),
        struct_token,
        ..
    }) = &ast.data
    {
        if let Some(first) = unnamed_fields.unnamed.first() {
            if first.vis != Visibility::Inherited {
                return Err(syn::Error::new(first.span(), "The collection in RelationshipTarget must be private to prevent users from directly mutating it, which could invalidate the correctness of relationships."));
            }
            first.ty.clone()
        } else {
            return Err(syn::Error::new(
                struct_token.span(),
                RELATIONSHIP_TARGET_FORMAT_MESSAGE,
            ));
        }
    } else {
        return Err(syn::Error::new(
            ast.span(),
            RELATIONSHIP_TARGET_FORMAT_MESSAGE,
        ));
    };

    let relationship = &relationship_target.relationship;
    let struct_name = &ast.ident;
    let (impl_generics, type_generics, where_clause) = &ast.generics.split_for_impl();
    Ok(Some(quote! {
        impl #impl_generics #bevy_ecs_path::relationship::RelationshipTarget for #struct_name #type_generics #where_clause {
            type Relationship = #relationship;
            type Collection = #collection;

            #[inline]
            fn collection(&self) -> &Self::Collection {
                &self.0
            }

            #[inline]
            fn collection_mut_risky(&mut self) -> &mut Self::Collection {
                &mut self.0
            }

            #[inline]
            fn from_collection_risky(collection: Self::Collection) -> Self {
                Self(collection)
            }
        }
    }))
}
