use alloc::sync::Arc;

use bevy_app::{Plugin, PostUpdate};
use bevy_asset::{Asset, AssetEventSystems, AssetId, Assets, Handle};
use bevy_color::ColorToComponents;
use bevy_ecs::{
    entity::Entity,
    query::{Added, Changed, Or},
    schedule::IntoScheduleConfigs,
    system::{Commands, Local, Query, Res, ResMut},
};

use bevy_image::{Image, TextureAtlasLayout};
use bevy_math::{vec2, FloatOrd};
use bevy_mesh::{
    mark_2d_meshes_as_changed_if_their_assets_changed, Mesh, Mesh2d, MeshBuilder,
    MeshCompressionArgs, Meshable,
};
use bevy_shape::Rectangle;

use bevy_platform::collections::HashMap;
use bevy_shader::load_shader_library;
use bevy_sprite::{prelude::Sprite, Anchor, SpriteAlphaMode};

mod sprite_extended_material;
pub use sprite_extended_material::*;

mod sprite_mesh_material;
pub use sprite_mesh_material::*;

use crate::{
    check_entities_needing_specialization, mark_2d_meshes_as_changed_if_their_materials_changed,
    MeshMaterial2d,
};

/// Plugin used to render a Sprite using a [`Mesh2d`] and a [`SpriteMaterial`]
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
                .before(mark_2d_meshes_as_changed_if_their_materials_changed::<SpriteMeshMaterial>)
                .before(mark_2d_meshes_as_changed_if_their_assets_changed)
                .before(AssetEventSystems),
        );
    }
}

// Insert a Mesh2d quad each time the Sprite component is added.
// The meshhandle is kept locally so they can be cloned.
fn add_mesh(
    sprites: Query<Entity, Added<Sprite>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut quad: Local<Option<Handle<Mesh>>>,
    mut commands: Commands,
) {
    let quad = quad.get_or_insert_with(|| {
        meshes.add({
            let mesh = Rectangle::from_size(vec2(1.0, 1.0))
                .mesh()
                .build()
                .with_removed_attribute(Mesh::ATTRIBUTE_NORMAL);
            mesh.compressed_mesh(&MeshCompressionArgs::regular())
                .unwrap()
        })
    });
    for entity in sprites {
        commands.entity(entity).insert(Mesh2d(quad.clone()));
    }
}

