extern crate proc_macro;

mod component;
mod fetch;

mod kw {
    syn::custom_keyword!(ignore);
    syn::custom_keyword!(infallible);
    syn::custom_keyword!(optional);
    syn::custom_keyword!(resultful);
}

use crate::fetch::derive_world_query_impl;
use bevy_macro_utils::{
    derive_boxed_label, derive_label, derive_set, get_named_struct_fields, BevyManifest,
};
use proc_macro::TokenStream;
use proc_macro2::Span;
use quote::{format_ident, quote};
use syn::{
    parenthesized,
    parse::{Parse, ParseStream},
    parse_macro_input,
    punctuated::Punctuated,
    spanned::Spanned,
    token::Comma,
    ConstParam, DeriveInput, GenericParam, Ident, Index, LitInt, Meta, MetaList, NestedMeta,
    Result, Token, TypeParam,
};

struct AllTuples {
    macro_ident: Ident,
    start: usize,
    end: usize,
    idents: Vec<Ident>,
}

impl Parse for AllTuples {
    fn parse(input: ParseStream) -> Result<Self> {
        let macro_ident = input.parse::<Ident>()?;
        input.parse::<Comma>()?;
        let start = input.parse::<LitInt>()?.base10_parse()?;
        input.parse::<Comma>()?;
        let end = input.parse::<LitInt>()?.base10_parse()?;
        input.parse::<Comma>()?;
        let mut idents = vec![input.parse::<Ident>()?];
        while input.parse::<Comma>().is_ok() {
            idents.push(input.parse::<Ident>()?);
        }

        Ok(AllTuples {
            macro_ident,
            start,
            end,
            idents,
        })
    }
}

#[proc_macro]
pub fn all_tuples(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as AllTuples);
    let len = input.end - input.start;
    let mut ident_tuples = Vec::with_capacity(len);
    for i in input.start..=input.end {
        let idents = input
            .idents
            .iter()
            .map(|ident| format_ident!("{}{}", ident, i));
        if input.idents.len() < 2 {
            ident_tuples.push(quote! {
                #(#idents)*
            });
        } else {
            ident_tuples.push(quote! {
                (#(#idents),*)
            });
        }
    }

    let macro_ident = &input.macro_ident;
    let invocations = (input.start..=input.end).map(|i| {
        let ident_tuples = &ident_tuples[..i];
        quote! {
            #macro_ident!(#(#ident_tuples),*);
        }
    });
    TokenStream::from(quote! {
        #(
            #invocations
        )*
    })
}

enum BundleFieldKind {
    Component,
    Ignore,
}

const BUNDLE_ATTRIBUTE_NAME: &str = "bundle";
const BUNDLE_ATTRIBUTE_IGNORE_NAME: &str = "ignore";

