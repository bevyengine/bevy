use crate::{
    help::snake_to_pascal_case,
    modules::{get_modules, get_path},
};
use proc_macro::TokenStream;
use proc_macro2::Span;
use quote::quote;
use syn::{
    parse::ParseStream, parse_macro_input, parse_quote, parse_str, punctuated::Punctuated,
    token::Comma, Data, DataStruct, DeriveInput, Field, Fields, Ident, Type,
};

// TODO: Make a version for assets ...

pub fn derive_animated_properties(input: TokenStream) -> TokenStream {
    let ast = parse_macro_input!(input as DeriveInput);

    let fields = match &ast.data {
        Data::Struct(DataStruct {
            fields: Fields::Named(fields),
            ..
        }) => &fields.named,
        _ => panic!("Expected a struct with named fields."),
    };

    let mut expanded: Vec<Vec<Field>> = vec![];
    expanded.resize_with(fields.len(), || vec![]);

    // Filter fields
    let fields = fields
        .iter()
        .enumerate()
        .filter(|(field_index, field)| {
            field
                .attrs
                .iter()
                .find(|a| *a.path.get_ident().as_ref().unwrap() == "animated")
                .map_or_else(
                    || true,
                    |a| {
                        syn::custom_keyword!(ignore);
                        syn::custom_keyword!(expand);
                        a.parse_args_with(|input: ParseStream| {
                            if input.parse::<Option<ignore>>()?.is_some() {
                                Ok(false)
                            } else if input.parse::<Option<expand>>()?.is_some() {
                                let content;
                                //syn::parenthesized!(content in input);
                                syn::braced!(content in input);
                                let fields: Punctuated<Field, Comma> =
                                    content.parse_terminated(Field::parse_named)?;
                                expanded[*field_index].extend(fields.iter().cloned());
                                Ok(true)
                            } else {
                                Ok(true)
                            }
                        })
                        .expect("Invalid 'animated' attribute format.")
                    },
                )
        })
        .map(|(_, field)| field)
        .collect::<Vec<&Field>>();

    let modules = get_modules(&ast.attrs);
    let bevy_animation = get_path(&modules.bevy_animation);

    let struct_name = &ast.ident;
    let root_ident = Ident::new(&format!("{}Properties", struct_name), Span::call_site());

    let mut field_ident = vec![];
    let mut field_inner: Vec<Type> = vec![];
    let mut field_type = vec![];
    let mut field_path = vec![];
    let mut field_inner_impls = vec![];

    fields
        .iter()
        .zip(expanded.iter())
        .for_each(|(field, extended_fields)| {
            let ident = field.ident.as_ref().unwrap();
            let ty = &field.ty;
            field_ident.push(ident);
            field_type.push(ty);
            field_path.push(format!("{}.{}", struct_name, ident));

            if extended_fields.len() == 0 {
                field_inner_impls.push(quote! {});
                field_inner.push(parse_quote!(()));
            } else {
                // TODO case style conversion
                let type_inner: Type = parse_str(&format!(
                    "{}{}Properties",
                    struct_name,
                    snake_to_pascal_case(&ident.to_string())
                ))
                .unwrap();

                let mut field_ident = vec![];
                let mut field_type = vec![];
                let mut field_path = vec![];
                extended_fields.iter().for_each(|field| {
                    let inner = field.ident.as_ref().unwrap();
                    field_ident.push(inner);
                    field_type.push(&field.ty);
                    field_path.push(format!("{}.{}.{}", struct_name, ident, inner));
                });

                field_inner_impls.push(quote! {
                    pub struct #type_inner;
                    impl #type_inner {
                        #(
                            pub fn #field_ident(&self) -> #bevy_animation::Prop<#field_type> {
                                #bevy_animation::Prop::borrowed(#field_path)
                            }
                        )*
                    }
                    impl std::ops::Deref for #bevy_animation::Prop<#ty, #type_inner> {
                        type Target = #type_inner;

                        #[inline(always)]
                        fn deref(&self) -> &Self::Target {
                            &#type_inner
                        }
                    }
                });
                field_inner.push(type_inner);
            }
        });

    let generics = ast.generics;
    let (impl_generics, ty_generics, _where_clause) = generics.split_for_impl();

    TokenStream::from(quote! {
        #(#field_inner_impls)*

        pub struct #root_ident;

        impl #root_ident {
            #(
                pub fn #field_ident(&self) -> #bevy_animation::Prop<#field_type, #field_inner> {
                    #bevy_animation::Prop::borrowed(#field_path)
                }
            )*
        }

        impl #impl_generics #bevy_animation::AnimatedProperties for #struct_name #ty_generics {
            type Props = #root_ident;

            #[inline(always)]
            fn props() -> Self::Props {
                #root_ident
            }
        }
    })
}
