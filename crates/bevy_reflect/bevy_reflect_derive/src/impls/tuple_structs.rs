use crate::impls::{impl_type_path, impl_typed};
use crate::utility::extend_where_clause;
use crate::ReflectStruct;
use bevy_macro_utils::fq_std::{FQAny, FQBox, FQDefault, FQOption, FQResult};
use quote::{quote, ToTokens};
use syn::{Index, Member};

/// Implements `TupleStruct`, `GetTypeRegistration`, and `Reflect` for the given derive data.
pub(crate) fn impl_tuple_struct(reflect_struct: &ReflectStruct) -> proc_macro2::TokenStream {
    let fqoption = FQOption.into_token_stream();

    let bevy_reflect_path = reflect_struct.meta().bevy_reflect_path();
    let struct_path = reflect_struct.meta().type_path();

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
                fn reflect_partial_eq(&self, value: &dyn #bevy_reflect_path::Reflect) -> #FQOption<bool> {
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

    let string_name = struct_path.get_ident().unwrap().to_string();

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
        reflect_struct.meta(),
        &where_clause_options,
        quote! {
            let fields = [#field_generator];
            let info = #info_generator;
            #bevy_reflect_path::TypeInfo::TupleStruct(info)
        },
    );

    let type_path_impl = impl_type_path(reflect_struct.meta(), &where_clause_options);

    let (impl_generics, ty_generics, where_clause) = reflect_struct
        .meta()
        .type_path()
        .generics()
        .split_for_impl();

    let where_reflect_clause = extend_where_clause(where_clause, &where_clause_options);

    quote! {
        #get_type_registration_impl

        #typed_impl

        #type_path_impl

        impl #impl_generics #bevy_reflect_path::TupleStruct for #struct_path #ty_generics #where_reflect_clause {
            fn field(&self, index: usize) -> #FQOption<&dyn #bevy_reflect_path::Reflect> {
                match index {
                    #(#field_indices => #fqoption::Some(&self.#field_idents),)*
                    _ => #FQOption::None,
                }
            }

            fn field_mut(&mut self, index: usize) -> #FQOption<&mut dyn #bevy_reflect_path::Reflect> {
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
                dynamic.set_represented_type(#bevy_reflect_path::Reflect::get_represented_type_info(self));
                #(dynamic.insert_boxed(#bevy_reflect_path::Reflect::clone_value(&self.#field_idents));)*
                dynamic
            }
        }

        impl #impl_generics #bevy_reflect_path::Reflect for #struct_path #ty_generics #where_reflect_clause {
            #[inline]
            fn type_name(&self) -> &str {
                ::core::any::type_name::<Self>()
            }

            #[inline]
            fn get_represented_type_info(&self) -> #FQOption<&'static #bevy_reflect_path::TypeInfo> {
                #FQOption::Some(<Self as #bevy_reflect_path::Typed>::type_info())
            }

            #[inline]
            fn into_any(self: #FQBox<Self>) -> #FQBox<dyn #FQAny> {
                self
            }

            #[inline]
            fn as_any(&self) -> &dyn #FQAny {
                self
            }

            #[inline]
            fn as_any_mut(&mut self) -> &mut dyn #FQAny {
                self
            }

            #[inline]
            fn into_reflect(self: #FQBox<Self>) -> #FQBox<dyn #bevy_reflect_path::Reflect> {
                self
            }

            #[inline]
            fn as_reflect(&self) -> &dyn #bevy_reflect_path::Reflect {
                self
            }

            #[inline]
            fn as_reflect_mut(&mut self) -> &mut dyn #bevy_reflect_path::Reflect {
                self
            }

            #[inline]
            fn clone_value(&self) -> #FQBox<dyn #bevy_reflect_path::Reflect> {
                #FQBox::new(#bevy_reflect_path::TupleStruct::clone_dynamic(self))
            }

            #[inline]
            fn set(&mut self, value: #FQBox<dyn #bevy_reflect_path::Reflect>) -> #FQResult<(), #FQBox<dyn #bevy_reflect_path::Reflect>> {
                *self = <dyn #bevy_reflect_path::Reflect>::take(value)?;
                #FQResult::Ok(())
            }

            #[inline]
            fn apply(&mut self, value: &dyn #bevy_reflect_path::Reflect) {
                if let #bevy_reflect_path::ReflectRef::TupleStruct(struct_value) = #bevy_reflect_path::Reflect::reflect_ref(value) {
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
    }
}
