use bevy_asset::Handle;
use bevy_ecs::{
    component::Component,
    entity::{Entity, EntityMap, MapEntities, MapEntitiesError},
    prelude::ReflectComponent,
    reflect::ReflectMapEntities,
};
use bevy_math::Mat4;
use bevy_reflect::{Reflect, TypeUuid};
use std::ops::Deref;

#[derive(Component, Debug, Default, Clone, Reflect)]
#[reflect(Component, MapEntities)]
pub struct SkinnedMesh {
    pub inverse_bindposes: Handle<SkinnedMeshInverseBindposes>,
    pub joints: Vec<Entity>,
}

impl MapEntities for SkinnedMesh {
    fn map_entities(&mut self, entity_map: &EntityMap) -> Result<(), MapEntitiesError> {
        for joint in &mut self.joints {
            *joint = entity_map.get(*joint)?;
        }

        Ok(())
    }
}

#[derive(Debug, TypeUuid)]
#[uuid = "b9f155a9-54ec-4026-988f-e0a03e99a76f"]
pub struct SkinnedMeshInverseBindposes(Box<[Mat4]>);

impl From<Vec<Mat4>> for SkinnedMeshInverseBindposes {
    fn from(value: Vec<Mat4>) -> Self {
        Self(value.into_boxed_slice())
    }
}

impl Deref for SkinnedMeshInverseBindposes {
    type Target = [Mat4];
    fn deref(&self) -> &Self::Target {
        &*self.0
    }
}
