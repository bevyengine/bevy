use crate::derive_data::StructField;
use crate::field_attributes::DefaultBehavior;
use crate::{derive_data::ReflectEnum, utility::ident_or_index};
use bevy_macro_utils::fq_std::{FQDefault, FQOption};
use proc_macro2::{Ident, TokenStream};
use quote::{format_ident, quote};
use syn::{Member, Path};

pub(crate) struct EnumVariantOutputData {
    /// The names of each variant as a string.
    ///
    /// For example, `Some` and `None` for the `Option` enum.
    pub variant_names: Vec<String>,
    /// The constructor portion of each variant.
    ///
    /// For example, `Option::Some { 0: value }` and `Option::None {}` for the `Option` enum.
    pub variant_constructors: Vec<proc_macro2::TokenStream>,
}

#[derive(Copy, Clone)]
struct VariantField<'a, 'b> {
    /// The pre-computed member for the field.
    pub member: &'a Member,
    /// The name of the variant that contains the field.
    pub variant_name: &'a str,
    /// The field data.
    pub field: &'a StructField<'b>,
}

/// Trait used to control how enum variants are built.
trait VariantBuilder: Sized {
    /// Returns a token stream that accesses a field of a variant as an `Option<dyn Reflect>`.
    ///
    /// The default implementation of this method will return a token stream
    /// which gets the field dynamically so as to support `dyn Enum`.
    ///
    /// # Parameters
    /// * `ident`: The identifier of the enum
    /// * `field`: The field to access
    fn access_field(&self, ident: &Ident, field: VariantField) -> proc_macro2::TokenStream {
        match &field.field.data.ident {
            Some(field_ident) => {
                let name = field_ident.to_string();
                quote!(#ident.field(#name))
            }
            None => {
                if let Some(field_index) = field.field.reflection_index {
                    quote!(#ident.field_at(#field_index))
                } else {
                    quote!(::core::compile_error!(
                        "internal bevy_reflect error: field should be active"
                    ))
                }
            }
        }
    }

    /// Returns a token stream that unwraps a field of a variant as a `&dyn Reflect`
    /// (from an `Option<dyn Reflect>`).
    ///
    /// # Parameters
    /// * `ident`: The identifier of the enum
    /// * `field`: The field to access
    fn unwrap_field(&self, ident: &Ident, field: VariantField) -> proc_macro2::TokenStream;

    /// Returns a token stream that constructs a field of a variant as a concrete type
    /// (from a `&dyn Reflect`).
    ///
    /// # Parameters
    /// * `ident`: The identifier of the enum
    /// * `field`: The field to access
    fn construct_field(&self, ident: &Ident, field: VariantField) -> proc_macro2::TokenStream;
}

struct EnumVariantOutputBuilder<'a, V: VariantBuilder> {
    /// A reference to the enum that is being reflected (e.g. `self`).
    this: &'a Ident,
    /// The reflect enum data.
    reflect_enum: &'a ReflectEnum<'a>,
    /// The variant builder.
    variant_builder: V,
}

impl<'a, V: VariantBuilder> EnumVariantOutputBuilder<'a, V> {
    pub fn new(this: &'a Ident, reflect_enum: &'a ReflectEnum<'a>, variant_builder: V) -> Self {
        Self {
            this,
            reflect_enum,
            variant_builder,
        }
    }

    pub fn build(self) -> EnumVariantOutputData {
        let variants = self.reflect_enum.variants();

        let mut variant_names = Vec::with_capacity(variants.len());
        let mut variant_constructors = Vec::with_capacity(variants.len());

        for variant in variants {
            let variant_ident = &variant.data.ident;
            let variant_name = variant_ident.to_string();
            let variant_path = self.reflect_enum.get_unit(variant_ident);

            let fields = variant.fields();

            let field_constructors = fields.iter().map(|field| {
                let member = ident_or_index(field.data.ident.as_ref(), field.declaration_index);

                let value = self.construct_field(VariantField {
                    member: &member,
                    variant_name: &variant_name,
                    field,
                });

                let constructor = quote! {
                    #member: #value
                };

                constructor
            });

            let constructor = quote! {
                #variant_path {
                    #( #field_constructors ),*
                }
            };

            variant_names.push(variant_name);
            variant_constructors.push(constructor);
        }

        EnumVariantOutputData {
            variant_names,
            variant_constructors,
        }
    }

