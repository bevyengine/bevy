#![expect(missing_docs, reason = "Not all docs are written yet, see #3492.")]
#![cfg_attr(docsrs, feature(doc_cfg))]

use core::ops::Not;

use bevy_macro_utils::BevyManifest;
use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, Data, DeriveInput, Field, Index, Member};

const BINNED: &str = "BinnedPhaseItem";
const SORTED: &str = "SortedPhaseItem";

const PHASE_ITEM_ATTR: &str = "phase_item";
const SKIP_ATTR: &str = "skip";

const PHASE_ITEM_TRAITS: [&str; 7] = [
    "PhaseItem",                     // 0
    "BinnedPhaseItem",               // 1
    "SortedPhaseItem",               // 2
    "CachedRenderPipelinePhaseItem", // 3
    "QueueBinnedPhaseItem",          // 4
    "QueueSortedPhaseItem",          // 5
    "PhaseItemExt",                  // 6
];

const BINNED_BLACKLIST: [usize; 1] = [1];
const SORTED_BLACKLIST: [usize; 1] = [5];

pub(crate) fn bevy_render_path() -> syn::Path {
    BevyManifest::shared(|manifest| manifest.get_path("bevy_render"))
}

pub(crate) fn bevy_ecs_path() -> syn::Path {
    BevyManifest::shared(|manifest| manifest.get_path("bevy_ecs"))
}

pub(crate) fn bevy_pbr_path() -> syn::Path {
    BevyManifest::shared(|manifest| manifest.get_path("bevy_pbr"))
}

/// Implements `PhaseItem`, `BinnedPhaseItem`, `CachedRenderPipelinePhaseItem`,
/// `QueueBinnedPhaseItem` and `PhaseItemExt` for a wrapper type.
///
/// ### Newtypes
/// For single-field tuple structs, all traits are derived automatically.
///
/// ### Non-newtype structs
/// For other struct forms, `BinnedPhaseItem` cannot be derived automatically and must be
/// skipped explicitly using `#[phase_item(skip(...))]`.
///
/// The `#[phase_item]` attribute is also responsible for indicating the inner phase item field.
#[proc_macro_derive(BinnedPhaseItem, attributes(phase_item))]
pub fn derive_binned_phase_item(input: TokenStream) -> TokenStream {
    derive_phase_item(input, true)
}

/// Implements `PhaseItem`, `SortedPhaseItem`, `CachedRenderPipelinePhaseItem`,
/// `QueueSortedPhaseItem` and `PhaseItemExt` for a wrapper type.
///
/// NOTE: Currently, we are using the default implementation of `sort` for `SortedPhaseItem`.
///
/// ### Newtypes
/// For single-field tuple structs, all traits are derived automatically.
///
/// ### Non-newtype structs
/// For other struct forms, `QueueSortedPhaseItem` cannot be derived automatically and must be
/// skipped explicitly using `#[phase_item(skip(...))]`.
///
/// The `#[phase_item]` attribute is responsible for indicating the inner phase item field.
#[proc_macro_derive(SortedPhaseItem, attributes(phase_item))]
pub fn derive_sorted_phase_item(input: TokenStream) -> TokenStream {
    derive_phase_item(input, false)
}

