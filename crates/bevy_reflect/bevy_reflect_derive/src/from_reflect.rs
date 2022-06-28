use crate::container_attributes::REFLECT_DEFAULT;
use crate::field_attributes::DefaultBehavior;
use crate::ReflectDeriveData;
use proc_macro::TokenStream;
use proc_macro2::Span;
use quote::quote;
use syn::{Field, Generics, Ident, Index, Lit, LitInt, LitStr, Member, Path};

/// Implements `FromReflect` for the given struct
pub(crate) fn impl_struct(derive_data: &ReflectDeriveData) -> TokenStream {
    impl_struct_internal(derive_data, false)
}

/// Implements `FromReflect` for the given tuple struct
pub(crate) fn impl_tuple_struct(derive_data: &ReflectDeriveData) -> TokenStream {
    impl_struct_internal(derive_data, true)
}

/// Implements `FromReflect` for the given value type
pub(crate) fn impl_value(
    type_name: &Ident,
    generics: &Generics,
    bevy_reflect_path: &Path,
) -> TokenStream {
    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();
    TokenStream::from(quote! {
        impl #impl_generics #bevy_reflect_path::FromReflect for #type_name #ty_generics #where_clause  {
            fn from_reflect(reflect: &dyn #bevy_reflect_path::Reflect) -> Option<Self> {
                Some(reflect.as_any().downcast_ref::<#type_name #ty_generics>()?.clone())
            }
        }
    })
}

/// Container for a struct's members (field name or index) and their
/// corresponding values.
struct MemberValuePair(Vec<Member>, Vec<proc_macro2::TokenStream>);

impl MemberValuePair {
    pub fn new(items: (Vec<Member>, Vec<proc_macro2::TokenStream>)) -> Self {
        Self(items.0, items.1)
    }
}

fn impl_struct_internal(derive_data: &ReflectDeriveData, is_tuple: bool) -> TokenStream {
    let struct_name = derive_data.type_name();
    let generics = derive_data.generics();
    let bevy_reflect_path = derive_data.bevy_reflect_path();

    let ref_struct = Ident::new("__ref_struct", Span::call_site());
    let ref_struct_type = if is_tuple {
        Ident::new("TupleStruct", Span::call_site())
    } else {
        Ident::new("Struct", Span::call_site())
    };

    let field_types = derive_data.active_types();
    let MemberValuePair(active_members, active_values) =
        get_active_fields(derive_data, &ref_struct, &ref_struct_type, is_tuple);

    let constructor = if derive_data.traits().contains(REFLECT_DEFAULT) {
        quote!(
            let mut __this = Self::default();
            #(
                if let Some(__field) = #active_values() {
                    // Iff field exists -> use its value
                    __this.#active_members = __field;
                }
            )*
            Some(__this)
        )
    } else {
        let MemberValuePair(ignored_members, ignored_values) =
            get_ignored_fields(derive_data, is_tuple);

        quote!(
            Some(
                Self {
                    #(#active_members: #active_values()?,)*
                    #(#ignored_members: #ignored_values,)*
                }
            )
        )
    };

    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();

    // Add FromReflect bound for each active field
    let mut where_from_reflect_clause = if where_clause.is_some() {
        quote! {#where_clause}
    } else if !active_members.is_empty() {
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
                if let #bevy_reflect_path::ReflectRef::#ref_struct_type(#ref_struct) = reflect.reflect_ref() {
                    #constructor
                } else {
                    None
                }
            }
        }
    })
}

/// Get the collection of ignored field definitions
///
/// Each value of the `MemberValuePair` is a token stream that generates a
/// a default value for the ignored field.
fn get_ignored_fields(derive_data: &ReflectDeriveData, is_tuple: bool) -> MemberValuePair {
    MemberValuePair::new(
        derive_data
            .ignored_fields()
            .map(|field| {
                let member = get_ident(field.data, field.index, is_tuple);

                let value = match &field.attrs.default {
                    DefaultBehavior::Func(path) => quote! {#path()},
                    _ => quote! {Default::default()},
                };

                (member, value)
            })
            .unzip(),
    )
}

/// Get the collection of active field definitions.
///
/// Each value of the `MemberValuePair` is a token stream that generates a
/// closure of type `fn() -> Option<T>` where `T` is that field's type.
fn get_active_fields(
    derive_data: &ReflectDeriveData,
    dyn_struct_name: &Ident,
    struct_type: &Ident,
    is_tuple: bool,
) -> MemberValuePair {
    let bevy_reflect_path = derive_data.bevy_reflect_path();

    MemberValuePair::new(
        derive_data
            .active_fields()
            .map(|field| {
                let member = get_ident(field.data, field.index, is_tuple);
                let accessor = get_field_accessor(field.data, field.index, is_tuple);
                let ty = field.data.ty.clone();

                let get_field = quote! {
                    #bevy_reflect_path::#struct_type::field(#dyn_struct_name, #accessor)
                };

                let value = match &field.attrs.default {
                    DefaultBehavior::Func(path) => quote! {
                        (||
                            if let Some(field) = #get_field {
                                <#ty as #bevy_reflect_path::FromReflect>::from_reflect(field)
                            } else {
                                Some(#path())
                            }
                        )
                    },
                    DefaultBehavior::Default => quote! {
                        (||
                            if let Some(field) = #get_field {
                                <#ty as #bevy_reflect_path::FromReflect>::from_reflect(field)
                            } else {
                                Some(Default::default())
                            }
                        )
                    },
                    DefaultBehavior::Required => quote! {
                        (|| <#ty as #bevy_reflect_path::FromReflect>::from_reflect(#get_field?))
                    },
                };

                (member, value)
            })
            .unzip(),
    )
}

/// Returns the member for a given field of a struct or tuple struct.
fn get_ident(field: &Field, index: usize, is_tuple: bool) -> Member {
    if is_tuple {
        Member::Unnamed(Index::from(index))
    } else {
        field
            .ident
            .as_ref()
            .map(|ident| Member::Named(ident.clone()))
            .unwrap_or_else(|| Member::Unnamed(Index::from(index)))
    }
}

/// Returns the accessor for a given field of a struct or tuple struct.
///
/// This differs from a member in that it needs to be a number for tuple structs
/// and a string for standard structs.
fn get_field_accessor(field: &Field, index: usize, is_tuple: bool) -> Lit {
    if is_tuple {
        Lit::Int(LitInt::new(&index.to_string(), Span::call_site()))
    } else {
        field
            .ident
            .as_ref()
            .map(|ident| Lit::Str(LitStr::new(&ident.to_string(), Span::call_site())))
            .unwrap_or_else(|| Lit::Str(LitStr::new(&index.to_string(), Span::call_site())))
    }
}
