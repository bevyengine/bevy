use crate::{
    help::snake_to_pascal_case,
    modules::{get_modules, get_path},
};
use proc_macro::TokenStream;
use proc_macro2::{Span, TokenStream as TokenStream2};
use quote::{quote, ToTokens, TokenStreamExt};
use syn::{
    parse::ParseStream, parse_macro_input, parse_quote, parse_str, punctuated::Punctuated,
    token::Comma, Data, DataStruct, DeriveInput, Field, Fields, Ident, Path, Type,
};

// pub struct PropDerive<'a> {
//     pub namespace: &'a Path,
//     pub root: &'a Ident,
//     pub name: Type,
//     pub field: Vec<Ident>,
//     pub ty: Vec<Type>,
//     pub index: Vec<usize>,
//     pub nested: Vec<Option<PropDerive<'a>>>,
// }

// impl<'a> ToTokens for PropDerive<'a> {
//     fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
//         for (n, ty) in self
//             .nested
//             .iter()
//             .zip(self.ty.iter())
//             .filter_map(|(n, ty)| n.as_ref().map(|n| (n, ty)))
//         {
//             n.to_tokens(tokens);

//             let namespace = &n.namespace;
//             let name = &n.name;

//             tokens.append_all(quote! {
//                 impl std::ops::Deref for #namespace::Prop<#ty, #name> {
//                     type Target = #name;

//                     #[inline(always)]
//                     fn deref(&self) -> &Self::Target {
//                         &#name
//                     }
//                 }
//             });
//         }

//         let unit: Type = parse_str("()").unwrap();
//         let namespace = &self.namespace;
//         let root = &self.root;
//         let name = &self.name;
//         let field = &self.field;
//         let ty = &self.ty;
//         let index = &self.index;
//         let nested = self.nested.iter().map(|n| n.map_or(&unit, |n| n.name));
//         tokens.append_all(quote! {
//             pub struct #name;
//             impl #name {
//                 #(pub const fn #field(&self) -> #namespace::Prop<#ty, #nested> {
//                     #namespace::Prop::borrowed( #root::PROPERTIES[#index] )
//                 })*
//             }
//         });
//     }
// }

pub fn query_prop_nested(
    namespace: &Path,
    struct_name: &Ident,
    nested: &Type,
    root_ty: &Type,
    field: &[&Ident],
    ty: &[&Type],
    index: &[usize],
) -> TokenStream2 {
    quote! {
        pub struct #nested;
        impl #nested {
            #(
                pub const fn #field(&self) -> #namespace::Prop<#ty> {
                    #namespace::Prop::borrowed( #struct_name::PROPERTIES[#index] )
                }
            )*
        }
        impl std::ops::Deref for #namespace::Prop<#root_ty, #nested> {
            type Target = #nested;

            #[inline(always)]
            fn deref(&self) -> &Self::Target {
                &#nested
            }
        }
    }
}

pub fn query_prop(
    namespace: &Path,
    struct_name: &Ident,
    root: &Type,
    field: &[&Ident],
    ty: &[&Type],
    nested: &[&Type],
    index: &[usize],
) -> TokenStream2 {
    quote! {
        pub struct #root;
        impl #root {
            #(
                pub const fn #field(&self) -> #namespace::Prop<#ty, #nested> {
                    #namespace::Prop::borrowed( #struct_name::PROPERTIES[#index] )
                }
            )*
        }
    }
}