fn derive_phase_item(input: TokenStream, is_binned: bool) -> TokenStream {
    let ast = parse_macro_input!(input as DeriveInput);

    let (member, inner_ty, skip_list) = match get_phase_item_field(&ast, is_binned) {
        Ok(value) => value,
        Err(err) => return err.to_compile_error().into(),
    };

    let bevy_render = bevy_render_path();
    let bevy_ecs = bevy_ecs_path();
    let bevy_pbr = bevy_pbr_path();

    let struct_name = &ast.ident;
    let generics = &ast.generics;
    let (impl_generics, type_generics, where_clause) = generics.split_for_impl();

    let phase_item_impl =
        skip_list
            .contains(&PHASE_ITEM_TRAITS[0])
            .not()
            .then_some(impl_phase_item(
                struct_name,
                &impl_generics,
                &type_generics,
                where_clause,
                &member,
                &bevy_render,
                &bevy_ecs,
            ));

    let x_phase_item_impl = if is_binned {
        skip_list
            .contains(&PHASE_ITEM_TRAITS[1])
            .not()
            .then_some(impl_binned_phase_item(
                struct_name,
                &impl_generics,
                &type_generics,
                where_clause,
                inner_ty,
                &bevy_render,
                &bevy_ecs,
            ))
    } else {
        skip_list
            .contains(&PHASE_ITEM_TRAITS[2])
            .not()
            .then_some(impl_sorted_phase_item(
                struct_name,
                &impl_generics,
                &type_generics,
                where_clause,
                &member,
                inner_ty,
                &bevy_render,
            ))
    };

    let cached_pipeline_impl =
        skip_list
            .contains(&PHASE_ITEM_TRAITS[3])
            .not()
            .then_some(impl_cached_pipeline(
                struct_name,
                &impl_generics,
                &type_generics,
                &member,
                where_clause,
                &bevy_render,
            ));

    let queue_x_phase_item_impl = if is_binned {
        skip_list
            .contains(&PHASE_ITEM_TRAITS[4])
            .not()
            .then_some(impl_queue_binned_phase_item(
                struct_name,
                &impl_generics,
                &type_generics,
                where_clause,
                inner_ty,
                &bevy_pbr,
                &bevy_render,
            ))
    } else {
        skip_list
            .contains(&PHASE_ITEM_TRAITS[5])
            .not()
            .then_some(impl_queue_sorted_phase_item(
                struct_name,
                &impl_generics,
                &type_generics,
                where_clause,
                inner_ty,
                &bevy_pbr,
            ))
    };

    let phase_item_ext_impl =
        skip_list
            .contains(&PHASE_ITEM_TRAITS[6])
            .not()
            .then_some(impl_phase_item_ext(
                struct_name,
                &impl_generics,
                &type_generics,
                where_clause,
                inner_ty,
                &bevy_pbr,
            ));

    TokenStream::from(quote! {
        #phase_item_impl
        #x_phase_item_impl
        #cached_pipeline_impl
        #queue_x_phase_item_impl
        #phase_item_ext_impl
    })
}

fn get_phase_item_field(
    ast: &DeriveInput,
    is_binned: bool,
) -> syn::Result<(Member, &syn::Type, Vec<&str>)> {
    let phase_item_kind = if is_binned { BINNED } else { SORTED };

    let Data::Struct(data_struct) = &ast.data else {
        return Err(syn::Error::new_spanned(
            &ast.ident,
            format!("`#[derive({phase_item_kind})]` is only supported for structs. Please ensure your type is a struct."),
        ));
    };

    if data_struct.fields.is_empty() {
        return Err(syn::Error::new_spanned(
            &ast.ident,
            format!("{phase_item_kind} cannot be derived on field-less structs"),
        ));
    }

    // Collect all fields with #[phase_item]
    let mut marked_fields: Vec<_> = data_struct
        .fields
        .iter()
        .enumerate()
        .filter_map(|(idx, field)| {
            field
                .attrs
                .iter()
                .find(|a| a.path().is_ident(PHASE_ITEM_ATTR))
                .map(|attr| (idx, field, attr))
        })
        .collect();

    if marked_fields.len() > 1 {
        let (_, _, second_attr) = marked_fields[1];
        return Err(syn::Error::new_spanned(
            second_attr,
            format!("`#[{PHASE_ITEM_ATTR}]` attribute can only be used on a single field"),
        ));
    }

    let (index, field, phase_item_attr) = match marked_fields.pop() {
        // Handle explicit marking
        Some((idx, field, attr)) => (idx, field, Some(attr)),
        // Auto select for single field
        None if data_struct.fields.len() == 1 => {
            (0, data_struct.fields.iter().next().unwrap(), None)
        }
        None => {
            return Err(syn::Error::new_spanned(
                &ast.ident,
                format!(
                    "`#[derive({phase_item_kind})]` requires a field with the `#[{PHASE_ITEM_ATTR}]` attribute on multi-field structs",
                ),
            ));
        }
    };

    let mut skip_list = Vec::new();
    if let Some(attr) = phase_item_attr {
        attr.parse_nested_meta(|meta| {
            if meta.path.is_ident(SKIP_ATTR) {
                meta.parse_nested_meta(|inner_meta| {
                    let trait_name = inner_meta
                        .path
                        .get_ident()
                        .ok_or_else(|| {
                            syn::Error::new_spanned(&inner_meta.path, "expected identifier")
                        })?
                        .to_string();

                    if let Some(idx) = PHASE_ITEM_TRAITS
                        .iter()
                        .position(|i| *i == trait_name.as_str())
                    {
                        skip_list.push(PHASE_ITEM_TRAITS[idx]);
                    } else {
                        return Err(syn::Error::new_spanned(
                            &inner_meta.path,
                            format!(
                                "unexpected trait `{}`, expected one of: {}",
                                trait_name,
                                PHASE_ITEM_TRAITS.join(", ")
                            ),
                        ));
                    }
                    Ok(())
                })
            } else {
                Err(meta.error(format!("unexpected attribute, expected `{SKIP_ATTR}`")))
            }
        })?;
    }

    let is_single_field = data_struct.fields.len() == 1;
    let is_tuple_struct = matches!(data_struct.fields, syn::Fields::Unnamed(_));

    if !(is_single_field && is_tuple_struct) {
        let blacklist: Vec<_> = if is_binned {
            &BINNED_BLACKLIST
        } else {
            &SORTED_BLACKLIST
        }
        .iter()
        .map(|&idx| PHASE_ITEM_TRAITS[idx])
        .collect();

        let all_traits_valid = blacklist.iter().all(|i| skip_list.contains(i));
        if !all_traits_valid {
            return Err(syn::Error::new_spanned(
                &ast.ident,
                format!("`#[derive({phase_item_kind})]` can only implement the trait `{}` for single-field tuple structs.\n\
                help: Use `#[phase_item(skip({}))]` on the field to skip it.", blacklist.join(", "), blacklist.join(", ")),
            ));
        }
    }

    let member = to_member(field, index);
    Ok((member, &field.ty, skip_list))
}

