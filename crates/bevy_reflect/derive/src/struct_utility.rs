use crate::derive_data::StructField;
use crate::{utility, ReflectStruct};
use bevy_macro_utils::fq_std::{FQCow, FQResult};
use quote::quote;

/// A helper struct for creating remote-aware field accessors.
///
/// These are "remote-aware" because when a field is a remote field, it uses a [`transmute`] internally
/// to access the field.
///
/// [`transmute`]: core::mem::transmute
pub(crate) struct FieldAccessors {
    /// The referenced field accessors, such as `&self.foo`.
    pub fields_ref: Vec<proc_macro2::TokenStream>,
    /// The mutably referenced field accessors, such as `&mut self.foo`.
    ///
    /// Each field accessor is wrapped in a `Result` that returns an error if the field is marked as readonly.
    pub fields_mut: Vec<proc_macro2::TokenStream>,
    /// The ordered set of field indices (basically just the range of [0, `field_count`).
    pub field_indices: Vec<usize>,
    /// The number of fields in the reflected struct.
    pub field_count: usize,
}

impl FieldAccessors {
    pub fn new(reflect_struct: &ReflectStruct) -> Self {
        let bevy_reflect_path = reflect_struct.meta().bevy_reflect_path();
        let fields_ref = Self::get_fields(reflect_struct, |field, accessor| {
            match &field.attrs.remote {
                Some(wrapper_ty) => {
                    quote! {
                        <#wrapper_ty as #bevy_reflect_path::ReflectRemote>::as_wrapper(&#accessor)
                    }
                }
                None => quote!(& #accessor),
            }
        });
        let fields_mut = Self::get_fields(reflect_struct, |field, accessor| {
            if field.attrs.readonly {
                let field_id = field.data.ident.as_ref().map(|ident| {
                    let name = ident.to_string();
                    quote!(#bevy_reflect_path::FieldId::Named(::std::convert::Into::into(#name.to_string())))
                }).unwrap_or_else(|| {
                    let index = field.reflection_index;
                    quote!(#bevy_reflect_path::FieldId::Unnamed(#index))
                });

                return quote! {
                    #FQResult::Err(
                        #bevy_reflect_path::error::ReflectFieldError::Readonly {
                            field: #field_id,
                            container_type_path: #FQCow::Borrowed(<Self as #bevy_reflect_path::TypePath>::type_path()),
                        },
                    )
                };
            }

            match &field.attrs.remote {
                Some(wrapper_ty) => {
                    quote! {
                        #FQResult::Ok(<#wrapper_ty as #bevy_reflect_path::ReflectRemote>::as_wrapper_mut(&mut #accessor))
                    }
                }
                None => quote!(#FQResult::Ok(&mut #accessor)),
            }
        });

        let field_count = fields_ref.len();
        let field_indices = (0..field_count).collect();

        Self {
            fields_ref,
            fields_mut,
            field_indices,
            field_count,
        }
    }

    fn get_fields<F>(
        reflect_struct: &ReflectStruct,
        mut wrapper_fn: F,
    ) -> Vec<proc_macro2::TokenStream>
    where
        F: FnMut(&StructField, proc_macro2::TokenStream) -> proc_macro2::TokenStream,
    {
        let is_remote = reflect_struct.meta().is_remote_wrapper();
        reflect_struct
            .active_fields()
            .map(|field| {
                let member =
                    utility::ident_or_index(field.data.ident.as_ref(), field.declaration_index);
                let accessor = if is_remote {
                    quote!(self.0.#member)
                } else {
                    quote!(self.#member)
                };

                wrapper_fn(field, accessor)
            })
            .collect::<Vec<_>>()
    }
}
