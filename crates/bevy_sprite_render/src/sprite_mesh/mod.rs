use bevy_app::{Plugin, PostUpdate};
use bevy_asset::{Asset, AssetEvent, AssetEventSystems, AssetId, Assets, Handle};
use bevy_color::ColorToComponents;
use bevy_ecs::{
    entity::Entity,
    message::MessageReader,
    query::{Added, Changed, Or},
    schedule::IntoScheduleConfigs,
    system::{Commands, Local, Query, Res, ResMut},
};

use bevy_image::{Image, TextureAtlasLayout};
use bevy_math::{vec2, FloatOrd};
use bevy_mesh::{
    mark_2d_meshes_as_changed_if_their_assets_changed, Mesh, Mesh2d, MeshAttributeCompressionFlags,
    MeshBuilder, Meshable,
};
use bevy_shape::Rectangle;

use bevy_platform::collections::{hash_map::Entry, HashMap};
use bevy_shader::load_shader_library;
use bevy_sprite::{prelude::Sprite, Anchor, SpriteAlphaMode};

mod sprite_material;
pub use sprite_material::*;

use crate::{check_entities_needing_specialization, MeshMaterial2d};

/// Plugin used to render a Sprite using a [`Mesh2d`] and a [`SpriteMaterial`]
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

// Insert a Mesh2d quad each time the Sprite component is added.
// The meshhandle is kept locally so they can be cloned.
fn add_mesh(
    sprites: Query<Entity, Added<Sprite>>,
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

/// Key used to determine in which bucket to cache the material for a [`Sprite`]
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

struct SpriteMaterialCache<M: Asset> {
    map: HashMap<SpriteMaterialBucketKey, Vec<(Sprite, AssetId<M>)>>,
    reversed: HashMap<AssetId<M>, SpriteMaterialBucketKey>,
}

impl<M: Asset> Default for SpriteMaterialCache<M> {
    fn default() -> Self {
        Self {
            map: Default::default(),
            reversed: Default::default(),
        }
    }
}

impl<M: Asset> SpriteMaterialCache<M> {
    fn clean(&mut self, id: AssetId<M>) {
        if let Some(key) = self.reversed.remove(&id)
            && let Entry::Occupied(mut bucket) = self.map.entry(key)
        {
            bucket
                .get_mut()
                .retain(|(_, cached_material_id)| *cached_material_id != id);

            if bucket.get().is_empty() {
                bucket.remove();
            }
        }
    }

    fn get_or_insert_with(
        &mut self,
        sprite: &Sprite,
        anchor: Anchor,
        materials: &mut Assets<M>,
        get: impl FnOnce() -> M,
    ) -> Handle<M> {
        let key = SpriteMaterialBucketKey::new(sprite, &anchor);
        let bucket = self.map.entry(key).or_default();
        let maybe_handle = bucket
            .iter()
            .find(|(cached_sprite, _)| cached_sprite == sprite)
            .and_then(|(_, id)| materials.get_strong_handle(*id));

        match maybe_handle {
            Some(handle) => handle,
            None => {
                let handle = materials.add(get());
                bucket.push((sprite.clone(), handle.id()));
                self.reversed.insert(handle.id(), key);
                handle
            }
        }
    }
}

/// Change the material when [`SpriteMesh`] is added / changed.
///
/// The materials are cached based on their [`Sprite`] and [`Anchor`].
///
/// Since not all fields of the [`Sprite`] are easy to hash, we keep multiple "buckets" keyed on
/// parts of the struct that are easy to hash.
///
/// NOTE: This also adds the [`TextureAtlasLayout`] into the [`SpriteMaterial`],
/// but this should instead be read later, similar to the images, allowing
/// for hot reload.
fn add_material(
    mut commands: Commands,
    sprites: Query<
        (Entity, &Sprite, &Anchor),
        Or<(Changed<Sprite>, Changed<Anchor>, Added<Mesh2d>)>,
    >,
    texture_atlas_layouts: Res<Assets<TextureAtlasLayout>>,
    mut cached_materials: Local<SpriteMaterialCache<SpriteMaterial>>,
    mut materials: ResMut<Assets<SpriteMaterial>>,
    mut material_events: MessageReader<AssetEvent<SpriteMaterial>>,
) {
    for event in material_events.read() {
        if let AssetEvent::Removed { id } = event {
            cached_materials.clean(*id);
        }
    }

    for (entity, sprite, anchor) in sprites {
        let handle = cached_materials.get_or_insert_with(sprite, *anchor, &mut materials, || {
            make_sprite_mesh_material(&texture_atlas_layouts, sprite, *anchor)
        });

        commands
            .entity(entity)
            .insert(MeshMaterial2d(handle.clone()));
    }
}

fn make_sprite_mesh_material(
    texture_atlas_layouts: &Assets<TextureAtlasLayout>,
    sprite: &Sprite,
    anchor: Anchor,
) -> SpriteMaterial {
    let mut material = SpriteMaterial::from_sprite(sprite.clone());
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

    #[test]
    fn sprite_material_cache() {
        let mut cache = SpriteMaterialCache::<SpriteMaterial>::default();
        let mut assets = Assets::default();
        let handle = cache.get_or_insert_with(
            &Sprite::default(),
            Anchor::default(),
            &mut assets,
            SpriteMaterial::default,
        );
        assert_eq!(cache.map.len(), 1);
        assert_eq!(cache.reversed.len(), 1);
        assert_eq!(
            assets.get(&handle).cloned(),
            Some(SpriteMaterial::default())
        );

        let handle2 = cache.get_or_insert_with(
            &Sprite::default(),
            Anchor::default(),
            &mut assets,
            SpriteMaterial::default,
        );
        assert_eq!(handle, handle2);
        assert_eq!(cache.reversed.len(), 1);
        assert_eq!(cache.map.len(), 1);

        let mat = SpriteMaterial {
            flip_x: true,
            ..Default::default()
        };
        let handle3 =
            cache.get_or_insert_with(&Sprite::default(), Anchor::BOTTOM_LEFT, &mut assets, || {
                mat.clone()
            });
        assert_eq!(cache.map.len(), 2);
        assert_eq!(cache.map.len(), 2);
        assert_ne!(handle, handle3);
        assert_eq!(assets.get(&handle3).cloned(), Some(mat.clone()));

        let handle4 =
            cache.get_or_insert_with(&Sprite::default(), Anchor::BOTTOM_LEFT, &mut assets, || {
                mat.clone()
            });
        assert_eq!(cache.map.len(), 2);
        assert_eq!(cache.map.len(), 2);
        assert_eq!(handle3, handle4);
        assert_eq!(assets.get(&handle4).cloned(), Some(mat.clone()));

        cache.clean(handle.id());
        assert_eq!(cache.map.len(), 1);
        assert_eq!(cache.reversed.len(), 1);

        cache.clean(handle3.id());
        assert_eq!(cache.map.len(), 0);
        assert_eq!(cache.reversed.len(), 0);
    }
}
