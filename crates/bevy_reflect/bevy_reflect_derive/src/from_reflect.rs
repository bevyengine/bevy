use crate::container_attributes::REFLECT_DEFAULT;
use crate::derive_data::ReflectEnum;
use crate::enum_utility::{get_variant_constructors, EnumVariantConstructors};
use crate::field_attributes::DefaultBehavior;
use crate::fq_std::{FQAny, FQBox, FQClone, FQCow, FQDefault, FQResult};
use crate::{ReflectMeta, ReflectStruct};
use proc_macro::TokenStream;
use proc_macro2::Span;
use quote::{quote, ToTokens};
use syn::{Field, Ident, Index, Lit, LitInt, LitStr, Member};

/// Implements `FromReflect` for the given struct
pub(crate) fn impl_struct(reflect_struct: &ReflectStruct) -> TokenStream {
    impl_struct_internal(reflect_struct, false)
}

/// Implements `FromReflect` for the given tuple struct
pub(crate) fn impl_tuple_struct(reflect_struct: &ReflectStruct) -> TokenStream {
    impl_struct_internal(reflect_struct, true)
}

/// Implements `FromReflect` for the given value type
pub(crate) fn impl_value(meta: &ReflectMeta) -> TokenStream {
    let type_name = meta.type_name();
    let bevy_reflect_path = meta.bevy_reflect_path();
    let (impl_generics, ty_generics, where_clause) = meta.generics().split_for_impl();
    TokenStream::from(quote! {
        impl #impl_generics #bevy_reflect_path::FromReflect for #type_name #ty_generics #where_clause  {
            fn from_reflect(reflect: &dyn #bevy_reflect_path::Reflect) -> #FQResult<Self, #bevy_reflect_path::FromReflectError> {
                #FQResult::Ok(#FQClone::clone(
                    <dyn #FQAny>::downcast_ref::<#type_name #ty_generics>(<dyn #bevy_reflect_path::Reflect>::as_any(reflect))
                    .ok_or_else(|| #bevy_reflect_path::FromReflectError::InvalidType {
                        from_type: #bevy_reflect_path::Reflect::get_type_info(reflect),
                        from_kind: #bevy_reflect_path::Reflect::reflect_kind(reflect),
                        to_type: <Self as #bevy_reflect_path::Typed>::type_info(),
                    })?
                ))
            }
        }
    })
}

/// Implements `FromReflect` for the given enum type
pub(crate) fn impl_enum(reflect_enum: &ReflectEnum) -> TokenStream {
    let fqresult = FQResult.into_token_stream();
    let fqbox = FQBox.into_token_stream();
    let fqcow = FQCow.into_token_stream();

    let type_name = reflect_enum.meta().type_name();
    let bevy_reflect_path = reflect_enum.meta().bevy_reflect_path();

    let ref_value = Ident::new("__param0", Span::call_site());
    let EnumVariantConstructors {
        variant_names,
        variant_constructors,
    } = get_variant_constructors(reflect_enum, &ref_value, false);

    let (impl_generics, ty_generics, where_clause) =
        reflect_enum.meta().generics().split_for_impl();
    TokenStream::from(quote! {
        impl #impl_generics #bevy_reflect_path::FromReflect for #type_name #ty_generics #where_clause  {
            fn from_reflect(#ref_value: &dyn #bevy_reflect_path::Reflect) -> #FQResult<Self, #bevy_reflect_path::FromReflectError> {
                if let #bevy_reflect_path::ReflectRef::Enum(#ref_value) = #bevy_reflect_path::Reflect::reflect_ref(#ref_value) {
                    match #bevy_reflect_path::Enum::variant_name(#ref_value) {
                        #(#variant_names => (|| #fqresult::Ok(#variant_constructors))().map_err(|err| {
                            #bevy_reflect_path::FromReflectError::VariantError {
                                from_type: #bevy_reflect_path::Reflect::get_type_info(#ref_value),
                                from_kind: #bevy_reflect_path::Reflect::reflect_kind(#ref_value),
                                to_type: <Self as #bevy_reflect_path::Typed>::type_info(),
                                variant: #fqcow::Borrowed(#variant_names),
                                source: #fqbox::new(err),
                            }
                        }),)*
                        name => #FQResult::Err(#bevy_reflect_path::FromReflectError::MissingVariant {
                            from_type: #bevy_reflect_path::Reflect::get_type_info(#ref_value),
                            from_kind: #bevy_reflect_path::Reflect::reflect_kind(#ref_value),
                            to_type: <Self as #bevy_reflect_path::Typed>::type_info(),
                            variant: #fqcow::Owned(name.to_string()),
                        }),
                    }
                } else {
                    #FQResult::Err(#bevy_reflect_path::FromReflectError::InvalidType {
                        from_type: #bevy_reflect_path::Reflect::get_type_info(#ref_value),
                        from_kind: #bevy_reflect_path::Reflect::reflect_kind(#ref_value),
                        to_type: <Self as #bevy_reflect_path::Typed>::type_info(),
                    })
                }
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

