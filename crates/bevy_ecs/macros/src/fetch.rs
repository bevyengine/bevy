use proc_macro::TokenStream;
use proc_macro2::{Ident, Span};
use quote::{quote, ToTokens};
use syn::{
    parse::{Parse, ParseStream},
    punctuated::Punctuated,
    Attribute, Data, DataStruct, DeriveInput, Field, Fields, GenericArgument, GenericParam,
    Lifetime, LifetimeDef, Path, PathArguments, ReturnType, Token, Type, TypePath,
};

use crate::bevy_ecs_path;

#[derive(Default)]
struct FetchStructAttributes {
    pub is_filter: bool,
    pub is_mutable: bool,
    pub derive_args: Punctuated<syn::NestedMeta, syn::token::Comma>,
}

static FILTER_ATTRIBUTE_NAME: &str = "filter";
static MUTABLE_ATTRIBUTE_NAME: &str = "mutable";
static DERIVE_ATTRIBUTE_NAME: &str = "derive";

mod field_attr_keywords {
    syn::custom_keyword!(ignore);
}

pub static WORLD_QUERY_ATTRIBUTE_NAME: &str = "world_query";

pub fn derive_world_query_impl(ast: DeriveInput) -> TokenStream {
    let visibility = ast.vis;

    let mut fetch_struct_attributes = FetchStructAttributes::default();
    for attr in &ast.attrs {
        if !attr
            .path
            .get_ident()
            .map_or(false, |ident| ident == WORLD_QUERY_ATTRIBUTE_NAME)
        {
            continue;
        }

        attr.parse_args_with(|input: ParseStream| {
            let meta = input.parse_terminated::<syn::Meta, syn::token::Comma>(syn::Meta::parse)?;
            for meta in meta {
                let ident = meta.path().get_ident().unwrap_or_else(|| {
                    panic!(
                        "Unrecognized attribute: `{}`",
                        meta.path().to_token_stream()
                    )
                });
                if ident == MUTABLE_ATTRIBUTE_NAME {
                    if let syn::Meta::Path(_) = meta {
                        fetch_struct_attributes.is_mutable = true;
                    } else {
                        panic!(
                            "The `{}` attribute is expected to have no value or arguments",
                            MUTABLE_ATTRIBUTE_NAME
                        );
                    }
                } else if ident == DERIVE_ATTRIBUTE_NAME {
                    if let syn::Meta::List(meta_list) = meta {
                        fetch_struct_attributes
                            .derive_args
                            .extend(meta_list.nested.iter().cloned());
                    } else {
                        panic!(
                            "Expected a structured list within the `{}` attribute",
                            DERIVE_ATTRIBUTE_NAME
                        );
                    }
                } else if ident == FILTER_ATTRIBUTE_NAME {
                    if let syn::Meta::Path(_) = meta {
                        fetch_struct_attributes.is_filter = true;
                    } else {
                        panic!(
                            "The `{}` attribute is expected to have no value or arguments",
                            FILTER_ATTRIBUTE_NAME
                        );
                    }
                } else {
                    panic!(
                        "Unrecognized attribute: `{}`",
                        meta.path().to_token_stream()
                    );
                }
            }
            Ok(())
        })
        .unwrap_or_else(|_| panic!("Invalid `{}` attribute format", WORLD_QUERY_ATTRIBUTE_NAME));
    }

    if fetch_struct_attributes.is_filter && fetch_struct_attributes.is_mutable {
        panic!(
            "The `{}` attribute is not expected to be used in conjunction with the `{}` attribute",
            FILTER_ATTRIBUTE_NAME, MUTABLE_ATTRIBUTE_NAME
        );
    }

    let world_lifetime = ast.generics.params.first().and_then(|param| match param {
        lt @ GenericParam::Lifetime(_) => Some(lt.clone()),
        _ => None,
    });
    // Fetch's HRTBs require substituting world lifetime with an additional one to make the
    // implementation compile. I don't fully understand why this works though. If anyone's curious
    // enough to try to find a better work around, I'll leave playground links here:
    // - https://play.rust-lang.org/?version=stable&mode=debug&edition=2018&gist=da5e260a5c2f3e774142d60a199e854a (this fails)
    // - https://play.rust-lang.org/?version=stable&mode=debug&edition=2018&gist=802517bb3d8f83c45ee8c0be360bb250 (this compiles)
    let fetch_lifetime_param =
        GenericParam::Lifetime(LifetimeDef::new(Lifetime::new("'fetch", Span::call_site())));

    let has_world_lifetime = world_lifetime.is_some();
    let world_lifetime_param = world_lifetime.unwrap_or_else(|| {
        GenericParam::Lifetime(LifetimeDef::new(Lifetime::new("'world", Span::call_site())))
    });
    let state_lifetime_param =
        GenericParam::Lifetime(LifetimeDef::new(Lifetime::new("'state", Span::call_site())));

    let mut fetch_trait_punctuated_lifetimes = Punctuated::<_, Token![,]>::new();
    fetch_trait_punctuated_lifetimes.push(world_lifetime_param.clone());
    fetch_trait_punctuated_lifetimes.push(state_lifetime_param.clone());

    let (impl_generics, ty_generics, where_clause) = ast.generics.split_for_impl();

    let struct_name = ast.ident.clone();
    let item_struct_name = Ident::new(&format!("{}Item", struct_name), Span::call_site());
    let read_only_item_struct_name = if fetch_struct_attributes.is_mutable {
        Ident::new(&format!("{}ReadOnlyItem", struct_name), Span::call_site())
    } else {
        item_struct_name.clone()
    };
    let fetch_struct_name = Ident::new(&format!("{}Fetch", struct_name), Span::call_site());
    let state_struct_name = Ident::new(&format!("{}State", struct_name), Span::call_site());
    let read_only_fetch_struct_name = if fetch_struct_attributes.is_mutable {
        Ident::new(&format!("{}ReadOnlyFetch", struct_name), Span::call_site())
    } else {
        fetch_struct_name.clone()
    };
    let fetch_associated_type = Ident::new("Fetch", Span::call_site());
    let read_only_fetch_associated_type = Ident::new("ReadOnlyFetch", Span::call_site());

    let fields = match &ast.data {
        Data::Struct(DataStruct {
            fields: Fields::Named(fields),
            ..
        }) => &fields.named,
        _ => panic!("Expected a struct with named fields"),
    };

    let mut ignored_field_attrs = Vec::new();
    let mut ignored_field_visibilities = Vec::new();
    let mut ignored_field_idents = Vec::new();
    let mut ignored_field_types = Vec::new();
    let mut field_attrs = Vec::new();
    let mut field_visibilities = Vec::new();
    let mut field_idents = Vec::new();
    let mut field_types = Vec::new();
    let mut fetch_init_types = Vec::new();

    let (world_lifetime, fetch_lifetime) = match (&world_lifetime_param, &fetch_lifetime_param) {
        (GenericParam::Lifetime(world), GenericParam::Lifetime(fetch)) => {
            (&world.lifetime, &fetch.lifetime)
        }
        _ => unreachable!(),
    };

    for field in fields.iter() {
        let WorldQueryFieldInfo {
            field_type,
            fetch_init_type: init_type,
            is_ignored,
            attrs,
        } = read_world_query_field_info(field, world_lifetime, fetch_lifetime);

        let field_ident = field.ident.as_ref().unwrap().clone();
        if is_ignored {
            ignored_field_attrs.push(attrs);
            ignored_field_visibilities.push(field.vis.clone());
            ignored_field_idents.push(field_ident.clone());
            ignored_field_types.push(field.ty.clone());
        } else {
            field_attrs.push(attrs);
            field_visibilities.push(field.vis.clone());
            field_idents.push(field_ident.clone());
            field_types.push(field_type);
            fetch_init_types.push(init_type);
        }
    }

    // We expect that only regular query declarations have a lifetime.
    if fetch_struct_attributes.is_filter {
        if has_world_lifetime {
            panic!("Expected a struct without a lifetime");
        }
    } else if !has_world_lifetime {
        panic!("Expected a struct with a lifetime");
    }

    let derive_macro_call = if fetch_struct_attributes.derive_args.is_empty() {
        quote! {}
    } else {
        let derive_args = &fetch_struct_attributes.derive_args;
        quote! { #[derive(#derive_args)] }
    };

    // Add `'state` and `'fetch` lifetimes that will be used in `Fetch` implementation.
    let mut fetch_generics = ast.generics.clone();
    if !has_world_lifetime {
        fetch_generics
            .params
            .insert(0, world_lifetime_param.clone());
    }
    fetch_generics.params.insert(1, state_lifetime_param);
    fetch_generics
        .params
        .insert(2, fetch_lifetime_param.clone());
    let (fetch_impl_generics, _, _) = fetch_generics.split_for_impl();

    // Replace lifetime `'world` with `'fetch`. See `replace_lifetime_for_type` for more details.
    let mut fetch_generics = ast.generics.clone();
    *fetch_generics.params.first_mut().unwrap() = fetch_lifetime_param;

    let fetch_ty_generics = if fetch_struct_attributes.is_filter {
        ty_generics.clone()
    } else {
        let (_, fetch_ty_generics, _) = fetch_generics.split_for_impl();
        fetch_ty_generics
    };

    let path = bevy_ecs_path();

    let impl_fetch = |is_filter: bool,
                      fetch_associated_type: Ident,
                      fetch_struct_name: Ident,
                      item_struct_name: Ident| {
        if is_filter {
            quote! {
                #[doc(hidden)]
                #visibility struct #fetch_struct_name #impl_generics #where_clause {
                    #(#field_idents: <#field_types as #path::query::WorldQuery>::#fetch_associated_type,)*
                    #(#ignored_field_idents: #ignored_field_types,)*
                }

                impl #fetch_impl_generics #path::query::Fetch<#fetch_trait_punctuated_lifetimes> for #fetch_struct_name #ty_generics #where_clause {
                    type Item = bool;
                    type State = #state_struct_name #ty_generics;

                    unsafe fn init(_world: &#path::world::World, state: &Self::State, _last_change_tick: u32, _change_tick: u32) -> Self {
                        #fetch_struct_name {
                            #(#field_idents: <#field_types as #path::query::WorldQuery>::ReadOnlyFetch::init(_world, &state.#field_idents, _last_change_tick, _change_tick),)*
                            #(#ignored_field_idents: Default::default(),)*
                        }
                    }

                    const IS_DENSE: bool = true #(&& <#field_types as #path::query::WorldQuery>::ReadOnlyFetch::IS_DENSE)*;

                    #[inline]
                    unsafe fn set_archetype(&mut self, _state: &Self::State, _archetype: &#path::archetype::Archetype, _tables: &#path::storage::Tables) {
                        #(self.#field_idents.set_archetype(&_state.#field_idents, _archetype, _tables);)*
                    }

                    #[inline]
                    unsafe fn set_table(&mut self, _state: &Self::State, _table: &#path::storage::Table) {
                        #(self.#field_idents.set_table(&_state.#field_idents, _table);)*
                    }

                    #[inline]
                    unsafe fn table_fetch(&mut self, _table_row: usize) -> Self::Item {
                        use #path::query::FilterFetch;
                        true #(&& self.#field_idents.table_filter_fetch(_table_row))*
                    }

                    #[inline]
                    unsafe fn archetype_fetch(&mut self, _archetype_index: usize) -> Self::Item {
                        use #path::query::FilterFetch;
                        true #(&& self.#field_idents.archetype_filter_fetch(_archetype_index))*
                    }
                }
            }
        } else {
            quote! {
                #derive_macro_call
                #visibility struct #item_struct_name #impl_generics #where_clause {
                    #(#(#field_attrs)* #field_visibilities #field_idents: <<#field_types as #path::query::WorldQuery>::#fetch_associated_type as #path::query::Fetch<#world_lifetime, #world_lifetime>>::Item,)*
                    #(#(#ignored_field_attrs)* #ignored_field_visibilities #ignored_field_idents: #ignored_field_types,)*
                }

                #[doc(hidden)]
                #visibility struct #fetch_struct_name #impl_generics #where_clause {
                    #(#field_idents: <#field_types as #path::query::WorldQuery>::#fetch_associated_type,)*
                    #(#ignored_field_idents: #ignored_field_types,)*
                }

                impl #fetch_impl_generics #path::query::Fetch<#fetch_trait_punctuated_lifetimes> for #fetch_struct_name #fetch_ty_generics #where_clause {
                    type Item = #item_struct_name #ty_generics;
                    type State = #state_struct_name #fetch_ty_generics;

                    unsafe fn init(_world: &#path::world::World, state: &Self::State, _last_change_tick: u32, _change_tick: u32) -> Self {
                        Self {
                            #(#field_idents: <#fetch_init_types as #path::query::WorldQuery>::#fetch_associated_type::init(_world, &state.#field_idents, _last_change_tick, _change_tick),)*
                            #(#ignored_field_idents: Default::default(),)*
                        }
                    }

                    const IS_DENSE: bool = true #(&& <#field_types as #path::query::WorldQuery>::#fetch_associated_type::IS_DENSE)*;

                    /// SAFETY: we call `set_archetype` for each member that implements `Fetch`
                    #[inline]
                    unsafe fn set_archetype(&mut self, _state: &Self::State, _archetype: &#path::archetype::Archetype, _tables: &#path::storage::Tables) {
                        #(self.#field_idents.set_archetype(&_state.#field_idents, _archetype, _tables);)*
                    }

                    /// SAFETY: we call `set_table` for each member that implements `Fetch`
                    #[inline]
                    unsafe fn set_table(&mut self, _state: &Self::State, _table: &#path::storage::Table) {
                        #(self.#field_idents.set_table(&_state.#field_idents, _table);)*
                    }

                    /// SAFETY: we call `table_fetch` for each member that implements `Fetch`.
                    #[inline]
                    unsafe fn table_fetch(&mut self, _table_row: usize) -> Self::Item {
                        Self::Item {
                            #(#field_idents: self.#field_idents.table_fetch(_table_row),)*
                            #(#ignored_field_idents: Default::default(),)*
                        }
                    }

                    /// SAFETY: we call `archetype_fetch` for each member that implements `Fetch`.
                    #[inline]
                    unsafe fn archetype_fetch(&mut self, _archetype_index: usize) -> Self::Item {
                        Self::Item {
                            #(#field_idents: self.#field_idents.archetype_fetch(_archetype_index),)*
                            #(#ignored_field_idents: Default::default(),)*
                        }
                    }
                }
            }
        }
    };

    let fetch_impl = impl_fetch(
        fetch_struct_attributes.is_filter,
        fetch_associated_type,
        fetch_struct_name.clone(),
        item_struct_name,
    );

    let state_impl = quote! {
        #[doc(hidden)]
        #visibility struct #state_struct_name #impl_generics #where_clause {
            #(#field_idents: <#field_types as #path::query::WorldQuery>::State,)*
            #(#ignored_field_idents: #ignored_field_types,)*
        }

        // SAFETY: `update_component_access` and `update_archetype_component_access` are called for each item in the struct
        unsafe impl #impl_generics #path::query::FetchState for #state_struct_name #ty_generics #where_clause {
            fn init(world: &mut #path::world::World) -> Self {
                #state_struct_name {
                    #(#field_idents: <<#field_types as #path::query::WorldQuery>::State as #path::query::FetchState>::init(world),)*
                    #(#ignored_field_idents: Default::default(),)*
                }
            }

            fn update_component_access(&self, _access: &mut #path::query::FilteredAccess<#path::component::ComponentId>) {
                #(self.#field_idents.update_component_access(_access);)*
            }

            fn update_archetype_component_access(&self, _archetype: &#path::archetype::Archetype, _access: &mut #path::query::Access<#path::archetype::ArchetypeComponentId>) {
                #(self.#field_idents.update_archetype_component_access(_archetype, _access);)*
            }

            fn matches_archetype(&self, _archetype: &#path::archetype::Archetype) -> bool {
                true #(&& self.#field_idents.matches_archetype(_archetype))*
            }

            fn matches_table(&self, _table: &#path::storage::Table) -> bool {
                true #(&& self.#field_idents.matches_table(_table))*
            }
        }
    };

    let read_only_impl = if fetch_struct_attributes.is_filter {
        quote! {}
    } else if fetch_struct_attributes.is_mutable {
        let fetch_impl = impl_fetch(
            false,
            read_only_fetch_associated_type,
            read_only_fetch_struct_name.clone(),
            read_only_item_struct_name.clone(),
        );

        quote! {
            #fetch_impl

            impl #impl_generics #path::query::WorldQuery for #read_only_item_struct_name #ty_generics #where_clause {
                type Fetch = #read_only_fetch_struct_name #ty_generics;
                type State = #state_struct_name #ty_generics;
                type ReadOnlyFetch = #read_only_fetch_struct_name #ty_generics;
            }
        }
    } else {
        quote! {
            // Statically checks that the safety guarantee actually holds true. We need this to make
            // sure that we don't compile `ReadOnlyFetch` if our struct contains nested `WorldQuery`
            // members that don't implement it.
            #[allow(dead_code)]
            const _: () = {
                fn assert_readonly<T: #path::query::ReadOnlyFetch>() {}

                // We generate a readonly assertion for every struct member.
                fn assert_all #impl_generics () #where_clause {
                    #(assert_readonly::<<#field_types as #path::query::WorldQuery>::Fetch>();)*
                }
            };
        }
    };

    let tokens = TokenStream::from(quote! {
        #fetch_impl

        #state_impl

        #read_only_impl

        impl #impl_generics #path::query::WorldQuery for #struct_name #ty_generics #where_clause {
            type Fetch = #fetch_struct_name #ty_generics;
            type State = #state_struct_name #ty_generics;
            type ReadOnlyFetch = #read_only_fetch_struct_name #ty_generics;
        }

        /// SAFETY: each item in the struct is read only
        unsafe impl #impl_generics #path::query::ReadOnlyFetch for #read_only_fetch_struct_name #ty_generics #where_clause {}

        // The original struct will most likely be left unused. As we don't want our users having
        // to specify `#[allow(dead_code)]` for their custom queries, we are using this cursed
        // workaround.
        #[allow(dead_code)]
        const _: () = {
            fn dead_code_workaround #impl_generics (q: #struct_name #ty_generics) #where_clause {
                #(q.#field_idents;)*
                #(q.#ignored_field_idents;)*
            }
        };
    });
    tokens
}

