use proc_macro::TokenStream;
use quote::quote;
use syn::{
    parse::Parse, parse_macro_input, parse_quote, spanned::Spanned, Data, DataStruct, DeriveInput,
    Fields, Ident, Path, Token, Visibility,
};

pub fn derive_relationship(input: TokenStream) -> TokenStream {
    let mut ast = parse_macro_input!(input as DeriveInput);
    let bevy_ecs_path: Path = crate::bevy_ecs_path();

    let Some(relationship_attribute) = ast.attrs.iter().find(|a| a.path().is_ident("relationship"))
    else {
        return syn::Error::new(
            ast.span(),
            "Relationship derives must define a #[relationship(relationship_sources = X)] attribute.",
        )
        .into_compile_error()
        .into();
    };
    let relationship_args = match relationship_attribute.parse_args::<RelationshipArgs>() {
        Ok(result) => result,
        Err(err) => return err.into_compile_error().into(),
    };

    let relationship_sources = relationship_args.relationship_sources;

    const RELATIONSHIP_FORMAT_MESSAGE: &str = "Relationship derives must be a tuple struct with the only element being an EntityTargets type (ex: ChildOf(Entity))";
    if let Data::Struct(DataStruct {
        fields: Fields::Unnamed(unnamed_fields),
        struct_token,
        ..
    }) = &ast.data
    {
        if unnamed_fields.unnamed.len() != 1 {
            return syn::Error::new(ast.span(), RELATIONSHIP_FORMAT_MESSAGE)
                .into_compile_error()
                .into();
        }
        if unnamed_fields.unnamed.first().is_none() {
            return syn::Error::new(struct_token.span(), RELATIONSHIP_FORMAT_MESSAGE)
                .into_compile_error()
                .into();
        }
    } else {
        return syn::Error::new(ast.span(), RELATIONSHIP_FORMAT_MESSAGE)
            .into_compile_error()
            .into();
    };

    ast.generics
        .make_where_clause()
        .predicates
        .push(parse_quote! { Self: Send + Sync + 'static });

    let struct_name = &ast.ident;
    let (impl_generics, type_generics, where_clause) = &ast.generics.split_for_impl();

    TokenStream::from(quote! {
        impl #impl_generics #bevy_ecs_path::relationship::Relationship for #struct_name #type_generics #where_clause {
            type RelationshipSources = #relationship_sources;

            #[inline(always)]
            fn get(&self) -> #bevy_ecs_path::entity::Entity {
                self.0
            }

            fn from(entity: #bevy_ecs_path::entity::Entity) -> Self {
                Self(entity)
            }
        }

        impl #impl_generics #bevy_ecs_path::component::Component for #struct_name #type_generics #where_clause {
            const STORAGE_TYPE: #bevy_ecs_path::component::StorageType = #bevy_ecs_path::component::StorageType::Table;
            type Mutability = #bevy_ecs_path::component::Immutable;

            fn register_component_hooks(hooks: &mut #bevy_ecs_path::component::ComponentHooks) {
                hooks.on_insert(<Self as Relationship>::on_insert);
                hooks.on_replace(<Self as Relationship>::on_replace);
            }
            fn get_component_clone_handler() -> #bevy_ecs_path::component::ComponentCloneHandler {
                use #bevy_ecs_path::component::{ComponentCloneBase, ComponentCloneViaClone};
                (&&&#bevy_ecs_path::component::ComponentCloneSpecializationWrapper::<Self>::default())
                    .get_component_clone_handler()
            }
        }
    })
}

pub struct RelationshipArgs {
    relationship_sources: Ident,
}

impl Parse for RelationshipArgs {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        syn::custom_keyword!(relationship_sources);
        input.parse::<relationship_sources>()?;
        input.parse::<Token![=]>()?;
        Ok(RelationshipArgs {
            relationship_sources: input.parse::<Ident>()?,
        })
    }
}

