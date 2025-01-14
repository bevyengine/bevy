use proc_macro::TokenStream;
use quote::quote;
use syn::{
    parse_macro_input, parse_quote, spanned::Spanned, Data, DataStruct, DeriveInput, Fields, Ident,
    Path, Visibility,
};

pub fn derive_relationship(input: TokenStream) -> TokenStream {
    let mut ast = parse_macro_input!(input as DeriveInput);
    let bevy_ecs_path: Path = crate::bevy_ecs_path();

    let Some(relationship_sources) = ast
        .attrs
        .iter()
        .find(|a| a.path().is_ident("relationship_sources"))
        .and_then(|a| a.parse_args::<Ident>().ok())
    else {
        return syn::Error::new(
            ast.span(),
            "Relationship derives must define a relationship_sources(SOURCES_COMPONENT) attribute.",
        )
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

            fn set(&mut self, entity: #bevy_ecs_path::entity::Entity) {
                self.0 = entity;
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

pub fn derive_relationship_sources(input: TokenStream) -> TokenStream {
    let mut ast = parse_macro_input!(input as DeriveInput);
    let bevy_ecs_path: Path = crate::bevy_ecs_path();

    let Some(relationship) = ast
        .attrs
        .iter()
        .find(|a| a.path().is_ident("relationship"))
        .and_then(|a| a.parse_args::<Ident>().ok())
    else {
        return syn::Error::new(
            ast.span(),
            "RelationshipSources derives must define a relationship(RELATIONSHIP) attribute.",
        )
        .into_compile_error()
        .into();
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
                return syn::Error::new(first.span(), RELATIONSHIP_SOURCES_FORMAT_MESSAGE)
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
                hooks.on_despawn(<Self as RelationshipSources>::on_despawn);
            }
            fn get_component_clone_handler() -> #bevy_ecs_path::component::ComponentCloneHandler {
                #bevy_ecs_path::component::ComponentCloneHandler::ignore()
            }
        }
    })
}