#[proc_macro_derive(Bundle, attributes(bundle))]
pub fn derive_bundle(input: TokenStream) -> TokenStream {
    let ast = parse_macro_input!(input as DeriveInput);
    let bevy_ecs = bevy_ecs_path();

    let named_fields = match get_named_struct_fields(&ast.data) {
        Ok(fields) => &fields.named,
        Err(e) => return e.into_compile_error().into(),
    };

    let mut field_kind = Vec::with_capacity(named_fields.len());

    'field_loop: for field in named_fields.iter() {
        for attr in &field.attrs {
            if attr.path.is_ident(BUNDLE_ATTRIBUTE_NAME) {
                if let Ok(Meta::List(MetaList { nested, .. })) = attr.parse_meta() {
                    if let Some(&NestedMeta::Meta(Meta::Path(ref path))) = nested.first() {
                        if path.is_ident(BUNDLE_ATTRIBUTE_IGNORE_NAME) {
                            field_kind.push(BundleFieldKind::Ignore);
                            continue 'field_loop;
                        }

                        return syn::Error::new(
                            path.span(),
                            format!(
                                "Invalid bundle attribute. Use `{BUNDLE_ATTRIBUTE_IGNORE_NAME}`"
                            ),
                        )
                        .into_compile_error()
                        .into();
                    }

                    return syn::Error::new(attr.span(), format!("Invalid bundle attribute. Use `#[{BUNDLE_ATTRIBUTE_NAME}({BUNDLE_ATTRIBUTE_IGNORE_NAME})]`")).into_compile_error().into();
                }
            }
        }

        field_kind.push(BundleFieldKind::Component);
    }

    let field = named_fields
        .iter()
        .map(|field| field.ident.as_ref().unwrap())
        .collect::<Vec<_>>();
    let field_type = named_fields
        .iter()
        .map(|field| &field.ty)
        .collect::<Vec<_>>();

    let mut field_component_ids = Vec::new();
    let mut field_get_components = Vec::new();
    let mut field_from_components = Vec::new();
    for ((field_type, field_kind), field) in
        field_type.iter().zip(field_kind.iter()).zip(field.iter())
    {
        match field_kind {
            BundleFieldKind::Component => {
                field_component_ids.push(quote! {
                <#field_type as #bevy_ecs::bundle::Bundle>::component_ids(components, storages, &mut *ids);
                });
                field_get_components.push(quote! {
                    self.#field.get_components(&mut *func);
                });
                field_from_components.push(quote! {
                    #field: <#field_type as #bevy_ecs::bundle::Bundle>::from_components(ctx, &mut *func),
                });
            }

            BundleFieldKind::Ignore => {
                field_from_components.push(quote! {
                    #field: ::std::default::Default::default(),
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
        unsafe impl #impl_generics #bevy_ecs::bundle::Bundle for #struct_name #ty_generics #where_clause {
            fn component_ids(
                components: &mut #bevy_ecs::component::Components,
                storages: &mut #bevy_ecs::storage::Storages,
                ids: &mut impl FnMut(#bevy_ecs::component::ComponentId)
            ){
                #(#field_component_ids)*
            }

            #[allow(unused_variables, non_snake_case)]
            unsafe fn from_components<__T, __F>(ctx: &mut __T, func: &mut __F) -> Self
            where
                __F: FnMut(&mut __T) -> #bevy_ecs::ptr::OwningPtr<'_>
            {
                Self {
                    #(#field_from_components)*
                }
            }

            #[allow(unused_variables)]
            #[inline]
            fn get_components(
                self,
                func: &mut impl FnMut(#bevy_ecs::component::StorageType, #bevy_ecs::ptr::OwningPtr<'_>)
            ) {
                #(#field_get_components)*
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

                fn init_state(world: &mut World, system_meta: &mut SystemMeta) -> Self::State {
                    #(
                        // Pretend to add each param to the system alone, see if it conflicts
                        let mut #meta = system_meta.clone();
                        #meta.component_access_set.clear();
                        #meta.archetype_component_access.clear();
                        #param::init_state(world, &mut #meta);
                        let #param = #param::init_state(world, &mut system_meta.clone());
                    )*
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

                fn new_archetype(state: &mut Self::State, archetype: &Archetype, system_meta: &mut SystemMeta) {
                    <(#(#param,)*) as SystemParam>::new_archetype(state, archetype, system_meta);
                }

                fn apply(state: &mut Self::State, system_meta: &SystemMeta, world: &mut World) {
                    <(#(#param,)*) as SystemParam>::apply(state, system_meta, world);
                }

                #[inline]
                unsafe fn get_param<'w, 's>(
                    state: &'s mut Self::State,
                    system_meta: &SystemMeta,
                    world: &'w World,
                    change_tick: u32,
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

#[derive(PartialEq, Clone)]
enum SystemParamFieldUsage {
    Ignore,
    Infallible,
    Optional,
    Resultful,
}

impl Parse for SystemParamFieldUsage {
    fn parse(input: ParseStream) -> Result<Self> {
        if input.parse::<Option<kw::ignore>>()?.is_some() {
            return Ok(Self::Ignore);
        }

        if input.parse::<Option<kw::infallible>>()?.is_some() {
            return Ok(Self::Infallible);
        }

        if input.parse::<Option<kw::optional>>()?.is_some() {
            return Ok(Self::Optional);
        }

        if input.parse::<Option<kw::resultful>>()?.is_some() {
            return Ok(Self::Resultful);
        }

        Err(input.error("Expected one of 'ignore', 'infallible', 'optional', or 'resultful'"))
    }
}

impl From<SystemParamStructUsage> for SystemParamFieldUsage {
    fn from(u: SystemParamStructUsage) -> Self {
        match u {
            SystemParamStructUsage::Infallible => SystemParamFieldUsage::Infallible,
            SystemParamStructUsage::Optional => SystemParamFieldUsage::Optional,
            SystemParamStructUsage::Resultful(_) => SystemParamFieldUsage::Resultful,
        }
    }
}

#[derive(PartialEq, Clone)]
enum SystemParamStructUsage {
    Infallible,
    Optional,
    Resultful(Ident),
}

impl Parse for SystemParamStructUsage {
    fn parse(input: ParseStream) -> Result<Self> {
        if input.parse::<Option<kw::infallible>>()?.is_some() {
            return Ok(Self::Infallible);
        }

        if input.parse::<Option<kw::optional>>()?.is_some() {
            return Ok(Self::Optional);
        }

        if input.parse::<Option<kw::resultful>>()?.is_some() {
            let content;
            parenthesized!(content in input);
            let err_ty = match content.parse::<Ident>() {
                Ok(o) => o,
                Err(_) => {
                    return Err(
                        input.error("Expected an identifier for `ResultfulSystemParam::Error`.")
                    )
                }
            };
            return Ok(Self::Resultful(err_ty));
        }

        Err(input.error("Expected one of 'infallible', 'optional', or 'resultful(ErrorType)'"))
    }
}

#[derive(Default)]
struct SystemParamAttributes {
    pub usage: Option<SystemParamFieldUsage>,
}

static SYSTEM_PARAM_ATTRIBUTE_NAME: &str = "system_param";

/// Implement `SystemParam` to use a struct as a parameter in a system
#[proc_macro_derive(SystemParam, attributes(system_param))]
pub fn derive_system_param(input: TokenStream) -> TokenStream {
    let bevy_ecs = bevy_ecs_path();
    let ast = parse_macro_input!(input as DeriveInput);
    let syn::Data::Struct(syn::DataStruct { fields, ..}) = ast.data else {
        return syn::Error::new(ast.span(), "Invalid `SystemParam` type: expected a `struct`")
            .into_compile_error()
            .into();
    };

    let mut fallibility = None;
    for attr in &ast.attrs {
        let Some(attr_ident) = attr.path.get_ident() else { continue; };
        if attr_ident == SYSTEM_PARAM_ATTRIBUTE_NAME {
            if fallibility.is_none() {
                let usage = match attr.parse_args_with(SystemParamStructUsage::parse) {
                    Ok(u) => u,
                    Err(e) => return e.into_compile_error().into(),
                };
                fallibility = Some(usage);
            } else {
                return syn::Error::new(
                    attr.span(),
                    "Multiple `system_param` struct attributes found.",
                )
                .into_compile_error()
                .into();
            }
        }
    }

    let fallibility = fallibility.unwrap_or(SystemParamStructUsage::Infallible);

    let associated_error = match fallibility {
        SystemParamStructUsage::Resultful(ref err_ty) => quote! { type Error = #err_ty; },
        _ => quote! {},
    };

    let mut field_idents = Vec::new();
    let mut field_getters = Vec::new();
    let mut field_patterns = Vec::new();
    let mut field_types = Vec::new();
    let mut ignored_fields = Vec::new();
    let mut ignored_field_types = Vec::new();
    for (i, field) in fields.iter().enumerate() {
        let mut field_attrs = SystemParamAttributes::default();
        for attr in &field.attrs {
            let Some(attr_ident) = attr.path.get_ident() else { continue; };
            if attr_ident == SYSTEM_PARAM_ATTRIBUTE_NAME {
                if field_attrs.usage.is_none() {
                    field_attrs.usage =
                        Some(match attr.parse_args_with(SystemParamFieldUsage::parse) {
                            Ok(o) => o,
                            Err(e) => return e.into_compile_error().into(),
                        });
                } else {
                    return syn::Error::new(
                        attr.span(),
                        "Multiple `system_param` field attributes found.",
                    )
                    .into_compile_error()
                    .into();
                }
            }
        }

        match field_attrs
            .usage
            .unwrap_or_else(|| fallibility.clone().into())
        {
            SystemParamFieldUsage::Ignore => {
                ignored_fields.push(match field.ident.as_ref() {
                    Some(s) => s,
                    None => {
                        return syn::Error::new(field.span(), "Field lacks an identifier.")
                            .into_compile_error()
                            .into()
                    }
                });
                ignored_field_types.push(&field.ty);
            }
            field_fallibility => {
                let ident = format_ident!("f{i}");
                let i = Index::from(i);
                let ty = &field.ty;
                field_idents.push(
                    field
                        .ident
                        .as_ref()
                        .map(|f| quote! { #f })
                        .unwrap_or_else(|| quote! { #i }),
                );
                field_getters.push(match fallibility {
					SystemParamStructUsage::Infallible => match field_fallibility {
						SystemParamFieldUsage::Infallible => quote! { #ident },
						SystemParamFieldUsage::Optional => quote! { #ident.expect("Optional system param was not infallible!") },
						SystemParamFieldUsage::Resultful => quote! { #ident.expect("Resultful system param was not infallible!") },
						_ => unreachable!(),
					},
					SystemParamStructUsage::Optional => match field_fallibility {
						SystemParamFieldUsage::Infallible => quote! { #ident },
						SystemParamFieldUsage::Optional => quote! { match #ident { Some(s) => s, None => return None, } },
						SystemParamFieldUsage::Resultful => quote! { match #ident { Ok(o) => o, Err(_) => return None, } },
						_ => unreachable!(),
					},
					SystemParamStructUsage::Resultful(_) => match field_fallibility {
						SystemParamFieldUsage::Infallible => quote! { #ident },
						SystemParamFieldUsage::Optional => quote! { match #ident { Some(s) => s, None => return Err(#bevy_ecs::system::SystemParamError::<#ty>::default().into()), } },
						SystemParamFieldUsage::Resultful => quote! { match #ident { Ok(o) => o, Err(e) => return Err(e.into()), } },
						_ => unreachable!(),
					},
				});
                field_types.push(match field_fallibility {
                    SystemParamFieldUsage::Infallible => quote! { #ty },
                    SystemParamFieldUsage::Optional => quote! { Option<#ty> },
                    SystemParamFieldUsage::Resultful => quote! { Result<#ty, <#ty as #bevy_ecs::system::ResultfulSystemParam>::Error> },
                    _ => unreachable!(),
                });
                field_patterns.push(quote! { #ident });
            }
        }
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

    let mut shadowed_lifetimes: Vec<_> = generics.lifetimes().map(|x| x.lifetime.clone()).collect();
    for lifetime in &mut shadowed_lifetimes {
        let shadowed_ident = format_ident!("_{}", lifetime.ident);
        lifetime.ident = shadowed_ident;
    }

    let mut punctuated_generics = Punctuated::<_, Token![,]>::new();
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

    let mut punctuated_generic_idents = Punctuated::<_, Token![,]>::new();
    punctuated_generic_idents.extend(lifetimeless_generics.iter().map(|g| match g {
        GenericParam::Type(g) => &g.ident,
        GenericParam::Const(g) => &g.ident,
        _ => unreachable!(),
    }));

    // If the number of fields exceeds the 16-parameter limit,
    // fold the fields into tuples of tuples until we are below the limit.
    const LIMIT: usize = 16;
    while field_types.len() > LIMIT {
        let end = Vec::from_iter(field_types.drain(..LIMIT));
        field_types.push(match syn::parse(quote! { (#(#end,)*) }.into()) {
            Ok(o) => o,
            Err(e) => {
                return syn::Error::new(
                    Span::call_site(),
                    format!("Failed to compact field types: {e}"),
                )
                .into_compile_error()
                .into()
            }
        });

        let end = Vec::from_iter(field_patterns.drain(..LIMIT));
        field_patterns.push(match syn::parse(quote! { (#(#end,)*) }.into()) {
            Ok(o) => o,
            Err(e) => {
                return syn::Error::new(
                    Span::call_site(),
                    format!("Failed to compact field patterns: {e}"),
                )
                .into_compile_error()
                .into()
            }
        });
    }

    // Create a where clause for the `ReadOnlySystemParam` impl.
    // Ensure that each field implements `ReadOnlySystemParam`.
    let mut read_only_generics = generics.clone();
    let read_only_where_clause = read_only_generics.make_where_clause();
    for field_type in &field_types {
        read_only_where_clause.predicates.push(
            match syn::parse(quote! { #field_type: #bevy_ecs::system::ReadOnlySystemParam }.into())
            {
                Ok(o) => o,
                Err(e) => {
                    return syn::Error::new(
                        Span::call_site(),
                        format!("Failed to create read-only predicate: {e}"),
                    )
                    .into_compile_error()
                    .into()
                }
            },
        );
    }

    let struct_name = &ast.ident;
    let state_struct_visibility = &ast.vis;

    let get_param_output = quote! {
        #struct_name {
            #(#field_idents: #field_getters,)*
            #(#ignored_fields: ::std::default::Default::default(),)*
        }
    };

    let (system_param, get_param_return, get_param_output) = match fallibility {
        SystemParamStructUsage::Infallible => (
            quote! { #bevy_ecs::system::SystemParam },
            quote! { Self::Item<'w2, 's2> },
            quote! { #get_param_output },
        ),
        SystemParamStructUsage::Optional => (
            quote! { #bevy_ecs::system::OptionalSystemParam },
            quote! { Option<Self::Item<'w2, 's2>> },
            quote! { Some(#get_param_output) },
        ),
        SystemParamStructUsage::Resultful(_) => (
            quote! { #bevy_ecs::system::ResultfulSystemParam },
            quote! { Result<Self::Item<'w2, 's2>, <Self::Item<'w2, 's2> as #bevy_ecs::system::ResultfulSystemParam>::Error> },
            quote! { Ok(#get_param_output) },
        ),
    };

    TokenStream::from(quote! {
        // We define the FetchState struct in an anonymous scope to avoid polluting the user namespace.
        // The struct can still be accessed via SystemParam::State, e.g. EventReaderState can be accessed via
        // <EventReader<'static, 'static, T> as SystemParam>::State
        const _: () = {
            #[doc(hidden)]
            #state_struct_visibility struct FetchState <'w, 's, #(#lifetimeless_generics,)*>
            #where_clause {
                state: (#(<#field_types as #bevy_ecs::system::SystemParam>::State,)*),
                marker: std::marker::PhantomData<(
                    <#bevy_ecs::prelude::Query<'w, 's, ()> as #bevy_ecs::system::SystemParam>::State,
                    #(fn() -> #ignored_field_types,)*
                )>,
            }

            unsafe impl<'w, 's, #punctuated_generics> #system_param for #struct_name #ty_generics #where_clause {
                type State = FetchState<'static, 'static, #punctuated_generic_idents>;
                type Item<'_w, '_s> = #struct_name <#(#shadowed_lifetimes,)* #punctuated_generic_idents>;
                #associated_error

                fn init_state(world: &mut #bevy_ecs::world::World, system_meta: &mut #bevy_ecs::system::SystemMeta) -> Self::State {
                    FetchState {
                        state: <(#(#field_types,)*) as #bevy_ecs::system::SystemParam>::init_state(world, system_meta),
                        marker: std::marker::PhantomData,
                    }
                }

                fn new_archetype(state: &mut Self::State, archetype: &#bevy_ecs::archetype::Archetype, system_meta: &mut #bevy_ecs::system::SystemMeta) {
                    <(#(#field_types,)*) as #bevy_ecs::system::SystemParam>::new_archetype(&mut state.state, archetype, system_meta)
                }

                fn apply(state: &mut Self::State, system_meta: &#bevy_ecs::system::SystemMeta, world: &mut #bevy_ecs::world::World) {
                    <(#(#field_types,)*) as #bevy_ecs::system::SystemParam>::apply(&mut state.state, system_meta, world);
                }

                unsafe fn get_param<'w2, 's2>(
                    state: &'s2 mut Self::State,
                    system_meta: &#bevy_ecs::system::SystemMeta,
                    world: &'w2 #bevy_ecs::world::World,
                    change_tick: u32,
                ) -> #get_param_return {
                    let (#(#field_patterns,)*) = <
                        (#(#field_types,)*) as #bevy_ecs::system::SystemParam
                    >::get_param(&mut state.state, system_meta, world, change_tick);
                    #get_param_output
                }
            }

            // Safety: Each field is `ReadOnlySystemParam`, so this can only read from the `World`
            unsafe impl<'w, 's, #punctuated_generics> #bevy_ecs::system::ReadOnlySystemParam for #struct_name #ty_generics #read_only_where_clause {}
        };
    })
}

/// Implement `WorldQuery` to use a struct as a parameter in a query
#[proc_macro_derive(WorldQuery, attributes(world_query))]
pub fn derive_world_query(input: TokenStream) -> TokenStream {
    let ast = parse_macro_input!(input as DeriveInput);
    derive_world_query_impl(ast)
}

/// Generates an impl of the `SystemLabel` trait.
///
/// This works only for unit structs, or enums with only unit variants.
/// You may force a struct or variant to behave as if it were fieldless with `#[system_label(ignore_fields)]`.
#[proc_macro_derive(SystemLabel, attributes(system_label))]
pub fn derive_system_label(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let mut bevy_ecs = bevy_ecs_path();
    bevy_ecs.segments.push(format_ident!("schedule").into());
    bevy_ecs.segments.push(format_ident!("SystemLabel").into());
    derive_label(input, &bevy_ecs, "system_label")
}

/// Generates an impl of the `StageLabel` trait.
///
/// This works only for unit structs, or enums with only unit variants.
/// You may force a struct or variant to behave as if it were fieldless with `#[stage_label(ignore_fields)]`.
#[proc_macro_derive(StageLabel, attributes(stage_label))]
pub fn derive_stage_label(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let mut bevy_ecs = bevy_ecs_path();
    bevy_ecs.segments.push(format_ident!("schedule").into());
    bevy_ecs.segments.push(format_ident!("StageLabel").into());
    derive_label(input, &bevy_ecs, "stage_label")
}

/// Generates an impl of the `RunCriteriaLabel` trait.
///
/// This works only for unit structs, or enums with only unit variants.
/// You may force a struct or variant to behave as if it were fieldless with `#[run_criteria_label(ignore_fields)]`.
#[proc_macro_derive(RunCriteriaLabel, attributes(run_criteria_label))]
pub fn derive_run_criteria_label(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let mut bevy_ecs = bevy_ecs_path();
    bevy_ecs.segments.push(format_ident!("schedule").into());
    bevy_ecs
        .segments
        .push(format_ident!("RunCriteriaLabel").into());
    derive_label(input, &bevy_ecs, "run_criteria_label")
}

/// Derive macro generating an impl of the trait `ScheduleLabel`.
#[proc_macro_derive(ScheduleLabel)]
pub fn derive_schedule_label(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let mut trait_path = bevy_ecs_path();
    trait_path
        .segments
        .push(format_ident!("schedule_v3").into());
    trait_path
        .segments
        .push(format_ident!("ScheduleLabel").into());
    derive_boxed_label(input, &trait_path)
}

/// Derive macro generating an impl of the trait `SystemSet`.
#[proc_macro_derive(SystemSet)]
pub fn derive_system_set(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let mut trait_path = bevy_ecs_path();
    trait_path
        .segments
        .push(format_ident!("schedule_v3").into());
    trait_path.segments.push(format_ident!("SystemSet").into());
    derive_set(input, &trait_path)
}

pub(crate) fn bevy_ecs_path() -> syn::Path {
    BevyManifest::default().get_path("bevy_ecs")
}

#[proc_macro_derive(Resource)]
pub fn derive_resource(input: TokenStream) -> TokenStream {
    component::derive_resource(input)
}

#[proc_macro_derive(Component, attributes(component))]
pub fn derive_component(input: TokenStream) -> TokenStream {
    component::derive_component(input)
}
