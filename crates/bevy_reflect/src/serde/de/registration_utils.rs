use crate::{serde::de::error_utils::make_custom_error, Type, TypeRegistration, TypeRegistry};
use serde::de::Error;

/// Attempts to find the [`TypeRegistration`] for a given [type].
///
/// [type]: Type
pub(super) fn try_get_registration<E: Error>(
    ty: Type,
    registry: &TypeRegistry,
) -> Result<&TypeRegistration, E> {
    let registration = registry.get(ty.id()).ok_or_else(|| {
        make_custom_error(format_args!("no registration found for type `{ty:?}`"))
    })?;
    Ok(registration)
}
