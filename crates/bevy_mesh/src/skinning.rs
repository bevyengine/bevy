use bevy_asset::{Asset, Handle};
use bevy_ecs::{
    component::Component,
    entity::{Entity, VisitEntities, VisitEntitiesMut},
    prelude::ReflectComponent,
    reflect::{ReflectMapEntities, ReflectVisitEntities, ReflectVisitEntitiesMut},
};
use bevy_math::Mat4;
use bevy_reflect::prelude::*;
use core::ops::Deref;

#[derive(Component, Debug, Default, Clone, Reflect, VisitEntities, VisitEntitiesMut)]
#[reflect(
    Component,
    MapEntities,
    VisitEntities,
    VisitEntitiesMut,
    Default,
    Debug
)]
pub struct SkinnedMesh {
    #[visit_entities(ignore)]
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
