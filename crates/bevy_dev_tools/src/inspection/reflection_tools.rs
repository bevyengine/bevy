//! Helpers for working with reflected values during inspection.

use bevy_ecs::{entity::Entity, world::World};
use bevy_utils::prelude::ShortName;
use core::any::TypeId;

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