    fn construct_field(&self, variant_field: VariantField) -> proc_macro2::TokenStream {
        let VariantField { field, .. } = variant_field;

        // Ignored fields (fall back to default value)
        if field.attrs.ignore.is_ignored() {
            return match &field.attrs.default {
                DefaultBehavior::Func(path) => quote! { #path() },
                _ => quote! { #FQDefault::default() },
            };
        }

        let field_accessor = self.variant_builder.access_field(self.this, variant_field);

        let field_ident = format_ident!("__field");
        let field_constructor = self
            .variant_builder
            .construct_field(&field_ident, variant_field);

        match &field.attrs.default {
            DefaultBehavior::Func(path) => quote! {
                if let #FQOption::Some(#field_ident) = #field_accessor {
                    #field_constructor
                } else {
                    #path()
                }
            },
            DefaultBehavior::Default => quote! {
                if let #FQOption::Some(#field_ident) = #field_accessor {
                    #field_constructor
                } else {
                    #FQDefault::default()
                }
            },
            DefaultBehavior::Required => {
                let field_unwrapper = self
                    .variant_builder
                    .unwrap_field(&field_ident, variant_field);

                quote! {{
                    // `#field_ident` is used by both the unwrapper and constructor
                    let #field_ident = #field_accessor;
                    let #field_ident = #field_unwrapper;
                    #field_constructor
                }}
            }
        }
    }
}

/// Generates the enum variant output data needed to build the `FromReflect::from_reflect` implementation.
pub(crate) fn generate_from_reflect_variants(
    reflect_enum: &ReflectEnum,
    this: &Ident,
) -> EnumVariantOutputData {
    struct FromReflectVariantBuilder<'a> {
        bevy_reflect_path: &'a Path,
    }

    impl<'a> VariantBuilder for FromReflectVariantBuilder<'a> {
        fn unwrap_field(&self, ident: &Ident, _field: VariantField) -> TokenStream {
            quote!(#ident?)
        }

        fn construct_field(&self, ident: &Ident, field: VariantField) -> TokenStream {
            let bevy_reflect_path = self.bevy_reflect_path;
            let field_ty = &field.field.data.ty;

            quote! {
                <#field_ty as #bevy_reflect_path::FromReflect>::from_reflect(#ident)?
            }
        }
    }

    EnumVariantOutputBuilder::new(
        this,
        reflect_enum,
        FromReflectVariantBuilder {
            bevy_reflect_path: reflect_enum.meta().bevy_reflect_path(),
        },
    )
    .build()
}

/// Generates the enum variant output data needed to build the `Reflect::try_apply` implementation.
pub(crate) fn generate_try_apply_variants(
    reflect_enum: &ReflectEnum,
    this: &Ident,
) -> EnumVariantOutputData {
    struct TryApplyVariantBuilder<'a> {
        bevy_reflect_path: &'a Path,
    }

    impl<'a> VariantBuilder for TryApplyVariantBuilder<'a> {
        fn unwrap_field(&self, ident: &Ident, field: VariantField) -> TokenStream {
            let VariantField {
                member,
                variant_name,
                ..
            } = field;

            let bevy_reflect_path = self.bevy_reflect_path;

            let field_name = match member {
                Member::Named(member_ident) => format!("{member_ident}"),
                Member::Unnamed(member_index) => format!(".{}", member_index.index),
            };

            quote! {
                #ident.ok_or(#bevy_reflect_path::ApplyError::MissingEnumField {
                    variant_name: ::core::convert::Into::into(#variant_name),
                    field_name: ::core::convert::Into::into(#field_name)
                })?
            }
        }

        fn construct_field(&self, ident: &Ident, field: VariantField) -> TokenStream {
            let bevy_reflect_path = self.bevy_reflect_path;
            let field_ty = &field.field.data.ty;

            quote! {
                <#field_ty as #bevy_reflect_path::FromReflect>::from_reflect(#ident)
                    .ok_or(#bevy_reflect_path::ApplyError::MismatchedTypes {
                        from_type: ::core::convert::Into::into(
                            #bevy_reflect_path::DynamicTypePath::reflect_type_path(#ident)
                        ),
                        to_type: ::core::convert::Into::into(<#field_ty as #bevy_reflect_path::TypePath>::type_path())
                    })?
            }
        }
    }

    EnumVariantOutputBuilder::new(
        this,
        reflect_enum,
        TryApplyVariantBuilder {
            bevy_reflect_path: reflect_enum.meta().bevy_reflect_path(),
        },
    )
    .build()
}
