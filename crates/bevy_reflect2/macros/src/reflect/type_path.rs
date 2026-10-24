use proc_macro2::TokenStream;
use quote::quote;
use syn::{GenericParam, Generics, Ident, Path};

/// Returns the `GenericTypePath` impl (only generated for generic types), followed by the full and short
/// `TypePathStr` used in `Typed::TYPE`.
pub(crate) fn type_path_strings(
    target: &TokenStream,
    generics: &Generics,
    bevy_reflect: &Path,
    extended_where_clause: &TokenStream,
    type_ident_str: &str,
    module_path: &TokenStream,
    ignored_params: &[Ident],
) -> (TokenStream, TokenStream, TokenStream) {
    let mut full_generics = Vec::new();
    let mut short_generics = Vec::new();
    for param in generics.params.iter() {
        match param {
            GenericParam::Lifetime(_) => {}
            GenericParam::Const(const_param) => {
                let ident = &const_param.ident;
                full_generics.push(quote! {#ident});
                short_generics.push(quote! {#ident});
            }
            // Ignored parameters are not `Typed`, so their paths come from `type_name`, like the
            // parameter of `PhantomData<T>`
            GenericParam::Type(type_param) if ignored_params.contains(&type_param.ident) => {
                let ident = &type_param.ident;
                full_generics.push(quote! {::core::any::type_name::<#ident>()});
                short_generics
                    .push(quote! {#bevy_reflect::__macro_exports::ShortName::of::<#ident>()});
            }
            GenericParam::Type(type_param) => {
                let ident = &type_param.ident;
                full_generics
                    .push(quote! {<#ident as #bevy_reflect::Typed>::TYPE.type_path().full()});
                short_generics
                    .push(quote! {<#ident as #bevy_reflect::Typed>::TYPE.type_path().short()});
            }
        }
    }
    if full_generics.is_empty() {
        // Non-generic paths are known at compile time, so they are stored directly in `Typed::TYPE`.
        return (
            quote! {},
            quote! {#bevy_reflect::TypePathStr::Static(#bevy_reflect::const_concat!(#module_path, "::", #type_ident_str))},
            quote! {#bevy_reflect::TypePathStr::Static(#type_ident_str)},
        );
    }

    // Generic paths depend on their generic parameters, so they are computed lazily by `GenericTypePath`.
    let generics_format_str = vec!["{}"; full_generics.len()].join(", ");
    let full_format_str = format!("{{}}::{type_ident_str}<{generics_format_str}>");
    let short_format_str = format!("{type_ident_str}<{generics_format_str}>");
    let (impl_generics, _, _) = generics.split_for_impl();
    let generic_type_path_impl = quote! {
        impl #impl_generics #bevy_reflect::GenericTypePath for #target #extended_where_clause {
            fn type_path() -> &'static str {
                static CELL: #bevy_reflect::GenericTypePathCell = #bevy_reflect::GenericTypePathCell::new();
                CELL.get_or_insert(::core::any::TypeId::of::<Self>(), || {
                    #bevy_reflect::__macro_exports::format!(#full_format_str, #module_path, #(#full_generics),*)
                })
            }

            fn short_type_path() -> &'static str {
                static CELL: #bevy_reflect::GenericTypePathCell = #bevy_reflect::GenericTypePathCell::new();
                CELL.get_or_insert(::core::any::TypeId::of::<Self>(), || {
                    #bevy_reflect::__macro_exports::format!(#short_format_str, #(#short_generics),*)
                })
            }
        }
    };
    (
        generic_type_path_impl,
        quote! {#bevy_reflect::TypePathStr::Lazy(<Self as #bevy_reflect::GenericTypePath>::type_path)},
        quote! {#bevy_reflect::TypePathStr::Lazy(<Self as #bevy_reflect::GenericTypePath>::short_type_path)},
    )
}

/// Returns a `TypePath` expression for a type named `type_ident_len` bytes long, defined in the
/// module at `module_path` (a `&'static str` expression).
pub(crate) fn type_path(
    bevy_reflect: &Path,
    module_path: &TokenStream,
    full_type_path: &TokenStream,
    short_type_path: &TokenStream,
    type_ident_len: usize,
) -> TokenStream {
    quote! {{
        let module_path: &'static str = #module_path;
        let module_len = module_path.len();
        let crate_len = match #bevy_reflect::find_ascii_char(module_path, b':') {
            Some(value) => value,
            None => module_len,
        };
        #bevy_reflect::TypePath::new(
            #full_type_path,
            #short_type_path,
            0..#type_ident_len,
            Some(0..crate_len),
            Some(0..module_len),
        )
    }}
}