struct WorldQueryFieldInfo {
    /// The original field type.
    field_type: Type,
    /// The same as `query_type` but with `'fetch` lifetime.
    fetch_init_type: Type,
    /// Has `#[fetch(ignore)]` or `#[filter_fetch(ignore)]` attribute.
    is_ignored: bool,
    /// All field attributes except for `world_query` ones.
    attrs: Vec<Attribute>,
}

fn read_world_query_field_info(
    field: &Field,
    world_lifetime: &Lifetime,
    fetch_lifetime: &Lifetime,
) -> WorldQueryFieldInfo {
    let is_ignored = field
        .attrs
        .iter()
        .find(|attr| {
            attr.path
                .get_ident()
                .map_or(false, |ident| ident == WORLD_QUERY_ATTRIBUTE_NAME)
        })
        .map_or(false, |attr| {
            let mut is_ignored = false;
            attr.parse_args_with(|input: ParseStream| {
                if input
                    .parse::<Option<field_attr_keywords::ignore>>()?
                    .is_some()
                {
                    is_ignored = true;
                }
                Ok(())
            })
            .unwrap_or_else(|_| {
                panic!("Invalid `{}` attribute format", WORLD_QUERY_ATTRIBUTE_NAME)
            });

            is_ignored
        });

    let attrs = field
        .attrs
        .iter()
        .filter(|attr| {
            attr.path
                .get_ident()
                .map_or(true, |ident| ident != WORLD_QUERY_ATTRIBUTE_NAME)
        })
        .cloned()
        .collect();

    let field_type = field.ty.clone();
    let mut fetch_init_type: Type = field_type.clone();

    replace_lifetime_for_type(&mut fetch_init_type, world_lifetime, fetch_lifetime);

    WorldQueryFieldInfo {
        field_type,
        fetch_init_type,
        is_ignored,
        attrs,
    }
}

