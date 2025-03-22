use bevy_ecs::{
    component::Component,
    entity::Entity,
    query::Has,
    system::{Query, ResMut},
};
use bevy_math::Affine3;
use bevy_render::{
    batching::NoAutomaticBatching,
    mesh::{Mesh2d, MeshTag},
    view::ViewVisibility,
    Extract,
};
use bevy_transform::components::GlobalTransform;

use super::{
    bind_group::Material2dBindGroupId,
    instancing::{RenderMesh2dInstance, RenderMesh2dInstances},
};

#[derive(Component)]
pub struct Mesh2dTransforms {
    pub world_from_local: Affine3,
    pub flags: u32,
}

// NOTE: These must match the bit flags in bevy_sprite/src/mesh2d/mesh2d.wgsl!
bitflags::bitflags! {
    #[repr(transparent)]
    pub struct MeshFlags: u32 {
        const NONE                       = 0;
        const UNINITIALIZED              = 0xFFFF;
    }
}

#[derive(Component, Default)]
pub(super) struct Mesh2dMarker;

pub(super) fn extract_mesh2d(
    mut render_mesh_instances: ResMut<RenderMesh2dInstances>,
    query: Extract<
        Query<(
            Entity,
            &ViewVisibility,
            &GlobalTransform,
            &Mesh2d,
            Option<&MeshTag>,
            Has<NoAutomaticBatching>,
        )>,
    >,
) {
    render_mesh_instances.clear();

    for (entity, view_visibility, transform, handle, tag, no_automatic_batching) in &query {
        if !view_visibility.get() {
            continue;
        }
        render_mesh_instances.insert(
            entity.into(),
            RenderMesh2dInstance {
                transforms: Mesh2dTransforms {
                    world_from_local: (&transform.affine()).into(),
                    flags: MeshFlags::empty().bits(),
                },
                mesh_asset_id: handle.0.id(),
                material_bind_group_id: Material2dBindGroupId::default(),
                automatic_batching: !no_automatic_batching,
                tag: tag.map_or(0, |i| **i),
            },
        );
    }
}
