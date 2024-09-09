use crate::serde::de::error_utils::make_custom_error;
use crate::{TypeInfo, TypeRegistration, TypeRegistry};
use serde::de::Error;

/// Attempts to find the [`TypeRegistration`] for a given [type].
///
/// [type]: Type
pub(super) fn try_get_registration<'a, E: Error>(
    info: Option<&TypeInfo>,
    registry: &'a TypeRegistry,
) -> Result<Option<&'a TypeRegistration>, E> {
    let Some(info) = info else {
        // The given `TypeInfo` represents a dynamic type
        return Ok(None);
    };

    let ty = info.ty();
    let registration = registry.get(ty.id()).ok_or_else(|| {
        make_custom_error(format_args!("no registration found for type `{ty:?}`"))
    })?;
    Ok(Some(registration))
}
