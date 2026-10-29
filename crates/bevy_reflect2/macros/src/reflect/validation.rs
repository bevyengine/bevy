//! Checks that reject reflected definitions that would make the generated type information unsound.

use proc_macro2::TokenStream;
use quote::quote;
use syn::{spanned::Spanned, Attribute, Path};

use crate::reflect::{
    data::{ReflectData, ReflectField, ReflectFields},
    REFLECT_ATTRIBUTE_NAME,
};

/// Returns an error if `attrs` contains `#[repr(packed)]` (or `#[repr(packed(N))]`). Fields of packed
/// structs can be unaligned, but reflection accesses fields through references, which must be
/// aligned.
pub(crate) fn reject_packed(attrs: &[Attribute]) -> Result<(), syn::Error> {
    for attr in attrs {
        if !attr.path().is_ident("repr") {
            continue;
        }
        let mut packed = None;
        attr.parse_nested_meta(|meta| {
            if meta.path.is_ident("packed") {
                packed = Some(meta.path.span());
            }
            // Skip arguments (ex: `packed(2)` or `align(8)`)
            if meta.input.peek(syn::token::Paren) {
                let arguments;
                syn::parenthesized!(arguments in meta.input);
                arguments.parse::<TokenStream>()?;
            }
            Ok(())
        })?;
        if let Some(span) = packed {
            return Err(syn::Error::new(
                span,
                format_args!(
                    "reflection doesn't support `#[repr(packed)]`, because fields of packed structs \
                    can be unaligned. Use `#[{REFLECT_ATTRIBUTE_NAME}(opaque)]` to reflect the type \
                    without its fields."
                ),
            ));
        }
    }
    Ok(())
}

/// Generates statements that fail to compile if the re-stated definition in `data` does not exactly
/// match the fields, variants, and field types of `Self`.
pub(crate) fn definition_check(data: &ReflectData, bevy_reflect: &Path) -> TokenStream {
    let exports = quote! {#bevy_reflect::__macro_exports};
    let check_fields = |fields: &ReflectFields| {
        let checks = fields.fields.iter().map(|field| {
            let binding = &field.binding;
            let field_ty = &field.field.ty;
            // The type is inferred from the binding without an expected type, which prevents coercions
            // (ex: `&Box<T>` to `&T`) from hiding a mismatch. `Invariant` prevents subtyping (ex:
            // `for<'a> fn(&'a u8)` to `fn(&'static u8)`), which would let reflection write a value of
            // the re-stated type into a field that requires the more general type.
            quote! {
                let #binding = #exports::invariant_of(#binding);
                let _: #exports::Invariant<#field_ty> = #binding;
            }
        });
        quote! { #(#checks)* }
    };
    let body = match data {
        ReflectData::Struct(fields) => {
            let patterns = fields.fields.iter().map(ReflectField::pattern);
            let checks = check_fields(fields);
            // The pattern does not use `..`, so it must list every field
            quote! {
                let Self { #(#patterns),* } = value;
                #checks
            }
        }
        ReflectData::Enum(variants) => {
            let arms = variants.iter().map(|variant| {
                let variant_ident = &variant.variant.ident;
                let patterns = variant.fields.fields.iter().map(ReflectField::pattern);
                let checks = check_fields(&variant.fields);
                quote! {
                    Self::#variant_ident { #(#patterns),* } => { #checks }
                }
            });
            let value_access = if variants.is_empty() {
                quote! {*value}
            } else {
                quote! {value}
            };
            // The match has no wildcard, so it must list every variant
            quote! {
                match #value_access {
                    #(#arms)*
                }
            }
        }
    };
    quote! {
        let _ = |value: &Self| { #body };
    }
}
