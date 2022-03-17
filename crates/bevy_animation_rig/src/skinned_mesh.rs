use bevy_asset::{Assets, Handle, HandleUntyped};
use bevy_ecs::{
    entity::{Entity, EntityMap, MapEntities, MapEntitiesError},
    reflect::{ReflectComponent, ReflectMapEntities},
    system::{Query, Res, ResMut},
};
use bevy_math::Mat4;
use bevy_reflect::{
    serde, DynamicStruct, FieldIter, Reflect, ReflectMut, ReflectRef, Struct, TypeUuid,
};
use bevy_transform::components::GlobalTransform;

/// The name of skinned mesh node
pub mod node {
    pub const SKINNED_MESH: &str = "skinned_mesh";
}

/// The name of skinned mesh buffer
pub mod buffer {
    pub const JOINT_TRANSFORMS: &str = "JointTransforms";
}
