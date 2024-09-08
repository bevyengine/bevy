use bevy_asset::{Asset, Handle};
use bevy_ecs::{
    component::Component,
    entity::{Entity, EntityMapper, MapEntities},
    prelude::ReflectComponent,
    reflect::ReflectMapEntities,
};
use bevy_math::Mat4;
use bevy_reflect::prelude::*;
use std::ops::Deref;

#[derive(Component, Debug, Default, Clone, Reflect)]
#[reflect(Component, MapEntities, Default)]
pub struct SkinnedMesh {
    pub inverse_bindposes: Handle<SkinnedMeshInverseBindposes>,
    pub joints: Vec<Entity>,
}

impl MapEntities for SkinnedMesh {
    fn map_entities<M: EntityMapper>(&mut self, entity_mapper: &mut M) {
        for joint in &mut self.joints {
            *joint = entity_mapper.map_entity(*joint);
        }
    }
}

#[derive(Asset, TypePath, Debug)]
pub struct SkinnedMeshInverseBindposes(Box<[Mat4]>);

impl From<Vec<Mat4>> for SkinnedMeshInverseBindposes {
    fn from(value: Vec<Mat4>) -> Self {
        Self(value.into_boxed_slice())
    }
}

impl Deref for SkinnedMeshInverseBindposes {
    type Target = [Mat4];
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