fn to_member(field: &Field, index: usize) -> Member {
    field
        .ident
        .as_ref()
        .map(|name| Member::Named(name.clone()))
        .unwrap_or_else(|| Member::Unnamed(Index::from(index)))
}

fn impl_phase_item(
    struct_name: &syn::Ident,
    impl_generics: &impl quote::ToTokens,
    type_generics: &impl quote::ToTokens,
    where_clause: Option<&syn::WhereClause>,
    member: &Member,
    bevy_render: &syn::Path,
    bevy_ecs: &syn::Path,
) -> proc_macro2::TokenStream {
    quote! {
        impl #impl_generics #bevy_render::render_phase::PhaseItem
            for #struct_name #type_generics #where_clause
        {
            #[inline]
            fn entity(&self) -> #bevy_ecs::entity::Entity {
                self.#member.entity()
            }

            #[inline]
            fn main_entity(&self) -> #bevy_render::sync_world::MainEntity {
                self.#member.main_entity()
            }

            #[inline]
            fn draw_function(&self) -> #bevy_render::render_phase::DrawFunctionId {
                self.#member.draw_function()
            }

            #[inline]
            fn batch_range(&self) -> &::core::ops::Range<u32> {
                self.#member.batch_range()
            }

            #[inline]
            fn batch_range_mut(&mut self) -> &mut ::core::ops::Range<u32> {
                self.#member.batch_range_mut()
            }

            #[inline]
            fn extra_index(&self) -> #bevy_render::render_phase::PhaseItemExtraIndex {
                self.#member.extra_index()
            }

            #[inline]
            fn batch_range_and_extra_index_mut(
                &mut self,
            ) -> (
                &mut ::core::ops::Range<u32>,
                &mut #bevy_render::render_phase::PhaseItemExtraIndex
            ) {
                self.#member.batch_range_and_extra_index_mut()
            }
        }
    }
}

fn impl_binned_phase_item(
    struct_name: &syn::Ident,
    impl_generics: &impl quote::ToTokens,
    type_generics: &impl quote::ToTokens,
    where_clause: Option<&syn::WhereClause>,
    inner_ty: &syn::Type,
    bevy_render: &syn::Path,
    bevy_ecs: &syn::Path,
) -> proc_macro2::TokenStream {
    quote! {
        impl #impl_generics #bevy_render::render_phase::BinnedPhaseItem
            for #struct_name #type_generics #where_clause
        {
            type BatchSetKey = <#inner_ty as #bevy_render::render_phase::BinnedPhaseItem>::BatchSetKey;
            type BinKey = <#inner_ty as #bevy_render::render_phase::BinnedPhaseItem>::BinKey;

            #[inline]
            fn new(
                batch_set_key: Self::BatchSetKey,
                bin_key: Self::BinKey,
                representative_entity: (#bevy_ecs::entity::Entity, #bevy_render::sync_world::MainEntity),
                batch_range: ::core::ops::Range<u32>,
                extra_index: #bevy_render::render_phase::PhaseItemExtraIndex,
            ) -> Self {
                Self(<#inner_ty as #bevy_render::render_phase::BinnedPhaseItem>::new(
                    batch_set_key,
                    bin_key,
                    representative_entity,
                    batch_range,
                    extra_index,
                ))
            }
        }
    }
}

