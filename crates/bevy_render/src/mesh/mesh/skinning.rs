use bevy_asset::{Asset, Handle};
use bevy_ecs::{
    component::Component,
    entity::{Entity, IterEntities},
    prelude::ReflectComponent,
    reflect::{ReflectMapEntities, ReflectVisitEntities},
};
use bevy_math::Mat4;
use bevy_reflect::prelude::*;
use std::ops::Deref;

#[derive(Component, Debug, Default, Clone, Reflect, IterEntities)]
#[reflect(Component, MapEntities, VisitEntities, Default, Debug)]
pub struct SkinnedMesh {
    #[iter_entities(ignore)]
    pub inverse_bindposes: Handle<SkinnedMeshInverseBindposes>,
    pub joints: Vec<Entity>,
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
