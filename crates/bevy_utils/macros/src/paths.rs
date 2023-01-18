use bevy_macro_utils::BevyManifest;
use quote::format_ident;

#[inline]
pub(crate) fn bevy_utils_path() -> syn::Path {
    BevyManifest::default().get_path("bevy_utils")
}

#[inline]
pub(crate) fn iterable_enum_path() -> syn::Path {
    let mut utils_path = bevy_utils_path();
    utils_path
        .segments
        .push(format_ident!("IterableEnum").into());
    utils_path
}
