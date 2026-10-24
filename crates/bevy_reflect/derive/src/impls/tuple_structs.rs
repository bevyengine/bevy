use crate::{
    impls::{common_reflect_trait_methods, impl_reflect_casts, impl_type_path, impl_typed},
    struct_utility::FieldAccessors,
    ReflectStruct,
};
use bevy_macro_utils::fq_std::FQOption;
use quote::{quote, ToTokens};

/// Implements `TupleStruct`, `GetTypeRegistration`, and `Reflect` for the given derive data.
pub(crate) fn impl_tuple_struct(reflect_struct: &ReflectStruct) -> proc_macro2::TokenStream {
    let fqoption = FQOption.into_token_stream();

    let bevy_reflect_path = reflect_struct.meta().bevy_reflect_path();
    let struct_path = reflect_struct.meta().type_path();

    let FieldAccessors {
        fields_ref,
        fields_mut,
        field_indices,
        field_count,
        ..
    } = FieldAccessors::new(reflect_struct);

    let where_clause_options = reflect_struct.where_clause_options();
    let get_type_registration_impl = reflect_struct.get_type_registration(&where_clause_options);

    let typed_impl = impl_typed(&where_clause_options, reflect_struct.to_info_tokens(true));

    let type_path_impl = impl_type_path(reflect_struct.meta());
    let reflect_casts_impl = impl_reflect_casts(reflect_struct.meta());
    let common_methods = common_reflect_trait_methods(
        reflect_struct.meta(),
        || Some(quote!(#bevy_reflect_path::tuple_struct::tuple_struct_partial_eq)),
        || None,
        || Some(quote!(#bevy_reflect_path::tuple_struct::tuple_struct_partial_cmp)),
    );
    let clone_fn = reflect_struct.get_clone_impl();

    #[cfg(not(feature = "functions"))]
    let function_impls = None::<proc_macro2::TokenStream>;
    #[cfg(feature = "functions")]
    let function_impls = crate::impls::impl_function_traits(&where_clause_options);

    let (impl_generics, ty_generics, where_clause) = reflect_struct
        .meta()
        .type_path()
        .generics()
        .split_for_impl();

    #[cfg(not(feature = "auto_register"))]
    let auto_register = None::<proc_macro2::TokenStream>;
    #[cfg(feature = "auto_register")]
    let auto_register = crate::impls::reflect_auto_registration(reflect_struct.meta());

    let where_reflect_clause = where_clause_options.extend_where_clause(where_clause);

    quote! {
        #get_type_registration_impl

        #typed_impl

        #type_path_impl

        #function_impls

        #auto_register

        impl #impl_generics #bevy_reflect_path::tuple_struct::TupleStruct for #struct_path #ty_generics #where_reflect_clause {
            fn field(&self, index: usize) -> #FQOption<&dyn #bevy_reflect_path::Reflect> {
                match index {
                    #(#field_indices => #fqoption::Some(#fields_ref),)*
                    _ => #FQOption::None,
                }
            }

            fn field_mut(&mut self, index: usize) -> #FQOption<&mut dyn #bevy_reflect_path::Reflect> {
                match index {
                    #(#field_indices => #fqoption::Some(#fields_mut),)*
                    _ => #FQOption::None,
                }
            }
            #[inline]
            fn field_len(&self) -> usize {
                #field_count
            }
            #[inline]
            fn iter_fields(&self) -> #bevy_reflect_path::tuple_struct::TupleStructFieldIter {
                #bevy_reflect_path::tuple_struct::TupleStructFieldIter::new(self)
            }
        }

        impl #impl_generics #bevy_reflect_path::Reflect for #struct_path #ty_generics #where_reflect_clause {
            #[inline]
            fn reflect_kind(&self) -> #bevy_reflect_path::ReflectKind {
                #bevy_reflect_path::ReflectKind::TupleStruct
            }
            #[inline]
            fn reflect_ref(&self) -> #bevy_reflect_path::ReflectRef {
                #bevy_reflect_path::ReflectRef::TupleStruct(self)
            }
            #[inline]
            fn reflect_mut(&mut self) -> #bevy_reflect_path::ReflectMut {
                #bevy_reflect_path::ReflectMut::TupleStruct(self)
            }
            #[inline]
            fn reflect_owned(self: #bevy_reflect_path::__macro_exports::alloc_utils::Box<Self>) -> #bevy_reflect_path::ReflectOwned {
                #bevy_reflect_path::ReflectOwned::TupleStruct(self)
            }

            #common_methods

            #clone_fn

            #reflect_casts_impl
        }
    }
}
