use crate::derive_data::StructField;
use crate::field_attributes::DefaultBehavior;
use crate::{derive_data::ReflectEnum, utility::ident_or_index};
use bevy_macro_utils::fq_std::{FQDefault, FQOption};
use proc_macro2::Ident;
use quote::{format_ident, quote};
use syn::Member;

pub(crate) struct EnumVariantOutputData {
    /// The names of each variant as a string.
    ///
    /// For example, `Some` and `None` for the `Option` enum.
    pub variant_names: Vec<String>,
    /// The pattern matching portion of each variant.
    ///
    /// For example, `Option::Some { 0: _0 }` and `Option::None {}` for the `Option` enum.
    #[allow(dead_code)]
    pub variant_patterns: Vec<proc_macro2::TokenStream>,
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

/// A builder for constructing [`EnumVariantOutputData`].
///
/// When a variant field needs to be constructed, the builder will:
/// 1. Use `field_accessor` to access the field as an `Option<&dyn Reflect>`.
/// 2. Use `field_unwrapper` to unwrap the field into a `&dyn Reflect`.
/// 3. Use `field_constructor` to construct the field into a concrete value.
struct EnumVariantOutputDataBuilder<'a> {
    /// A reference to the enum that is being reflected (e.g. `self`).
    this: &'a Ident,
    /// The reflect enum data.
    reflect_enum: &'a ReflectEnum<'a>,
    /// A function to generate tokens that will extract the field value from the enum
    /// as an `Option<&dyn Reflect>`.
    field_accessor: Box<dyn Fn(&Ident, VariantField) -> proc_macro2::TokenStream + 'a>,
    /// A function to generate tokens that will unwrap the accessed field value into a
    /// `&dyn Reflect`.
    field_unwrapper: Box<dyn Fn(&Ident, VariantField) -> proc_macro2::TokenStream + 'a>,
    /// A function to generate tokens that will construct a new concrete value from the accessed field.
    field_constructor: Box<dyn Fn(&Ident, VariantField) -> proc_macro2::TokenStream + 'a>,
}

impl<'a> EnumVariantOutputDataBuilder<'a> {
    pub fn new(this: &'a Ident, reflect_enum: &'a ReflectEnum<'a>) -> Self {
        Self {
            this,
            reflect_enum,
            field_accessor: Box::new(Self::generate_dynamic_field_accessor),
            field_unwrapper: Box::new(
                |_, _| quote! {::core::compile_error!("internal bevy_reflect error: failed to define field unwrapper")},
            ),
            field_constructor: Box::new(
                |_, _| quote! {::core::compile_error!("internal bevy_reflect error: failed to define field constructor")},
            ),
        }
    }

    pub fn with_field_unwrapper(
        mut self,
        unwrapper: impl Fn(&Ident, VariantField) -> proc_macro2::TokenStream + 'a,
    ) -> Self {
        self.field_unwrapper = Box::new(unwrapper);
        self
    }

    pub fn with_field_constructor(
        mut self,
        constructor: impl Fn(&Ident, VariantField) -> proc_macro2::TokenStream + 'a,
    ) -> Self {
        self.field_constructor = Box::new(constructor);
        self
    }

    pub fn build(self) -> EnumVariantOutputData {
        let variants = self.reflect_enum.variants();

        let mut variant_names = Vec::with_capacity(variants.len());
        let mut variant_patterns = Vec::with_capacity(variants.len());
        let mut variant_constructors = Vec::with_capacity(variants.len());

        for variant in variants {
            let variant_ident = &variant.data.ident;
            let variant_name = variant_ident.to_string();
            let variant_path = self.reflect_enum.get_unit(variant_ident);

            let fields = variant.fields();

            let (field_patterns, field_constructors): (Vec<_>, Vec<_>) = fields
                .iter()
                .map(|field| {
                    let member = ident_or_index(field.data.ident.as_ref(), field.declaration_index);
                    let alias = format_ident!("_{}", member);

                    let value = self.construct_field(VariantField {
                        member: &member,
                        variant_name: &variant_name,
                        field,
                    });

                    let pattern = quote! {
                        #member: #alias
                    };

                    let constructor = quote! {
                        #member: #value
                    };

                    (pattern, constructor)
                })
                .unzip();

            let pattern = quote! {
                #variant_path { #( #field_patterns ),* }
            };

            let constructor = quote! {
                #variant_path {
                    #( #field_constructors ),*
                }
            };

            variant_names.push(variant_name);
            variant_patterns.push(pattern);
            variant_constructors.push(constructor);
        }

        EnumVariantOutputData {
            variant_names,
            variant_patterns,
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

        let field_accessor = (self.field_accessor)(self.this, variant_field);

        let field_ident = format_ident!("__field");
        let field_constructor = (self.field_constructor)(&field_ident, variant_field);

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
                let field_unwrapper = (self.field_unwrapper)(&field_ident, variant_field);

                quote! {{
                    let #field_ident = #field_accessor;
                    let #field_ident = #field_unwrapper;
                    #field_constructor
                }}
            }
        }
    }

    /// Generates a dynamic field accessor for the given variant field.
    ///
    /// This will result in either `this.field("field_name")` or `this.field_at(index)`
    /// depending on whether the field is named or unnamed.
    ///
    /// By using this accessor, we can extract the field dynamically from a `dyn Enum`
    /// without having to know what it's concrete type is.
    fn generate_dynamic_field_accessor(
        this: &Ident,
        variant_field: VariantField,
    ) -> proc_macro2::TokenStream {
        match variant_field.member {
            Member::Named(ident) => {
                let name = ident.to_string();
                quote!(#this.field(#name))
            }
            Member::Unnamed(reflect_index) => {
                quote!(#this.field_at(#reflect_index))
            }
        }
    }
}

/// Generates the enum variant output data needed to build the `FromReflect::from_reflect` implementation.
pub(crate) fn generate_from_reflect_variants(
    reflect_enum: &ReflectEnum,
    this: &Ident,
) -> EnumVariantOutputData {
    let bevy_reflect_path = reflect_enum.meta().bevy_reflect_path();

    EnumVariantOutputDataBuilder::new(this, reflect_enum)
        .with_field_unwrapper(|ident, _| {
            quote! {
                #ident?
            }
        })
        .with_field_constructor(|ident, variant_field| {
            let field_ty = &variant_field.field.data.ty;

            quote! {
                <#field_ty as #bevy_reflect_path::FromReflect>::from_reflect(#ident)?
            }
        })
        .build()
}

/// Generates the enum variant output data needed to build the `Reflect::try_apply` implementation.
pub(crate) fn generate_try_apply_variants(
    reflect_enum: &ReflectEnum,
    this: &Ident,
) -> EnumVariantOutputData {
    let bevy_reflect_path = reflect_enum.meta().bevy_reflect_path();

    EnumVariantOutputDataBuilder::new(this, reflect_enum)
        .with_field_unwrapper(|ident, variant_field| {
            let VariantField {
                member,
                variant_name,
                ..
            } = variant_field;

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
        })
        .with_field_constructor(|ident, variant_field| {
            let field_ty = &variant_field.field.data.ty;

            quote! {
                <#field_ty as #bevy_reflect_path::FromReflect>::from_reflect(#ident)
                    .ok_or(#bevy_reflect_path::ApplyError::MismatchedTypes {
                        from_type: ::core::convert::Into::into(
                            #bevy_reflect_path::DynamicTypePath::reflect_type_path(#ident)
                        ),
                        to_type: ::core::convert::Into::into(<#field_ty as #bevy_reflect_path::TypePath>::type_path())
                    })?
            }
        })
        .build()
}
