use crate::serde::ser::error_utils::make_custom_error;
#[cfg(feature = "debug_stack")]
use crate::serde::ser::error_utils::TYPE_INFO_STACK;
use crate::serde::ReflectSerializeWithRegistry;
use crate::{PartialReflect, ReflectSerialize, TypeRegistry};
use core::borrow::Borrow;
use serde::{Serialize, Serializer};

/// Attempts to serialize a [`PartialReflect`] value with custom [`ReflectSerialize`]
/// or [`ReflectSerializeWithRegistry`] type data.
///
/// On success, returns the result of the serialization.
/// On failure, returns the original serializer and the error that occurred.
pub(super) fn try_custom_serialize<S: Serializer>(
    value: &dyn PartialReflect,
    type_registry: &TypeRegistry,
    serializer: S,
) -> Result<Result<S::Ok, S::Error>, (S, S::Error)> {
    let Some(value) = value.try_as_reflect() else {
        return Err((
            serializer,
            make_custom_error(format_args!(
                "type `{}` does not implement `Reflect`",
                value.reflect_type_path()
            )),
        ));
    };

    let info = value.reflect_type_info();

    let Some(registration) = type_registry.get(info.type_id()) else {
        return Err((
            serializer,
            make_custom_error(format_args!(
                "type `{}` is not registered in the type registry",
                info.type_path(),
            )),
        ));
    };

    if let Some(reflect_serialize) = registration.data::<ReflectSerialize>() {
        #[cfg(feature = "debug_stack")]
        TYPE_INFO_STACK.with_borrow_mut(crate::type_info_stack::TypeInfoStack::pop);

        Ok(reflect_serialize
            .get_serializable(value)
            .borrow()
            .serialize(serializer))
    } else if let Some(reflect_serialize_with_registry) =
        registration.data::<ReflectSerializeWithRegistry>()
    {
        #[cfg(feature = "debug_stack")]
        TYPE_INFO_STACK.with_borrow_mut(crate::type_info_stack::TypeInfoStack::pop);

        Ok(reflect_serialize_with_registry.serialize(value, serializer, type_registry))
    } else {
        Err((serializer, make_custom_error(format_args!(
            "type `{}` did not register the `ReflectSerialize` or `ReflectSerializeWithRegistry` type data. For certain types, this may need to be registered manually using `register_type_data`",
            info.type_path(),
        ))))
    }
}
