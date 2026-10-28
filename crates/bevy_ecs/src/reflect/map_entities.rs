use crate::entity::{EntityMapper, MapEntities};
use bevy_reflect::{CreateTypeData, FromDynamic, Reflect};

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
    map_entities: fn(&mut dyn Reflect, &mut dyn EntityMapper),
}

impl ReflectMapEntities {
    /// A general method for remapping entities in a reflected value via an [`EntityMapper`].
    ///
    /// # Panics
    /// Panics if the type of the reflected value doesn't match.
    pub fn map_entities(&self, reflected: &mut dyn Reflect, mapper: &mut dyn EntityMapper) {
        (self.map_entities)(reflected, mapper);
    }
}

impl<C: Reflect + FromDynamic + MapEntities> CreateTypeData<C> for ReflectMapEntities {
    fn create_type_data(_input: ()) -> Self {
        ReflectMapEntities {
            map_entities: |reflected, mut mapper| {
                let concrete = reflected
                    .downcast_mut::<C>()
                    .expect("reflected type should match");
                concrete.map_entities(&mut mapper);
            },
        }
    }
}
