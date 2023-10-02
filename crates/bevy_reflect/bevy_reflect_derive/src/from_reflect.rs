use crate::container_attributes::REFLECT_DEFAULT;
use crate::derive_data::ReflectEnum;
use crate::enum_utility::{get_variant_constructors, EnumVariantConstructors};
use crate::field_attributes::DefaultBehavior;
use crate::utility::{extend_where_clause, ident_or_index, WhereClauseOptions};
use crate::{ReflectMeta, ReflectStruct};
use bevy_macro_utils::fq_std::{FQAny, FQClone, FQDefault, FQOption};
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

pub(crate) fn impl_value(meta: &ReflectMeta) -> proc_macro2::TokenStream {
    let type_path = meta.type_path();
    let bevy_reflect_path = meta.bevy_reflect_path();
    let (impl_generics, ty_generics, where_clause) = type_path.generics().split_for_impl();
    let where_from_reflect_clause =
        extend_where_clause(where_clause, &WhereClauseOptions::new_value(meta));
    quote! {
        impl #impl_generics #bevy_reflect_path::FromReflect for #type_path #ty_generics #where_from_reflect_clause  {
            fn from_reflect(reflect: &dyn #bevy_reflect_path::Reflect) -> #FQOption<Self> {
                #FQOption::Some(#FQClone::clone(<dyn #FQAny>::downcast_ref::<#type_path #ty_generics>(<dyn #bevy_reflect_path::Reflect>::as_any(reflect))?))
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
    let EnumVariantConstructors {
        variant_names,
        variant_constructors,
    } = get_variant_constructors(reflect_enum, &ref_value, false);

    let (impl_generics, ty_generics, where_clause) = enum_path.generics().split_for_impl();

    // Add FromReflect bound for each active field
    let where_from_reflect_clause = extend_where_clause(
        where_clause,
        &WhereClauseOptions::new_with_bounds(
            reflect_enum.meta(),
            reflect_enum.active_fields(),
            reflect_enum.ignored_fields(),
            |field| match &field.attrs.default {
                DefaultBehavior::Default => Some(quote!(#FQDefault)),
                _ => None,
            },
            |field| match &field.attrs.default {
                DefaultBehavior::Func(_) => None,
                _ => Some(quote!(#FQDefault)),
            },
        ),
    );

    quote! {
        impl #impl_generics #bevy_reflect_path::FromReflect for #enum_path #ty_generics #where_from_reflect_clause  {
            fn from_reflect(#ref_value: &dyn #bevy_reflect_path::Reflect) -> #FQOption<Self> {
                if let #bevy_reflect_path::ReflectRef::Enum(#ref_value) = #bevy_reflect_path::Reflect::reflect_ref(#ref_value) {
                    match #bevy_reflect_path::Enum::variant_name(#ref_value) {
                        #(#variant_names => #fqoption::Some(#variant_constructors),)*
                        name => panic!("variant with name `{}` does not exist on enum `{}`", name, ::core::any::type_name::<Self>()),
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
    let bevy_reflect_path = reflect_struct.meta().bevy_reflect_path();

    let ref_struct = Ident::new("__ref_struct", Span::call_site());
    let ref_struct_type = if is_tuple {
        Ident::new("TupleStruct", Span::call_site())
    } else {
        Ident::new("Struct", Span::call_site())
    };

    let MemberValuePair(active_members, active_values) =
        get_active_fields(reflect_struct, &ref_struct, &ref_struct_type, is_tuple);

    let is_defaultable = reflect_struct.meta().traits().contains(REFLECT_DEFAULT);
    let constructor = if is_defaultable {
        quote!(
            let mut __this: Self = #FQDefault::default();
            #(
                if let #fqoption::Some(__field) = #active_values() {
                    // Iff field exists -> use its value
                    __this.#active_members = __field;
                }
            )*
            #FQOption::Some(__this)
        )
    } else {
        let MemberValuePair(ignored_members, ignored_values) = get_ignored_fields(reflect_struct);

        quote!(
            #FQOption::Some(
                Self {
                    #(#active_members: #active_values()?,)*
                    #(#ignored_members: #ignored_values,)*
                }
            )
        )
    };

    let (impl_generics, ty_generics, where_clause) = reflect_struct
        .meta()
        .type_path()
        .generics()
        .split_for_impl();

    // Add FromReflect bound for each active field
    let where_from_reflect_clause = extend_where_clause(
        where_clause,
        &WhereClauseOptions::new_with_bounds(
            reflect_struct.meta(),
            reflect_struct.active_fields(),
            reflect_struct.ignored_fields(),
            |field| match &field.attrs.default {
                DefaultBehavior::Default => Some(quote!(#FQDefault)),
                _ => None,
            },
            |field| {
                if is_defaultable {
                    None
                } else {
                    match &field.attrs.default {
                        DefaultBehavior::Func(_) => None,
                        _ => Some(quote!(#FQDefault)),
                    }
                }
            },
        ),
    );

    quote! {
        impl #impl_generics #bevy_reflect_path::FromReflect for #struct_path #ty_generics #where_from_reflect_clause {
            fn from_reflect(reflect: &dyn #bevy_reflect_path::Reflect) -> #FQOption<Self> {
                if let #bevy_reflect_path::ReflectRef::#ref_struct_type(#ref_struct) = #bevy_reflect_path::Reflect::reflect_ref(reflect) {
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
                let member = ident_or_index(field.data.ident.as_ref(), field.index);

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
                let member = ident_or_index(field.data.ident.as_ref(), field.index);
                let accessor = get_field_accessor(field.data, field.index, is_tuple);
                let ty = field.data.ty.clone();

                let get_field = quote! {
                    #bevy_reflect_path::#struct_type::field(#dyn_struct_name, #accessor)
                };

                let value = match &field.attrs.default {
                    DefaultBehavior::Func(path) => quote! {
                        (||
                            if let #FQOption::Some(field) = #get_field {
                                <#ty as #bevy_reflect_path::FromReflect>::from_reflect(field)
                            } else {
                                #FQOption::Some(#path())
                            }
                        )
                    },
                    DefaultBehavior::Default => quote! {
                        (||
                            if let #FQOption::Some(field) = #get_field {
                                <#ty as #bevy_reflect_path::FromReflect>::from_reflect(field)
                            } else {
                                #FQOption::Some(#FQDefault::default())
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
