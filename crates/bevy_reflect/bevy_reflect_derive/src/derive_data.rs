use crate::container_attributes::ReflectTraits;
use crate::field_attributes::{parse_field_attrs, ReflectFieldAttr, ReflectFieldIgnore};
use crate::utility::get_bevy_reflect_path;
use crate::{REFLECT_ATTRIBUTE_NAME, REFLECT_VALUE_ATTRIBUTE_NAME};
use syn::{Data, DataStruct, DeriveInput, Field, Fields, Generics, Ident, Meta, Path};

pub(crate) enum DeriveType {
    Struct,
    TupleStruct,
    UnitStruct,
    Value,
}

/// Represents a field on a struct or tuple struct.
pub(crate) struct StructField<'a> {
    /// The raw field.
    pub data: &'a Field,
    /// The reflection-based attributes on the field.
    pub attrs: ReflectFieldAttr,
    /// The index of this field within the struct.
    pub index: usize,
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
pub(crate) struct ReflectDeriveData<'a> {
    derive_type: DeriveType,
    traits: ReflectTraits,
    type_name: &'a Ident,
    generics: &'a Generics,
    fields: Vec<StructField<'a>>,
    bevy_reflect_path: Path,
}

impl<'a> ReflectDeriveData<'a> {
    pub fn from_input(input: &'a DeriveInput) -> Result<Self, syn::Error> {
        let mut output = Self {
            type_name: &input.ident,
            derive_type: DeriveType::Value,
            generics: &input.generics,
            fields: Vec::new(),
            traits: ReflectTraits::default(),
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
                    output.traits = ReflectTraits::from_nested_metas(&meta_list.nested);
                } else if ident == REFLECT_VALUE_ATTRIBUTE_NAME {
                    force_reflect_value = true;
                    output.traits = ReflectTraits::from_nested_metas(&meta_list.nested);
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
            .map(|(index, field)| {
                let attrs = parse_field_attrs(&field.attrs).unwrap_or_else(|err| {
                    if let Some(ref mut errors) = errors {
                        errors.combine(err);
                    } else {
                        errors = Some(err);
                    }
                    ReflectFieldAttr::default()
                });

                StructField {
                    index,
                    attrs,
                    data: field,
                }
            })
            .collect::<Vec<StructField>>();
        if let Some(errs) = errors {
            return Err(errs);
        }

        Ok(output)
    }

    /// Get an iterator over the fields satisfying the given predicate
    fn fields_with<F: FnMut(&&StructField) -> bool>(
        &self,
        predicate: F,
    ) -> impl Iterator<Item = &StructField<'a>> {
        self.fields.iter().filter(predicate)
    }

    /// Get a collection of all field types satisfying the given predicate
    fn types_with<F: FnMut(&&StructField) -> bool>(&self, predicate: F) -> Vec<syn::Type> {
        self.fields_with(predicate)
            .map(|field| field.data.ty.clone())
            .collect::<Vec<_>>()
    }

    /// Get a collection of types which are exposed to either the scene serialization or reflection API
    pub fn active_types(&self) -> Vec<syn::Type> {
        self.types_with(|field| field.attrs.ignore != ReflectFieldIgnore::IgnoreAlways)
    }

    /// Get a collection of fields which are exposed to the reflection API
    pub fn active_fields(&self) -> impl Iterator<Item = &StructField<'a>> {
        self.fields_with(|field| field.attrs.ignore != ReflectFieldIgnore::IgnoreAlways)
    }

    /// Get a collection of fields which are ignored from the reflection API
    pub fn ignored_fields(&self) -> impl Iterator<Item = &StructField<'a>> {
        self.fields_with(|field| field.attrs.ignore != ReflectFieldIgnore::None)
    }

    /// The [`DeriveType`] of this struct.
    pub fn derive_type(&self) -> &DeriveType {
        &self.derive_type
    }

    /// The registered reflect traits on this struct.
    pub fn traits(&self) -> &ReflectTraits {
        &self.traits
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
    pub fn fields(&self) -> &[StructField<'a>] {
        &self.fields
    }

    /// The cached `bevy_reflect` path.
    pub fn bevy_reflect_path(&self) -> &Path {
        &self.bevy_reflect_path
    }

    /// Returns the `GetTypeRegistration` impl as a `TokenStream`.
    pub fn get_type_registration(&self) -> proc_macro2::TokenStream {
        let mut idxs = Vec::default();
        self.fields.iter().fold(0, |next_idx, field| {
            if field.attrs.ignore == ReflectFieldIgnore::IgnoreAlways {
                idxs.push(next_idx);
                next_idx
            } else if field.attrs.ignore == ReflectFieldIgnore::IgnoreSerialization {
                idxs.push(next_idx);
                next_idx + 1
            } else {
                next_idx + 1
            }
        });

        crate::registration::impl_get_type_registration(
            self.type_name,
            &self.bevy_reflect_path,
            self.traits.idents(),
            self.generics,
            &idxs,
        )
    }
}
