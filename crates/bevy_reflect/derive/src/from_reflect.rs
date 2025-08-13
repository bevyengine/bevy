use crate::{
    container_attributes::REFLECT_DEFAULT,
    derive_data::ReflectEnum,
    enum_utility::{EnumVariantOutputData, FromReflectVariantBuilder, VariantBuilder},
    field_attributes::DefaultBehavior,
    ident::ident_or_index,
    where_clause_options::WhereClauseOptions,
    ReflectMeta, ReflectStruct,
};
use bevy_macro_utils::fq_std::{FQClone, FQDefault, FQOption};
use proc_macro2::Span;
use quote::{quote, ToTokens};
use syn::{Field, Ident, Lit, LitInt, LitStr, Member};

/// Implements `FromReflect` for the given struct
pub(crate) fn impl_struct(reflect_struct: &ReflectStruct) -> proc_macro2::TokenStream {
    impl_struct_internal(reflect_struct, false)
}

/// Implements `FromReflect` for the given tuple struct
pub(crate) fn impl_tuple_struct(reflect_struct: &ReflectStruct) -> proc_macro2::TokenStream {
    impl_struct_internal(reflect_struct, true)
}

pub(crate) fn impl_opaque(meta: &ReflectMeta) -> proc_macro2::TokenStream {
    let type_path = meta.type_path();
    let bevy_reflect_path = meta.bevy_reflect_path();
    let (impl_generics, ty_generics, where_clause) = type_path.generics().split_for_impl();
    let where_from_reflect_clause = WhereClauseOptions::new(meta).extend_where_clause(where_clause);
    quote! {
        impl #impl_generics #bevy_reflect_path::FromReflect for #type_path #ty_generics #where_from_reflect_clause  {
            fn from_reflect(reflect: &dyn #bevy_reflect_path::PartialReflect) -> #FQOption<Self> {
                #FQOption::Some(
                    #FQClone::clone(
                        <dyn #bevy_reflect_path::PartialReflect>::try_downcast_ref::<#type_path #ty_generics>(reflect)?
                    )
                )
            }
        }
    }
}

/// Implements `FromReflect` for the given enum type
pub(crate) fn impl_enum(reflect_enum: &ReflectEnum) -> proc_macro2::TokenStream {
    let fqoption = FQOption.into_token_stream();

    let enum_path = reflect_enum.meta().type_path();
    let bevy_reflect_path = reflect_enum.meta().bevy_reflect_path();

    let ref_value = Ident::new("__param0", Span::call_site());

    let EnumVariantOutputData {
        variant_names,
        variant_constructors,
        ..
    } = FromReflectVariantBuilder::new(reflect_enum).build(&ref_value);

    let match_branches = if reflect_enum.meta().is_remote_wrapper() {
        quote! {
            #(#variant_names => #fqoption::Some(Self(#variant_constructors)),)*
        }
    } else {
        quote! {
            #(#variant_names => #fqoption::Some(#variant_constructors),)*
        }
    };

    let (impl_generics, ty_generics, where_clause) = enum_path.generics().split_for_impl();

    // Add FromReflect bound for each active field
    let where_from_reflect_clause = reflect_enum
        .where_clause_options()
        .extend_where_clause(where_clause);

    quote! {
        impl #impl_generics #bevy_reflect_path::FromReflect for #enum_path #ty_generics #where_from_reflect_clause  {
            fn from_reflect(#ref_value: &dyn #bevy_reflect_path::PartialReflect) -> #FQOption<Self> {
                if let #bevy_reflect_path::ReflectRef::Enum(#ref_value) =
                    #bevy_reflect_path::PartialReflect::reflect_ref(#ref_value)
                {
                    match #bevy_reflect_path::Enum::variant_name(#ref_value) {
                        #match_branches
                        name => panic!("variant with name `{}` does not exist on enum `{}`", name, <Self as #bevy_reflect_path::TypePath>::type_path()),
                    }
                } else {
                    #FQOption::None
                }
            }
        }
    }
}

/// Container for a struct's members (field name or index) and their
/// corresponding values.
struct MemberValuePair(Vec<Member>, Vec<proc_macro2::TokenStream>);

impl MemberValuePair {
    pub fn new(items: (Vec<Member>, Vec<proc_macro2::TokenStream>)) -> Self {
        Self(items.0, items.1)
    }
}

