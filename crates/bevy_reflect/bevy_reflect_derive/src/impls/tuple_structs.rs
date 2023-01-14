use crate::fq_std::{FQBox, FQDefault, FQOption, FQResult};
use crate::impls::impl_typed;
use crate::utility::extend_where_clause;
use crate::ReflectStruct;
use proc_macro::TokenStream;
use quote::{quote, ToTokens};
use syn::{Index, Member};

use super::impl_full_reflect;

/// Implements `TupleStruct`, `GetTypeRegistration`, and `Reflect` for the given derive data.
pub(crate) fn impl_tuple_struct(reflect_struct: &ReflectStruct) -> TokenStream {
    let fqoption = FQOption.into_token_stream();

    let bevy_reflect_path = reflect_struct.meta().bevy_reflect_path();
    let struct_name = reflect_struct.meta().type_name();

    let field_idents = reflect_struct
        .active_fields()
        .map(|field| Member::Unnamed(Index::from(field.index)))
        .collect::<Vec<_>>();
    let field_types = reflect_struct.active_types();
    let field_count = field_idents.len();
    let field_indices = (0..field_count).collect::<Vec<usize>>();

    let where_clause_options = reflect_struct.where_clause_options();
    let get_type_registration_impl = reflect_struct.get_type_registration(&where_clause_options);

    let hash_fn = reflect_struct
        .meta()
        .traits()
        .get_hash_impl(bevy_reflect_path);
    let debug_fn = reflect_struct.meta().traits().get_debug_impl();
    let partial_eq_fn = reflect_struct
        .meta()
        .traits()
        .get_partial_eq_impl(bevy_reflect_path)
        .unwrap_or_else(|| {
            quote! {
                fn reflect_partial_eq(&self, value: &dyn #bevy_reflect_path::PartialReflect) -> #FQOption<bool> {
                    #bevy_reflect_path::tuple_struct_partial_eq(self, value)
                }
            }
        });

    #[cfg(feature = "documentation")]
    let field_generator = {
        let docs = reflect_struct
            .active_fields()
            .map(|field| quote::ToTokens::to_token_stream(&field.doc));
        quote! {
            #(#bevy_reflect_path::UnnamedField::new::<#field_types>(#field_idents).with_docs(#docs) ,)*
        }
    };

    #[cfg(not(feature = "documentation"))]
    let field_generator = {
        quote! {
            #(#bevy_reflect_path::UnnamedField::new::<#field_types>(#field_idents) ,)*
        }
    };

    let string_name = struct_name.to_string();

    #[cfg(feature = "documentation")]
    let info_generator = {
        let doc = reflect_struct.meta().doc();
        quote! {
           #bevy_reflect_path::TupleStructInfo::new::<Self>(#string_name, &fields).with_docs(#doc)
        }
    };

    #[cfg(not(feature = "documentation"))]
    let info_generator = {
        quote! {
            #bevy_reflect_path::TupleStructInfo::new::<Self>(#string_name, &fields)
        }
    };

    let typed_impl = impl_typed(
        struct_name,
        reflect_struct.meta().generics(),
        &where_clause_options,
        quote! {
            let fields = [#field_generator];
            let info = #info_generator;
            #bevy_reflect_path::TypeInfo::TupleStruct(info)
        },
        bevy_reflect_path,
    );

    let impl_full_reflect = impl_full_reflect(reflect_struct.meta());

    let (impl_generics, ty_generics, where_clause) =
        reflect_struct.meta().generics().split_for_impl();

    let where_reflect_clause = extend_where_clause(where_clause, &where_clause_options);

    TokenStream::from(quote! {
        #impl_full_reflect

        #get_type_registration_impl

        #typed_impl

        impl #impl_generics #bevy_reflect_path::TupleStruct for #struct_name #ty_generics #where_reflect_clause {
            fn field(&self, index: usize) -> #FQOption<&dyn #bevy_reflect_path::PartialReflect> {
                match index {
                    #(#field_indices => #fqoption::Some(&self.#field_idents),)*
                    _ => #FQOption::None,
                }
            }

            fn field_mut(&mut self, index: usize) -> #FQOption<&mut dyn #bevy_reflect_path::PartialReflect> {
                match index {
                    #(#field_indices => #fqoption::Some(&mut self.#field_idents),)*
                    _ => #FQOption::None,
                }
            }

            fn field_len(&self) -> usize {
                #field_count
            }

            fn iter_fields(&self) -> #bevy_reflect_path::TupleStructFieldIter {
                #bevy_reflect_path::TupleStructFieldIter::new(self)
            }

            fn clone_dynamic(&self) -> #bevy_reflect_path::DynamicTupleStruct {
                let mut dynamic: #bevy_reflect_path::DynamicTupleStruct = #FQDefault::default();
                dynamic.set_name(::std::string::ToString::to_string(#bevy_reflect_path::PartialReflect::type_name(self)));
                #(dynamic.insert_boxed(#bevy_reflect_path::PartialReflect::clone_value(&self.#field_idents));)*
                dynamic
            }
        }

        impl #impl_generics #bevy_reflect_path::PartialReflect for #struct_name #ty_generics #where_reflect_clause {
            #[inline]
            fn type_name(&self) -> &str {
                ::core::any::type_name::<Self>()
            }

            #[inline]
            fn get_type_info(&self) -> &'static #bevy_reflect_path::TypeInfo {
                <Self as #bevy_reflect_path::Typed>::type_info()
            }

            fn as_full(&self) -> #FQOption<&dyn #bevy_reflect_path::Reflect> {
                Some(self)
            }

            fn as_full_mut(&mut self) -> #FQOption<&mut dyn #bevy_reflect_path::Reflect> {
                Some(self)
            }

            fn into_full(self: Box<Self>) -> #FQResult<Box<dyn #bevy_reflect_path::Reflect>, Box<dyn #bevy_reflect_path::PartialReflect>> {
                Ok(self)
            }

            #[inline]
            fn clone_value(&self) -> #FQBox<dyn #bevy_reflect_path::PartialReflect> {
                #FQBox::new(#bevy_reflect_path::TupleStruct::clone_dynamic(self))
            }

            #[inline]
            fn apply(&mut self, value: &dyn #bevy_reflect_path::PartialReflect) {
                if let #bevy_reflect_path::ReflectRef::TupleStruct(struct_value) = #bevy_reflect_path::PartialReflect::reflect_ref(value) {
                    for (i, value) in ::core::iter::Iterator::enumerate(#bevy_reflect_path::TupleStruct::iter_fields(struct_value)) {
                        #bevy_reflect_path::TupleStruct::field_mut(self, i).map(|v| v.apply(value));
                    }
                } else {
                    panic!("Attempted to apply non-TupleStruct type to TupleStruct type.");
                }
            }

            fn reflect_ref(&self) -> #bevy_reflect_path::ReflectRef {
                #bevy_reflect_path::ReflectRef::TupleStruct(self)
            }

            fn reflect_mut(&mut self) -> #bevy_reflect_path::ReflectMut {
                #bevy_reflect_path::ReflectMut::TupleStruct(self)
            }

            fn reflect_owned(self: #FQBox<Self>) -> #bevy_reflect_path::ReflectOwned {
                #bevy_reflect_path::ReflectOwned::TupleStruct(self)
            }

            #hash_fn

            #partial_eq_fn

            #debug_fn
        }
    })
}
