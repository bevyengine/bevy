extern crate proc_macro;

mod component;
mod fetch;

use crate::fetch::derive_world_query_impl;
use bevy_macro_utils::{derive_label, get_named_struct_fields, BevyManifest};
use proc_macro::TokenStream;
use proc_macro2::Span;
use quote::{format_ident, quote};
use syn::{
    parse::{Parse, ParseStream},
    parse_macro_input,
    punctuated::Punctuated,
    token::Comma,
    DeriveInput, Field, GenericParam, Ident, Index, LitInt, Result, Token, TypeParam,
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
        let ident_tuples = &ident_tuples[0..i - input.start];
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

static BUNDLE_ATTRIBUTE_NAME: &str = "bundle";

#[proc_macro_derive(Bundle, attributes(bundle))]
pub fn derive_bundle(input: TokenStream) -> TokenStream {
    let ast = parse_macro_input!(input as DeriveInput);
    let ecs_path = bevy_ecs_path();

    let named_fields = match get_named_struct_fields(&ast.data) {
        Ok(fields) => &fields.named,
        Err(e) => return e.into_compile_error().into(),
    };

    let is_bundle = named_fields
        .iter()
        .map(|field| {
            field
                .attrs
                .iter()
                .any(|a| *a.path.get_ident().as_ref().unwrap() == BUNDLE_ATTRIBUTE_NAME)
        })
        .collect::<Vec<bool>>();
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
    for ((field_type, is_bundle), field) in
        field_type.iter().zip(is_bundle.iter()).zip(field.iter())
    {
        if *is_bundle {
            field_component_ids.push(quote! {
                component_ids.extend(<#field_type as #ecs_path::bundle::Bundle>::component_ids(components, storages));
            });
            field_get_components.push(quote! {
                self.#field.get_components(&mut func);
            });
            field_from_components.push(quote! {
                #field: <#field_type as #ecs_path::bundle::Bundle>::from_components(ctx, &mut func),
            });
        } else {
            field_component_ids.push(quote! {
                component_ids.push(components.init_component::<#field_type>(storages));
            });
            field_get_components.push(quote! {
                #ecs_path::ptr::OwningPtr::make(self.#field, &mut func);
            });
            field_from_components.push(quote! {
                #field: func(ctx).read::<#field_type>(),
            });
        }
    }
    let field_len = field.len();
    let generics = ast.generics;
    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();
    let struct_name = &ast.ident;

    TokenStream::from(quote! {
        /// SAFETY: ComponentId is returned in field-definition-order. [from_components] and [get_components] use field-definition-order
        unsafe impl #impl_generics #ecs_path::bundle::Bundle for #struct_name #ty_generics #where_clause {
            fn component_ids(
                components: &mut #ecs_path::component::Components,
                storages: &mut #ecs_path::storage::Storages,
            ) -> Vec<#ecs_path::component::ComponentId> {
                let mut component_ids = Vec::with_capacity(#field_len);
                #(#field_component_ids)*
                component_ids
            }

            #[allow(unused_variables, unused_mut, non_snake_case)]
            unsafe fn from_components<__T, __F>(ctx: &mut __T, mut func: __F) -> Self
            where
                __F: FnMut(&mut __T) -> #ecs_path::ptr::OwningPtr<'_>
            {
                Self {
                    #(#field_from_components)*
                }
            }

            #[allow(unused_variables, unused_mut, forget_copy, forget_ref)]
            fn get_components(self, mut func: impl FnMut(#ecs_path::ptr::OwningPtr<'_>)) {
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
    let params = get_idents(|i| format!("P{}", i), max_params);
    let params_fetch = get_idents(|i| format!("PF{}", i), max_params);
    let metas = get_idents(|i| format!("m{}", i), max_params);
    let mut param_fn_muts = Vec::new();
    for (i, param) in params.iter().enumerate() {
        let fn_name = Ident::new(&format!("p{}", i), Span::call_site());
        let index = Index::from(i);
        param_fn_muts.push(quote! {
            pub fn #fn_name<'a>(&'a mut self) -> <#param::Fetch as SystemParamFetch<'a, 'a>>::Item {
                // SAFETY: systems run without conflicts with other systems.
                // Conflicting params in ParamSet are not accessible at the same time
                // ParamSets are guaranteed to not conflict with other SystemParams
                unsafe {
                    <#param::Fetch as SystemParamFetch<'a, 'a>>::get_param(&mut self.param_states.#index, &self.system_meta, self.world, self.change_tick)
                }
            }
        });
    }

    for param_count in 1..=max_params {
        let param = &params[0..param_count];
        let param_fetch = &params_fetch[0..param_count];
        let meta = &metas[0..param_count];
        let param_fn_mut = &param_fn_muts[0..param_count];
        tokens.extend(TokenStream::from(quote! {
            impl<'w, 's, #(#param: SystemParam,)*> SystemParam for ParamSet<'w, 's, (#(#param,)*)>
            {
                type Fetch = ParamSetState<(#(#param::Fetch,)*)>;
            }

            // SAFETY: All parameters are constrained to ReadOnlyFetch, so World is only read

            unsafe impl<#(#param_fetch: for<'w1, 's1> SystemParamFetch<'w1, 's1>,)*> ReadOnlySystemParamFetch for ParamSetState<(#(#param_fetch,)*)>
            where #(#param_fetch: ReadOnlySystemParamFetch,)*
            { }

            // SAFETY: Relevant parameter ComponentId and ArchetypeComponentId access is applied to SystemMeta. If any ParamState conflicts
            // with any prior access, a panic will occur.

            unsafe impl<#(#param_fetch: for<'w1, 's1> SystemParamFetch<'w1, 's1>,)*> SystemParamState for ParamSetState<(#(#param_fetch,)*)>
            {
                fn init(world: &mut World, system_meta: &mut SystemMeta) -> Self {
                    #(
                        // Pretend to add each param to the system alone, see if it conflicts
                        let mut #meta = system_meta.clone();
                        #meta.component_access_set.clear();
                        #meta.archetype_component_access.clear();
                        #param_fetch::init(world, &mut #meta);
                        let #param = #param_fetch::init(world, &mut system_meta.clone());
                    )*
                    #(
                        system_meta
                            .component_access_set
                            .extend(#meta.component_access_set);
                        system_meta
                            .archetype_component_access
                            .extend(&#meta.archetype_component_access);
                    )*
                    ParamSetState((#(#param,)*))
                }

                fn new_archetype(&mut self, archetype: &Archetype, system_meta: &mut SystemMeta) {
                    let (#(#param,)*) = &mut self.0;
                    #(
                        #param.new_archetype(archetype, system_meta);
                    )*
                }

                fn apply(&mut self, world: &mut World) {
                    self.0.apply(world)
                }
            }



            impl<'w, 's, #(#param_fetch: for<'w1, 's1> SystemParamFetch<'w1, 's1>,)*> SystemParamFetch<'w, 's> for ParamSetState<(#(#param_fetch,)*)>
            {
                type Item = ParamSet<'w, 's, (#(<#param_fetch as SystemParamFetch<'w, 's>>::Item,)*)>;

                #[inline]
                unsafe fn get_param(
                    state: &'s mut Self,
                    system_meta: &SystemMeta,
                    world: &'w World,
                    change_tick: u32,
                ) -> Self::Item {
                    ParamSet {
                        param_states: &mut state.0,
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

#[derive(Default)]
struct SystemParamFieldAttributes {
    pub ignore: bool,
}

static SYSTEM_PARAM_ATTRIBUTE_NAME: &str = "system_param";

/// Implement `SystemParam` to use a struct as a parameter in a system
#[proc_macro_derive(SystemParam, attributes(system_param))]
pub fn derive_system_param(input: TokenStream) -> TokenStream {
    let ast = parse_macro_input!(input as DeriveInput);
    let fields = match get_named_struct_fields(&ast.data) {
        Ok(fields) => &fields.named,
        Err(e) => return e.into_compile_error().into(),
    };
    let path = bevy_ecs_path();

    let field_attributes = fields
        .iter()
        .map(|field| {
            (
                field,
                field
                    .attrs
                    .iter()
                    .find(|a| *a.path.get_ident().as_ref().unwrap() == SYSTEM_PARAM_ATTRIBUTE_NAME)
                    .map_or_else(SystemParamFieldAttributes::default, |a| {
                        syn::custom_keyword!(ignore);
                        let mut attributes = SystemParamFieldAttributes::default();
                        a.parse_args_with(|input: ParseStream| {
                            if input.parse::<Option<ignore>>()?.is_some() {
                                attributes.ignore = true;
                            }
                            Ok(())
                        })
                        .expect("Invalid 'system_param' attribute format.");

                        attributes
                    }),
            )
        })
        .collect::<Vec<(&Field, SystemParamFieldAttributes)>>();
    let mut fields = Vec::new();
    let mut field_indices = Vec::new();
    let mut field_types = Vec::new();
    let mut ignored_fields = Vec::new();
    let mut ignored_field_types = Vec::new();
    for (i, (field, attrs)) in field_attributes.iter().enumerate() {
        if attrs.ignore {
            ignored_fields.push(field.ident.as_ref().unwrap());
            ignored_field_types.push(&field.ty);
        } else {
            fields.push(field.ident.as_ref().unwrap());
            field_types.push(&field.ty);
            field_indices.push(Index::from(i));
        }
    }

    let generics = ast.generics;
    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();

    let lifetimeless_generics: Vec<_> = generics
        .params
        .iter()
        .filter(|g| matches!(g, GenericParam::Type(_)))
        .collect();

    let mut punctuated_generics = Punctuated::<_, Token![,]>::new();
    punctuated_generics.extend(lifetimeless_generics.iter().map(|g| match g {
        GenericParam::Type(g) => GenericParam::Type(TypeParam {
            default: None,
            ..g.clone()
        }),
        _ => unreachable!(),
    }));

    let mut punctuated_generic_idents = Punctuated::<_, Token![,]>::new();
    punctuated_generic_idents.extend(lifetimeless_generics.iter().map(|g| match g {
        GenericParam::Type(g) => &g.ident,
        _ => unreachable!(),
    }));

    let struct_name = &ast.ident;
    let fetch_struct_visibility = &ast.vis;

    TokenStream::from(quote! {
        // We define the FetchState struct in an anonymous scope to avoid polluting the user namespace.
        // The struct can still be accessed via SystemParam::Fetch, e.g. EventReaderState can be accessed via
        // <EventReader<'static, 'static, T> as SystemParam>::Fetch
        const _: () = {
            impl #impl_generics #path::system::SystemParam for #struct_name #ty_generics #where_clause {
                type Fetch = FetchState <(#(<#field_types as #path::system::SystemParam>::Fetch,)*), #punctuated_generic_idents>;
            }

            #[doc(hidden)]
            #fetch_struct_visibility struct FetchState <TSystemParamState, #punctuated_generic_idents> {
                state: TSystemParamState,
                marker: std::marker::PhantomData<fn()->(#punctuated_generic_idents)>
            }

            unsafe impl<TSystemParamState: #path::system::SystemParamState, #punctuated_generics> #path::system::SystemParamState for FetchState <TSystemParamState, #punctuated_generic_idents> #where_clause {
                fn init(world: &mut #path::world::World, system_meta: &mut #path::system::SystemMeta) -> Self {
                    Self {
                        state: TSystemParamState::init(world, system_meta),
                        marker: std::marker::PhantomData,
                    }
                }

                fn new_archetype(&mut self, archetype: &#path::archetype::Archetype, system_meta: &mut #path::system::SystemMeta) {
                    self.state.new_archetype(archetype, system_meta)
                }

                fn apply(&mut self, world: &mut #path::world::World) {
                    self.state.apply(world)
                }
            }

            impl #impl_generics #path::system::SystemParamFetch<'w, 's> for FetchState <(#(<#field_types as #path::system::SystemParam>::Fetch,)*), #punctuated_generic_idents> #where_clause {
                type Item = #struct_name #ty_generics;
                unsafe fn get_param(
                    state: &'s mut Self,
                    system_meta: &#path::system::SystemMeta,
                    world: &'w #path::world::World,
                    change_tick: u32,
                ) -> Self::Item {
                    #struct_name {
                        #(#fields: <<#field_types as #path::system::SystemParam>::Fetch as #path::system::SystemParamFetch>::get_param(&mut state.state.#field_indices, system_meta, world, change_tick),)*
                        #(#ignored_fields: <#ignored_field_types>::default(),)*
                    }
                }
            }

            // Safety: The `ParamState` is `ReadOnlySystemParamFetch`, so this can only read from the `World`
            unsafe impl<TSystemParamState: #path::system::SystemParamState + #path::system::ReadOnlySystemParamFetch, #punctuated_generics> #path::system::ReadOnlySystemParamFetch for FetchState <TSystemParamState, #punctuated_generic_idents> #where_clause {}
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
    let mut trait_path = bevy_ecs_path();
    trait_path.segments.push(format_ident!("schedule").into());
    trait_path
        .segments
        .push(format_ident!("SystemLabel").into());
    derive_label(input, &trait_path, "system_label")
}

/// Generates an impl of the `StageLabel` trait.
///
/// This works only for unit structs, or enums with only unit variants.
/// You may force a struct or variant to behave as if it were fieldless with `#[stage_label(ignore_fields)]`.
#[proc_macro_derive(StageLabel, attributes(stage_label))]
pub fn derive_stage_label(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let mut trait_path = bevy_ecs_path();
    trait_path.segments.push(format_ident!("schedule").into());
    trait_path.segments.push(format_ident!("StageLabel").into());
    derive_label(input, &trait_path, "stage_label")
}

/// Generates an impl of the `AmbiguitySetLabel` trait.
///
/// This works only for unit structs, or enums with only unit variants.
/// You may force a struct or variant to behave as if it were fieldless with `#[ambiguity_set_label(ignore_fields)]`.
#[proc_macro_derive(AmbiguitySetLabel, attributes(ambiguity_set_label))]
pub fn derive_ambiguity_set_label(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let mut trait_path = bevy_ecs_path();
    trait_path.segments.push(format_ident!("schedule").into());
    trait_path
        .segments
        .push(format_ident!("AmbiguitySetLabel").into());
    derive_label(input, &trait_path, "ambiguity_set_label")
}

/// Generates an impl of the `RunCriteriaLabel` trait.
///
/// This works only for unit structs, or enums with only unit variants.
/// You may force a struct or variant to behave as if it were fieldless with `#[run_criteria_label(ignore_fields)]`.
#[proc_macro_derive(RunCriteriaLabel, attributes(run_criteria_label))]
pub fn derive_run_criteria_label(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let mut trait_path = bevy_ecs_path();
    trait_path.segments.push(format_ident!("schedule").into());
    trait_path
        .segments
        .push(format_ident!("RunCriteriaLabel").into());
    derive_label(input, &trait_path, "run_criteria_label")
}

pub(crate) fn bevy_ecs_path() -> syn::Path {
    BevyManifest::default().get_path("bevy_ecs")
}

#[proc_macro_derive(Component, attributes(component))]
pub fn derive_component(input: TokenStream) -> TokenStream {
    component::derive_component(input)
}