pub fn derive_relationship_sources(input: TokenStream) -> TokenStream {
    let mut ast = parse_macro_input!(input as DeriveInput);
    let bevy_ecs_path: Path = crate::bevy_ecs_path();

    let Some(relationship_sources_attribute) = ast
        .attrs
        .iter()
        .find(|a| a.path().is_ident("relationship_sources"))
    else {
        return syn::Error::new(
            ast.span(),
            "RelationshipSources derives must define a #[relationship_sources(relationship = X)] attribute.",
        )
        .into_compile_error()
        .into();
    };
    let relationship_sources_args =
        match relationship_sources_attribute.parse_args::<RelationshipSourcesArgs>() {
            Ok(result) => result,
            Err(err) => return err.into_compile_error().into(),
        };

    const RELATIONSHIP_SOURCES_FORMAT_MESSAGE: &str = "RelationshipSources derives must be a tuple struct with the first element being a private RelationshipSourceCollection (ex: Children(Vec<Entity>))";
    let collection = if let Data::Struct(DataStruct {
        fields: Fields::Unnamed(unnamed_fields),
        struct_token,
        ..
    }) = &ast.data
    {
        if let Some(first) = unnamed_fields.unnamed.first() {
            if first.vis != Visibility::Inherited {
                return syn::Error::new(first.span(), "The collection in RelationshipSources must be private to prevent users from directly mutating it, which could invalidate the correctness of relationships.")
                    .into_compile_error()
                    .into();
            }
            first.ty.clone()
        } else {
            return syn::Error::new(struct_token.span(), RELATIONSHIP_SOURCES_FORMAT_MESSAGE)
                .into_compile_error()
                .into();
        }
    } else {
        return syn::Error::new(ast.span(), RELATIONSHIP_SOURCES_FORMAT_MESSAGE)
            .into_compile_error()
            .into();
    };

    ast.generics
        .make_where_clause()
        .predicates
        .push(parse_quote! { Self: Send + Sync + 'static });

    let struct_name = &ast.ident;
    let (impl_generics, type_generics, where_clause) = &ast.generics.split_for_impl();

    let relationship = relationship_sources_args.relationship;
    let despawn_descendants = relationship_sources_args
        .despawn_descendants
        .then(|| quote! {hooks.on_despawn(<Self as RelationshipSources>::on_despawn);});
    // NOTE: The Component impl is mutable for RelationshipSources for the following reasons:
    // 1. RelationshipSources like Children will want user-facing APIs to reorder children, as order may be semantically relevant in some cases
    //    (or may just be organizational ... ex: dragging to reorder children in the editor). This does not violate the relationship correctness,
    //    so it can / should be allowed.
    // 2. The immutable model doesn't really makes sense, given that we're appending to / removing from a list regularly as new children are added / removed.
    //    We could hack around this, but that would break the user-facing immutable data model.
    TokenStream::from(quote! {
        impl #impl_generics #bevy_ecs_path::relationship::RelationshipSources for #struct_name #type_generics #where_clause {
            type Relationship = #relationship;
            type Collection = #collection;

            fn collection(&self) -> &Self::Collection {
                &self.0
            }

            fn collection_mut(&mut self) -> &mut Self::Collection {
                &mut self.0
            }

            fn from_collection(collection: Self::Collection) -> Self {
                Self(collection)
            }
        }

        impl #impl_generics #bevy_ecs_path::component::Component for #struct_name #type_generics #where_clause {
            const STORAGE_TYPE: #bevy_ecs_path::component::StorageType = #bevy_ecs_path::component::StorageType::Table;
            type Mutability = #bevy_ecs_path::component::Mutable;

            fn register_component_hooks(hooks: &mut #bevy_ecs_path::component::ComponentHooks) {
                hooks.on_replace(<Self as RelationshipSources>::on_replace);
                #despawn_descendants
            }
            fn get_component_clone_handler() -> #bevy_ecs_path::component::ComponentCloneHandler {
                #bevy_ecs_path::component::ComponentCloneHandler::ignore()
            }
        }
    })
}

pub struct RelationshipSourcesArgs {
    relationship: Ident,
    despawn_descendants: bool,
}

impl Parse for RelationshipSourcesArgs {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
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

        let relationship = relationship_ident.ok_or_else(|| syn::Error::new(input.span(), "RelationshipSources derive must specify a relationship via #[relationship_sources(relationship = X)"))?;
        Ok(RelationshipSourcesArgs {
            relationship,
            despawn_descendants: despawn_descendants_exists,
        })
    }
}
