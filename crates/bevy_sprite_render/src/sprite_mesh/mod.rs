use bevy_app::{Plugin, PostUpdate};
use bevy_color::ColorToComponents;
use bevy_ecs::{
    entity::Entity,
    message::MessageReader,
    query::{Added, Changed, Or},
    schedule::IntoScheduleConfigs,
    system::{Commands, Local, Query, Res, ResMut},
};

use bevy_asset::{AssetEvent, AssetEventSystems, AssetId, Assets, Handle};

use bevy_image::{Image, TextureAtlasLayout};
use bevy_math::{primitives::Rectangle, vec2, FloatOrd};
use bevy_mesh::{
    mark_2d_meshes_as_changed_if_their_assets_changed, Mesh, Mesh2d, MeshAttributeCompressionFlags,
    MeshBuilder, Meshable,
};

use bevy_platform::collections::{hash_map::Entry, HashMap};
use bevy_shader::load_shader_library;
use bevy_sprite::{prelude::SpriteMesh, Anchor, SpriteAlphaMode};

mod sprite_material;
pub use sprite_material::*;

use crate::{check_entities_needing_specialization, MeshMaterial2d};

pub struct SpriteMeshPlugin;

impl Plugin for SpriteMeshPlugin {
    fn build(&self, app: &mut bevy_app::App) {
        load_shader_library!(app, "bindings.wesl");
        load_shader_library!(app, "functions.wesl");
        load_shader_library!(app, "types.wesl");

        app.add_plugins(SpriteMaterialPlugin);

        app.add_systems(
            PostUpdate,
            (add_mesh, add_material)
                .chain()
                .before(check_entities_needing_specialization::<SpriteMaterial>)
                .before(mark_2d_meshes_as_changed_if_their_assets_changed)
                .after(AssetEventSystems),
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

/// Key used to determine in which bucket to cache the material for a [`SpriteMesh`]
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
struct SpriteMaterialBucketKey {
    image: AssetId<Image>,
    texture_atlas_layout: Option<AssetId<TextureAtlasLayout>>,
    color: [FloatOrd; 4],
    flip_x: bool,
    flip_y: bool,
    custom_size: Option<[FloatOrd; 2]>,
    rect: Option<([FloatOrd; 2], [FloatOrd; 2])>,
    alpha_mode: SpriteAlphaModeKey,
    anchor: [FloatOrd; 2],
}

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
enum SpriteAlphaModeKey {
    Opaque,
    Mask(FloatOrd),
    Blend,
}

impl SpriteMaterialBucketKey {
    fn new(sprite: &SpriteMesh, anchor: &Anchor) -> Self {
        Self {
            image: sprite.image.id(),
            texture_atlas_layout: sprite.texture_atlas.as_ref().map(|a| a.layout.id()),
            color: sprite.color.to_linear().to_f32_array().map(FloatOrd),
            flip_x: sprite.flip_x,
            flip_y: sprite.flip_y,
            custom_size: sprite.custom_size.map(|v| v.to_array().map(FloatOrd)),
            rect: sprite.rect.map(|r| {
                (
                    r.min.to_array().map(FloatOrd),
                    r.max.to_array().map(FloatOrd),
                )
            }),
            alpha_mode: match sprite.alpha_mode {
                SpriteAlphaMode::Opaque => SpriteAlphaModeKey::Opaque,
                SpriteAlphaMode::Mask(mask) => SpriteAlphaModeKey::Mask(FloatOrd(mask)),
                SpriteAlphaMode::Blend => SpriteAlphaModeKey::Blend,
            },
            anchor: anchor.0.to_array().map(FloatOrd),
        }
    }
}

/// Change the material when [`SpriteMesh`] is added / changed.
///
/// The materials are cached based on their [`SpriteMesh`] and [`Anchor`].
///
/// Since not all fields of the [`SpriteMesh`] are easy to hash, we keep multiple "buckets" keyed on
/// parts of the struct that are easy to hash.
///
/// NOTE: This also adds the [`TextureAtlasLayout`] into the [`SpriteMaterial`],
/// but this should instead be read later, similar to the images, allowing
/// for hot reload.
fn add_material(
    mut commands: Commands,
    sprites: Query<
        (Entity, &SpriteMesh, &Anchor),
        Or<(Changed<SpriteMesh>, Changed<Anchor>, Added<Mesh2d>)>,
    >,
    texture_atlas_layouts: Res<Assets<TextureAtlasLayout>>,
    mut cached_materials: Local<
        HashMap<SpriteMaterialBucketKey, Vec<(SpriteMesh, AssetId<SpriteMaterial>)>>,
    >,
    mut reversed_cached_materials: Local<HashMap<AssetId<SpriteMaterial>, SpriteMaterialBucketKey>>,
    mut materials: ResMut<Assets<SpriteMaterial>>,
    mut material_events: MessageReader<AssetEvent<SpriteMaterial>>,
) {
    // Remove materials from the cache
    for event in material_events.read() {
        if let AssetEvent::Removed { id } = event
            && let Some(key) = reversed_cached_materials.remove(id)
            && let Entry::Occupied(mut bucket) = cached_materials.entry(key)
        {
            bucket
                .get_mut()
                .retain(|(_, cached_material_id)| cached_material_id != id);
            if bucket.get().is_empty() {
                bucket.remove();
            }
        }
    }

    for (entity, sprite, anchor) in sprites {
        // Get the bucket for the sprite and anchor
        let bucket_key = SpriteMaterialBucketKey::new(sprite, anchor);
        let bucket = cached_materials.entry(bucket_key).or_default();

        let maybe_handle = bucket
            .iter()
            .find(|(cached_sprite, _)| cached_sprite == sprite)
            .and_then(|(_, id)| materials.get_strong_handle(*id));

        let handle = match maybe_handle {
            Some(handle) => handle,
            None => {
                let mut material = SpriteMaterial::from_sprite_mesh(sprite.clone());
                material.anchor = **anchor;

                if let Some(texture_atlas) = &sprite.texture_atlas
                    && let Some(texture_atlas_layout) =
                        texture_atlas_layouts.get(texture_atlas.layout.id())
                {
                    material.texture_atlas_layout = Some(texture_atlas_layout.clone());
                    material.texture_atlas_index = texture_atlas.index;
                }

                let handle = materials.add(material);
                bucket.push((sprite.clone(), handle.id()));
                handle
            }
        };

        commands
            .entity(entity)
            .insert(MeshMaterial2d(handle.clone()));
    }
}
