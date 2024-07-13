// FIXME(3492): remove once docs are ready
#![allow(missing_docs)]
#![cfg_attr(docsrs, feature(doc_auto_cfg))]

use bevy_macro_utils::{get_lit_str, get_struct_fields, BevyManifest, Symbol};
use proc_macro::{Span, TokenStream};
use quote::{format_ident, quote, ToTokens};
use syn::{parse_macro_input, Data, DeriveInput, Field, Path};

pub(crate) fn bevy_app_path() -> Path {
    BevyManifest::default().get_path("bevy_app")
}

pub(crate) fn bevy_asset_path() -> Path {
    BevyManifest::default().get_path("bevy_asset")
}

const DEPENDENCY_ATTRIBUTE: &str = "dependency";

#[proc_macro_derive(Asset, attributes(dependency))]
pub fn derive_asset(input: TokenStream) -> TokenStream {
    let ast = parse_macro_input!(input as DeriveInput);
    let bevy_asset_path: Path = bevy_asset_path();

    let struct_name = &ast.ident;
    let (impl_generics, type_generics, where_clause) = &ast.generics.split_for_impl();
    let dependency_visitor = match derive_dependency_visitor_internal(&ast, &bevy_asset_path) {
        Ok(dependency_visitor) => dependency_visitor,
        Err(err) => return err.into_compile_error().into(),
    };

    TokenStream::from(quote! {
        impl #impl_generics #bevy_asset_path::Asset for #struct_name #type_generics #where_clause { }
        #dependency_visitor
    })
}

#[proc_macro_derive(VisitAssetDependencies, attributes(dependency))]
pub fn derive_asset_dependency_visitor(input: TokenStream) -> TokenStream {
    let ast = parse_macro_input!(input as DeriveInput);
    let bevy_asset_path: Path = bevy_asset_path();
    match derive_dependency_visitor_internal(&ast, &bevy_asset_path) {
        Ok(dependency_visitor) => TokenStream::from(dependency_visitor),
        Err(err) => err.into_compile_error().into(),
    }
}

