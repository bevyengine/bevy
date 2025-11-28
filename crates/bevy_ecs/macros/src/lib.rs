//! Macros for deriving ECS traits.

#![cfg_attr(docsrs, feature(doc_cfg))]

extern crate proc_macro;

mod component;
mod event;
mod message;
mod query_data;
mod query_filter;
mod system_composition;
mod world_query;

use crate::{
    component::map_entities, query_data::derive_query_data_impl,
    query_filter::derive_query_filter_impl,
};
use bevy_macro_utils::{derive_label, ensure_no_collision, get_struct_fields, BevyManifest};
use proc_macro::TokenStream;
use proc_macro2::{Ident, Span};
use quote::{format_ident, quote, ToTokens};
use syn::{
    parse_macro_input, parse_quote, punctuated::Punctuated, token::Comma, ConstParam, Data,
    DeriveInput, GenericParam, TypeParam,
};

enum BundleFieldKind {
    Component,
    Ignore,
}

const BUNDLE_ATTRIBUTE_NAME: &str = "bundle";
const BUNDLE_ATTRIBUTE_IGNORE_NAME: &str = "ignore";
const BUNDLE_ATTRIBUTE_NO_FROM_COMPONENTS: &str = "ignore_from_components";

#[derive(Debug)]
struct BundleAttributes {
    impl_from_components: bool,
}

impl Default for BundleAttributes {
    fn default() -> Self {
        Self {
            impl_from_components: true,
        }
    }
}

