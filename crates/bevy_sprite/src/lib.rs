#![expect(missing_docs, reason = "Not all docs are written yet, see #3492.")]
#![cfg_attr(docsrs, feature(doc_cfg))]
#![forbid(unsafe_code)]
#![doc(
    html_logo_url = "https://bevy.org/assets/icon.png",
    html_favicon_url = "https://bevy.org/assets/icon.png"
)]

//! Provides 2D sprite functionality.

extern crate alloc;

#[cfg(feature = "bevy_picking")]
mod picking_backend;
mod sprite;
#[cfg(feature = "bevy_text")]
mod text2d;
mod texture_slice;

/// The sprite prelude.
///
/// This includes the most common types in this crate, re-exported for your convenience.
pub mod prelude {
    #[cfg(feature = "bevy_picking")]
    #[doc(hidden)]
    pub use crate::picking_backend::{
        SpritePickingCamera, SpritePickingMode, SpritePickingPlugin, SpritePickingSettings,
    };
    #[cfg(feature = "bevy_text")]
    #[doc(hidden)]
    pub use crate::text2d::{Text2d, Text2dReader, Text2dWriter};
    #[doc(hidden)]
    pub use crate::{
        sprite::{Sprite, SpriteImageMode},
        texture_slice::{BorderRect, SliceScaleMode, TextureSlice, TextureSlicer},
        SpriteScalingMode,
    };
}

use bevy_asset::Assets;
use bevy_camera::{
    primitives::{Aabb, MeshAabb},
    visibility::NoFrustumCulling,
    visibility::VisibilitySystems,
};
use bevy_mesh::{Mesh, Mesh2d};
#[cfg(feature = "bevy_picking")]
pub use picking_backend::*;
pub use sprite::*;
#[cfg(feature = "bevy_text")]
pub use text2d::*;
pub use texture_slice::*;

use bevy_app::prelude::*;
use bevy_asset::prelude::AssetChanged;
use bevy_camera::visibility::NoAutoAabb;
use bevy_ecs::prelude::*;
use bevy_image::{Image, TextureAtlasLayout, TextureAtlasPlugin};
use bevy_math::Vec2;

/// Adds support for 2D sprites.
#[derive(Default)]
pub struct SpritePlugin;

/// System set for sprite rendering.
#[derive(Debug, Hash, PartialEq, Eq, Clone, SystemSet)]
pub enum SpriteSystems {
    ExtractSprites,
    ComputeSlices,
}

impl Plugin for SpritePlugin {
    fn build(&self, app: &mut App) {
        if !app.is_plugin_added::<TextureAtlasPlugin>() {
            app.add_plugins(TextureAtlasPlugin);
        }
        app.add_systems(
            PostUpdate,
            calculate_bounds_2d.in_set(VisibilitySystems::CalculateBounds),
        );

        #[cfg(feature = "bevy_text")]
        app.add_systems(
            PostUpdate,
            (
                bevy_text::detect_text_needs_rerender::<Text2d>,
                update_text2d_layout.after(bevy_camera::CameraUpdateSystems),
                calculate_bounds_text2d.in_set(VisibilitySystems::CalculateBounds),
            )
                .chain()
                .after(bevy_text::load_font_assets_into_fontdb_system)
                .in_set(bevy_text::Text2dUpdateSystems)
                .after(bevy_app::AnimationSystems),
        );

        #[cfg(feature = "bevy_picking")]
        app.add_plugins(SpritePickingPlugin);
    }
}

