use crate::container_attributes::ReflectTraits;
use crate::impls::impl_typed;
use proc_macro::TokenStream;
use proc_macro2::Ident;
use quote::quote;
use syn::{Generics, Path};

/// Implements `GetTypeRegistration` and `Reflect` for the given type data.
pub(crate) fn impl_value(
    type_name: &Ident,
    generics: &Generics,
    get_type_registration_impl: proc_macro2::TokenStream,
    bevy_reflect_path: &Path,
    reflect_attrs: &ReflectTraits,
) -> TokenStream {
    let hash_fn = reflect_attrs.get_hash_impl(bevy_reflect_path);
    let partial_eq_fn = reflect_attrs.get_partial_eq_impl(bevy_reflect_path);
    let debug_fn = reflect_attrs.get_debug_impl();

    let typed_impl = impl_typed(
        type_name,
        generics,
        quote! {
            let info = #bevy_reflect_path::ValueInfo::new::<Self>();
            #bevy_reflect_path::TypeInfo::Value(info)
        },
        bevy_reflect_path,
    );

    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();

    TokenStream::from(quote! {
        #get_type_registration_impl

        #typed_impl

        impl #impl_generics #bevy_reflect_path::Reflect for #type_name #ty_generics #where_clause  {
            #[inline]
            fn type_name(&self) -> &str {
                std::any::type_name::<Self>()
            }

            #[inline]
            fn get_type_info(&self) -> &'static #bevy_reflect_path::TypeInfo {
                <Self as #bevy_reflect_path::Typed>::type_info()
            }

            #[inline]
            fn into_any(self: Box<Self>) -> Box<dyn std::any::Any> {
                self
            }

            #[inline]
            fn as_any(&self) -> &dyn std::any::Any {
                self
            }

            #[inline]
            fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
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
            fn clone_value(&self) -> Box<dyn #bevy_reflect_path::Reflect> {
                Box::new(self.clone())
            }

            #[inline]
            fn apply(&mut self, value: &dyn #bevy_reflect_path::Reflect) {
                let value = value.as_any();
                if let Some(value) = value.downcast_ref::<Self>() {
                    *self = value.clone();
                } else {
                    panic!("Value is not {}.", std::any::type_name::<Self>());
                }
            }

            #[inline]
            fn set(&mut self, value: Box<dyn #bevy_reflect_path::Reflect>) -> Result<(), Box<dyn #bevy_reflect_path::Reflect>> {
                *self = value.take()?;
                Ok(())
            }

            fn reflect_ref(&self) -> #bevy_reflect_path::ReflectRef {
                #bevy_reflect_path::ReflectRef::Value(self)
            }

            fn reflect_mut(&mut self) -> #bevy_reflect_path::ReflectMut {
                #bevy_reflect_path::ReflectMut::Value(self)
            }

            #hash_fn

            #partial_eq_fn

            #debug_fn
        }
    })
}