/// Implement the `Bundle` trait.
#[proc_macro_derive(Bundle, attributes(bundle))]
pub fn derive_bundle(input: TokenStream) -> TokenStream {
    let ast = parse_macro_input!(input as DeriveInput);
    let ecs_path = bevy_ecs_path();

    let mut attributes = BundleAttributes::default();

    for attr in &ast.attrs {
        if attr.path().is_ident(BUNDLE_ATTRIBUTE_NAME) {
            let parsing = attr.parse_nested_meta(|meta| {
                if meta.path.is_ident(BUNDLE_ATTRIBUTE_NO_FROM_COMPONENTS) {
                    attributes.impl_from_components = false;
                    return Ok(());
                }

                Err(meta.error(format!("Invalid bundle container attribute. Allowed attributes: `{BUNDLE_ATTRIBUTE_NO_FROM_COMPONENTS}`")))
            });

            if let Err(e) = parsing {
                return e.into_compile_error().into();
            }
        }
    }

    let fields = match get_struct_fields(&ast.data, "derive(Bundle)") {
        Ok(fields) => fields,
        Err(e) => return e.into_compile_error().into(),
    };

    let mut field_kinds = Vec::with_capacity(fields.len());

    for field in fields {
        let mut kind = BundleFieldKind::Component;

        for attr in field
            .attrs
            .iter()
            .filter(|a| a.path().is_ident(BUNDLE_ATTRIBUTE_NAME))
        {
            if let Err(error) = attr.parse_nested_meta(|meta| {
                if meta.path.is_ident(BUNDLE_ATTRIBUTE_IGNORE_NAME) {
                    kind = BundleFieldKind::Ignore;
                    Ok(())
                } else {
                    Err(meta.error(format!(
                        "Invalid bundle attribute. Use `{BUNDLE_ATTRIBUTE_IGNORE_NAME}`"
                    )))
                }
            }) {
                return error.into_compile_error().into();
            }
        }

        field_kinds.push(kind);
    }

    let field_types = fields.iter().map(|field| &field.ty).collect::<Vec<_>>();

    let mut active_field_types = Vec::new();
    let mut active_field_members = Vec::new();
    let mut active_field_locals = Vec::new();
    let mut inactive_field_members = Vec::new();
    for ((field_member, field_type), field_kind) in
        fields.members().zip(field_types).zip(field_kinds)
    {
        let field_local = format_ident!("field_{}", field_member);

        match field_kind {
            BundleFieldKind::Component => {
                active_field_types.push(field_type);
                active_field_locals.push(field_local);
                active_field_members.push(field_member);
            }
            BundleFieldKind::Ignore => inactive_field_members.push(field_member),
        }
    }
    let generics = ast.generics;
    let generics_ty_list = generics.type_params().map(|p| p.ident.clone());
    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();
    let struct_name = &ast.ident;

    let bundle_impl = quote! {
        // SAFETY:
        // - ComponentId is returned in field-definition-order. [get_components] uses field-definition-order
        // - `Bundle::get_components` is exactly once for each member. Rely's on the Component -> Bundle implementation to properly pass
        //   the correct `StorageType` into the callback.
        unsafe impl #impl_generics #ecs_path::bundle::Bundle for #struct_name #ty_generics #where_clause {
            fn component_ids(
                components: &mut #ecs_path::component::ComponentsRegistrator,
            ) -> impl Iterator<Item = #ecs_path::component::ComponentId> + use<#(#generics_ty_list,)*> {
                core::iter::empty()#(.chain(<#active_field_types as #ecs_path::bundle::Bundle>::component_ids(components)))*
            }

            fn get_component_ids(
                components: &#ecs_path::component::Components,
            ) -> impl Iterator<Item = Option<#ecs_path::component::ComponentId>> {
                core::iter::empty()#(.chain(<#active_field_types as #ecs_path::bundle::Bundle>::get_component_ids(components)))*
            }
        }
    };

    let dynamic_bundle_impl = quote! {
        impl #impl_generics #ecs_path::bundle::DynamicBundle for #struct_name #ty_generics #where_clause {
            type Effect = ();
            #[allow(unused_variables)]
            #[inline]
            unsafe fn get_components(
                ptr: #ecs_path::ptr::MovingPtr<'_, Self>,
                func: &mut impl FnMut(#ecs_path::component::StorageType, #ecs_path::ptr::OwningPtr<'_>)
            ) {
                use #ecs_path::__macro_exports::DebugCheckedUnwrap;

                #ecs_path::ptr::deconstruct_moving_ptr!({
                    let #struct_name { #(#active_field_members: #active_field_locals,)* #(#inactive_field_members: _,)* } = ptr;
                });
                #(
                    <#active_field_types as #ecs_path::bundle::DynamicBundle>::get_components(
                        #active_field_locals,
                        func
                    );
                )*
            }

            #[allow(unused_variables)]
            #[inline]
            unsafe fn apply_effect(
                ptr: #ecs_path::ptr::MovingPtr<'_, core::mem::MaybeUninit<Self>>,
                func: &mut #ecs_path::world::EntityWorldMut<'_>,
            ) {
            }
        }
    };

    let from_components_impl = attributes.impl_from_components.then(|| quote! {
        // SAFETY:
        // - ComponentId is returned in field-definition-order. [from_components] uses field-definition-order
        unsafe impl #impl_generics #ecs_path::bundle::BundleFromComponents for #struct_name #ty_generics #where_clause {
            #[allow(unused_variables, non_snake_case)]
            unsafe fn from_components<__T, __F>(ctx: &mut __T, func: &mut __F) -> Self
            where
                __F: FnMut(&mut __T) -> #ecs_path::ptr::OwningPtr<'_>
            {
                Self {
                    #(#active_field_members: <#active_field_types as #ecs_path::bundle::BundleFromComponents>::from_components(ctx, &mut *func),)*
                    #(#inactive_field_members: ::core::default::Default::default(),)*
                }
            }
        }
    });
    TokenStream::from(quote! {
        #bundle_impl
        #from_components_impl
        #dynamic_bundle_impl
    })
}

/// Implement the `MapEntities` trait.
#[proc_macro_derive(MapEntities, attributes(entities))]
pub fn derive_map_entities(input: TokenStream) -> TokenStream {
    let ast = parse_macro_input!(input as DeriveInput);
    let ecs_path = bevy_ecs_path();

    let map_entities_impl = map_entities(
        &ast.data,
        &ecs_path,
        Ident::new("self", Span::call_site()),
        false,
        false,
        None,
    );

    let struct_name = &ast.ident;
    let (impl_generics, type_generics, where_clause) = &ast.generics.split_for_impl();
    TokenStream::from(quote! {
        impl #impl_generics #ecs_path::entity::MapEntities for #struct_name #type_generics #where_clause {
            fn map_entities<M: #ecs_path::entity::EntityMapper>(&mut self, mapper: &mut M) {
                #map_entities_impl
            }
        }
    })
}

