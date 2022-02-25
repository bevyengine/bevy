use proc_macro::TokenStream;
use quote::quote;
use syn::{Field, Generics, Ident, Index, Member, Path};

pub fn impl_struct(
    struct_name: &Ident,
    generics: &Generics,
    bevy_reflect_path: &Path,
    active_fields: &[(&Field, usize)],
    ignored_fields: &[(&Field, usize)],
) -> TokenStream {
    let field_names = active_fields
        .iter()
        .map(|(field, index)| {
            field
                .ident
                .as_ref()
                .map(|i| i.to_string())
                .unwrap_or_else(|| index.to_string())
        })
        .collect::<Vec<String>>();
    let field_idents = active_fields
        .iter()
        .map(|(field, index)| {
            field
                .ident
                .as_ref()
                .map(|ident| Member::Named(ident.clone()))
                .unwrap_or_else(|| Member::Unnamed(Index::from(*index)))
        })
        .collect::<Vec<_>>();

    let field_types = active_fields
        .iter()
        .map(|(field, _index)| field.ty.clone())
        .collect::<Vec<_>>();
    let field_count = active_fields.len();
    let ignored_field_idents = ignored_fields
        .iter()
        .map(|(field, index)| {
            field
                .ident
                .as_ref()
                .map(|ident| Member::Named(ident.clone()))
                .unwrap_or_else(|| Member::Unnamed(Index::from(*index)))
        })
        .collect::<Vec<_>>();

    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();

    // Add FromReflect bound for each active field
    let mut where_from_reflect_clause = if where_clause.is_some() {
        quote! {#where_clause}
    } else if field_count > 0 {
        quote! {where}
    } else {
        quote! {}
    };
    where_from_reflect_clause.extend(quote! {
        #(#field_types: #bevy_reflect_path::FromReflect,)*
    });

    TokenStream::from(quote! {
        impl #impl_generics #bevy_reflect_path::FromReflect for #struct_name #ty_generics #where_from_reflect_clause
        {
            fn from_reflect(reflect: &dyn #bevy_reflect_path::Reflect) -> Option<Self> {
                use #bevy_reflect_path::Struct;
                if let #bevy_reflect_path::ReflectRef::Struct(ref_struct) = reflect.reflect_ref() {
                    Some(
                        Self{
                            #(#field_idents: {
                                <#field_types as #bevy_reflect_path::FromReflect>::from_reflect(ref_struct.field(#field_names)?)?
                            },)*
                            #(#ignored_field_idents: Default::default(),)*
                        }
                    )
                } else {
                    None
                }
            }
        }
    })
}

pub fn impl_tuple_struct(
    struct_name: &Ident,
    generics: &Generics,
    bevy_reflect_path: &Path,
    active_fields: &[(&Field, usize)],
    ignored_fields: &[(&Field, usize)],
) -> TokenStream {
    let field_idents = active_fields
        .iter()
        .map(|(_field, index)| Member::Unnamed(Index::from(*index)))
        .collect::<Vec<_>>();
    let field_types = active_fields
        .iter()
        .map(|(field, _index)| field.ty.clone())
        .collect::<Vec<_>>();
    let field_count = active_fields.len();
    let field_indices = (0..field_count).collect::<Vec<usize>>();
    let ignored_field_idents = ignored_fields
        .iter()
        .map(|(_field, index)| Member::Unnamed(Index::from(*index)))
        .collect::<Vec<_>>();

    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();
    // Add FromReflect bound for each active field
    let mut where_from_reflect_clause = if where_clause.is_some() {
        quote! {#where_clause}
    } else if field_count > 0 {
        quote! {where}
    } else {
        quote! {}
    };
    where_from_reflect_clause.extend(quote! {
        #(#field_types: #bevy_reflect_path::FromReflect,)*
    });

    TokenStream::from(quote! {
        impl #impl_generics #bevy_reflect_path::FromReflect for #struct_name #ty_generics #where_from_reflect_clause
        {
            fn from_reflect(reflect: &dyn #bevy_reflect_path::Reflect) -> Option<Self> {
                use #bevy_reflect_path::TupleStruct;
                if let #bevy_reflect_path::ReflectRef::TupleStruct(ref_tuple_struct) = reflect.reflect_ref() {
                    Some(
                        Self{
                            #(#field_idents:
                                <#field_types as #bevy_reflect_path::FromReflect>::from_reflect(ref_tuple_struct.field(#field_indices)?)?
                            ,)*
                            #(#ignored_field_idents: Default::default(),)*
                        }
                    )
                } else {
                    None
                }
            }
        }
    })
}

pub fn impl_value(type_name: &Ident, generics: &Generics, bevy_reflect_path: &Path) -> TokenStream {
    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();
    TokenStream::from(quote! {
        impl #impl_generics #bevy_reflect_path::FromReflect for #type_name #ty_generics #where_clause  {
            fn from_reflect(reflect: &dyn #bevy_reflect_path::Reflect) -> Option<Self> {
                Some(reflect.any().downcast_ref::<#type_name #ty_generics>()?.clone())
            }
        }
    })
}
