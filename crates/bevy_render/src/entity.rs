use crate::{mesh::Mesh, ActiveCamera, ActiveCamera2d, Camera, Renderable, OrthographicCamera, PerspectiveCamera};
use bevy_asset::Handle;
use bevy_derive::EntityArchetype;
use bevy_transform::components::{LocalToWorld, Rotation, Scale, Translation};

#[derive(EntityArchetype, Default)]
pub struct MeshMaterialEntity<T: Default + Send + Sync + 'static> {
    pub mesh: Handle<Mesh>,
    pub material: Handle<T>,
    pub renderable: Renderable,
    pub local_to_world: LocalToWorld,
    pub translation: Translation,
    pub rotation: Rotation,
    pub scale: Scale,
}

#[derive(EntityArchetype, Default)]
pub struct CameraEntity {
    pub camera: Camera,
    pub perspective_camera: PerspectiveCamera,
    pub active_camera: ActiveCamera,
    pub local_to_world: LocalToWorld,
}

#[derive(EntityArchetype, Default)]
pub struct OrthographicCameraEntity {
    pub camera: Camera,
    pub orthographic_camera: OrthographicCamera,
    pub active_camera_2d: ActiveCamera2d,
}