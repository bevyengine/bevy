use crate::container_attributes::ReflectAttrs;
use crate::field_attributes::{
    parse_field_attrs, ReflectFieldAttr, REFLECT_ATTRIBUTE_NAME, REFLECT_VALUE_ATTRIBUTE_NAME,
};
use crate::utility::get_bevy_reflect_path;
use syn::{Data, DataStruct, DeriveInput, Field, Fields, Generics, Ident, Meta, Path};

pub enum DeriveType {
    Struct,
    TupleStruct,
    UnitStruct,
    Value,
}

/// Data used by derive macros for `Reflect` and `FromReflect`
///
/// # Example
/// ```ignore
/// //                          attrs
/// //        |----------------------------------------|
/// #[reflect(PartialEq, Serialize, Deserialize, Default)]
/// //            type_name       generics
/// //     |-------------------||----------|
/// struct ThingThatImReflecting<T1, T2, T3> {
///     x: T1, // |
///     y: T2, // |- fields
///     z: T3  // |
/// }
/// ```
pub struct ReflectDeriveData<'a> {
    derive_type: DeriveType,
    attrs: ReflectAttrs,
    type_name: &'a Ident,
    generics: &'a Generics,
    fields: Vec<(&'a Field, ReflectFieldAttr, usize)>,
    bevy_reflect_path: Path,
}

impl<'a> ReflectDeriveData<'a> {
    pub fn from_input(input: &'a DeriveInput) -> Result<Self, syn::Error> {
        let mut output = Self {
            type_name: &input.ident,
            derive_type: DeriveType::Value,
            generics: &input.generics,
            fields: Vec::new(),
            attrs: ReflectAttrs::default(),
            bevy_reflect_path: get_bevy_reflect_path(),
        };

        // Should indicate whether `#[reflect_value]` was used
        let mut force_reflect_value = false;

        for attribute in input.attrs.iter().filter_map(|attr| attr.parse_meta().ok()) {
            let meta_list = if let Meta::List(meta_list) = attribute {
                meta_list
            } else {
                continue;
            };

            if let Some(ident) = meta_list.path.get_ident() {
                if ident == REFLECT_ATTRIBUTE_NAME {
                    output.attrs = ReflectAttrs::from_nested_metas(&meta_list.nested);
                } else if ident == REFLECT_VALUE_ATTRIBUTE_NAME {
                    force_reflect_value = true;
                    output.attrs = ReflectAttrs::from_nested_metas(&meta_list.nested);
                }
            }
        }

        let fields = match &input.data {
            Data::Struct(DataStruct {
                fields: Fields::Named(fields),
                ..
            }) => {
                if !force_reflect_value {
                    output.derive_type = DeriveType::Struct;
                }
                &fields.named
            }
            Data::Struct(DataStruct {
                fields: Fields::Unnamed(fields),
                ..
            }) => {
                if !force_reflect_value {
                    output.derive_type = DeriveType::TupleStruct;
                }
                &fields.unnamed
            }
            Data::Struct(DataStruct {
                fields: Fields::Unit,
                ..
            }) => {
                if !force_reflect_value {
                    output.derive_type = DeriveType::UnitStruct;
                }
                return Ok(output);
            }
            _ => {
                return Ok(output);
            }
        };

        let mut errors: Option<syn::Error> = None;
        output.fields = fields
            .iter()
            .enumerate()
            .map(|(i, f)| {
                (
                    f,
                    parse_field_attrs(&f.attrs).unwrap_or_else(|err| {
                        if let Some(ref mut error) = errors {
                            error.combine(err);
                        } else {
                            errors = Some(err);
                        }
                        ReflectFieldAttr::default()
                    }),
                    i,
                )
            })
            .collect::<Vec<(&Field, ReflectFieldAttr, usize)>>();

        if let Some(errs) = errors {
            return Err(errs);
        }

        Ok(output)
    }

    /// Get an iterator over the active fields
    pub fn active_fields(&self) -> impl Iterator<Item = &(&Field, ReflectFieldAttr, usize)> {
        self.fields
            .iter()
            .filter(|(_field, attrs, _i)| !attrs.ignore.unwrap_or(false))
    }

    /// Get an iterator over the ignored fields
    pub fn ignored_fields(&self) -> impl Iterator<Item = &(&Field, ReflectFieldAttr, usize)> {
        self.fields
            .iter()
            .filter(|(_field, attrs, _i)| attrs.ignore.unwrap_or(false))
    }

    /// Get a collection of all active types
    pub fn active_types(&self) -> Vec<syn::Type> {
        self.active_fields()
            .map(|(field, ..)| field.ty.clone())
            .collect::<Vec<_>>()
    }

    /// The [`DeriveType`] of this struct.
    pub fn derive_type(&self) -> &DeriveType {
        &self.derive_type
    }

    /// The list of reflect-related attributes on this struct.
    pub fn attrs(&self) -> &ReflectAttrs {
        &self.attrs
    }

    /// The name of this struct.
    pub fn type_name(&self) -> &'a Ident {
        self.type_name
    }

    /// The generics associated with this struct.
    pub fn generics(&self) -> &'a Generics {
        self.generics
    }

    /// The complete set of fields in this struct.
    #[allow(dead_code)]
    pub fn fields(&self) -> &Vec<(&'a Field, ReflectFieldAttr, usize)> {
        &self.fields
    }

    /// The cached `bevy_reflect` path.
    pub fn bevy_reflect_path(&self) -> &Path {
        &self.bevy_reflect_path
    }

    /// Returns the `GetTypeRegistration` impl as a `TokenStream`.
    pub fn get_type_registration(&self) -> proc_macro2::TokenStream {
        crate::registration::impl_get_type_registration(
            self.type_name,
            &self.bevy_reflect_path,
            self.attrs.data(),
            self.generics,
        )
    }
}
