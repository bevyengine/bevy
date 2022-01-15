use proc_macro::TokenStream;
use proc_macro2::{Ident, Span};
use quote::{quote, ToTokens};
use syn::{
    parse_macro_input, parse_quote, punctuated::Punctuated, Data, DataStruct, DeriveInput, Fields,
    GenericArgument, GenericParam, ImplGenerics, Lifetime, LifetimeDef, Path, PathArguments,
    ReturnType, Token, Type, TypeGenerics, TypePath, WhereClause,
};

use crate::bevy_ecs_path;

static MUTABLE_ATTRIBUTE_NAME: &str = "mutable";
static READ_ONLY_DERIVE_ATTRIBUTE_NAME: &str = "read_only_derive";

pub fn derive_fetch_impl(input: TokenStream) -> TokenStream {
    let ast = parse_macro_input!(input as DeriveInput);

    let FetchImplTokens {
        struct_name,
        struct_name_read_only,
        fetch_struct_name,
        state_struct_name,
        read_only_fetch_struct_name,
        fetch_trait_punctuated_lifetimes,
        impl_generics,
        ty_generics,
        where_clause,
        has_world_lifetime,
        has_mutable_attr,
        world_lifetime,
        state_lifetime,
        fetch_lifetime,
        phantom_field_idents,
        phantom_field_types,
        field_idents,
        field_types: _,
        query_types,
        fetch_init_types,
    } = fetch_impl_tokens(&ast);

    if !has_world_lifetime {
        panic!("Expected a struct with a lifetime");
    }

    let read_only_derive_attr = ast.attrs.iter().find(|attr| {
        attr.path
            .get_ident()
            .map_or(false, |ident| ident == READ_ONLY_DERIVE_ATTRIBUTE_NAME)
    });
    let read_only_derive_macro_call = if let Some(read_only_derive_attr) = read_only_derive_attr {
        if !has_mutable_attr {
            panic!("Attribute `read_only_derive` can only be be used for a struct marked with the `mutable` attribute");
        }
        let derive_args = &read_only_derive_attr.tokens;
        quote! { #[derive #derive_args] }
    } else {
        quote! {}
    };

    // Add `'state` and `'fetch` lifetimes that will be used in `Fetch` implementation.
    let mut fetch_generics = ast.generics.clone();
    fetch_generics.params.insert(1, state_lifetime);
    fetch_generics.params.insert(2, fetch_lifetime.clone());
    let (fetch_impl_generics, _, _) = fetch_generics.split_for_impl();

    // Replace lifetime `'world` with `'fetch`. See `replace_lifetime_for_type` for more details.
    let mut fetch_generics = ast.generics.clone();
    *fetch_generics.params.first_mut().unwrap() = fetch_lifetime;
    let (_, fetch_ty_generics, _) = fetch_generics.split_for_impl();

    let path = bevy_ecs_path();

    let struct_read_only_declaration = if has_mutable_attr {
        quote! {
            // TODO: it would be great to be able to dedup this by just deriving `Fetch` again
            //  without the `mutable` attribute, but we'd need a way to avoid creating a redundant
            //  `State` struct.
            #read_only_derive_macro_call
            struct #struct_name_read_only #impl_generics #where_clause {
                #(#field_idents: <<#query_types as #path::query::WorldQuery>::ReadOnlyFetch as #path::query::Fetch<#world_lifetime, #world_lifetime>>::Item,)*
                #(#phantom_field_idents: #phantom_field_types,)*
            }

            struct #read_only_fetch_struct_name #impl_generics #where_clause {
                #(#field_idents: <#query_types as #path::query::WorldQuery>::ReadOnlyFetch,)*
                #(#phantom_field_idents: #phantom_field_types,)*
            }

            impl #fetch_impl_generics #path::query::Fetch<#fetch_trait_punctuated_lifetimes> for #read_only_fetch_struct_name #fetch_ty_generics #where_clause {
                type Item = #struct_name_read_only #ty_generics;
                type State = #state_struct_name #fetch_ty_generics;

                unsafe fn init(_world: &#path::world::World, state: &Self::State, _last_change_tick: u32, _change_tick: u32) -> Self {
                    #read_only_fetch_struct_name {
                        #(#field_idents: <#fetch_init_types as #path::query::WorldQuery>::ReadOnlyFetch::init(_world, &state.#field_idents, _last_change_tick, _change_tick),)*
                        #(#phantom_field_idents: Default::default(),)*
                    }
                }

                const IS_DENSE: bool = true #(&& <#query_types as #path::query::WorldQuery>::ReadOnlyFetch::IS_DENSE)*;

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
                    #struct_name_read_only {
                        #(#field_idents: self.#field_idents.table_fetch(_table_row),)*
                        #(#phantom_field_idents: Default::default(),)*
                    }
                }

                /// SAFETY: we call `archetype_fetch` for each member that implements `Fetch`.
                #[inline]
                unsafe fn archetype_fetch(&mut self, _archetype_index: usize) -> Self::Item {
                    #struct_name_read_only {
                        #(#field_idents: self.#field_idents.archetype_fetch(_archetype_index),)*
                        #(#phantom_field_idents: Default::default(),)*
                    }
                }
            }

            impl #impl_generics #path::query::WorldQuery for #struct_name_read_only #ty_generics #where_clause {
                type Fetch = #read_only_fetch_struct_name #ty_generics;
                type State = #state_struct_name #ty_generics;
                type ReadOnlyFetch = #read_only_fetch_struct_name #ty_generics;
            }

            impl #impl_generics #path::query::FetchedItem for #struct_name_read_only #ty_generics #where_clause {
                type Query = Self;
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
                    #(assert_readonly::<<#query_types as #path::query::WorldQuery>::Fetch>();)*
                }
            };
        }
    };

    let tokens = TokenStream::from(quote! {
        struct #fetch_struct_name #impl_generics #where_clause {
            #(#field_idents: <#query_types as #path::query::WorldQuery>::Fetch,)*
            #(#phantom_field_idents: #phantom_field_types,)*
        }

        struct #state_struct_name #impl_generics #where_clause {
            #(#field_idents: <#query_types as #path::query::WorldQuery>::State,)*
            #(#phantom_field_idents: #phantom_field_types,)*
        }

        impl #fetch_impl_generics #path::query::Fetch<#fetch_trait_punctuated_lifetimes> for #fetch_struct_name #fetch_ty_generics #where_clause {
            type Item = #struct_name #ty_generics;
            type State = #state_struct_name #fetch_ty_generics;

            unsafe fn init(_world: &#path::world::World, state: &Self::State, _last_change_tick: u32, _change_tick: u32) -> Self {
                #fetch_struct_name {
                    #(#field_idents: <#fetch_init_types as #path::query::WorldQuery>::Fetch::init(_world, &state.#field_idents, _last_change_tick, _change_tick),)*
                    #(#phantom_field_idents: Default::default(),)*
                }
            }

            const IS_DENSE: bool = true #(&& <#query_types as #path::query::WorldQuery>::Fetch::IS_DENSE)*;

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
                #struct_name {
                    #(#field_idents: self.#field_idents.table_fetch(_table_row),)*
                    #(#phantom_field_idents: Default::default(),)*
                }
            }

            /// SAFETY: we call `archetype_fetch` for each member that implements `Fetch`.
            #[inline]
            unsafe fn archetype_fetch(&mut self, _archetype_index: usize) -> Self::Item {
                #struct_name {
                    #(#field_idents: self.#field_idents.archetype_fetch(_archetype_index),)*
                    #(#phantom_field_idents: Default::default(),)*
                }
            }
        }

        // SAFETY: `update_component_access` and `update_archetype_component_access` are called for each item in the struct
        unsafe impl #impl_generics #path::query::FetchState for #state_struct_name #ty_generics #where_clause {
            fn init(world: &mut #path::world::World) -> Self {
                #state_struct_name {
                    #(#field_idents: <<#query_types as #path::query::WorldQuery>::State as #path::query::FetchState>::init(world),)*
                    #(#phantom_field_idents: Default::default(),)*
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

        #struct_read_only_declaration

        impl #impl_generics #path::query::WorldQuery for #struct_name #ty_generics #where_clause {
            type Fetch = #fetch_struct_name #ty_generics;
            type State = #state_struct_name #ty_generics;
            type ReadOnlyFetch = #read_only_fetch_struct_name #ty_generics;
        }

        impl #impl_generics #path::query::FetchedItem for #struct_name #ty_generics #where_clause {
            type Query = Self;
        }

        /// SAFETY: each item in the struct is read only
        unsafe impl #impl_generics #path::query::ReadOnlyFetch for #read_only_fetch_struct_name #ty_generics #where_clause {}
    });
    tokens
}

