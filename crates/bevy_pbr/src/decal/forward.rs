use bevy_app::prelude::{App, Plugin};
use bevy_asset::{load_internal_asset, Assets, Handle};
use bevy_ecs::prelude::Component;
use bevy_math::{prelude::Rectangle, Quat, Vec2, Vec3};
use bevy_render::{
    mesh::{Mesh, Mesh3d, MeshBuilder, Meshable},
    render_resource::Shader,
};

const FORWARD_DECAL_MESH_HANDLE: Handle<Mesh> = Handle::weak_from_u128(09376620402995522466);
const FORWARD_DECAL_SHADER_HANDLE: Handle<Shader> = Handle::weak_from_u128(19376620402995522466);

/// TODO: Docs.
pub struct ForwardDecalPlugin;

impl Plugin for ForwardDecalPlugin {
    fn build(&self, app: &mut App) {
        let plane_mesh = Rectangle::from_size(Vec2::ONE)
            .mesh()
            .build()
            .rotated_by(Quat::from_rotation_arc(Vec3::Z, Vec3::Y))
            .with_generated_tangents()
            .unwrap();

        app.world_mut()
            .resource_mut::<Assets<Mesh>>()
            .insert(FORWARD_DECAL_MESH_HANDLE.id(), plane_mesh);

        load_internal_asset!(
            app,
            FORWARD_DECAL_SHADER_HANDLE,
            "forward_decal.wgsl",
            Shader::from_wgsl
        );
    }
}

/// TODO: Docs.
#[derive(Component)]
#[require(Mesh3d(|| Mesh3d(FORWARD_DECAL_MESH_HANDLE)))]
pub struct ForwardDecal;
