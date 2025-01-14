use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::{format_ident, quote, ToTokens};
use syn::{
    parse_macro_input, punctuated::Punctuated, spanned::Spanned, Data, DataStruct, DeriveInput,
    Index, Member, Meta, MetaNameValue, Path, Token, Type,
};

const KEY_ATTR_IDENT: &str = "key";

pub fn derive_specialize(input: TokenStream, target_path: Path) -> TokenStream {
    let bevy_render_path: Path = crate::bevy_render_path();
    let specialize_path = {
        let mut path = bevy_render_path.clone();
        path.segments.push(format_ident!("render_resource").into());
        path.segments.push(format_ident!("specialize").into());
        path
    };

    let ast = parse_macro_input!(input as DeriveInput);
    let struct_name = &ast.ident;
    let (impl_generics, type_generics, where_clause) = &ast.generics.split_for_impl();

    let Data::Struct(DataStruct { fields, .. }) = &ast.data else {
        let error_span = match ast.data {
            Data::Struct(_) => unreachable!(),
            Data::Enum(data_enum) => data_enum.enum_token.span(),
            Data::Union(data_union) => data_union.union_token.span(),
        };
        return syn::Error::new(error_span, "#[derive(Specialize)]` only supports structs") //TODO: proper name here
            .into_compile_error()
            .into();
    };

    let mut key_elems: Punctuated<Type, Token![,]> = Punctuated::new();
    let mut sub_specializers: Vec<TokenStream2> = Vec::new();

    for (index, field) in fields.iter().enumerate() {
        let field_ty = field.ty.clone();
        let field_member = field.ident.clone().map_or(
            Member::Unnamed(Index {
                index: index as u32,
                span: field.span(),
            }),
            Member::Named,
        );

        let mut key_expr = quote!(key.#{key_elems.len() as u32});
        let mut use_key_field = true;
        for attr in &field.attrs {
            if let Meta::NameValue(MetaNameValue { path, value, .. }) = &attr.meta {
                if path.is_ident(&KEY_ATTR_IDENT) {
                    key_expr = value.to_token_stream();
                    use_key_field = false;
                }
            }
        }

        if use_key_field {
            key_elems.push(field_ty.clone());
        }

        sub_specializers.push(quote!(<#field_ty as #specialize_path::Specialize<#target_path>>::specialize(&self.#field_member, #key_expr, descriptor);));
    }

    TokenStream::from(quote! {
        impl #impl_generics #specialize_path::Specialize<#target_path> for #struct_name #type_generics #where_clause {
            type Key = (#key_elems);

            fn specialize(&self, key: Self::Key, descriptor: &mut <#target_path as #specialize_path::SpecializeTarget>::Descriptor) {
                #(#sub_specializers)*
            }
        }
    })
}