/// Implement `SystemParam` to use a struct as a parameter in a system
#[proc_macro_derive(SystemParam, attributes(system_param))]
pub fn derive_system_param(input: TokenStream) -> TokenStream {
    let token_stream = input.clone();
    let ast = parse_macro_input!(input as DeriveInput);

    match derive_system_param_impl(token_stream, ast) {
        Ok(t) => t,
        Err(e) => e.into_compile_error().into(),
    }
}
fn derive_system_param_impl(
    token_stream: TokenStream,
    ast: DeriveInput,
) -> syn::Result<TokenStream> {
    let fields = get_struct_fields(&ast.data, "derive(SystemParam)")?;
    let path = bevy_ecs_path();

    let field_locals = fields
        .members()
        .map(|m| format_ident!("field{}", m))
        .collect::<Vec<_>>();
    let field_members = fields.members().collect::<Vec<_>>();
    let field_types = fields.iter().map(|f| &f.ty).collect::<Vec<_>>();

    let field_validation_names = fields.members().map(|m| format!("::{}", quote! { #m }));
    let mut field_validation_messages = Vec::with_capacity(fields.len());
    for attr in fields
        .iter()
        .map(|f| f.attrs.iter().find(|a| a.path().is_ident("system_param")))
    {
        let mut field_validation_message = None;
        if let Some(attr) = attr {
            attr.parse_nested_meta(|nested| {
                if nested.path.is_ident("validation_message") {
                    field_validation_message = Some(nested.value()?.parse()?);
                    Ok(())
                } else {
                    Err(nested.error("Unsupported attribute"))
                }
            })?;
        }
        field_validation_messages
            .push(field_validation_message.unwrap_or_else(|| quote! { err.message }));
    }

    let generics = ast.generics;

    // Emit an error if there's any unrecognized lifetime names.
    let w = format_ident!("w");
    let s = format_ident!("s");
    for lt in generics.lifetimes() {
        let ident = &lt.lifetime.ident;
        if ident != &w && ident != &s {
            return Err(syn::Error::new_spanned(
                lt,
                r#"invalid lifetime name: expected `'w` or `'s`
 'w -- refers to data stored in the World.
 's -- refers to data stored in the SystemParam's state.'"#,
            ));
        }
    }

    let (_impl_generics, ty_generics, where_clause) = generics.split_for_impl();

    let lifetimeless_generics: Vec<_> = generics
        .params
        .iter()
        .filter(|g| !matches!(g, GenericParam::Lifetime(_)))
        .collect();

    let shadowed_lifetimes: Vec<_> = generics.lifetimes().map(|_| quote!('_)).collect();

    let mut punctuated_generics = Punctuated::<_, Comma>::new();
    punctuated_generics.extend(lifetimeless_generics.iter().map(|g| match g {
        GenericParam::Type(g) => GenericParam::Type(TypeParam {
            default: None,
            ..g.clone()
        }),
        GenericParam::Const(g) => GenericParam::Const(ConstParam {
            default: None,
            ..g.clone()
        }),
        _ => unreachable!(),
    }));

    let mut punctuated_generic_idents = Punctuated::<_, Comma>::new();
    punctuated_generic_idents.extend(lifetimeless_generics.iter().map(|g| match g {
        GenericParam::Type(g) => &g.ident,
        GenericParam::Const(g) => &g.ident,
        _ => unreachable!(),
    }));

    let punctuated_generics_no_bounds: Punctuated<_, Comma> = lifetimeless_generics
        .iter()
        .map(|&g| match g.clone() {
            GenericParam::Type(mut g) => {
                g.bounds.clear();
                GenericParam::Type(g)
            }
            g => g,
        })
        .collect();

    let mut tuple_types: Vec<_> = field_types.iter().map(ToTokens::to_token_stream).collect();
    let mut tuple_patterns: Vec<_> = field_locals.iter().map(ToTokens::to_token_stream).collect();

    // If the number of fields exceeds the 16-parameter limit,
    // fold the fields into tuples of tuples until we are below the limit.
    const LIMIT: usize = 16;
    while tuple_types.len() > LIMIT {
        let end = Vec::from_iter(tuple_types.drain(..LIMIT));
        tuple_types.push(parse_quote!( (#(#end,)*) ));

        let end = Vec::from_iter(tuple_patterns.drain(..LIMIT));
        tuple_patterns.push(parse_quote!( (#(#end,)*) ));
    }
    // Create a where clause for the `ReadOnlySystemParam` impl.
    // Ensure that each field implements `ReadOnlySystemParam`.
    let mut read_only_generics = generics.clone();
    let read_only_where_clause = read_only_generics.make_where_clause();
    for field_type in &field_types {
        read_only_where_clause
            .predicates
            .push(syn::parse_quote!(#field_type: #path::system::ReadOnlySystemParam));
    }

    let fields_alias =
        ensure_no_collision(format_ident!("__StructFieldsAlias"), token_stream.clone());

    let struct_name = &ast.ident;
    let state_struct_visibility = &ast.vis;
    let state_struct_name = ensure_no_collision(format_ident!("FetchState"), token_stream);

    let mut builder_name = None;
    for meta in ast
        .attrs
        .iter()
        .filter(|a| a.path().is_ident("system_param"))
    {
        meta.parse_nested_meta(|nested| {
            if nested.path.is_ident("builder") {
                builder_name = Some(format_ident!("{struct_name}Builder"));
                Ok(())
            } else {
                Err(nested.error("Unsupported attribute"))
            }
        })?;
    }

    let builder = builder_name.map(|builder_name| {
        let builder_type_parameters: Vec<Ident> = field_members.iter().map(|m| format_ident!("B{}", m)).collect();
        let builder_doc_comment = format!("A [`SystemParamBuilder`] for a [`{struct_name}`].");
        let builder_struct = quote! {
            #[doc = #builder_doc_comment]
            struct #builder_name<#(#builder_type_parameters,)*> {
                #(#field_members: #builder_type_parameters,)*
            }
        };
        let lifetimes: Vec<_> = generics.lifetimes().collect();
        let generic_struct = quote!{ #struct_name <#(#lifetimes,)* #punctuated_generic_idents> };
        let builder_impl = quote!{
            // SAFETY: This delegates to the `SystemParamBuilder` for tuples.
            unsafe impl<
                #(#lifetimes,)*
                #(#builder_type_parameters: #path::system::SystemParamBuilder<#field_types>,)*
                #punctuated_generics
            > #path::system::SystemParamBuilder<#generic_struct> for #builder_name<#(#builder_type_parameters,)*>
                #where_clause
            {
                fn build(self, world: &mut #path::world::World) -> <#generic_struct as #path::system::SystemParam>::State {
                    let #builder_name { #(#field_members: #field_locals,)* } = self;
                    #state_struct_name {
                        state: #path::system::SystemParamBuilder::build((#(#tuple_patterns,)*), world)
                    }
                }
            }
        };
        (builder_struct, builder_impl)
    });
    let (builder_struct, builder_impl) = builder.unzip();

    Ok(TokenStream::from(quote! {
        // We define the FetchState struct in an anonymous scope to avoid polluting the user namespace.
        // The struct can still be accessed via SystemParam::State, e.g. MessageReaderState can be accessed via
        // <MessageReader<'static, 'static, T> as SystemParam>::State
        const _: () = {
            // Allows rebinding the lifetimes of each field type.
            type #fields_alias <'w, 's, #punctuated_generics_no_bounds> = (#(#tuple_types,)*);

            #[doc(hidden)]
            #state_struct_visibility struct #state_struct_name <#(#lifetimeless_generics,)*>
            #where_clause {
                state: <#fields_alias::<'static, 'static, #punctuated_generic_idents> as #path::system::SystemParam>::State,
            }

            unsafe impl<#punctuated_generics> #path::system::SystemParam for
                #struct_name <#(#shadowed_lifetimes,)* #punctuated_generic_idents> #where_clause
            {
                type State = #state_struct_name<#punctuated_generic_idents>;
                type Item<'w, 's> = #struct_name #ty_generics;

                fn init_state(world: &mut #path::world::World) -> Self::State {
                    #state_struct_name {
                        state: <#fields_alias::<'_, '_, #punctuated_generic_idents> as #path::system::SystemParam>::init_state(world),
                    }
                }

                fn init_access(state: &Self::State, system_meta: &mut #path::system::SystemMeta, component_access_set: &mut #path::query::FilteredAccessSet, world: &mut #path::world::World) {
                    <#fields_alias::<'_, '_, #punctuated_generic_idents> as #path::system::SystemParam>::init_access(&state.state, system_meta, component_access_set, world);
                }

                fn apply(state: &mut Self::State, system_meta: &#path::system::SystemMeta, world: &mut #path::world::World) {
                    <#fields_alias::<'_, '_, #punctuated_generic_idents> as #path::system::SystemParam>::apply(&mut state.state, system_meta, world);
                }

                fn queue(state: &mut Self::State, system_meta: &#path::system::SystemMeta, world: #path::world::DeferredWorld) {
                    <#fields_alias::<'_, '_, #punctuated_generic_idents> as #path::system::SystemParam>::queue(&mut state.state, system_meta, world);
                }

                #[inline]
                unsafe fn validate_param<'w, 's>(
                    state: &'s mut Self::State,
                    _system_meta: &#path::system::SystemMeta,
                    _world: #path::world::unsafe_world_cell::UnsafeWorldCell<'w>,
                ) -> Result<(), #path::system::SystemParamValidationError> {
                    let #state_struct_name { state: (#(#tuple_patterns,)*) } = state;
                    #(
                        <#field_types as #path::system::SystemParam>::validate_param(#field_locals, _system_meta, _world)
                            .map_err(|err| #path::system::SystemParamValidationError::new::<Self>(err.skipped, #field_validation_messages, #field_validation_names))?;
                    )*
                    Result::Ok(())
                }

                #[inline]
                unsafe fn get_param<'w, 's>(
                    state: &'s mut Self::State,
                    system_meta: &#path::system::SystemMeta,
                    world: #path::world::unsafe_world_cell::UnsafeWorldCell<'w>,
                    change_tick: #path::change_detection::Tick,
                ) -> Self::Item<'w, 's> {
                    let (#(#tuple_patterns,)*) = <
                        (#(#tuple_types,)*) as #path::system::SystemParam
                    >::get_param(&mut state.state, system_meta, world, change_tick);
                    #struct_name {
                        #(#field_members: #field_locals,)*
                    }
                }
            }

            // Safety: Each field is `ReadOnlySystemParam`, so this can only read from the `World`
            unsafe impl<'w, 's, #punctuated_generics> #path::system::ReadOnlySystemParam for #struct_name #ty_generics #read_only_where_clause {}

            #builder_impl
        };

        #builder_struct
    }))
}

/// Implement `QueryData` to use a struct as a data parameter in a query
#[proc_macro_derive(QueryData, attributes(query_data))]
pub fn derive_query_data(input: TokenStream) -> TokenStream {
    derive_query_data_impl(input)
}

/// Implement `QueryFilter` to use a struct as a filter parameter in a query
#[proc_macro_derive(QueryFilter, attributes(query_filter))]
pub fn derive_query_filter(input: TokenStream) -> TokenStream {
    derive_query_filter_impl(input)
}

/// Derive macro generating an impl of the trait `ScheduleLabel`.
///
/// This does not work for unions.
#[proc_macro_derive(ScheduleLabel)]
pub fn derive_schedule_label(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let mut trait_path = bevy_ecs_path();
    trait_path.segments.push(format_ident!("schedule").into());
    trait_path
        .segments
        .push(format_ident!("ScheduleLabel").into());
    derive_label(input, "ScheduleLabel", &trait_path)
}

/// Derive macro generating an impl of the trait `SystemSet`.
///
/// This does not work for unions.
#[proc_macro_derive(SystemSet)]
pub fn derive_system_set(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let mut trait_path = bevy_ecs_path();
    trait_path.segments.push(format_ident!("schedule").into());
    trait_path.segments.push(format_ident!("SystemSet").into());
    derive_label(input, "SystemSet", &trait_path)
}

pub(crate) fn bevy_ecs_path() -> syn::Path {
    BevyManifest::shared(|manifest| manifest.get_path("bevy_ecs"))
}

/// Implement the `Event` trait.
#[proc_macro_derive(Event, attributes(event))]
pub fn derive_event(input: TokenStream) -> TokenStream {
    event::derive_event(input)
}

/// Cheat sheet for derive syntax,
/// see full explanation on `EntityEvent` trait docs.
///
/// ```ignore
/// #[derive(EntityEvent)]
/// /// Enable propagation, which defaults to using the ChildOf component
/// #[entity_event(propagate)]
/// /// Enable propagation using the given Traversal implementation
/// #[entity_event(propagate = &'static ChildOf)]
/// /// Always propagate
/// #[entity_event(auto_propagate)]
/// struct MyEvent;
/// ```
#[proc_macro_derive(EntityEvent, attributes(entity_event, event_target))]
pub fn derive_entity_event(input: TokenStream) -> TokenStream {
    event::derive_entity_event(input)
}

/// Implement the `Message` trait.
#[proc_macro_derive(Message)]
pub fn derive_message(input: TokenStream) -> TokenStream {
    message::derive_message(input)
}

/// Implement the `Resource` trait.
#[proc_macro_derive(Resource)]
pub fn derive_resource(input: TokenStream) -> TokenStream {
    component::derive_resource(input)
}

/// Cheat sheet for derive syntax,
/// see full explanation and examples on the `Component` trait doc.
///
/// ## Immutability
/// ```ignore
/// #[derive(Component)]
/// #[component(immutable)]
/// struct MyComponent;
/// ```
///
/// ## Sparse instead of table-based storage
/// ```ignore
/// #[derive(Component)]
/// #[component(storage = "SparseSet")]
/// struct MyComponent;
/// ```
///
/// ## Required Components
///
/// ```ignore
/// #[derive(Component)]
/// #[require(
///     // `Default::default()`
///     A,
///     // tuple structs
///     B(1),
///     // named-field structs
///     C {
///         x: 1,
///         ..default()
///     },
///     // unit structs/variants
///     D::One,
///     // associated consts
///     E::ONE,
///     // constructors
///     F::new(1),
///     // arbitrary expressions
///     G = make(1, 2, 3)
/// )]
/// struct MyComponent;
/// ```
///
/// ## Relationships
/// ```ignore
/// #[derive(Component)]
/// #[relationship(relationship_target = Children)]
/// pub struct ChildOf {
///     // Marking the field is not necessary if there is only one.
///     #[relationship]
///     pub parent: Entity,
///     internal: u8,
/// };
///
/// #[derive(Component)]
/// #[relationship_target(relationship = ChildOf)]
/// pub struct Children(Vec<Entity>);
/// ```
///
/// On despawn, also despawn all related entities:
/// ```ignore
/// #[derive(Component)]
/// #[relationship_target(relationship_target = Children, linked_spawn)]
/// pub struct Children(Vec<Entity>);
/// ```
///
/// ## Hooks
/// ```ignore
/// #[derive(Component)]
/// #[component(hook_name = function)]
/// struct MyComponent;
/// ```
/// where `hook_name` is `on_add`, `on_insert`, `on_replace` or `on_remove`;  
/// `function` can be either a path, e.g. `some_function::<Self>`,
/// or a function call that returns a function that can be turned into
/// a `ComponentHook`, e.g. `get_closure("Hi!")`.
/// `function` can be elided if the path is `Self::on_add`, `Self::on_insert` etc.
///
/// ## Ignore this component when cloning an entity
/// ```ignore
/// #[derive(Component)]
/// #[component(clone_behavior = Ignore)]
/// struct MyComponent;
/// ```
#[proc_macro_derive(
    Component,
    attributes(component, require, relationship, relationship_target, entities)
)]
pub fn derive_component(input: TokenStream) -> TokenStream {
    component::derive_component(input)
}

/// Implement the `FromWorld` trait.
#[proc_macro_derive(FromWorld, attributes(from_world))]
pub fn derive_from_world(input: TokenStream) -> TokenStream {
    let bevy_ecs_path = bevy_ecs_path();
    let ast = parse_macro_input!(input as DeriveInput);
    let name = ast.ident;
    let (impl_generics, ty_generics, where_clauses) = ast.generics.split_for_impl();

    let (fields, variant_ident) = match &ast.data {
        Data::Struct(data) => (&data.fields, None),
        Data::Enum(data) => {
            match data.variants.iter().find(|variant| {
                variant
                    .attrs
                    .iter()
                    .any(|attr| attr.path().is_ident("from_world"))
            }) {
                Some(variant) => (&variant.fields, Some(&variant.ident)),
                None => {
                    return syn::Error::new(
                        Span::call_site(),
                        "No variant found with the `#[from_world]` attribute",
                    )
                    .into_compile_error()
                    .into();
                }
            }
        }
        Data::Union(_) => {
            return syn::Error::new(
                Span::call_site(),
                "#[derive(FromWorld)]` does not support unions",
            )
            .into_compile_error()
            .into();
        }
    };

    let field_init_expr = quote!(#bevy_ecs_path::world::FromWorld::from_world(world));
    let members = fields.members();

    let field_initializers = match variant_ident {
        Some(variant_ident) => quote!( Self::#variant_ident {
            #(#members: #field_init_expr),*
        }),
        None => quote!( Self {
            #(#members: #field_init_expr),*
        }),
    };

    TokenStream::from(quote! {
            impl #impl_generics #bevy_ecs_path::world::FromWorld for #name #ty_generics #where_clauses {
                fn from_world(world: &mut #bevy_ecs_path::world::World) -> Self {
                    #field_initializers
                }
            }
    })
}

/// Automatically compose systems together with function syntax.
///
/// This macro provides some nice syntax on top of the `SystemRunner` `SystemParam`
/// to allow running systems inside other systems. Overall, the macro accepts normal
/// closure syntax:
///
/// ```ignore
/// let system_a = |world: &mut World| { 10 };
/// let system_b = |a: In<u32>, world: &mut World| { println!("{}", *a + 12) };
/// compose! {
///     || -> Result<(), RunSystemError> {
///         let a = run!(system-a)?;
///         run!(system_b);
///     }
/// }
/// ```
///
/// What's special is that the macro will expand any invocations of `run!()` into
/// calls to `SystemRunner::run` or `SystemRunner::run_with`. The `run!()` accepts
/// two parameters: first, a system identifier (or a path to one), and second, an
/// optional input to invoke the system with.
///
/// Notes:
/// 1. All system runners are passed through a `ParamSet`, so invoked systems will
///    not conflict with each other. However, invoked systems may still conflict
///    with system params in the outer closure.
///
/// 2. `run!` will not accept expressions that evaluate to systems, only direct
///    identifiers or paths. So, if you want to call something like:
///
///    ```ignore
///    run!(|query: Query<(&A, &B, &mut C)>| { ... })`
///    ```
///
///    Assign the expression to a variable first.
#[proc_macro]
pub fn compose(input: TokenStream) -> TokenStream {
    system_composition::compose(input, false)
}

/// Automatically compose systems together with function syntax.
///
/// Unlike [`compose`], this macro allows generating systems that take input.
///
/// This macro provides some nice syntax on top of the `SystemRunner` `SystemParam`
/// to allow running systems inside other systems. Overall, the macro accepts normal
/// closure syntax:
///
/// ```ignore
/// let system_a = |input: In<u32>, world: &mut World| { *input + 10 };
/// let system_b = |a: In<u32>, world: &mut World| { println!("{}", *a + 12) };
/// compose_with! {
///     |input: In<u32>| -> Result<(), RunSystemError> {
///         let a = run!(system_a, input)?;
///         run!(system_b);
///     }
/// }
/// ```
///
/// What's special is that the macro will expand any invocations of `run!()` into
/// calls to `SystemRunner::run` or `SystemRunner::run_with`. The `run!()` accepts
/// two parameters: first, a system identifier (or a path to one), and second, an
/// optional input to invoke the system with.
///
/// Notes:
/// 1. All system runners are passed through a `ParamSet`, so invoked systems will
///    not conflict with each other. However, invoked systems may still conflict
///    with system params in the outer closure.
///
/// 2. `run!` will not accept expressions that evaluate to systems, only direct
///    identifiers or paths. So, if you want to call something like:
///
///    ```ignore
///    run!(|query: Query<(&A, &B, &mut C)>| { ... })`
///    ```
///
///    Assign the expression to a variable first.
#[proc_macro]
pub fn compose_with(input: TokenStream) -> TokenStream {
    system_composition::compose(input, true)
}
