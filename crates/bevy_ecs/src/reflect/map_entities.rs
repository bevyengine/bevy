use crate::entity::{EntityMapper, MapEntities};
use bevy_reflect::{FromReflect, FromType, PartialReflect};

/// For a specific type of value, this maps any fields with values of type [`Entity`] to a new world.
///
/// Since a given `Entity` ID is only valid for the world it came from, when performing deserialization
/// any stored IDs need to be re-allocated in the destination world.
///
/// See [`EntityMapper`] and [`MapEntities`] for more information.
///
/// [`Entity`]: crate::entity::Entity
/// [`EntityMapper`]: crate::entity::EntityMapper
#[derive(Clone)]
pub struct ReflectMapEntities {
    map_entities: fn(&mut dyn PartialReflect, &mut dyn EntityMapper),
}

impl ReflectMapEntities {
    /// A general method for remapping entities in a reflected value via an [`EntityMapper`].
    ///
    /// # Panics
    /// Panics if the type of the reflected value doesn't match.
    pub fn map_entities(&self, reflected: &mut dyn PartialReflect, mapper: &mut dyn EntityMapper) {
        (self.map_entities)(reflected, mapper);
    }
}

impl<C: FromReflect + MapEntities> FromType<C> for ReflectMapEntities {
    fn from_type() -> Self {
        ReflectMapEntities {
            map_entities: |reflected, mut mapper| {
                let mut concrete = C::from_reflect(reflected).expect("reflected type should match");
                concrete.map_entities(&mut mapper);
                reflected.apply(&concrete);
            },
        }
    }
}
