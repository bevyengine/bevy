#![expect(missing_docs, reason = "Not all docs are written yet, see #3492.")]
#![cfg_attr(docsrs, feature(doc_auto_cfg))]
#![forbid(unsafe_code)]
#![doc(
    html_logo_url = "https://bevy.org/assets/icon.png",
    html_favicon_url = "https://bevy.org/assets/icon.png"
)]

//! Provides 2D sprite functionality.

extern crate alloc;

#[cfg(feature = "bevy_sprite_picking_backend")]
mod picking_backend;
mod sprite;
#[cfg(feature = "bevy_text")]
mod text2d;
mod texture_slice;

/// The sprite prelude.
///
/// This includes the most common types in this crate, re-exported for your convenience.
pub mod prelude {
    #[cfg(feature = "bevy_sprite_picking_backend")]
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
        ScalingMode,
    };
}

use bevy_asset::Assets;
use bevy_camera::{
    primitives::{Aabb, MeshAabb},
    visibility::NoFrustumCulling,
    visibility::VisibilitySystems,
};
use bevy_mesh::{Mesh, Mesh2d};
#[cfg(feature = "bevy_sprite_picking_backend")]
pub use picking_backend::*;
pub use sprite::*;
#[cfg(feature = "bevy_text")]
pub use text2d::*;
pub use texture_slice::*;

use bevy_app::prelude::*;
use bevy_ecs::prelude::*;
use bevy_image::{Image, TextureAtlasLayout, TextureAtlasPlugin};

/// Adds support for 2D sprites.
#[derive(Default)]
pub struct SpritePlugin;

/// System set for sprite rendering.
#[derive(Debug, Hash, PartialEq, Eq, Clone, SystemSet)]
pub enum SpriteSystems {
    ExtractSprites,
    ComputeSlices,
}

/// Deprecated alias for [`SpriteSystems`].
#[deprecated(since = "0.17.0", note = "Renamed to `SpriteSystems`.")]
pub type SpriteSystem = SpriteSystems;

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
                update_text2d_layout
                    .after(bevy_camera::CameraUpdateSystems)
                    .after(bevy_text::remove_dropped_font_atlas_sets),
                calculate_bounds_text2d.in_set(VisibilitySystems::CalculateBounds),
            )
                .chain()
                .in_set(bevy_text::Text2dUpdateSystems)
                .after(bevy_app::AnimationSystems),
        );

        #[cfg(feature = "bevy_sprite_picking_backend")]
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
    meshes_without_aabb: Query<(Entity, &Mesh2d), (Without<Aabb>, Without<NoFrustumCulling>)>,
    sprites_to_recalculate_aabb: Query<
        (Entity, &Sprite, &Anchor),
        (
            Or<(Without<Aabb>, Changed<Sprite>, Changed<Anchor>)>,
            Without<NoFrustumCulling>,
        ),
    >,
) {
    for (entity, mesh_handle) in &meshes_without_aabb {
        if let Some(mesh) = meshes.get(&mesh_handle.0)
            && let Some(aabb) = mesh.compute_aabb()
        {
            commands.entity(entity).try_insert(aabb);
        }
    }
    for (entity, sprite, anchor) in &sprites_to_recalculate_aabb {
        if let Some(size) = sprite
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
        {
            let aabb = Aabb {
                center: (-anchor.as_vec() * size).extend(0.0).into(),
                half_extents: (0.5 * size).extend(0.0).into(),
            };
            commands.entity(entity).try_insert(aabb);
        }
    }
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