pub fn derive_filter_fetch_impl(input: TokenStream) -> TokenStream {
    let ast = parse_macro_input!(input as DeriveInput);

    let FetchImplTokens {
        struct_name,
        struct_name_read_only: _,
        fetch_struct_name,
        state_struct_name,
        read_only_fetch_struct_name: _,
        fetch_trait_punctuated_lifetimes,
        impl_generics,
        ty_generics,
        where_clause,
        has_mutable_attr: _,
        has_world_lifetime,
        world_lifetime,
        state_lifetime,
        fetch_lifetime,
        phantom_field_idents,
        phantom_field_types,
        field_idents,
        field_types,
        query_types: _,
        fetch_init_types: _,
    } = fetch_impl_tokens(&ast);

    if has_world_lifetime {
        panic!("Expected a struct without a lifetime");
    }

    // Add `'world`, `'state` and `'fetch` lifetimes that will be used in `Fetch` implementation.
    let mut fetch_generics = ast.generics.clone();
    fetch_generics.params.insert(0, world_lifetime);
    fetch_generics.params.insert(1, state_lifetime);
    fetch_generics.params.insert(2, fetch_lifetime);
    let (fetch_impl_generics, _, _) = fetch_generics.split_for_impl();

    let path = bevy_ecs_path();

    let tokens = TokenStream::from(quote! {
        struct #fetch_struct_name #impl_generics #where_clause {
            #(#field_idents: <#field_types as #path::query::WorldQuery>::Fetch,)*
            #(#phantom_field_idents: #phantom_field_types,)*
        }

        struct #state_struct_name #impl_generics #where_clause {
            #(#field_idents: <#field_types as #path::query::WorldQuery>::State,)*
            #(#phantom_field_idents: #phantom_field_types,)*
        }

        impl #fetch_impl_generics #path::query::Fetch<#fetch_trait_punctuated_lifetimes> for #fetch_struct_name #ty_generics #where_clause {
            type Item = bool;
            type State = #state_struct_name #ty_generics;

            unsafe fn init(_world: &#path::world::World, state: &Self::State, _last_change_tick: u32, _change_tick: u32) -> Self {
                #fetch_struct_name {
                    #(#field_idents: <#field_types as #path::query::WorldQuery>::ReadOnlyFetch::init(_world, &state.#field_idents, _last_change_tick, _change_tick),)*
                    #(#phantom_field_idents: Default::default(),)*
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

        // SAFETY: update_component_access and update_archetype_component_access are called for each item in the struct
        unsafe impl #impl_generics #path::query::FetchState for #state_struct_name #ty_generics #where_clause {
            fn init(world: &mut #path::world::World) -> Self {
                #state_struct_name {
                    #(#field_idents: <<#field_types as #path::query::WorldQuery>::State as #path::query::FetchState>::init(world),)*
                    #(#phantom_field_idents: Default::default(),)*
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

        impl #impl_generics #path::query::WorldQuery for #struct_name #ty_generics #where_clause {
            type Fetch = #fetch_struct_name #ty_generics;
            type State = #state_struct_name #ty_generics;
            type ReadOnlyFetch = #fetch_struct_name #ty_generics;
        }

        /// SAFETY: each item in the struct is read-only as filters never actually fetch any data that could be mutated
        unsafe impl #impl_generics #path::query::ReadOnlyFetch for #fetch_struct_name #ty_generics #where_clause {}
    });
    tokens
}

