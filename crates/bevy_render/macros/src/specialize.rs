use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::{format_ident, quote, ToTokens};
use syn::{
    parse::Parse, parse_macro_input, parse_quote, punctuated::Punctuated, spanned::Spanned, Data,
    DataStruct, DeriveInput, Expr, Index, Member, Meta, MetaList, MetaNameValue, Path, Stmt, Token,
    Type,
};

const KEY_ATTR_IDENT: &str = "key";

pub fn derive_specialize(input: TokenStream, target_path: Path, derive_name: &str) -> TokenStream {
    let bevy_render_path: Path = crate::bevy_render_path();
    let specialize_path = {
        let mut path = bevy_render_path.clone();
        path.segments.push(format_ident!("render_resource").into());
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
        return syn::Error::new(
            error_span,
            format!("#[derive({derive_name})]` only supports structs"),
        )
        .into_compile_error()
        .into();
    };

    let mut key_elems: Punctuated<Type, Token![,]> = Punctuated::new();
    let mut sub_specializers: Vec<(Type, Member, Expr)> = Vec::new();
    let mut single_index = 0;

    for (index, field) in fields.iter().enumerate() {
        let field_ty = field.ty.clone();
        let field_member = field.ident.clone().map_or(
            Member::Unnamed(Index {
                index: index as u32,
                span: field.span(),
            }),
            Member::Named,
        );
        let key_member = Member::Unnamed(Index {
            index: key_elems.len() as u32,
            span: field.span(),
        });

        let mut key_expr: Expr = parse_quote!(key.#key_member);
        let mut use_key_field = true;
        for attr in &field.attrs {
            if let Meta::List(MetaList { path, tokens, .. }) = &attr.meta {
                if path.is_ident(&KEY_ATTR_IDENT) {
                    let owned_tokens = tokens.clone().into();
                    key_expr = parse_macro_input!(owned_tokens as Expr);
                    use_key_field = false;
                }
            }
        }

        if use_key_field {
            single_index = index;
            key_elems
                .push(parse_quote!(<#field_ty as #specialize_path::Specialize<#target_path>>::Key));
        }

        sub_specializers.push((field_ty, field_member, key_expr));
    }

    if key_elems.len() == 1 {
        sub_specializers[single_index].2 = parse_quote!(key);
    }

    let sub_specializers = sub_specializers.into_iter().map(|(field_ty, field_member, key_expr)| {
        parse_quote!(<#field_ty as #specialize_path::Specialize<#target_path>>::specialize(&self.#field_member, #key_expr, descriptor);)
    }).collect::<Vec<Stmt>>();

    TokenStream::from(quote! {
        impl #impl_generics #specialize_path::Specialize<#target_path> for #struct_name #type_generics #where_clause {
            type Key = (#key_elems);

            fn specialize(&self, key: Self::Key, descriptor: &mut <#target_path as #specialize_path::SpecializeTarget>::Descriptor) {
                #(#sub_specializers)*
            }
        }
    })
}
