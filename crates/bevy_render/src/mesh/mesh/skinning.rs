use bevy_asset::{Asset, Handle};
use bevy_ecs::{
    component::Component, entity::Entity, prelude::ReflectComponent, reflect::ReflectMapEntities,
};
use bevy_math::Mat4;
use bevy_reflect::prelude::*;
use std::ops::Deref;

#[derive(Component, Debug, Default, Clone, Reflect)]
#[reflect(Component, MapEntities, Default, Debug)]
pub struct SkinnedMesh {
    pub inverse_bindposes: Handle<SkinnedMeshInverseBindposes>,
    pub joints: Vec<Entity>,
}

impl<'a> IntoIterator for &'a mut SkinnedMesh {
    type IntoIter = std::slice::IterMut<'a, Entity>;
    type Item = <Self::IntoIter as Iterator>::Item;

    fn into_iter(self) -> Self::IntoIter {
        self.joints.iter_mut()
    }
}

impl<'a> IntoIterator for &'a SkinnedMesh {
    type IntoIter = std::iter::Copied<std::slice::Iter<'a, Entity>>;
    type Item = <Self::IntoIter as Iterator>::Item;

    fn into_iter(self) -> Self::IntoIter {
        self.joints.iter().copied()
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
