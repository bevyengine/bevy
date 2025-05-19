use bevy_reflect::{PartialReflect, ReflectFromReflect, TypeRegistration};

/// Attempts to clone a [`PartialReflect`] value using various methods.
///
/// This first attempts to clone via [`PartialReflect::reflect_clone`].
/// then falls back to [`ReflectFromReflect::from_reflect`],
/// and finally [`PartialReflect::to_dynamic`] if the first two methods fail.
///
/// This helps ensure that the original type and type data is retained,
/// and only returning a dynamic type if all other methods fail.
pub(super) fn clone_reflect_value(
    value: &dyn PartialReflect,
    type_registration: &TypeRegistration,
) -> Box<dyn PartialReflect> {
    value
        .reflect_clone()
        .map(PartialReflect::into_partial_reflect)
        .unwrap_or_else(|_| {
            type_registration
                .data::<ReflectFromReflect>()
                .and_then(|fr| fr.from_reflect(value.as_partial_reflect()))
                .map(PartialReflect::into_partial_reflect)
                .unwrap_or_else(|| value.to_dynamic())
        })
}