fn derive_dependency_visitor_internal(
    ast: &DeriveInput,
    bevy_asset_path: &Path,
) -> Result<proc_macro2::TokenStream, syn::Error> {
    let struct_name = &ast.ident;
    let (impl_generics, type_generics, where_clause) = &ast.generics.split_for_impl();

    let visit_dep = |to_read| quote!(#bevy_asset_path::VisitAssetDependencies::visit_dependencies(#to_read, visit););
    let is_dep_attribute = |a: &syn::Attribute| a.path().is_ident(DEPENDENCY_ATTRIBUTE);
    let field_has_dep = |f: &syn::Field| f.attrs.iter().any(is_dep_attribute);

    let body = match &ast.data {
        Data::Struct(data_struct) => {
            let fields = data_struct.fields.iter();
            let field_visitors = fields.enumerate().filter(|(_, f)| field_has_dep(f));
            let field_visitors = field_visitors.map(|(i, field)| match &field.ident {
                Some(ident) => visit_dep(quote!(&self.#ident)),
                None => {
                    let index = syn::Index::from(i);
                    visit_dep(quote!(&self.#index))
                }
            });
            Some(quote!( #(#field_visitors)* ))
        }
        Data::Enum(data_enum) => {
            let variant_has_dep = |v: &syn::Variant| v.fields.iter().any(field_has_dep);
            let any_case_required = data_enum.variants.iter().any(variant_has_dep);
            let cases = data_enum.variants.iter().filter(|v| variant_has_dep(v));
            let cases = cases.map(|variant| {
                let ident = &variant.ident;
                let fields = &variant.fields;

                let field_visitors = fields.iter().enumerate().filter(|(_, f)| field_has_dep(f));

                let field_visitors = field_visitors.map(|(i, field)| match &field.ident {
                    Some(ident) => visit_dep(quote!(#ident)),
                    None => {
                        let ident = format_ident!("member{i}");
                        visit_dep(quote!(#ident))
                    }
                });
                let fields = match fields {
                    syn::Fields::Named(fields) => {
                        let named = fields.named.iter().map(|f| f.ident.as_ref());
                        quote!({ #(#named,)* .. })
                    }
                    syn::Fields::Unnamed(fields) => {
                        let named = (0..fields.unnamed.len()).map(|i| format_ident!("member{i}"));
                        quote!( ( #(#named,)* ) )
                    }
                    syn::Fields::Unit => unreachable!("Can't pass filter is_dep_attribute"),
                };
                quote!(Self::#ident #fields => {
                    #(#field_visitors)*
                })
            });

            any_case_required.then(|| quote!(match self { #(#cases)*, _ => {} }))
        }
        Data::Union(_) => {
            return Err(syn::Error::new(
                Span::call_site().into(),
                "Asset derive currently doesn't work on unions",
            ));
        }
    };

    // prevent unused variable warning in case there are no dependencies
    let visit = if body.is_none() {
        quote! { _visit }
    } else {
        quote! { visit }
    };

    Ok(quote! {
        impl #impl_generics #bevy_asset_path::VisitAssetDependencies for #struct_name #type_generics #where_clause {
            fn visit_dependencies(&self, #visit: &mut impl FnMut(#bevy_asset_path::UntypedAssetId)) {
                #body
            }
        }
    })
}

const EMBEDDED_ATTRIBUTE: &str = "embedded";
const LOAD_ATTRIBUTE: &str = "load";
const ASSET_SRC_ATTRIBUTE: &str = "src_path";

#[proc_macro_derive(AssetPack, attributes(embedded, load, src_path))]
pub fn derive_asset_pack(input: TokenStream) -> TokenStream {
    let ast = parse_macro_input!(input as DeriveInput);
    let bevy_app_path: Path = bevy_app_path();
    let bevy_asset_path: Path = bevy_asset_path();
    match derive_asset_pack_internal(&ast, &bevy_app_path, &bevy_asset_path) {
        Ok(tokens) => TokenStream::from(tokens),
        Err(err) => err.into_compile_error().into(),
    }
}

enum FieldLoadMethod {
    Embedded(syn::Ident, syn::Result<syn::LitStr>),
    Load(syn::Ident, syn::Result<syn::Expr>),
    Unknown(syn::Error),
}

impl FieldLoadMethod {
    fn new(field: &Field) -> Self {
        let ident = field.ident.as_ref().unwrap(); //get_struct_fields rejects tuple structs
        field
            .attrs
            .iter()
            .find_map(|attr| {
                if attr.path().is_ident(EMBEDDED_ATTRIBUTE) {
                    Some(Self::Embedded(
                        ident.clone(),
                        attr.parse_args::<syn::LitStr>(),
                    ))
                } else if attr.path().is_ident(LOAD_ATTRIBUTE) {
                    Some(Self::Load(ident.clone(), attr.parse_args::<syn::Expr>()))
                } else {
                    None
                }
            })
            .unwrap_or_else(|| {
                Self::Unknown(syn::Error::new_spanned(
                    field,
                    "missing attribute: use #[embedded(\"...\")] or #[load(\"...\")]",
                ))
            })
    }

    fn error(&self) -> proc_macro2::TokenStream {
        match self {
            FieldLoadMethod::Unknown(err) => err.to_compile_error(),
            _ => proc_macro2::TokenStream::new(),
        }
    }

    fn init(&self, bevy_asset_path: &Path) -> proc_macro2::TokenStream {
        match self {
            FieldLoadMethod::Embedded(_, path) => match path {
                Ok(path) => quote!(#bevy_asset_path::embedded_asset!(app, SRC_PATH, #path);),
                Err(err) => err.to_compile_error(),
            },
            _ => proc_macro2::TokenStream::new(),
        }
    }

    fn load(&self, bevy_asset_path: &Path) -> proc_macro2::TokenStream {
        match self {
            FieldLoadMethod::Embedded(ident, path) => match path {
                Ok(path) => quote!(#ident: {
                    let embedded_path = #bevy_asset_path::embedded_path!(SRC_PATH, #path);
                    let asset_path = #bevy_asset_path::AssetPath::from_path(embedded_path.as_path()).with_source(embedded_source_id);
                    asset_server.load(asset_path)
                },),
                Err(err) => err.to_compile_error(),
            },
            FieldLoadMethod::Load(ident, path) => match path {
                Ok(path) => quote!(#ident: asset_server.load(#path),),
                Err(err) => err.to_compile_error(),
            },
            _ => proc_macro2::TokenStream::new(),
        }
    }
}

fn derive_asset_pack_internal(
    ast: &DeriveInput,
    bevy_app_path: &Path,
    bevy_asset_path: &Path,
) -> syn::Result<proc_macro2::TokenStream> {
    let struct_name = &ast.ident;
    let (impl_generics, type_generics, where_clause) = &ast.generics.split_for_impl();

    let src_path = ast
        .attrs
        .iter()
        .find_map(|attr| match &attr.meta {
            syn::Meta::NameValue(syn::MetaNameValue { path, value, .. }) => {
                if path.is_ident(ASSET_SRC_ATTRIBUTE) {
                    Some(get_lit_str(Symbol("src_path"), value).cloned())
                } else {
                    None
                }
            }
            _ => None,
        })
        .unwrap_or_else(|| Ok(syn::LitStr::new("src", proc_macro2::Span::call_site())));

    let src_path_tokens = match src_path {
        Ok(path) => path.into_token_stream(),
        Err(err) => err.to_compile_error(),
    };

    let fields = get_struct_fields(&ast.data)?;

    let load_methods = fields.iter().map(FieldLoadMethod::new).collect::<Vec<_>>();
    let error = load_methods.iter().map(FieldLoadMethod::error);
    let init = load_methods.iter().map(|field| field.init(bevy_asset_path));
    let load = load_methods.iter().map(|field| field.load(bevy_asset_path));

    Ok(quote! {
        #(#error)*

        impl #impl_generics #bevy_asset_path::io::pack::AssetPack for #struct_name #type_generics #where_clause {
            fn init(app: &mut #bevy_app_path::App) {
                const SRC_PATH: &str = #src_path_tokens;
                #(#init)*
            }

            fn load(asset_server: &#bevy_asset_path::AssetServer) -> Self {
                const SRC_PATH: &str = #src_path_tokens;
                let embedded_source_id = #bevy_asset_path::io::AssetSourceId::from("embedded");
                Self {
                    #(#load)*
                }
            }
        }
    })
}
