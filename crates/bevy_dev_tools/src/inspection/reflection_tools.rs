//! Helpers for working with reflected values during inspection.

use bevy_ecs::{entity::Entity, world::World};
use bevy_reflect::{PartialReflect, ReflectCloneError, ReflectRef};
use bevy_utils::prelude::ShortName;
use core::any::TypeId;

/// Clones a reflected value, falling back to a dynamic copy when a concrete clone fails.
///
/// Opaque values have no dynamic form, so their clone error is returned as-is.
pub fn clone_incomplete(
    reflected: &dyn PartialReflect,
) -> Result<Box<dyn PartialReflect>, ReflectCloneError> {
    match reflected.reflect_clone() {
        Ok(cloned) => Ok(cloned.into_partial_reflect()),
        Err(err) => match reflected.reflect_ref() {
            ReflectRef::Opaque(_) => Err(err),
            _ => reflected.to_dynamic(),
        },
    }
}

/// Formats the value of the component identified by `type_id` on `entity` for debugging.
///
/// Type paths in the output are collapsed to short names unless `full_type_names` is set.
pub fn component_value_to_string(
    world: &World,
    entity: Entity,
    type_id: Option<TypeId>,
    full_type_names: bool,
) -> String {
    match type_id {
        Some(type_id) => match world.get_reflect(entity, type_id) {
            Ok(reflected) => {
                let value = format!("{reflected}");

                if full_type_names {
                    value
                } else {
                    ShortName::from(value.as_str()).to_string()
                }
            }
            Err(err) => format!("<Unreflectable: {err}>"),
        },
        None => "Dynamic Type".to_string(),
    }
}