pub fn derive_animated_properties_for_component(input: TokenStream) -> TokenStream {
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
                                syn::parenthesized!(content in input);
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
    let mut available_properties = vec![];

    let mut field_ident = vec![];
    let mut field_inner: Vec<Type> = vec![];
    let mut field_type = vec![];
    let mut field_property_index = vec![];
    let mut field_inner_impls = vec![];

    fields
        .iter()
        .zip(expanded.iter())
        .for_each(|(field, extended_fields)| {
            let ident = field.ident.as_ref().unwrap();
            let ty = &field.ty;
            field_ident.push(ident);
            field_type.push(ty);

            field_property_index.push(available_properties.len());
            available_properties.push(format!("{}.{}", struct_name, ident));

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
                let mut field_property_index = vec![];
                extended_fields.iter().for_each(|field| {
                    let inner = field.ident.as_ref().unwrap();
                    field_ident.push(inner);
                    field_type.push(&field.ty);
                    field_property_index.push(available_properties.len());
                    available_properties.push(format!("{}.{}.{}", struct_name, ident, inner));
                });

                field_inner_impls.push(quote! {
                    pub struct #type_inner;
                    impl #type_inner {
                        #(
                            pub const fn #field_ident(&self) -> #bevy_animation::Prop<#field_type> {
                                #bevy_animation::Prop::borrowed( #struct_name::PROPERTIES[#field_property_index] )
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
                pub const fn #field_ident(&self) -> #bevy_animation::Prop<#field_type, #field_inner> {
                    #bevy_animation::Prop::borrowed( #struct_name::PROPERTIES[#field_property_index] )
                }
            )*
        }

        impl #impl_generics #bevy_animation::AnimatedProperties for #struct_name #ty_generics {
            type Props = #root_ident;

            const PROPERTIES: &'static [&'static str] = &[ #( #available_properties, )* ];

            #[inline(always)]
            fn props() -> Self::Props {
                #root_ident
            }
        }
    })
}

pub fn derive_animated_properties_for_asset(input: TokenStream) -> TokenStream {
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
                                syn::parenthesized!(content in input);
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
    let fields_struct = Ident::new(
        &format!("{}FieldsProperties", struct_name),
        Span::call_site(),
    );

    let mut available_properties = vec![];
    available_properties.push(format!("Handle<{}>", struct_name));

    let mut field_ident = vec![];
    let mut field_inner: Vec<Type> = vec![];
    let mut field_type = vec![];
    let mut field_property_index = vec![];
    let mut field_inner_impls = vec![];

    fields
        .iter()
        .zip(expanded.iter())
        .for_each(|(field, extended_fields)| {
            let ident = field.ident.as_ref().unwrap();
            let ty = &field.ty;
            field_ident.push(ident);
            field_type.push(ty);
            field_property_index.push(available_properties.len());
            available_properties.push(format!("Handle<{}>.{}", struct_name, ident));

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
                let mut field_property_index = vec![];
                extended_fields.iter().for_each(|field| {
                    let inner = field.ident.as_ref().unwrap();
                    field_ident.push(inner);
                    field_type.push(&field.ty);
                    field_property_index.push(available_properties.len());
                    available_properties
                        .push(format!("Handle<{}>.{}.{}", struct_name, ident, inner));
                });

                field_inner_impls.push(quote! {
                    pub struct #type_inner;
                    impl #type_inner {
                        #(
                            pub const fn #field_ident(&self) -> #bevy_animation::Prop<#field_type> {
                                #bevy_animation::Prop::borrowed(#struct_name::PROPERTIES[#field_property_index])
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

        pub struct #fields_struct;
        impl #fields_struct {
            #(
                pub const fn #field_ident(&self) -> #bevy_animation::Prop<#field_type, #field_inner> {
                    #bevy_animation::Prop::borrowed(#struct_name::PROPERTIES[#field_property_index])
                }
            )*
        }

        pub struct #root_ident;
        impl #root_ident {
            pub const fn handle(&self) -> #bevy_animation::Prop<Handle<#struct_name>> {
                #bevy_animation::Prop::borrowed(#struct_name::PROPERTIES[0usize])
            }
            pub const fn fields(&self) -> #fields_struct {
                #fields_struct
            }
        }

        impl #impl_generics #bevy_animation::AnimatedProperties for #struct_name #ty_generics {
            type Props = #root_ident;

            const PROPERTIES: &'static [&'static str] = &[ #( #available_properties, )* ];

            #[inline(always)]
            fn props() -> Self::Props {
                #root_ident
            }
        }
    })
}