// Fetch's HRTBs require substituting world lifetime with an additional one to make the
// implementation compile. I don't fully understand why this works though. If anyone's curious
// enough to try to find a better work around, I'll leave playground links here:
// - https://play.rust-lang.org/?version=stable&mode=debug&edition=2018&gist=da5e260a5c2f3e774142d60a199e854a (this fails)
// - https://play.rust-lang.org/?version=stable&mode=debug&edition=2018&gist=802517bb3d8f83c45ee8c0be360bb250 (this compiles)
fn replace_lifetime_for_type(ty: &mut Type, world_lifetime: &Lifetime, fetch_lifetime: &Lifetime) {
    match ty {
        Type::Path(ref mut path) => {
            replace_world_lifetime_for_type_path(path, world_lifetime, fetch_lifetime)
        }
        Type::Reference(ref mut reference) => {
            if let Some(lifetime) = reference.lifetime.as_mut() {
                replace_lifetime(lifetime, world_lifetime, fetch_lifetime);
            }
            replace_lifetime_for_type(reference.elem.as_mut(), world_lifetime, fetch_lifetime);
        }
        Type::Tuple(tuple) => {
            for ty in tuple.elems.iter_mut() {
                replace_lifetime_for_type(ty, world_lifetime, fetch_lifetime);
            }
        }
        ty => panic!("Unsupported type: {}", ty.to_token_stream()),
    }
}

