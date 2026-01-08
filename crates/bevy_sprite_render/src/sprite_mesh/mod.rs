use bevy_app::{Plugin, Update};
use bevy_ecs::{
    entity::Entity,
    query::{Added, Changed},
    system::{Commands, Local, Query, ResMut},
};

use bevy_asset::{Assets, Handle};

use bevy_math::{primitives::Rectangle, vec2};
use bevy_mesh::{Mesh, Mesh2d};

use bevy_sprite::prelude::SpriteMesh;

mod sprite_material;
pub use sprite_material::*;

use crate::MeshMaterial2d;

pub struct SpriteMeshPlugin;

impl Plugin for SpriteMeshPlugin {
    fn build(&self, app: &mut bevy_app::App) {
        app.add_plugins(SpriteMaterialPlugin);

        app.add_systems(Update, (add_mesh, add_material));
    }
}

// Insert a Mesh2d quad each time the SpriteMesh component is added.
// The mesh handle is kept locally so it can be cloned.
fn add_mesh(
    sprites: Query<Entity, Added<SpriteMesh>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut quad: Local<Option<Handle<Mesh>>>,
    mut commands: Commands,
) {
    if quad.is_none() {
        *quad = Some(meshes.add(Rectangle::from_size(vec2(1.0, 1.0))));
    }

    for entity in sprites {
        commands
            .entity(entity)
            .insert(Mesh2d(quad.clone().unwrap()));
    }
}

// Insert the material when SpriteMesh is added / changed.
fn add_material(
    sprites: Query<(Entity, &SpriteMesh), Changed<SpriteMesh>>,
    mut materials: ResMut<Assets<SpriteMaterial>>,
    mut commands: Commands,
) {
    for (entity, sprite) in sprites {
        let material = SpriteMaterial::from_sprite_mesh(sprite.clone());

        commands
            .entity(entity)
            .insert(MeshMaterial2d(materials.add(material)));
    }
}
