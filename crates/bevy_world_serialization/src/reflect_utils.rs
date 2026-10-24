use bevy_reflect::{dynamic::DynamicValue, Reflect, ReflectFromDynamic, TypeRegistration};

/// Attempts to clone a [`Reflect`] value using various methods.
///
/// This first attempts to clone via [`Reflect::reflect_clone`].
/// then falls back to [`ReflectFromDynamic::from_dynamic`],
/// and finally [`Reflect::to_dynamic`] if the first two methods fail.
///
/// This helps ensure that the original type and type data is retained,
/// and only returning a dynamic type if all other methods fail.
pub(super) fn clone_reflect_value(
    value: &dyn Reflect,
    type_registration: &TypeRegistration,
) -> Box<dyn Reflect> {
    value
        .reflect_clone()
        .map(Reflect::into_partial_reflect)
        .unwrap_or_else(|_| {
            type_registration
                .data::<ReflectFromDynamic>()
                .and_then(|fr| fr.from_dynamic(value.as_partial_reflect()))
                .map(Reflect::into_partial_reflect)
                .unwrap_or_else(|| {
                    DynamicValue::try_from_reflect(value)
                        .unwrap()
                        .as_boxed_partial_reflect()
                })
        })
}