/// Key used to determine in which bucket to cache the material for a [`Sprite`]
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
struct SpriteMeshMaterialBucketKey {
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

impl SpriteMeshMaterialBucketKey {
    fn new(sprite: &Sprite, anchor: &Anchor) -> Self {
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

type SpriteMaterialCache<M> = HashMap<SpriteMeshMaterialBucketKey, Vec<(Sprite, Handle<M>)>>;

fn evict_unused_materials<M: Asset>(cache: &mut SpriteMaterialCache<M>) {
    cache.retain(|_, bucket| {
        bucket.retain(|(_, handle)| {
            !matches!(handle, Handle::Strong(handle) if Arc::strong_count(handle) == 1)
        });
        !bucket.is_empty()
    });
}

fn get_or_insert_material<M: Asset>(
    cache: &mut SpriteMaterialCache<M>,
    sprite: &Sprite,
    anchor: Anchor,
    materials: &mut Assets<M>,
    get: impl FnOnce() -> M,
) -> Handle<M> {
    let bucket = cache
        .entry(SpriteMeshMaterialBucketKey::new(sprite, &anchor))
        .or_default();
    if let Some((_, handle)) = bucket
        .iter()
        .find(|(cached_sprite, _)| cached_sprite == sprite)
    {
        return handle.clone();
    }

    let handle = materials.add(get());
    bucket.push((sprite.clone(), handle.clone()));
    handle
}

/// Change the material when [`Sprite`] is added / changed.
///
/// The materials are cached based on their [`Sprite`] and [`Anchor`].
///
/// Since not all fields of the [`Sprite`] are easy to hash, we keep multiple "buckets" keyed on
/// parts of the struct that are easy to hash.
///
/// NOTE: This also adds the [`TextureAtlasLayout`] into the [`SpriteMeshMaterial`],
/// but this should instead be read later, similar to the images, allowing
/// for hot reload.
fn add_material(
    mut commands: Commands,
    sprites: Query<
        (Entity, &Sprite, &Anchor, Option<&SpriteMaterialCount>),
        Or<(
            Changed<Sprite>,
            Changed<Anchor>,
            Added<Mesh2d>,
            Changed<SpriteMaterialCount>,
        )>,
    >,
    texture_atlas_layouts: Res<Assets<TextureAtlasLayout>>,
    mut cached_materials: Local<SpriteMaterialCache<SpriteMeshMaterial>>,
    mut materials: ResMut<Assets<SpriteMeshMaterial>>,
) {
    evict_unused_materials(&mut cached_materials);

    for (entity, sprite, anchor, count) in sprites {
        if count.is_some_and(|c| c.0 != 0) {
            continue;
        }

        let handle = get_or_insert_material(
            &mut cached_materials,
            sprite,
            *anchor,
            &mut materials,
            || make_sprite_mesh_material(&texture_atlas_layouts, sprite, *anchor),
        );

        commands.entity(entity).insert(MeshMaterial2d(handle));
    }
}

fn make_sprite_mesh_material(
    texture_atlas_layouts: &Assets<TextureAtlasLayout>,
    sprite: &Sprite,
    anchor: Anchor,
) -> SpriteMeshMaterial {
    let mut material = SpriteMeshMaterial::from_sprite(sprite.clone());
    material.anchor = *anchor;

    if let Some(texture_atlas) = &sprite.texture_atlas
        && let Some(texture_atlas_layout) = texture_atlas_layouts.get(texture_atlas.layout.id())
    {
        material.texture_atlas_layout = Some(texture_atlas_layout.clone());
        material.texture_atlas_index = texture_atlas.index;
    }

    material
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Default)]
    struct Fixture {
        cache: SpriteMaterialCache<SpriteMeshMaterial>,
        assets: Assets<SpriteMeshMaterial>,
    }

    impl Fixture {
        fn get(
            &mut self,
            anchor: Anchor,
            material: &SpriteMeshMaterial,
        ) -> Handle<SpriteMeshMaterial> {
            get_or_insert_material(
                &mut self.cache,
                &Sprite::default(),
                anchor,
                &mut self.assets,
                || material.clone(),
            )
        }
    }

    #[test]
    fn sprite_material_cache() {
        let mut fx = Fixture::default();
        let default = SpriteMeshMaterial::default();
        let flipped = SpriteMeshMaterial {
            flip_x: true,
            ..Default::default()
        };

        let handle = fx.get(Anchor::default(), &default);
        assert_eq!(fx.cache.len(), 1);
        assert_eq!(fx.assets.get(&handle), Some(&default));

        let handle2 = fx.get(Anchor::default(), &default);
        assert_eq!(handle, handle2);
        assert_eq!(fx.cache.len(), 1);

        let handle3 = fx.get(Anchor::BOTTOM_LEFT, &flipped);
        assert_ne!(handle, handle3);
        assert_eq!(fx.cache.len(), 2);
        assert_eq!(fx.assets.get(&handle3), Some(&flipped));

        let handle4 = fx.get(Anchor::BOTTOM_LEFT, &flipped);
        assert_eq!(handle3, handle4);
        assert_eq!(fx.cache.len(), 2);

        evict_unused_materials(&mut fx.cache);
        assert_eq!(fx.cache.len(), 2);

        drop((handle, handle2));
        evict_unused_materials(&mut fx.cache);
        assert_eq!(fx.cache.len(), 1);

        drop((handle3, handle4));
        evict_unused_materials(&mut fx.cache);
        assert_eq!(fx.cache.len(), 0);
    }
}
