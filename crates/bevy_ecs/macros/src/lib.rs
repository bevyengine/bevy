extern crate proc_macro;

use bevy_macro_utils::BevyManifest;
use proc_macro::TokenStream;
use proc_macro2::{Span, TokenStream as TokenStream2};
use quote::{format_ident, quote};
use syn::{
    parse::{Parse, ParseStream},
    parse_macro_input,
    punctuated::Punctuated,
    token::Comma,
    Data, DataStruct, DeriveInput, Error, Field, Fields, GenericParam, Ident, Index, Lifetime,
    LitInt, Member, Result, Token,
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
    let ident_tuples: Vec<_> = (input.start..=input.end)
        .map(|i| {
            let idents = input
                .idents
                .iter()
                .map(|ident| format_ident!("{}{}", ident, i));
            if input.idents.len() < 2 {
                quote! { #(#idents)* }
            } else {
                quote! { (#(#idents),*) }
            }
        })
        .collect();

    let macro_ident = &input.macro_ident;
    let invocations = (input.start..=input.end).map(|i| {
        let ident_tuples = &ident_tuples[0..i];
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

/// Derives the Bundle trait for a struct.
/// The #[bundle] attribute may be used on a field of the struct to flatten the field's fields into this bundle.
///
/// ```ignore
/// #[derive(Bundle)]
/// struct A {
///     x: i32,
///     y: u64,
/// }
///
/// #[derive(Bundle)]
/// struct B {
///     #[bundle]
///     a: A,
///     z: String,
/// }
/// ```
#[proc_macro_derive(Bundle, attributes(bundle))]
pub fn derive_bundle(input: TokenStream) -> TokenStream {
    derive_bundle_impl(parse_macro_input!(input as DeriveInput))
        .unwrap_or_else(|err| err.to_compile_error())
        .into()
}

fn derive_bundle_impl(input: DeriveInput) -> Result<TokenStream2> {
    let ecs_path = bevy_ecs_path();

    let (num_fields, fields) = match input.data {
        Data::Struct(DataStruct { fields, .. }) if !fields.is_empty() => (fields.len(), fields),
        _ => {
            return Err(Error::new_spanned(
                input,
                "`Bundle` can only be derived on a struct with at least one field",
            ));
        }
    };

    let fields = fields.into_iter().enumerate().map(|(idx, field)| {
        let is_bundle = field
            .attrs
            .iter()
            .flat_map(|attr| attr.path.get_ident())
            .any(|ident| ident == BUNDLE_ATTRIBUTE_NAME);
        let ident = field.ident.map_or_else(
            || {
                Member::Unnamed(syn::Index {
                    index: idx as u32,
                    span: Span::call_site(),
                })
            },
            Member::Named,
        );
        (is_bundle, ident, field.ty)
    });

    let mut field_component_ids = Vec::new();
    let mut field_get_components = Vec::new();
    let mut field_from_components = Vec::new();
    for (is_bundle, field, field_type) in fields {
        if is_bundle {
            field_component_ids.push(quote! {
                component_ids.extend(<#field_type as #ecs_path::bundle::Bundle>::component_ids(components));
            });
            field_get_components.push(quote! {
                self.#field.get_components(&mut func);
            });
            field_from_components.push(quote! {
                #field: <#field_type as #ecs_path::bundle::Bundle>::from_components(&mut func),
            });
        } else {
            field_component_ids.push(quote! {
                component_ids.push(components.get_or_insert_id::<#field_type>());
            });
            field_get_components.push(quote! {
                func((&mut self.#field as *mut #field_type).cast::<u8>());
                ::std::mem::forget(self.#field);
            });
            field_from_components.push(quote! {
                #field: func().cast::<#field_type>().read(),
            });
        }
    }

    let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();
    let ident = input.ident;

    Ok(quote! {
        /// SAFE: TypeInfo is returned in field-definition-order. [from_components] and [get_components] use field-definition-order
        unsafe impl #impl_generics #ecs_path::bundle::Bundle for #ident#ty_generics #where_clause {
            fn component_ids(
                components: &mut #ecs_path::component::Components,
            ) -> ::std::vec::Vec<#ecs_path::component::ComponentId> {
                let mut component_ids = ::std::vec::Vec::with_capacity(#num_fields);
                #(#field_component_ids)*
                component_ids
            }

            #[allow(unused_variables, unused_mut, non_snake_case)]
            unsafe fn from_components(mut func: impl FnMut() -> *mut u8) -> Self {
                Self {
                    #(#field_from_components)*
                }
            }

            #[allow(unused_variables, unused_mut, forget_copy, forget_ref)]
            fn get_components(mut self, mut func: impl FnMut(*mut u8)) {
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

fn get_lifetimes(fmt_string: fn(usize) -> String, count: usize) -> Vec<Lifetime> {
    (0..count)
        .map(|i| Lifetime::new(&fmt_string(i), Span::call_site()))
        .collect::<Vec<Lifetime>>()
}

#[proc_macro]
pub fn impl_query_set(_input: TokenStream) -> TokenStream {
    let mut tokens = TokenStream::new();
    let max_queries = 4;
    let queries = get_idents(|i| format!("Q{}", i), max_queries);
    let filters = get_idents(|i| format!("F{}", i), max_queries);
    let lifetimes = get_lifetimes(|i| format!("'q{}", i), max_queries);
    let mut query_fns = Vec::new();
    let mut query_fn_muts = Vec::new();
    for i in 0..max_queries {
        let query = &queries[i];
        let filter = &filters[i];
        let lifetime = &lifetimes[i];
        let fn_name = Ident::new(&format!("q{}", i), Span::call_site());
        let fn_name_mut = Ident::new(&format!("q{}_mut", i), Span::call_site());
        let index = Index::from(i);
        query_fns.push(quote! {
            pub fn #fn_name(&self) -> &Query<#lifetime, #query, #filter> {
                &self.0.#index
            }
        });
        query_fn_muts.push(quote! {
            pub fn #fn_name_mut(&mut self) -> &mut Query<#lifetime, #query, #filter> {
                &mut self.0.#index
            }
        });
    }

    for query_count in 1..=max_queries {
        let query = &queries[0..query_count];
        let filter = &filters[0..query_count];
        let lifetime = &lifetimes[0..query_count];
        let query_fn = &query_fns[0..query_count];
        let query_fn_mut = &query_fn_muts[0..query_count];
        tokens.extend(TokenStream::from(quote! {
            impl<#(#lifetime,)*  #(#query: WorldQuery + 'static,)* #(#filter: WorldQuery + 'static,)*> SystemParam for QuerySet<(#(Query<#lifetime, #query, #filter>,)*)>
                where #(#filter::Fetch: FilterFetch,)*
            {
                type Fetch = QuerySetState<(#(QueryState<#query, #filter>,)*)>;
            }

            // SAFE: All Queries are constrained to ReadOnlyFetch, so World is only read
            unsafe impl<#(#query: WorldQuery + 'static,)* #(#filter: WorldQuery + 'static,)*> ReadOnlySystemParamFetch for QuerySetState<(#(QueryState<#query, #filter>,)*)>
            where #(#query::Fetch: ReadOnlyFetch,)* #(#filter::Fetch: FilterFetch,)*
            { }

            // SAFE: Relevant query ComponentId and ArchetypeComponentId access is applied to SystemMeta. If any QueryState conflicts
            // with any prior access, a panic will occur.
            unsafe impl<#(#query: WorldQuery + 'static,)* #(#filter: WorldQuery + 'static,)*> SystemParamState for QuerySetState<(#(QueryState<#query, #filter>,)*)>
                where #(#filter::Fetch: FilterFetch,)*
            {
                type Config = ();
                fn init(world: &mut World, system_meta: &mut SystemMeta, config: Self::Config) -> Self {
                    #(
                        let mut #query = QueryState::<#query, #filter>::new(world);
                        assert_component_access_compatibility(
                            &system_meta.name,
                            std::any::type_name::<#query>(),
                            std::any::type_name::<#filter>(),
                            &system_meta.component_access_set,
                            &#query.component_access,
                            world,
                        );
                    )*
                    #(
                        system_meta
                            .component_access_set
                            .add(#query.component_access.clone());
                        system_meta
                            .archetype_component_access
                            .extend(&#query.archetype_component_access);
                    )*
                    QuerySetState((#(#query,)*))
                }

                fn new_archetype(&mut self, archetype: &Archetype, system_meta: &mut SystemMeta) {
                    let (#(#query,)*) = &mut self.0;
                    #(
                        #query.new_archetype(archetype);
                        system_meta
                            .archetype_component_access
                            .extend(&#query.archetype_component_access);
                    )*
                }

                fn default_config() {}
            }

            impl<'a, #(#query: WorldQuery + 'static,)* #(#filter: WorldQuery + 'static,)*> SystemParamFetch<'a> for QuerySetState<(#(QueryState<#query, #filter>,)*)>
                where #(#filter::Fetch: FilterFetch,)*
            {
                type Item = QuerySet<(#(Query<'a, #query, #filter>,)*)>;

                #[inline]
                unsafe fn get_param(
                    state: &'a mut Self,
                    system_meta: &SystemMeta,
                    world: &'a World,
                    change_tick: u32,
                ) -> Self::Item {
                    let (#(#query,)*) = &state.0;
                    QuerySet((#(Query::new(world, #query, system_meta.last_change_tick, change_tick),)*))
                }
            }

            impl<#(#lifetime,)* #(#query: WorldQuery,)* #(#filter: WorldQuery,)*> QuerySet<(#(Query<#lifetime, #query, #filter>,)*)>
                where #(#filter::Fetch: FilterFetch,)*
            {
                #(#query_fn)*
                #(#query_fn_mut)*
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
    let fields = match &ast.data {
        Data::Struct(DataStruct {
            fields: Fields::Named(fields),
            ..
        }) => &fields.named,
        _ => panic!("Expected a struct with named fields."),
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
                    .find(|attr| {
                        attr.path
                            .get_ident()
                            .map_or(false, |ident| ident == SYSTEM_PARAM_ATTRIBUTE_NAME)
                    })
                    .map_or_else(SystemParamFieldAttributes::default, |a| {
                        syn::custom_keyword!(ignore);
                        let mut attributes = SystemParamFieldAttributes::default();
                        a.parse_args_with(|input: ParseStream| {
                            attributes.ignore |= input.parse::<Option<ignore>>()?.is_some();
                            Ok(())
                        })
                        .expect("Invalid 'render_resources' attribute format.");
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
        let ident = field.ident.as_ref().unwrap();
        if attrs.ignore {
            ignored_fields.push(ident);
            ignored_field_types.push(&field.ty);
        } else {
            fields.push(ident);
            field_types.push(&field.ty);
            field_indices.push(Index::from(i));
        }
    }

    let generics = ast.generics;
    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();

    let (punctuated_generics, punctuated_generic_idents): (
        Punctuated<_, Token![,]>,
        Punctuated<_, Token![,]>,
    ) = generics
        .params
        .iter()
        .filter_map(|g| match g {
            GenericParam::Type(tp) => Some((g, &tp.ident)),
            _ => None,
        })
        .unzip();

    let struct_name = &ast.ident;
    let fetch_struct_name = Ident::new(&format!("{}State", struct_name), Span::call_site());
    let fetch_struct_visibility = &ast.vis;

    TokenStream::from(quote! {
        impl #impl_generics #path::system::SystemParam for #struct_name#ty_generics #where_clause {
            type Fetch = #fetch_struct_name <(#(<#field_types as #path::system::SystemParam>::Fetch,)*), #punctuated_generic_idents>;
        }

        #[doc(hidden)]
        #fetch_struct_visibility struct #fetch_struct_name<TSystemParamState, #punctuated_generic_idents> {
            state: TSystemParamState,
            marker: ::std::marker::PhantomData<(#punctuated_generic_idents)>
        }

        unsafe impl<TSystemParamState: #path::system::SystemParamState, #punctuated_generics> #path::system::SystemParamState for #fetch_struct_name<TSystemParamState, #punctuated_generic_idents> {
            type Config = TSystemParamState::Config;
            fn init(world: &mut #path::world::World, system_meta: &mut #path::system::SystemMeta, config: Self::Config) -> Self {
                Self {
                    state: TSystemParamState::init(world, system_meta, config),
                    marker: ::std::marker::PhantomData,
                }
            }

            fn new_archetype(&mut self, archetype: &#path::archetype::Archetype, system_meta: &mut #path::system::SystemMeta) {
                self.state.new_archetype(archetype, system_meta)
            }

            fn default_config() -> TSystemParamState::Config {
                TSystemParamState::default_config()
            }

            fn apply(&mut self, world: &mut #path::world::World) {
                self.state.apply(world)
            }
        }

        impl #impl_generics #path::system::SystemParamFetch<'a> for #fetch_struct_name <(#(<#field_types as #path::system::SystemParam>::Fetch,)*), #punctuated_generic_idents> {
            type Item = #struct_name#ty_generics;
            unsafe fn get_param(
                state: &'a mut Self,
                system_meta: &#path::system::SystemMeta,
                world: &'a #path::world::World,
                change_tick: u32,
            ) -> Self::Item {
                #struct_name {
                    #(#fields: <<#field_types as #path::system::SystemParam>::Fetch as #path::system::SystemParamFetch>::get_param(&mut state.state.#field_indices, system_meta, world, change_tick),)*
                    #(#ignored_fields: <#ignored_field_types>::default(),)*
                }
            }
        }
    })
}

#[proc_macro_derive(SystemLabel)]
pub fn derive_system_label(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    derive_label(input, "SystemLabel").into()
}

#[proc_macro_derive(StageLabel)]
pub fn derive_stage_label(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    derive_label(input, "StageLabel").into()
}

#[proc_macro_derive(AmbiguitySetLabel)]
pub fn derive_ambiguity_set_label(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    derive_label(input, "AmbiguitySetLabel").into()
}

#[proc_macro_derive(RunCriteriaLabel)]
pub fn derive_run_criteria_label(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    derive_label(input, "RunCriteriaLabel").into()
}

fn derive_label(input: DeriveInput, label_type: &str) -> TokenStream2 {
    let label_type = Ident::new(label_type, Span::call_site());
    let ident = input.ident;
    let ecs_path = bevy_ecs_path();

    let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();
    let mut where_clause = where_clause.cloned().unwrap_or_else(|| syn::WhereClause {
        where_token: Default::default(),
        predicates: Default::default(),
    });
    let self_pred = syn::parse2(
        quote! { Self: Eq + ::std::fmt::Debug + ::std::hash::Hash + Clone + Send + Sync + 'static },
    )
    .unwrap();
    where_clause.predicates.push(self_pred);

    quote! {
        impl #impl_generics #ecs_path::schedule::#label_type for #ident #ty_generics #where_clause {
            fn dyn_clone(&self) -> Box<dyn #ecs_path::schedule::#label_type> {
                Box::new(Clone::clone(self))
            }
        }
    }
}

fn bevy_ecs_path() -> syn::Path {
    BevyManifest::default().get_path("bevy_ecs")
}