fn impl_struct_internal(reflect_struct: &ReflectStruct, is_tuple: bool) -> TokenStream {
    let fqresult = FQResult.into_token_stream();

    let struct_name = reflect_struct.meta().type_name();
    let generics = reflect_struct.meta().generics();
    let bevy_reflect_path = reflect_struct.meta().bevy_reflect_path();

    let ref_struct = Ident::new("__ref_struct", Span::call_site());
    let ref_struct_type = if is_tuple {
        Ident::new("TupleStruct", Span::call_site())
    } else {
        Ident::new("Struct", Span::call_site())
    };

    let field_types = reflect_struct.active_types();
    let MemberValuePair(active_members, active_values) =
        get_active_fields(reflect_struct, &ref_struct, &ref_struct_type, is_tuple);

    let error = quote!(#bevy_reflect_path::FromReflectError::InvalidType {
        from_type: #bevy_reflect_path::Reflect::get_type_info(reflect),
        from_kind: #bevy_reflect_path::Reflect::reflect_kind(reflect),
        to_type: <Self as #bevy_reflect_path::Typed>::type_info(),
    });

    let constructor = if reflect_struct.meta().traits().contains(REFLECT_DEFAULT) {
        quote!(
            let mut __this: Self = #FQDefault::default();
            #(
                if let #fqresult::Ok(__field) = #active_values() {
                    // Iff field exists -> use its value
                    __this.#active_members = __field;
                }
            )*
            #FQResult::Ok(__this)
        )
    } else {
        let MemberValuePair(ignored_members, ignored_values) =
            get_ignored_fields(reflect_struct, is_tuple);

        quote!(
            #FQResult::Ok(
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
            fn from_reflect(reflect: &dyn #bevy_reflect_path::Reflect) -> #FQResult<Self, #bevy_reflect_path::FromReflectError> {
                if let #bevy_reflect_path::ReflectRef::#ref_struct_type(#ref_struct) = #bevy_reflect_path::Reflect::reflect_ref(reflect) {
                    #constructor
                } else {
                    #FQResult::Err(#error)
                }
            }
        }
    })
}

/// Get the collection of ignored field definitions
///
/// Each value of the `MemberValuePair` is a token stream that generates a
/// a default value for the ignored field.
fn get_ignored_fields(reflect_struct: &ReflectStruct, is_tuple: bool) -> MemberValuePair {
    MemberValuePair::new(
        reflect_struct
            .ignored_fields()
            .map(|field| {
                let member = get_ident(field.data, field.index, is_tuple);

                let value = match &field.attrs.default {
                    DefaultBehavior::Func(path) => quote! {#path()},
                    _ => quote! {#FQDefault::default()},
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
    reflect_struct: &ReflectStruct,
    dyn_struct_name: &Ident,
    struct_type: &Ident,
    is_tuple: bool,
) -> MemberValuePair {
    let bevy_reflect_path = reflect_struct.meta().bevy_reflect_path();

    MemberValuePair::new(
        reflect_struct
            .active_fields()
            .map(|field| {
                let member = get_ident(field.data, field.index, is_tuple);
                let accessor = get_field_accessor(field.data, field.index, is_tuple);
                let ty = field.data.ty.clone();

                let missing_error = if is_tuple {
                    quote!(|| #bevy_reflect_path::FromReflectError::MissingUnnamedField {
                        from_type: #bevy_reflect_path::Reflect::get_type_info(reflect),
                        from_kind: #bevy_reflect_path::Reflect::reflect_kind(reflect),
                        to_type: <Self as #bevy_reflect_path::Typed>::type_info(),
                        index: #accessor,
                    })
                } else {
                    quote!(|| #bevy_reflect_path::FromReflectError::MissingNamedField {
                        from_type: #bevy_reflect_path::Reflect::get_type_info(reflect),
                        from_kind: #bevy_reflect_path::Reflect::reflect_kind(reflect),
                        to_type: <Self as #bevy_reflect_path::Typed>::type_info(),
                        field: #accessor,
                    })
                };

                let get_field = quote! {
                    #bevy_reflect_path::#struct_type::field(#dyn_struct_name, #accessor).ok_or_else(#missing_error)
                };

                let error = if is_tuple {
                    quote!(|err| #bevy_reflect_path::FromReflectError::UnnamedFieldError {
                        from_type: #bevy_reflect_path::Reflect::get_type_info(reflect),
                        from_kind: #bevy_reflect_path::Reflect::reflect_kind(reflect),
                        to_type: <Self as #bevy_reflect_path::Typed>::type_info(),
                        index: #accessor,
                        source: #FQBox::new(err),
                    })
                } else {
                    quote!(|err| #bevy_reflect_path::FromReflectError::NamedFieldError {
                        from_type: #bevy_reflect_path::Reflect::get_type_info(reflect),
                        from_kind: #bevy_reflect_path::Reflect::reflect_kind(reflect),
                        to_type: <Self as #bevy_reflect_path::Typed>::type_info(),
                        field: #accessor,
                        source: #FQBox::new(err),
                    })
                };

                let value = match &field.attrs.default {
                    DefaultBehavior::Func(path) => quote! {
                        (||
                            if let #FQResult::Ok(field) = #get_field {
                                <#ty as #bevy_reflect_path::FromReflect>::from_reflect(field).map_err(#error)
                            } else {
                                #FQResult::Ok(#path())
                            }
                        )
                    },
                    DefaultBehavior::Default => quote! {
                        (||
                            if let #FQResult::Ok(field) = #get_field {
                                <#ty as #bevy_reflect_path::FromReflect>::from_reflect(field).map_err(#error)
                            } else {
                                #FQResult::Ok(#FQDefault::default())
                            }
                        )
                    },
                    DefaultBehavior::Required => quote! {
                        (|| <#ty as #bevy_reflect_path::FromReflect>::from_reflect(#get_field?).map_err(#error))
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
