use bevy_app::{Plugin, PostUpdate};
use bevy_ecs::{
    entity::Entity,
    query::{Added, Changed, Or},
    schedule::IntoScheduleConfigs,
    system::{Commands, Local, Query, Res, ResMut},
};

use bevy_asset::{Assets, Handle};

use bevy_image::TextureAtlasLayout;
use bevy_math::{primitives::Rectangle, vec2};
use bevy_mesh::{
    mark_2d_meshes_as_changed_if_their_assets_changed, Mesh, Mesh2d, MeshAttributeCompressionFlags,
    MeshBuilder, Meshable,
};

use bevy_platform::collections::HashMap;
use bevy_shader::load_shader_library;
use bevy_sprite::{prelude::SpriteMesh, Anchor};

mod sprite_extended_material;
pub use sprite_extended_material::*;

mod sprite_mesh_material;
pub use sprite_mesh_material::*;

use crate::{check_entities_needing_specialization, MeshMaterial2d};

pub struct SpriteMeshPlugin;

impl Plugin for SpriteMeshPlugin {
    fn build(&self, app: &mut bevy_app::App) {
        load_shader_library!(app, "bindings.wesl");
        load_shader_library!(app, "functions.wesl");
        load_shader_library!(app, "types.wesl");

        app.add_plugins(SpriteMeshMaterialPlugin);

        app.add_systems(
            PostUpdate,
            (add_mesh, add_material)
                .chain()
                .before(check_entities_needing_specialization::<SpriteMeshMaterial>)
                .before(mark_2d_meshes_as_changed_if_their_assets_changed),
        );
    }
}

// Insert a Mesh2d quad each time the SpriteMesh component is added.
// The meshhandle is kept locally so they can be cloned.
fn add_mesh(
    sprites: Query<Entity, Added<SpriteMesh>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut quad: Local<Option<Handle<Mesh>>>,
    mut commands: Commands,
) {
    let quad = quad.get_or_insert_with(|| {
        meshes.add(
            Rectangle::from_size(vec2(1.0, 1.0))
                .mesh()
                .build()
                .with_removed_attribute(Mesh::ATTRIBUTE_NORMAL)
                .compressed_mesh(
                    MeshAttributeCompressionFlags::COMPRESS_POSITION
                        | MeshAttributeCompressionFlags::COMPRESS_UV0,
                    true,
                ),
        )
    });
    for entity in sprites {
        commands.entity(entity).insert(Mesh2d(quad.clone()));
    }
}

// Change the material when SpriteMesh is added / changed.
//
// NOTE: This also adds the SpriteAtlasLayout into the SpriteMeshMaterial,
// but this should instead be read later, similar to the images, allowing
// for hot reload.
fn add_material(
    sprites: Query<
        (Entity, &SpriteMesh, &Anchor, Option<&SpriteMaterialCount>),
        Or<(
            Changed<SpriteMesh>,
            Changed<Anchor>,
            Changed<SpriteMaterialCount>,
            Added<Mesh2d>,
        )>,
    >,
    texture_atlas_layouts: Res<Assets<TextureAtlasLayout>>,
    mut cached_materials: Local<HashMap<(SpriteMesh, Anchor), Handle<SpriteMeshMaterial>>>,
    mut materials: ResMut<Assets<SpriteMeshMaterial>>,
    mut commands: Commands,
) {
    for (entity, sprite, anchor, count) in sprites {
        if count.is_some_and(|c| c.0 != 0) {
            continue;
        }

        if let Some(handle) = cached_materials.get(&(sprite.clone(), *anchor)) {
            commands
                .entity(entity)
                .insert(MeshMaterial2d(handle.clone()));
        } else {
            let material = make_sprite_mesh_material(&texture_atlas_layouts, sprite, anchor);

            let handle = materials.add(material);
            cached_materials.insert((sprite.clone(), *anchor), handle.clone());

            commands
                .entity(entity)
                .insert(MeshMaterial2d(handle.clone()));
        }
    }
}

fn make_sprite_mesh_material(
    texture_atlas_layouts: &Assets<TextureAtlasLayout>,
    sprite: &SpriteMesh,
    anchor: &Anchor,
) -> SpriteMeshMaterial {
    let mut material = SpriteMeshMaterial::from_sprite_mesh(sprite.clone());
    material.anchor = **anchor;

    if let Some(texture_atlas) = &sprite.texture_atlas
        && let Some(texture_atlas_layout) = texture_atlas_layouts.get(texture_atlas.layout.id())
    {
        material.texture_atlas_layout = Some(texture_atlas_layout.clone());
        material.texture_atlas_index = texture_atlas.index;
    }

    material
}
