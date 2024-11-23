// FIXME(15321): solve CI failures, then replace with `#![expect()]`.
#![allow(missing_docs, reason = "Not all docs are written yet, see #3492.")]
#![cfg_attr(docsrs, feature(doc_auto_cfg))]

extern crate proc_macro;

mod component;
mod query_data;
mod query_filter;
mod states;
mod world_query;

use crate::{query_data::derive_query_data_impl, query_filter::derive_query_filter_impl};
use bevy_macro_utils::{derive_label, ensure_no_collision, get_struct_fields, BevyManifest};
use proc_macro::TokenStream;
use proc_macro2::Span;
use proc_macro2::TokenStream as TokenStream2;
use quote::{format_ident, quote};
use syn::{
    parse_macro_input, parse_quote, punctuated::Punctuated, spanned::Spanned, token::Comma,
    ConstParam, DeriveInput, GenericParam, Ident, Index, TypeParam,
};

enum BundleFieldKind {
    Component,
    Ignore,
}

const BUNDLE_ATTRIBUTE_NAME: &str = "bundle";
const BUNDLE_ATTRIBUTE_IGNORE_NAME: &str = "ignore";

#[proc_macro_derive(Bundle, attributes(bundle))]
pub fn derive_bundle(input: TokenStream) -> TokenStream {
    let ast = parse_macro_input!(input as DeriveInput);
    let ecs_path = bevy_ecs_path();

    let named_fields = match get_struct_fields(&ast.data) {
        Ok(fields) => fields,
        Err(e) => return e.into_compile_error().into(),
    };

    let mut field_kind = Vec::with_capacity(named_fields.len());

    for field in named_fields {
        for attr in field
            .attrs
            .iter()
            .filter(|a| a.path().is_ident(BUNDLE_ATTRIBUTE_NAME))
        {
            if let Err(error) = attr.parse_nested_meta(|meta| {
                if meta.path.is_ident(BUNDLE_ATTRIBUTE_IGNORE_NAME) {
                    field_kind.push(BundleFieldKind::Ignore);
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

        field_kind.push(BundleFieldKind::Component);
    }

    let field = named_fields
        .iter()
        .map(|field| field.ident.as_ref())
        .collect::<Vec<_>>();

    let field_type = named_fields
        .iter()
        .map(|field| &field.ty)
        .collect::<Vec<_>>();

    let mut field_component_ids = Vec::new();
    let mut field_get_component_ids = Vec::new();
    let mut field_get_components = Vec::new();
    let mut field_from_components = Vec::new();
    let mut field_required_components = Vec::new();
    for (((i, field_type), field_kind), field) in field_type
        .iter()
        .enumerate()
        .zip(field_kind.iter())
        .zip(field.iter())
    {
        match field_kind {
            BundleFieldKind::Component => {
                field_component_ids.push(quote! {
                <#field_type as #ecs_path::bundle::Bundle>::component_ids(components, storages, &mut *ids);
                });
                field_required_components.push(quote! {
                    <#field_type as #ecs_path::bundle::Bundle>::register_required_components(components, storages, required_components);
                });
                field_get_component_ids.push(quote! {
                    <#field_type as #ecs_path::bundle::Bundle>::get_component_ids(components, &mut *ids);
                });
                match field {
                    Some(field) => {
                        field_get_components.push(quote! {
                            self.#field.get_components(&mut *func);
                        });
                        field_from_components.push(quote! {
                            #field: <#field_type as #ecs_path::bundle::Bundle>::from_components(ctx, &mut *func),
                        });
                    }
                    None => {
                        let index = Index::from(i);
                        field_get_components.push(quote! {
                            self.#index.get_components(&mut *func);
                        });
                        field_from_components.push(quote! {
                            #index: <#field_type as #ecs_path::bundle::Bundle>::from_components(ctx, &mut *func),
                        });
                    }
                }
            }

            BundleFieldKind::Ignore => {
                field_from_components.push(quote! {
                    #field: ::core::default::Default::default(),
                });
            }
        }
    }
    let generics = ast.generics;
    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();
    let struct_name = &ast.ident;

    TokenStream::from(quote! {
        // SAFETY:
        // - ComponentId is returned in field-definition-order. [from_components] and [get_components] use field-definition-order
        // - `Bundle::get_components` is exactly once for each member. Rely's on the Component -> Bundle implementation to properly pass
        //   the correct `StorageType` into the callback.
        unsafe impl #impl_generics #ecs_path::bundle::Bundle for #struct_name #ty_generics #where_clause {
            fn component_ids(
                components: &mut #ecs_path::component::Components,
                storages: &mut #ecs_path::storage::Storages,
                ids: &mut impl FnMut(#ecs_path::component::ComponentId)
            ){
                #(#field_component_ids)*
            }

            fn get_component_ids(
                components: &#ecs_path::component::Components,
                ids: &mut impl FnMut(Option<#ecs_path::component::ComponentId>)
            ){
                #(#field_get_component_ids)*
            }

            #[allow(unused_variables, non_snake_case)]
            unsafe fn from_components<__T, __F>(ctx: &mut __T, func: &mut __F) -> Self
            where
                __F: FnMut(&mut __T) -> #ecs_path::ptr::OwningPtr<'_>
            {
                Self{
                    #(#field_from_components)*
                }
            }

            fn register_required_components(
                components: &mut #ecs_path::component::Components,
                storages: &mut #ecs_path::storage::Storages,
                required_components: &mut #ecs_path::component::RequiredComponents
            ){
                #(#field_required_components)*
            }
        }

        impl #impl_generics #ecs_path::bundle::DynamicBundle for #struct_name #ty_generics #where_clause {
            #[allow(unused_variables)]
            #[inline]
            fn get_components(
                self,
                func: &mut impl FnMut(#ecs_path::component::StorageType, #ecs_path::ptr::OwningPtr<'_>)
            ) {
                #(#field_get_components)*
            }
        }
    })
}

fn derive_visit_entities_base(
    input: TokenStream,
    trait_name: TokenStream2,
    gen_methods: impl FnOnce(Vec<TokenStream2>) -> TokenStream2,
) -> TokenStream {
    let ast = parse_macro_input!(input as DeriveInput);
    let ecs_path = bevy_ecs_path();

    let named_fields = match get_struct_fields(&ast.data) {
        Ok(fields) => fields,
        Err(e) => return e.into_compile_error().into(),
    };

    let field = named_fields
        .iter()
        .filter_map(|field| {
            if let Some(attr) = field
                .attrs
                .iter()
                .find(|a| a.path().is_ident("visit_entities"))
            {
                let ignore = attr.parse_nested_meta(|meta| {
                    if meta.path.is_ident("ignore") {
                        Ok(())
                    } else {
                        Err(meta.error("Invalid visit_entities attribute. Use `ignore`"))
                    }
                });
                return match ignore {
                    Ok(()) => None,
                    Err(e) => Some(Err(e)),
                };
            }
            Some(Ok(field))
        })
        .map(|res| res.map(|field| field.ident.as_ref()))
        .collect::<Result<Vec<_>, _>>();

    let field = match field {
        Ok(field) => field,
        Err(e) => return e.into_compile_error().into(),
    };

    if field.is_empty() {
        return syn::Error::new(
            ast.span(),
            format!("Invalid `{}` type: at least one field", trait_name),
        )
        .into_compile_error()
        .into();
    }

    let field_access = field
        .iter()
        .enumerate()
        .map(|(n, f)| {
            if let Some(ident) = f {
                quote! {
                    self.#ident
                }
            } else {
                let idx = Index::from(n);
                quote! {
                    self.#idx
                }
            }
        })
        .collect::<Vec<_>>();

    let methods = gen_methods(field_access);

    let generics = ast.generics;
    let (impl_generics, ty_generics, _) = generics.split_for_impl();
    let struct_name = &ast.ident;

    TokenStream::from(quote! {
        impl #impl_generics #ecs_path::entity:: #trait_name for #struct_name #ty_generics {
            #methods
        }
    })
}

#[proc_macro_derive(VisitEntitiesMut, attributes(visit_entities))]
pub fn derive_visit_entities_mut(input: TokenStream) -> TokenStream {
    derive_visit_entities_base(input, quote! { VisitEntitiesMut }, |field| {
        quote! {
            fn visit_entities_mut<F: FnMut(&mut Entity)>(&mut self, mut f: F) {
                #(#field.visit_entities_mut(&mut f);)*
            }
        }
    })
}

#[proc_macro_derive(VisitEntities, attributes(visit_entities))]
pub fn derive_visit_entities(input: TokenStream) -> TokenStream {
    derive_visit_entities_base(input, quote! { VisitEntities }, |field| {
        quote! {
            fn visit_entities<F: FnMut(Entity)>(&self, mut f: F) {
                #(#field.visit_entities(&mut f);)*
            }
        }
    })
}

fn get_idents(fmt_string: fn(usize) -> String, count: usize) -> Vec<Ident> {
    (0..count)
        .map(|i| Ident::new(&fmt_string(i), Span::call_site()))
        .collect::<Vec<Ident>>()
}

#[proc_macro]
pub fn impl_param_set(_input: TokenStream) -> TokenStream {
    let mut tokens = TokenStream::new();
    let max_params = 8;
    let params = get_idents(|i| format!("P{i}"), max_params);
    let metas = get_idents(|i| format!("m{i}"), max_params);
    let mut param_fn_muts = Vec::new();
    for (i, param) in params.iter().enumerate() {
        let fn_name = Ident::new(&format!("p{i}"), Span::call_site());
        let index = Index::from(i);
        let ordinal = match i {
            1 => "1st".to_owned(),
            2 => "2nd".to_owned(),
            3 => "3rd".to_owned(),
            x => format!("{x}th"),
        };
        let comment =
            format!("Gets exclusive access to the {ordinal} parameter in this [`ParamSet`].");
        param_fn_muts.push(quote! {
            #[doc = #comment]
            /// No other parameters may be accessed while this one is active.
            pub fn #fn_name<'a>(&'a mut self) -> SystemParamItem<'a, 'a, #param> {
                // SAFETY: systems run without conflicts with other systems.
                // Conflicting params in ParamSet are not accessible at the same time
                // ParamSets are guaranteed to not conflict with other SystemParams
                unsafe {
                    #param::get_param(&mut self.param_states.#index, &self.system_meta, self.world, self.change_tick)
                }
            }
        });
    }

    for param_count in 1..=max_params {
        let param = &params[0..param_count];
        let meta = &metas[0..param_count];
        let param_fn_mut = &param_fn_muts[0..param_count];
        tokens.extend(TokenStream::from(quote! {
            // SAFETY: All parameters are constrained to ReadOnlySystemParam, so World is only read
            unsafe impl<'w, 's, #(#param,)*> ReadOnlySystemParam for ParamSet<'w, 's, (#(#param,)*)>
            where #(#param: ReadOnlySystemParam,)*
            { }

            // SAFETY: Relevant parameter ComponentId and ArchetypeComponentId access is applied to SystemMeta. If any ParamState conflicts
            // with any prior access, a panic will occur.
            unsafe impl<'_w, '_s, #(#param: SystemParam,)*> SystemParam for ParamSet<'_w, '_s, (#(#param,)*)>
            {
                type State = (#(#param::State,)*);
                type Item<'w, 's> = ParamSet<'w, 's, (#(#param,)*)>;

                // Note: We allow non snake case so the compiler don't complain about the creation of non_snake_case variables
                #[allow(non_snake_case)]
                fn init_state(world: &mut World, system_meta: &mut SystemMeta) -> Self::State {
                    #(
                        // Pretend to add each param to the system alone, see if it conflicts
                        let mut #meta = system_meta.clone();
                        #meta.component_access_set.clear();
                        #meta.archetype_component_access.clear();
                        #param::init_state(world, &mut #meta);
                        // The variable is being defined with non_snake_case here
                        let #param = #param::init_state(world, &mut system_meta.clone());
                    )*
                    // Make the ParamSet non-send if any of its parameters are non-send.
                    if false #(|| !#meta.is_send())* {
                        system_meta.set_non_send();
                    }
                    #(
                        system_meta
                            .component_access_set
                            .extend(#meta.component_access_set);
                        system_meta
                            .archetype_component_access
                            .extend(&#meta.archetype_component_access);
                    )*
                    (#(#param,)*)
                }

                unsafe fn new_archetype(state: &mut Self::State, archetype: &Archetype, system_meta: &mut SystemMeta) {
                    // SAFETY: The caller ensures that `archetype` is from the World the state was initialized from in `init_state`.
                    unsafe { <(#(#param,)*) as SystemParam>::new_archetype(state, archetype, system_meta); }
                }

                fn apply(state: &mut Self::State, system_meta: &SystemMeta, world: &mut World) {
                    <(#(#param,)*) as SystemParam>::apply(state, system_meta, world);
                }

                fn queue(state: &mut Self::State, system_meta: &SystemMeta, mut world: DeferredWorld) {
                    <(#(#param,)*) as SystemParam>::queue(state, system_meta, world.reborrow());
                }

                #[inline]
                unsafe fn validate_param<'w, 's>(
                    state: &'s Self::State,
                    system_meta: &SystemMeta,
                    world: UnsafeWorldCell<'w>,
                ) -> bool {
                    <(#(#param,)*) as SystemParam>::validate_param(state, system_meta, world)
                }

                #[inline]
                unsafe fn get_param<'w, 's>(
                    state: &'s mut Self::State,
                    system_meta: &SystemMeta,
                    world: UnsafeWorldCell<'w>,
                    change_tick: Tick,
                ) -> Self::Item<'w, 's> {
                    ParamSet {
                        param_states: state,
                        system_meta: system_meta.clone(),
                        world,
                        change_tick,
                    }
                }
            }

            impl<'w, 's, #(#param: SystemParam,)*> ParamSet<'w, 's, (#(#param,)*)>
            {
                #(#param_fn_mut)*
            }
        }));
    }

    tokens
}

/// Implement `SystemParam` to use a struct as a parameter in a system
#[proc_macro_derive(SystemParam, attributes(system_param))]
pub fn derive_system_param(input: TokenStream) -> TokenStream {
    let token_stream = input.clone();
    let ast = parse_macro_input!(input as DeriveInput);
    let syn::Data::Struct(syn::DataStruct {
        fields: field_definitions,
        ..
    }) = ast.data
    else {
        return syn::Error::new(
            ast.span(),
            "Invalid `SystemParam` type: expected a `struct`",
        )
        .into_compile_error()
        .into();
    };
    let path = bevy_ecs_path();

    let mut field_locals = Vec::new();
    let mut fields = Vec::new();
    let mut field_types = Vec::new();
    for (i, field) in field_definitions.iter().enumerate() {
        field_locals.push(format_ident!("f{i}"));
        let i = Index::from(i);
        fields.push(
            field
                .ident
                .as_ref()
                .map(|f| quote! { #f })
                .unwrap_or_else(|| quote! { #i }),
        );
        field_types.push(&field.ty);
    }

    let generics = ast.generics;

    // Emit an error if there's any unrecognized lifetime names.
    for lt in generics.lifetimes() {
        let ident = &lt.lifetime.ident;
        let w = format_ident!("w");
        let s = format_ident!("s");
        if ident != &w && ident != &s {
            return syn::Error::new_spanned(
                lt,
                r#"invalid lifetime name: expected `'w` or `'s`
 'w -- refers to data stored in the World.
 's -- refers to data stored in the SystemParam's state.'"#,
            )
            .into_compile_error()
            .into();
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

    let mut tuple_types: Vec<_> = field_types.iter().map(|x| quote! { #x }).collect();
    let mut tuple_patterns: Vec<_> = field_locals.iter().map(|x| quote! { #x }).collect();

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
        if let Err(e) = meta.parse_nested_meta(|nested| {
            if nested.path.is_ident("builder") {
                builder_name = Some(format_ident!("{struct_name}Builder"));
                Ok(())
            } else {
                Err(nested.error("Unsupported attribute"))
            }
        }) {
            return e.into_compile_error().into();
        }
    }

    let builder = builder_name.map(|builder_name| {
        let builder_type_parameters: Vec<_> = (0..fields.len()).map(|i| format_ident!("B{i}")).collect();
        let builder_doc_comment = format!("A [`SystemParamBuilder`] for a [`{struct_name}`].");
        let builder_struct = quote! {
            #[doc = #builder_doc_comment]
            struct #builder_name<#(#builder_type_parameters,)*> {
                #(#fields: #builder_type_parameters,)*
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
                fn build(self, world: &mut #path::world::World, meta: &mut #path::system::SystemMeta) -> <#generic_struct as #path::system::SystemParam>::State {
                    let #builder_name { #(#fields: #field_locals,)* } = self;
                    #state_struct_name {
                        state: #path::system::SystemParamBuilder::build((#(#tuple_patterns,)*), world, meta)
                    }
                }
            }
        };
        (builder_struct, builder_impl)
    });
    let (builder_struct, builder_impl) = builder.unzip();

    TokenStream::from(quote! {
        // We define the FetchState struct in an anonymous scope to avoid polluting the user namespace.
        // The struct can still be accessed via SystemParam::State, e.g. EventReaderState can be accessed via
        // <EventReader<'static, 'static, T> as SystemParam>::State
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

                fn init_state(world: &mut #path::world::World, system_meta: &mut #path::system::SystemMeta) -> Self::State {
                    #state_struct_name {
                        state: <#fields_alias::<'_, '_, #punctuated_generic_idents> as #path::system::SystemParam>::init_state(world, system_meta),
                    }
                }

                unsafe fn new_archetype(state: &mut Self::State, archetype: &#path::archetype::Archetype, system_meta: &mut #path::system::SystemMeta) {
                    // SAFETY: The caller ensures that `archetype` is from the World the state was initialized from in `init_state`.
                    unsafe { <#fields_alias::<'_, '_, #punctuated_generic_idents> as #path::system::SystemParam>::new_archetype(&mut state.state, archetype, system_meta) }
                }

                fn apply(state: &mut Self::State, system_meta: &#path::system::SystemMeta, world: &mut #path::world::World) {
                    <#fields_alias::<'_, '_, #punctuated_generic_idents> as #path::system::SystemParam>::apply(&mut state.state, system_meta, world);
                }

                fn queue(state: &mut Self::State, system_meta: &#path::system::SystemMeta, world: #path::world::DeferredWorld) {
                    <#fields_alias::<'_, '_, #punctuated_generic_idents> as #path::system::SystemParam>::queue(&mut state.state, system_meta, world);
                }

                #[inline]
                unsafe fn validate_param<'w, 's>(
                    state: &'s Self::State,
                    system_meta: &#path::system::SystemMeta,
                    world: #path::world::unsafe_world_cell::UnsafeWorldCell<'w>,
                ) -> bool {
                    <(#(#tuple_types,)*) as #path::system::SystemParam>::validate_param(&state.state, system_meta, world)
                }

                #[inline]
                unsafe fn get_param<'w, 's>(
                    state: &'s mut Self::State,
                    system_meta: &#path::system::SystemMeta,
                    world: #path::world::unsafe_world_cell::UnsafeWorldCell<'w>,
                    change_tick: #path::component::Tick,
                ) -> Self::Item<'w, 's> {
                    let (#(#tuple_patterns,)*) = <
                        (#(#tuple_types,)*) as #path::system::SystemParam
                    >::get_param(&mut state.state, system_meta, world, change_tick);
                    #struct_name {
                        #(#fields: #field_locals,)*
                    }
                }
            }

            // Safety: Each field is `ReadOnlySystemParam`, so this can only read from the `World`
            unsafe impl<'w, 's, #punctuated_generics> #path::system::ReadOnlySystemParam for #struct_name #ty_generics #read_only_where_clause {}

            #builder_impl
        };

        #builder_struct
    })
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
    let mut dyn_eq_path = trait_path.clone();
    trait_path
        .segments
        .push(format_ident!("ScheduleLabel").into());
    dyn_eq_path.segments.push(format_ident!("DynEq").into());
    derive_label(input, "ScheduleLabel", &trait_path, &dyn_eq_path)
}

/// Derive macro generating an impl of the trait `SystemSet`.
///
/// This does not work for unions.
#[proc_macro_derive(SystemSet)]
pub fn derive_system_set(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let mut trait_path = bevy_ecs_path();
    trait_path.segments.push(format_ident!("schedule").into());
    let mut dyn_eq_path = trait_path.clone();
    trait_path.segments.push(format_ident!("SystemSet").into());
    dyn_eq_path.segments.push(format_ident!("DynEq").into());
    derive_label(input, "SystemSet", &trait_path, &dyn_eq_path)
}

pub(crate) fn bevy_ecs_path() -> syn::Path {
    BevyManifest::default().get_path("bevy_ecs")
}

#[proc_macro_derive(Event)]
pub fn derive_event(input: TokenStream) -> TokenStream {
    component::derive_event(input)
}

#[proc_macro_derive(Resource)]
pub fn derive_resource(input: TokenStream) -> TokenStream {
    component::derive_resource(input)
}

#[proc_macro_derive(Component, attributes(component, require))]
pub fn derive_component(input: TokenStream) -> TokenStream {
    component::derive_component(input)
}

#[proc_macro_derive(States)]
pub fn derive_states(input: TokenStream) -> TokenStream {
    states::derive_states(input)
}

#[proc_macro_derive(SubStates, attributes(source))]
pub fn derive_substates(input: TokenStream) -> TokenStream {
    states::derive_substates(input)
}