/// System calculating and inserting an [`Aabb`] component to entities with either:
/// - a `Mesh2d` component,
/// - a `Sprite` and `Handle<Image>` components,
///   and without a [`NoFrustumCulling`] component.
///
/// Used in system set [`VisibilitySystems::CalculateBounds`].
pub fn calculate_bounds_2d(
    mut commands: Commands,
    meshes: Res<Assets<Mesh>>,
    images: Res<Assets<Image>>,
    atlases: Res<Assets<TextureAtlasLayout>>,
    new_mesh_aabb: Query<
        (Entity, &Mesh2d),
        (
            Without<Aabb>,
            Without<NoFrustumCulling>,
            Without<NoAutoAabb>,
        ),
    >,
    mut update_mesh_aabb: Query<
        (&Mesh2d, &mut Aabb),
        (
            Or<(AssetChanged<Mesh2d>, Changed<Mesh2d>)>,
            Without<NoFrustumCulling>,
            Without<NoAutoAabb>,
            Without<Sprite>, // disjoint mutable query
        ),
    >,
    new_sprite_aabb: Query<
        (Entity, &Sprite, &Anchor),
        (
            Without<Aabb>,
            Without<NoFrustumCulling>,
            Without<NoAutoAabb>,
        ),
    >,
    mut update_sprite_aabb: Query<
        (&Sprite, &mut Aabb, &Anchor),
        (
            Or<(Changed<Sprite>, Changed<Anchor>)>,
            Without<NoFrustumCulling>,
            Without<NoAutoAabb>,
            Without<Mesh2d>, // disjoint mutable query
        ),
    >,
) {
    // New meshes require inserting a component
    for (entity, mesh_handle) in &new_mesh_aabb {
        if let Some(mesh) = meshes.get(mesh_handle)
            && let Some(aabb) = mesh.compute_aabb()
        {
            commands.entity(entity).try_insert(aabb);
        }
    }

    // Updated meshes can take the fast path with parallel component mutation
    update_mesh_aabb
        .par_iter_mut()
        .for_each(|(mesh_handle, mut aabb)| {
            if let Some(new_aabb) = meshes.get(mesh_handle).and_then(MeshAabb::compute_aabb) {
                aabb.set_if_neq(new_aabb);
            }
        });

    // Sprite helper
    let sprite_size = |sprite: &Sprite| -> Option<Vec2> {
        sprite
            .custom_size
            .or_else(|| sprite.rect.map(|rect| rect.size()))
            .or_else(|| match &sprite.texture_atlas {
                // We default to the texture size for regular sprites
                None => images.get(&sprite.image).map(Image::size_f32),
                // We default to the drawn rect for atlas sprites
                Some(atlas) => atlas
                    .texture_rect(&atlases)
                    .map(|rect| rect.size().as_vec2()),
            })
    };

    // New sprites require inserting a component
    for (size, (entity, anchor)) in new_sprite_aabb
        .iter()
        .filter_map(|(entity, sprite, anchor)| sprite_size(sprite).zip(Some((entity, anchor))))
    {
        let aabb = Aabb {
            center: (-anchor.as_vec() * size).extend(0.0).into(),
            half_extents: (0.5 * size).extend(0.0).into(),
        };
        commands.entity(entity).try_insert(aabb);
    }

    // Updated sprites can take the fast path with parallel component mutation
    update_sprite_aabb
        .par_iter_mut()
        .for_each(|(sprite, mut aabb, anchor)| {
            if let Some(size) = sprite_size(sprite) {
                aabb.set_if_neq(Aabb {
                    center: (-anchor.as_vec() * size).extend(0.0).into(),
                    half_extents: (0.5 * size).extend(0.0).into(),
                });
            }
        });
}

#[cfg(test)]
mod test {
    use super::*;
    use bevy_math::{Rect, Vec2, Vec3A};