fn replace_world_lifetime_for_type_path(
    path: &mut TypePath,
    world_lifetime: &Lifetime,
    fetch_lifetime: &Lifetime,
) {
    if let Some(qself) = path.qself.as_mut() {
        replace_lifetime_for_type(qself.ty.as_mut(), world_lifetime, fetch_lifetime);
    }

    replace_world_lifetime_for_path(&mut path.path, world_lifetime, fetch_lifetime);
}

fn replace_world_lifetime_for_path(
    path: &mut Path,
    world_lifetime: &Lifetime,
    fetch_lifetime: &Lifetime,
) {
    for segment in path.segments.iter_mut() {
        match segment.arguments {
            PathArguments::None => {}
            PathArguments::AngleBracketed(ref mut args) => {
                for arg in args.args.iter_mut() {
                    match arg {
                        GenericArgument::Lifetime(lifetime) => {
                            replace_lifetime(lifetime, world_lifetime, fetch_lifetime);
                        }
                        GenericArgument::Type(ty) => {
                            replace_lifetime_for_type(ty, world_lifetime, fetch_lifetime)
                        }
                        _ => {}
                    }
                }
            }
            PathArguments::Parenthesized(ref mut args) => {
                for input in args.inputs.iter_mut() {
                    replace_lifetime_for_type(input, world_lifetime, fetch_lifetime);
                }
                if let ReturnType::Type(_, _) = args.output {
                    panic!("Function types aren't supported");
                }
            }
        }
    }
}

fn replace_lifetime(lifetime: &mut Lifetime, world_lifetime: &Lifetime, fetch_lifetime: &Lifetime) {
    if lifetime.ident == world_lifetime.ident {
        lifetime.ident = fetch_lifetime.ident.clone();
    }
}
