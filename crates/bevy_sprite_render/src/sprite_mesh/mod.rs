use bevy_app::{Plugin, Update};
use bevy_ecs::{
    entity::Entity,
    message::MessageReader,
    query::{Added, Changed},
    system::{Commands, Local, Query, Res, ResMut},
};

use bevy_asset::{AssetEvent, Assets, Handle};

use bevy_image::Image;
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

        app.add_systems(
            Update,
            (add_mesh, update_material_component, update_material_image),
        );
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

// Insert the material when SpriteMesh is added / changed and the image is already loaded.
fn update_material_component(
    sprites: Query<(Entity, &SpriteMesh), Changed<SpriteMesh>>,
    images: Res<Assets<Image>>,
    mut materials: ResMut<Assets<SpriteMaterial>>,
    mut commands: Commands,
) {
    for (entity, sprite) in sprites {
        if let Some(image) = images.get(sprite.image.id()) {
            let material = SpriteMaterial::from_sprite_mesh(sprite.clone(), image.size());

            commands
                .entity(entity)
                .insert(MeshMaterial2d(materials.add(material)));
        }
    }
}

// SpriteMaterial needs to know the sprite image size.

// This system checks every SpriteMesh when an image is loaded / modified to potentially update or insert its material.
// Could be optimized, e.g. by storing an (AssetId<Image>, Entity) hashmap.
// Alternatively, we could queue up new SpriteMeshes when they're added, but lose hot reloading.

// The normal Sprites try accessing the image from the Res<RenderAssets<GpuImage>> in the
// Prepare render step instead.

fn update_material_image(
    mut image_events: MessageReader<AssetEvent<Image>>,
    images: Res<Assets<Image>>,
    sprites: Query<(Entity, &SpriteMesh)>,
    mut materials: ResMut<Assets<SpriteMaterial>>,
    mut commands: Commands,
) {
    for event in image_events.read() {
        match event {
            AssetEvent::Added { id } => {
                if let Some((entity, sprite)) = sprites
                    .iter()
                    .find(|(_, sprite_mesh)| sprite_mesh.image.id() == *id)
                {
                    let image = images.get(*id).unwrap();
                    let material = SpriteMaterial::from_sprite_mesh(sprite.clone(), image.size());

                    commands
                        .entity(entity)
                        .insert(MeshMaterial2d(materials.add(material)));
                }
            }
            AssetEvent::Modified { id } => {
                if let Some((entity, sprite)) = sprites
                    .iter()
                    .find(|(_, sprite_mesh)| sprite_mesh.image.id() == *id)
                {
                    let image = images.get(*id).unwrap();
                    let material = SpriteMaterial::from_sprite_mesh(sprite.clone(), image.size());

                    commands
                        .entity(entity)
                        .insert(MeshMaterial2d(materials.add(material)));
                }
            }
            _ => {}
        }
    }
}
