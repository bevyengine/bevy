use proc_macro::TokenStream;
use quote::quote;
use syn::{
    parse_macro_input, parse_quote, spanned::Spanned, Data, DataStruct, DeriveInput, Fields, Index,
    Member, Path, Result, Token, Type,
};

pub const EVENT: &str = "event";
pub const ENTITY_EVENT: &str = "entity_event";
pub const PROPAGATE: &str = "propagate";
#[deprecated(since = "0.17.0", note = "This has been renamed to `propagate`.")]
pub const TRAVERSAL: &str = "traversal";
pub const AUTO_PROPAGATE: &str = "auto_propagate";
pub const TRIGGER: &str = "trigger";
pub const EVENT_TARGET: &str = "event_target";

pub fn derive_event(input: TokenStream) -> TokenStream {
    let mut ast = parse_macro_input!(input as DeriveInput);
    let bevy_ecs_path: Path = crate::bevy_ecs_path();

    ast.generics
        .make_where_clause()
        .predicates
        .push(parse_quote! { Self: Send + Sync + 'static });

    let mut processed_attrs = Vec::new();
    let mut trigger: Option<Type> = None;

    for attr in ast.attrs.iter().filter(|attr| attr.path().is_ident(EVENT)) {
        if let Err(e) = attr.parse_nested_meta(|meta| match meta.path.get_ident() {
            Some(ident) if processed_attrs.iter().any(|i| ident == i) => {
                Err(meta.error(format!("duplicate attribute: {ident}")))
            }
            Some(ident) if ident == TRIGGER => {
                trigger = Some(meta.value()?.parse()?);
                processed_attrs.push(TRIGGER);
                Ok(())
            }
            Some(ident) => Err(meta.error(format!("unsupported attribute: {ident}"))),
            None => Err(meta.error("expected identifier")),
        }) {
            return e.to_compile_error().into();
        }
    }

    let trigger = if let Some(trigger) = trigger {
        quote! {#trigger}
    } else {
        quote! {#bevy_ecs_path::event::GlobalTrigger}
    };

    let struct_name = &ast.ident;
    let (impl_generics, type_generics, where_clause) = &ast.generics.split_for_impl();

    TokenStream::from(quote! {
        impl #impl_generics #bevy_ecs_path::event::Event for #struct_name #type_generics #where_clause {
            type Trigger<'a> = #trigger;
        }
    })
}

pub fn derive_entity_event(input: TokenStream) -> TokenStream {
    let mut ast = parse_macro_input!(input as DeriveInput);

    ast.generics
        .make_where_clause()
        .predicates
        .push(parse_quote! { Self: Send + Sync + 'static });

    let mut auto_propagate = false;
    let mut propagate = false;
    let mut traversal: Option<Type> = None;
    let mut trigger: Option<Type> = None;
    let bevy_ecs_path: Path = crate::bevy_ecs_path();

    let mut processed_attrs = Vec::new();

    for attr in ast
        .attrs
        .iter()
        .filter(|attr| attr.path().is_ident(ENTITY_EVENT))
    {
        if let Err(e) = attr.parse_nested_meta(|meta| match meta.path.get_ident() {
            Some(ident) if processed_attrs.iter().any(|i| ident == i) => {
                Err(meta.error(format!("duplicate attribute: {ident}")))
            }
            Some(ident) if ident == AUTO_PROPAGATE => {
                propagate = true;
                auto_propagate = true;
                processed_attrs.push(AUTO_PROPAGATE);
                Ok(())
            }
            #[expect(deprecated, reason = "we want to continue supporting this for a release")]
            Some(ident) if ident == TRAVERSAL => {
                Err(meta.error(
                    "`traversal` has been renamed to `propagate`, use that instead. If you were writing `traversal = &'static ChildOf`, you can now just write `propagate`, which defaults to the `ChildOf` traversal."
                ))
            }
            Some(ident) if ident == PROPAGATE => {
                propagate = true;
                if meta.input.peek(Token![=]) {
                    traversal = Some(meta.value()?.parse()?);
                }
                processed_attrs.push(PROPAGATE);
                Ok(())
            }
            Some(ident) if ident == TRIGGER => {
                trigger = Some(meta.value()?.parse()?);
                processed_attrs.push(TRIGGER);
                Ok(())
            }
            Some(ident) => Err(meta.error(format!("unsupported attribute: {ident}"))),
            None => Err(meta.error("expected identifier")),
        }) {
            return e.to_compile_error().into();
        }
    }

    if trigger.is_some() && propagate {
        return syn::Error::new(
            ast.span(),
            "Cannot define both #[entity_event(trigger)] and #[entity_event(propagate)]",
        )
        .into_compile_error()
        .into();
    }

    let entity_field = match get_event_target_field(&ast) {
        Ok(value) => value,
        Err(err) => return err.into_compile_error().into(),
    };

    let struct_name = &ast.ident;
    let (impl_generics, type_generics, where_clause) = &ast.generics.split_for_impl();

    let trigger = if let Some(trigger) = trigger {
        quote! {#trigger}
    } else if propagate {
        let traversal = traversal
            .unwrap_or_else(|| parse_quote! { &'static #bevy_ecs_path::hierarchy::ChildOf});
        quote! {#bevy_ecs_path::event::PropagateEntityTrigger<#auto_propagate, Self, #traversal>}
    } else {
        quote! {#bevy_ecs_path::event::EntityTrigger}
    };
    TokenStream::from(quote! {
        impl #impl_generics #bevy_ecs_path::event::Event for #struct_name #type_generics #where_clause {
            type Trigger<'a> = #trigger;
        }

        impl #impl_generics #bevy_ecs_path::event::EntityEvent for #struct_name #type_generics #where_clause {
            fn event_target(&self) -> #bevy_ecs_path::entity::Entity {
                self.#entity_field
            }

            fn event_target_mut(&mut self) -> &mut #bevy_ecs_path::entity::Entity {
                &mut self.#entity_field
            }
        }

    })
}

/// Returns the field with the `#[event_target]` attribute, the only field if unnamed,
/// or the field with the name "entity".
fn get_event_target_field(ast: &DeriveInput) -> Result<Member> {
    let Data::Struct(DataStruct { fields, .. }) = &ast.data else {
        return Err(syn::Error::new(
            ast.span(),
            "EntityEvent can only be derived for structs.",
        ));
    };
    match fields {
        Fields::Named(fields) => fields.named.iter().find_map(|field| {
            if field.ident.as_ref().is_some_and(|i| i == "entity") || field
                .attrs
                .iter()
                .any(|attr| attr.path().is_ident(EVENT_TARGET)) {
                    Some(Member::Named(field.ident.clone()?))
                } else {
                    None
                }
        }).ok_or(syn::Error::new(
            fields.span(),
            "EntityEvent derive expected a field name 'entity' or a field annotated with #[event_target]."
        )),
        Fields::Unnamed(fields) if fields.unnamed.len() == 1 => Ok(Member::Unnamed(Index::from(0))),
        Fields::Unnamed(fields) => fields.unnamed.iter().enumerate().find_map(|(index, field)| {
                if field
                    .attrs
                    .iter()
                    .any(|attr| attr.path().is_ident(EVENT_TARGET)) {
                        Some(Member::Unnamed(Index::from(index)))
                    } else {
                        None
                    }
            })
            .ok_or(syn::Error::new(
                fields.span(),
                "EntityEvent derive expected unnamed structs with one field or with a field annotated with #[event_target].",
            )),
        Fields::Unit => Err(syn::Error::new(
            fields.span(),
            "EntityEvent derive does not work on unit structs. Your type must have a field to store the `Entity` target, such as `Attack(Entity)` or `Attack { entity: Entity }`.",
        )),
    }
}
