use crate::{
    StructInfo, StructVariantInfo, TupleInfo, TupleStructInfo, TupleVariantInfo, Type,
    TypeRegistration, TypeRegistry,
};
use serde::de::Error;

/// A helper trait for getting the [`TypeRegistration`] of a particular field.
pub(super) trait GetFieldRegistration {
    fn get_field_registration<'a, E: Error>(
        &self,
        index: usize,
        registry: &'a TypeRegistry,
    ) -> Result<&'a TypeRegistration, E>;
}

impl GetFieldRegistration for StructInfo {
    fn get_field_registration<'a, E: Error>(
        &self,
        index: usize,
        registry: &'a TypeRegistry,
    ) -> Result<&'a TypeRegistration, E> {
        let field = self.field_at(index).ok_or_else(|| {
            Error::custom(format_args!(
                "no field at index {} on struct {}",
                index,
                self.type_path(),
            ))
        })?;
        try_get_registration(*field.ty(), registry)
    }
}

impl GetFieldRegistration for StructVariantInfo {
    fn get_field_registration<'a, E: Error>(
        &self,
        index: usize,
        registry: &'a TypeRegistry,
    ) -> Result<&'a TypeRegistration, E> {
        let field = self.field_at(index).ok_or_else(|| {
            Error::custom(format_args!(
                "no field at index {} on variant {}",
                index,
                self.name(),
            ))
        })?;
        try_get_registration(*field.ty(), registry)
    }
}

impl GetFieldRegistration for TupleInfo {
    fn get_field_registration<'a, E: Error>(
        &self,
        index: usize,
        registry: &'a TypeRegistry,
    ) -> Result<&'a TypeRegistration, E> {
        let field = self.field_at(index).ok_or_else(|| {
            Error::custom(format_args!(
                "no field at index {} on tuple {}",
                index,
                self.type_path(),
            ))
        })?;
        try_get_registration(*field.ty(), registry)
    }
}

impl GetFieldRegistration for TupleStructInfo {
    fn get_field_registration<'a, E: Error>(
        &self,
        index: usize,
        registry: &'a TypeRegistry,
    ) -> Result<&'a TypeRegistration, E> {
        let field = self.field_at(index).ok_or_else(|| {
            Error::custom(format_args!(
                "no field at index {} on tuple struct {}",
                index,
                self.type_path(),
            ))
        })?;
        try_get_registration(*field.ty(), registry)
    }
}

impl GetFieldRegistration for TupleVariantInfo {
    fn get_field_registration<'a, E: Error>(
        &self,
        index: usize,
        registry: &'a TypeRegistry,
    ) -> Result<&'a TypeRegistration, E> {
        let field = self.field_at(index).ok_or_else(|| {
            Error::custom(format_args!(
                "no field at index {} on tuple variant {}",
                index,
                self.name(),
            ))
        })?;
        try_get_registration(*field.ty(), registry)
    }
}

/// Attempts to find the [`TypeRegistration`] for a given [type].
///
/// [type]: Type
pub(super) fn try_get_registration<E: Error>(
    ty: Type,
    registry: &TypeRegistry,
) -> Result<&TypeRegistration, E> {
    let registration = registry
        .get(ty.id())
        .ok_or_else(|| Error::custom(format_args!("no registration found for type `{ty:?}`")))?;
    Ok(registration)
}