// This struct is used to share common tokens between `Fetch` and `FilterFetch` implementations.
struct FetchImplTokens<'a> {
    struct_name: Ident,
    // Equals `struct_name` if `has_mutable_attr` is false.
    struct_name_read_only: Ident,
    fetch_struct_name: Ident,
    state_struct_name: Ident,
    read_only_fetch_struct_name: Ident,
    fetch_trait_punctuated_lifetimes: Punctuated<GenericParam, Token![,]>,
    impl_generics: ImplGenerics<'a>,
    ty_generics: TypeGenerics<'a>,
    where_clause: Option<&'a WhereClause>,
    has_mutable_attr: bool,
    has_world_lifetime: bool,
    world_lifetime: GenericParam,
    state_lifetime: GenericParam,
    fetch_lifetime: GenericParam,
    phantom_field_idents: Vec<Ident>,
    phantom_field_types: Vec<Type>,
    field_idents: Vec<Ident>,
    field_types: Vec<Type>,
    query_types: Vec<Type>,
    fetch_init_types: Vec<Type>,
}

fn fetch_impl_tokens(ast: &DeriveInput) -> FetchImplTokens {
    let has_mutable_attr = ast.attrs.iter().any(|attr| {
        attr.path
            .get_ident()
            .map_or(false, |ident| ident == MUTABLE_ATTRIBUTE_NAME)
    });

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
    let struct_name_read_only = if has_mutable_attr {
        Ident::new(&format!("{}ReadOnly", struct_name), Span::call_site())
    } else {
        ast.ident.clone()
    };
    let fetch_struct_name = Ident::new(&format!("{}Fetch", struct_name), Span::call_site());
    let state_struct_name = Ident::new(&format!("{}State", struct_name), Span::call_site());
    let read_only_fetch_struct_name = if has_mutable_attr {
        Ident::new(&format!("{}ReadOnlyFetch", struct_name), Span::call_site())
    } else {
        fetch_struct_name.clone()
    };

    let fields = match &ast.data {
        Data::Struct(DataStruct {
            fields: Fields::Named(fields),
            ..
        }) => &fields.named,
        _ => panic!("Expected a struct with named fields"),
    };

    let mut phantom_field_idents = Vec::new();
    let mut phantom_field_types = Vec::new();
    let mut field_idents = Vec::new();
    let mut field_types = Vec::new();
    let mut query_types = Vec::new();
    let mut fetch_init_types = Vec::new();

    let (world_lifetime, fetch_lifetime) = match (&world_lifetime_param, &fetch_lifetime_param) {
        (GenericParam::Lifetime(world), GenericParam::Lifetime(fetch)) => {
            (&world.lifetime, &fetch.lifetime)
        }
        _ => unreachable!(),
    };
    for field in fields.iter() {
        let WorldQueryFieldTypeInfo {
            query_type,
            fetch_init_type: init_type,
            is_phantom_data,
        } = read_world_query_field_type_info(&field.ty, world_lifetime, fetch_lifetime);

        let field_ident = field.ident.as_ref().unwrap().clone();
        if is_phantom_data {
            phantom_field_idents.push(field_ident.clone());
            phantom_field_types.push(field.ty.clone());
        } else {
            field_idents.push(field_ident.clone());
            field_types.push(field.ty.clone());
            query_types.push(query_type);
            fetch_init_types.push(init_type);
        }
    }

    FetchImplTokens {
        struct_name,
        struct_name_read_only,
        fetch_struct_name,
        state_struct_name,
        read_only_fetch_struct_name,
        fetch_trait_punctuated_lifetimes,
        impl_generics,
        ty_generics,
        where_clause,
        has_mutable_attr,
        has_world_lifetime,
        world_lifetime: world_lifetime_param,
        state_lifetime: state_lifetime_param,
        fetch_lifetime: fetch_lifetime_param,
        phantom_field_idents,
        phantom_field_types,
        field_idents,
        field_types,
        query_types,
        fetch_init_types,
    }
}

struct WorldQueryFieldTypeInfo {
    /// We convert `Mut<T>` to `&mut T` (because this is the type that implements `WorldQuery`)
    /// and store it here.
    query_type: Type,
    /// The same as `query_type` but with `'fetch` lifetime.
    fetch_init_type: Type,
    is_phantom_data: bool,
}

fn read_world_query_field_type_info(
    ty: &Type,
    world_lifetime: &Lifetime,
    fetch_lifetime: &Lifetime,
) -> WorldQueryFieldTypeInfo {
    let path = bevy_ecs_path();

    let query_type: Type = parse_quote!(<#ty as #path::query::FetchedItem>::Query);
    let mut fetch_init_type: Type = query_type.clone();

    let is_phantom_data = match ty {
        Type::Path(path) => {
            if let Some(segment) = path.path.segments.last() {
                segment.ident == "PhantomData"
            } else {
                false
            }
        }
        _ => false,
    };

    replace_lifetime_for_type(&mut fetch_init_type, world_lifetime, fetch_lifetime);

    WorldQueryFieldTypeInfo {
        query_type,
        fetch_init_type,
        is_phantom_data,
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
