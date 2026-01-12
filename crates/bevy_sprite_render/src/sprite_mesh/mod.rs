use bevy_app::{Plugin, Update};
use bevy_ecs::{
    entity::Entity,
    query::{Added, Changed, Or},
    schedule::IntoScheduleConfigs,
    system::{Commands, Local, Query, Res, ResMut},
};

use bevy_asset::{Assets, Handle};

use bevy_image::TextureAtlasLayout;
use bevy_math::{primitives::Rectangle, vec2};
use bevy_mesh::{Mesh, Mesh2d};

use bevy_sprite::{prelude::SpriteMesh, Anchor};

mod sprite_material;
pub use sprite_material::*;
use tracing::warn;

use crate::MeshMaterial2d;

pub struct SpriteMeshPlugin;

impl Plugin for SpriteMeshPlugin {
    fn build(&self, app: &mut bevy_app::App) {
        app.add_plugins(SpriteMaterialPlugin);

        app.add_systems(Update, (add_mesh, add_material).chain());
    }
}

// Insert a Mesh2d quad and a MeshMaterial2d each time the SpriteMesh component is added.
// The mesh handle is kept locally so it can be cloned. The material is later mutated.
fn add_mesh(
    sprites: Query<Entity, Added<SpriteMesh>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut quad: Local<Option<Handle<Mesh>>>,
    mut materials: ResMut<Assets<SpriteMaterial>>,
    mut commands: Commands,
) {
    if quad.is_none() {
        *quad = Some(meshes.add(Rectangle::from_size(vec2(1.0, 1.0))));
    }

    for entity in sprites {
        commands.entity(entity).insert((
            Mesh2d(quad.clone().unwrap()),
            MeshMaterial2d(materials.add(SpriteMaterial::default())),
        ));
    }
}

// Mutate the material when SpriteMesh is added / changed.
//
// NOTE: This also adds the SpriteAtlasLayout into the SpriteMaterial,
// but this should instead be read later, similar to the images, allowing
// for hot reload.
fn add_material(
    sprites: Query<
        (&SpriteMesh, &Anchor, &MeshMaterial2d<SpriteMaterial>),
        Or<(
            Changed<SpriteMesh>,
            Changed<Anchor>,
            Added<MeshMaterial2d<SpriteMaterial>>,
        )>,
    >,
    texture_atlas_layouts: Res<Assets<TextureAtlasLayout>>,
    mut materials: ResMut<Assets<SpriteMaterial>>,
) {
    for (sprite, anchor, material) in sprites {
        let Some(material) = materials.get_mut(material.id()) else {
            warn!("SpriteMesh material not found!");
            continue;
        };
        warn!("hm..");

        *material = SpriteMaterial::from_sprite_mesh(sprite.clone());

        material.anchor = **anchor;

        if let Some(texture_atlas) = &sprite.texture_atlas
            && let Some(texture_atlas_layout) = texture_atlas_layouts.get(texture_atlas.layout.id())
        {
            material.texture_atlas_layout = Some(texture_atlas_layout.clone());
            material.texture_atlas_index = texture_atlas.index;
        }
    }
}
