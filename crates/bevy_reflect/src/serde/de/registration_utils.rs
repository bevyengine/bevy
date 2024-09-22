use crate::serde::de::error_utils::make_custom_error;
use crate::{Type, TypeInfo, TypeRegistration, TypeRegistry};
use serde::de::Error;

/// A helper enum for indicating how to deserialize a type.
#[derive(Copy, Clone)]
pub(super) enum RegistrationData<'a> {
    /// The data to deserialize is exactly the type that was registered.
    Concrete(&'a TypeRegistration),
    /// The data to deserialize is a dynamic type.
    #[cfg_attr(
        not(feature = "debug_stack"),
        expect(
            dead_code,
            reason = "the field is only read when the `debug_stack` feature is enabled"
        )
    )]
    Dynamic(Type),
}

/// Attempts to return the [`RegistrationData`] for the given type.
///
/// For all types, `ty` should be the actual type of the value
/// (e.g. `i32` for `i32` and `DynamicStruct` for `DynamicStruct`).
///
/// For dynamic types, `info` should be `None`.
pub(super) fn try_get_registration_data<'a, E: Error>(
    ty: Type,
    info: Option<&TypeInfo>,
    registry: &'a TypeRegistry,
) -> Result<RegistrationData<'a>, E> {
    let Some(info) = info else {
        // The given `TypeInfo` represents a dynamic type
        return Ok(RegistrationData::Dynamic(ty));
    };

    let ty = info.ty();
    let registration = registry.get(ty.id()).ok_or_else(|| {
        make_custom_error(format_args!("no registration found for type `{ty:?}`"))
    })?;
    Ok(RegistrationData::Concrete(registration))
}
