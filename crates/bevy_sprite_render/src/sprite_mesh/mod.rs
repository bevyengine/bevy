use bevy_app::{Plugin, Startup, Update};
use bevy_ecs::{
    entity::Entity,
    query::Added,
    resource::Resource,
    system::{Commands, Query, Res, ResMut},
};

use bevy_asset::{Assets, Handle};

use bevy_math::{primitives::Rectangle, vec2, vec3};
use bevy_mesh::{Mesh, Mesh2d};

use bevy_sprite::prelude::SpriteMesh;

mod sprite_material;
use bevy_transform::components::Transform;
pub use sprite_material::*;

use crate::MeshMaterial2d;

pub struct SpriteMeshPlugin;

impl Plugin for SpriteMeshPlugin {
    fn build(&self, app: &mut bevy_app::App) {
        app.init_resource::<SpriteQuad>();
        app.add_plugins(SpriteMaterialPlugin);

        app.add_systems(Startup, init_quad);
        app.add_systems(Update, add_mesh);
    }
}

/// Handle to a Mesh Quad / Rectangle that will be cloned for each `SpriteMesh`.
#[derive(Resource, Default)]
pub struct SpriteQuad(pub Option<Handle<Mesh>>);

fn init_quad(mut quad: ResMut<SpriteQuad>, mut meshes: ResMut<Assets<Mesh>>) {
    quad.0 = Some(meshes.add(Rectangle::from_size(vec2(1.0, 1.0))));
}

fn add_mesh(
    sprites: Query<(Entity, &SpriteMesh), Added<SpriteMesh>>,
    quad: Res<SpriteQuad>,
    mut materials: ResMut<Assets<SpriteMaterial>>,
    mut commands: Commands,
) {
    let Some(quad) = &quad.0 else { return };

    for (entity, sprite) in sprites {
        let mut material: SpriteMaterial = sprite.clone().into();

        // TODO: replace this with the image's size
        material.scale = vec2(100.0, 100.0);

        commands.entity(entity).insert((
            Mesh2d(quad.clone()),
            MeshMaterial2d(materials.add(material)),
        ));
    }
}