fn impl_struct_internal(
    reflect_struct: &ReflectStruct,
    is_tuple: bool,
) -> proc_macro2::TokenStream {
    let fqoption = FQOption.into_token_stream();

    let struct_path = reflect_struct.meta().type_path();
    let remote_ty = reflect_struct.meta().remote_ty();
    let bevy_reflect_path = reflect_struct.meta().bevy_reflect_path();

    let ref_struct = Ident::new("__ref_struct", Span::call_site());
    let ref_struct_type = if is_tuple {
        Ident::new("TupleStruct", Span::call_site())
    } else {
        Ident::new("Struct", Span::call_site())
    };

    let MemberValuePair(active_members, active_values) =
        get_active_fields(reflect_struct, &ref_struct, &ref_struct_type, is_tuple);

    let is_defaultable = reflect_struct.meta().attrs().contains(REFLECT_DEFAULT);

    // The constructed "Self" ident
    let __this = Ident::new("__this", Span::call_site());

    // The reflected type: either `Self` or a remote type
    let (reflect_ty, constructor, retval) = if let Some(remote_ty) = remote_ty {
        let constructor = match remote_ty.as_expr_path() {
            Ok(path) => path,
            Err(err) => return err.into_compile_error(),
        };
        let remote_ty = remote_ty.type_path();

        (
            quote!(#remote_ty),
            quote!(#constructor),
            quote!(Self(#__this)),
        )
    } else {
        (quote!(Self), quote!(Self), quote!(#__this))
    };

    let constructor = if is_defaultable {
        quote! {
            let mut #__this = <#reflect_ty as #FQDefault>::default();
            #(
                // The closure catches any failing `?` within `active_values`.
                if let #fqoption::Some(__field) = (|| #active_values)() {
                    // Iff field exists -> use its value
                    #__this.#active_members = __field;
                }
            )*
            #FQOption::Some(#retval)
        }
    } else {
        let MemberValuePair(ignored_members, ignored_values) = get_ignored_fields(reflect_struct);

        quote! {
            let #__this = #constructor {
                #(#active_members: #active_values?,)*
                #(#ignored_members: #ignored_values,)*
            };
            #FQOption::Some(#retval)
        }
    };

    let (impl_generics, ty_generics, where_clause) = reflect_struct
        .meta()
        .type_path()
        .generics()
        .split_for_impl();

    // Add FromReflect bound for each active field
    let where_from_reflect_clause = reflect_struct
        .where_clause_options()
        .extend_where_clause(where_clause);

    quote! {
        impl #impl_generics #bevy_reflect_path::FromReflect for #struct_path #ty_generics #where_from_reflect_clause {
            fn from_reflect(reflect: &dyn #bevy_reflect_path::PartialReflect) -> #FQOption<Self> {
                if let #bevy_reflect_path::ReflectRef::#ref_struct_type(#ref_struct)
                    = #bevy_reflect_path::PartialReflect::reflect_ref(reflect)
                {
                    #constructor
                } else {
                    #FQOption::None
                }
            }
        }
    }
}

/// Get the collection of ignored field definitions
///
/// Each value of the `MemberValuePair` is a token stream that generates a
/// a default value for the ignored field.
fn get_ignored_fields(reflect_struct: &ReflectStruct) -> MemberValuePair {
    MemberValuePair::new(
        reflect_struct
            .ignored_fields()
            .map(|field| {
                let member = ident_or_index(field.data.ident.as_ref(), field.declaration_index);

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
                let member = ident_or_index(field.data.ident.as_ref(), field.declaration_index);
                let accessor = get_field_accessor(
                    field.data,
                    field.reflection_index.expect("field should be active"),
                    is_tuple,
                );
                let ty = field.reflected_type().clone();
                let real_ty = &field.data.ty;

                let get_field = quote! {
                    #bevy_reflect_path::#struct_type::field(#dyn_struct_name, #accessor)
                };

                let into_remote = |value: proc_macro2::TokenStream| {
                    if field.attrs.is_remote_generic().unwrap_or_default() {
                        quote! {
                            #FQOption::Some(
                                // SAFETY: The remote type should always be a `#[repr(transparent)]` for the actual field type
                                unsafe {
                                    ::core::mem::transmute_copy::<#ty, #real_ty>(
                                        &::core::mem::ManuallyDrop::new(#value?)
                                    )
                                }
                            )
                        }
                    } else if field.attrs().remote.is_some() {
                        quote! {
                            #FQOption::Some(
                                // SAFETY: The remote type should always be a `#[repr(transparent)]` for the actual field type
                                unsafe {
                                    ::core::mem::transmute::<#ty, #real_ty>(#value?)
                                }
                            )
                        }
                    } else {
                        value
                    }
                };

                let value = match &field.attrs.default {
                    DefaultBehavior::Func(path) => {
                        let value = into_remote(quote! {
                            <#ty as #bevy_reflect_path::FromReflect>::from_reflect(field)
                        });
                        quote! {
                            if let #FQOption::Some(field) = #get_field {
                                #value
                            } else {
                                #FQOption::Some(#path())
                            }
                        }
                    }
                    DefaultBehavior::Default => {
                        let value = into_remote(quote! {
                            <#ty as #bevy_reflect_path::FromReflect>::from_reflect(field)
                        });
                        quote! {
                            if let #FQOption::Some(field) = #get_field {
                                #value
                            } else {
                                #FQOption::Some(#FQDefault::default())
                            }
                        }
                    }
                    DefaultBehavior::Required => {
                        let value = into_remote(quote! {
                            <#ty as #bevy_reflect_path::FromReflect>::from_reflect(#get_field?)
                        });
                        quote! {
                            #value
                        }
                    }
                };

                (member, value)
            })
            .unzip(),
    )
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