fn impl_sorted_phase_item(
    struct_name: &syn::Ident,
    impl_generics: &impl quote::ToTokens,
    type_generics: &impl quote::ToTokens,
    where_clause: Option<&syn::WhereClause>,
    member: &Member,
    inner_ty: &syn::Type,
    bevy_render: &syn::Path,
) -> proc_macro2::TokenStream {
    quote! {
        impl #impl_generics #bevy_render::render_phase::SortedPhaseItem
            for #struct_name #type_generics #where_clause
        {
            type SortKey = <#inner_ty as #bevy_render::render_phase::SortedPhaseItem>::SortKey;

            #[inline]
            fn sort_key(&self) -> Self::SortKey {
                <#inner_ty as #bevy_render::render_phase::SortedPhaseItem>::sort_key(&self.#member)
            }

            // NOTE: Currently, we are using the default implementation of `sort`.
            // #[inline]
            // fn sort(items: &mut [Self]) {
            //     // To address this, we need to convert `&mut [Newtype]` to `&mut [Inner]`.
            //     <#inner_ty as #bevy_render::render_phase::SortedPhaseItem>::sort(items)
            //
            //     // The simplest solution might be to reexport `radsort` and use it directly here.
            //     radsort::sort_by_key(items, |item| item.sort_key().#member)
            // }

            #[inline]
            fn indexed(&self) -> bool {
                self.#member.indexed()
            }
        }
    }
}

fn impl_cached_pipeline(
    struct_name: &syn::Ident,
    impl_generics: &impl quote::ToTokens,
    type_generics: &impl quote::ToTokens,
    member: &Member,
    where_clause: Option<&syn::WhereClause>,
    bevy_render: &syn::Path,
) -> proc_macro2::TokenStream {
    quote! {
        impl #impl_generics #bevy_render::render_phase::CachedRenderPipelinePhaseItem
            for #struct_name #type_generics #where_clause
        {
            #[inline]
            fn cached_pipeline(&self) -> #bevy_render::render_resource::CachedRenderPipelineId {
                self.#member.cached_pipeline()
            }
        }
    }
}

fn impl_queue_binned_phase_item(
    struct_name: &syn::Ident,
    impl_generics: &impl quote::ToTokens,
    type_generics: &impl quote::ToTokens,
    where_clause: Option<&syn::WhereClause>,
    inner_ty: &syn::Type,
    bevy_pbr: &syn::Path,
    bevy_render: &syn::Path,
) -> proc_macro2::TokenStream {
    quote! {
        impl #impl_generics #bevy_pbr::QueueBinnedPhaseItem
            for #struct_name #type_generics #where_clause
        {
            #[inline]
            fn queue_item<BPI>(context: &#bevy_pbr::PhaseContext, render_phase: &mut #bevy_render::render_phase::BinnedRenderPhase<BPI>)
            where
                BPI: #bevy_render::render_phase::BinnedPhaseItem<BatchSetKey = Self::BatchSetKey, BinKey = Self::BinKey>,
            {
                <#inner_ty as #bevy_pbr::QueueBinnedPhaseItem>::queue_item(context, render_phase)
            }
        }
    }
}

fn impl_queue_sorted_phase_item(
    struct_name: &syn::Ident,
    impl_generics: &impl quote::ToTokens,
    type_generics: &impl quote::ToTokens,
    where_clause: Option<&syn::WhereClause>,
    inner_ty: &syn::Type,
    bevy_pbr: &syn::Path,
) -> proc_macro2::TokenStream {
    quote! {
        impl #impl_generics #bevy_pbr::QueueSortedPhaseItem
            for #struct_name #type_generics #where_clause
        {
            #[inline]
            fn get_item(context: &#bevy_pbr::PhaseContext) -> Option<Self> {
                <#inner_ty as #bevy_pbr::QueueSortedPhaseItem>::get_item(context).map(Self)
            }
        }
    }
}

fn impl_phase_item_ext(
    struct_name: &syn::Ident,
    impl_generics: &impl quote::ToTokens,
    type_generics: &impl quote::ToTokens,
    where_clause: Option<&syn::WhereClause>,
    inner_ty: &syn::Type,
    bevy_pbr: &syn::Path,
) -> proc_macro2::TokenStream {
    quote! {
        impl #impl_generics #bevy_pbr::PhaseItemExt
            for #struct_name #type_generics #where_clause
        {
            type PhaseFamily = <#inner_ty as #bevy_pbr::PhaseItemExt>::PhaseFamily;
            type ExtractCondition = <#inner_ty as #bevy_pbr::PhaseItemExt>::ExtractCondition;
            type RenderCommand = <#inner_ty as #bevy_pbr::PhaseItemExt>::RenderCommand;
            const PHASE_TYPES: RenderPhaseType = <#inner_ty as #bevy_pbr::PhaseItemExt>::PHASE_TYPES;
        }
    }
}