    #[test]
    fn calculate_bounds_2d_create_aabb_for_image_sprite_entity() {
        // Setup app
        let mut app = App::new();

        // Add resources and get handle to image
        let mut image_assets = Assets::<Image>::default();
        let image_handle = image_assets.add(Image::default());
        app.insert_resource(image_assets);
        let mesh_assets = Assets::<Mesh>::default();
        app.insert_resource(mesh_assets);
        let texture_atlas_assets = Assets::<TextureAtlasLayout>::default();
        app.insert_resource(texture_atlas_assets);

        // Add system
        app.add_systems(Update, calculate_bounds_2d);

        // Add entities
        let entity = app.world_mut().spawn(Sprite::from_image(image_handle)).id();

        // Verify that the entity does not have an AABB
        assert!(!app
            .world()
            .get_entity(entity)
            .expect("Could not find entity")
            .contains::<Aabb>());

        // Run system
        app.update();

        // Verify the AABB exists
        assert!(app
            .world()
            .get_entity(entity)
            .expect("Could not find entity")
            .contains::<Aabb>());
    }

    #[test]
    fn calculate_bounds_2d_update_aabb_when_sprite_custom_size_changes_to_some() {
        // Setup app
        let mut app = App::new();

        // Add resources and get handle to image
        let mut image_assets = Assets::<Image>::default();
        let image_handle = image_assets.add(Image::default());
        app.insert_resource(image_assets);
        let mesh_assets = Assets::<Mesh>::default();
        app.insert_resource(mesh_assets);
        let texture_atlas_assets = Assets::<TextureAtlasLayout>::default();
        app.insert_resource(texture_atlas_assets);

        // Add system
        app.add_systems(Update, calculate_bounds_2d);

        // Add entities
        let entity = app
            .world_mut()
            .spawn(Sprite {
                custom_size: Some(Vec2::ZERO),
                image: image_handle,
                ..Sprite::default()
            })
            .id();

        // Create initial AABB
        app.update();

        // Get the initial AABB
        let first_aabb = *app
            .world()
            .get_entity(entity)
            .expect("Could not find entity")
            .get::<Aabb>()
            .expect("Could not find initial AABB");

        // Change `custom_size` of sprite
        let mut binding = app
            .world_mut()
            .get_entity_mut(entity)
            .expect("Could not find entity");
        let mut sprite = binding
            .get_mut::<Sprite>()
            .expect("Could not find sprite component of entity");
        sprite.custom_size = Some(Vec2::ONE);

        // Re-run the `calculate_bounds_2d` system to get the new AABB
        app.update();

        // Get the re-calculated AABB
        let second_aabb = *app
            .world()
            .get_entity(entity)
            .expect("Could not find entity")
            .get::<Aabb>()
            .expect("Could not find second AABB");

        // Check that the AABBs are not equal
        assert_ne!(first_aabb, second_aabb);
    }

    #[test]
    fn calculate_bounds_2d_correct_aabb_for_sprite_with_custom_rect() {
        // Setup app
        let mut app = App::new();

        // Add resources and get handle to image
        let mut image_assets = Assets::<Image>::default();
        let image_handle = image_assets.add(Image::default());
        app.insert_resource(image_assets);
        let mesh_assets = Assets::<Mesh>::default();
        app.insert_resource(mesh_assets);
        let texture_atlas_assets = Assets::<TextureAtlasLayout>::default();
        app.insert_resource(texture_atlas_assets);

        // Add system
        app.add_systems(Update, calculate_bounds_2d);

        // Add entities
        let entity = app
            .world_mut()
            .spawn((
                Sprite {
                    rect: Some(Rect::new(0., 0., 0.5, 1.)),
                    image: image_handle,
                    ..Sprite::default()
                },
                Anchor::TOP_RIGHT,
            ))
            .id();

        // Create AABB
        app.update();

        // Get the AABB
        let aabb = *app
            .world_mut()
            .get_entity(entity)
            .expect("Could not find entity")
            .get::<Aabb>()
            .expect("Could not find AABB");

        // Verify that the AABB is at the expected position
        assert_eq!(aabb.center, Vec3A::new(-0.25, -0.5, 0.));

        // Verify that the AABB has the expected size
        assert_eq!(aabb.half_extents, Vec3A::new(0.25, 0.5, 0.));
    }
}
