#![expect(missing_docs, reason = "Not all docs are written yet, see #3492.")]
#![cfg_attr(docsrs, feature(doc_auto_cfg))]
#![forbid(unsafe_code)]
#![doc(
    html_logo_url = "https://bevyengine.org/assets/icon.png",
    html_favicon_url = "https://bevyengine.org/assets/icon.png"
)]

//! Provides 2D sprite rendering functionality.

extern crate alloc;

mod mesh2d;
#[cfg(feature = "bevy_sprite_picking_backend")]
mod picking_backend;
mod render;
mod sprite;
mod texture_slice;

/// The sprite prelude.
///
/// This includes the most common types in this crate, re-exported for your convenience.
pub mod prelude {
    #[doc(hidden)]
    pub use crate::{
        sprite::{Sprite, SpriteImageMode},
        texture_slice::{BorderRect, SliceScaleMode, TextureSlice, TextureSlicer},
        ColorMaterial, MeshMaterial2d, ScalingMode,
    };
}

pub use mesh2d::*;
#[cfg(feature = "bevy_sprite_picking_backend")]
pub use picking_backend::*;
pub use render::*;
pub use sprite::*;
pub use texture_slice::*;

use bevy_app::prelude::*;
use bevy_asset::{load_internal_asset, Assets, Handle};
use bevy_core_pipeline::core_2d::Transparent2d;
use bevy_ecs::prelude::*;
use bevy_image::{prelude::*, TextureAtlasPlugin};
use bevy_render::{
    mesh::{Mesh, Mesh2d, MeshAabb},
    primitives::Aabb,
    render_phase::AddRenderCommand,
    render_resource::{Shader, SpecializedRenderPipelines},
    view::{NoFrustumCulling, VisibilitySystems},
    ExtractSchedule, Render, RenderApp, RenderSet,
};

/// Adds support for 2D sprite rendering.
pub struct SpritePlugin {
    /// Whether to add the sprite picking backend to the app.
    #[cfg(feature = "bevy_sprite_picking_backend")]
    pub add_picking: bool,
}

#[expect(
    clippy::allow_attributes,
    reason = "clippy::derivable_impls is not always linted"
)]
#[allow(
    clippy::derivable_impls,
    reason = "Known false positive with clippy: <https://github.com/rust-lang/rust-clippy/issues/13160>"
)]
impl Default for SpritePlugin {
    fn default() -> Self {
        Self {
            #[cfg(feature = "bevy_sprite_picking_backend")]
            add_picking: true,
        }
    }
}

pub const SPRITE_SHADER_HANDLE: Handle<Shader> = Handle::weak_from_u128(2763343953151597127);
pub const SPRITE_VIEW_BINDINGS_SHADER_HANDLE: Handle<Shader> =
    Handle::weak_from_u128(8846920112458963210);

/// System set for sprite rendering.
#[derive(Debug, Hash, PartialEq, Eq, Clone, SystemSet)]
pub enum SpriteSystem {
    ExtractSprites,
    ComputeSlices,
}

impl Plugin for SpritePlugin {
    fn build(&self, app: &mut App) {
        load_internal_asset!(
            app,
            SPRITE_SHADER_HANDLE,
            "render/sprite.wgsl",
            Shader::from_wgsl
        );
        load_internal_asset!(
            app,
            SPRITE_VIEW_BINDINGS_SHADER_HANDLE,
            "render/sprite_view_bindings.wgsl",
            Shader::from_wgsl
        );

        if !app.is_plugin_added::<TextureAtlasPlugin>() {
            app.add_plugins(TextureAtlasPlugin);
        }

        app.register_type::<Sprite>()
            .register_type::<SpriteImageMode>()
            .register_type::<TextureSlicer>()
            .register_type::<Anchor>()
            .register_type::<Mesh2d>()
            .add_plugins((Mesh2dRenderPlugin, ColorMaterialPlugin))
            .add_systems(
                PostUpdate,
                (
                    calculate_bounds_2d.in_set(VisibilitySystems::CalculateBounds),
                    (
                        compute_slices_on_asset_event,
                        compute_slices_on_sprite_change,
                    )
                        .in_set(SpriteSystem::ComputeSlices),
                ),
            );

        #[cfg(feature = "bevy_sprite_picking_backend")]
        if self.add_picking {
            app.add_plugins(SpritePickingPlugin);
        }

        if let Some(render_app) = app.get_sub_app_mut(RenderApp) {
            render_app
                .init_resource::<ImageBindGroups>()
                .init_resource::<SpecializedRenderPipelines<SpritePipeline>>()
                .init_resource::<SpriteMeta>()
                .init_resource::<ExtractedSprites>()
                .init_resource::<SpriteAssetEvents>()
                .add_render_command::<Transparent2d, DrawSprite>()
                .add_systems(
                    ExtractSchedule,
                    (
                        extract_sprites.in_set(SpriteSystem::ExtractSprites),
                        extract_sprite_events,
                    ),
                )
                .add_systems(
                    Render,
                    (
                        queue_sprites
                            .in_set(RenderSet::Queue)
                            .ambiguous_with(queue_material2d_meshes::<ColorMaterial>),
                        prepare_sprite_image_bind_groups.in_set(RenderSet::PrepareBindGroups),
                        prepare_sprite_view_bind_groups.in_set(RenderSet::PrepareBindGroups),
                    ),
                );
        };
    }

    fn finish(&self, app: &mut App) {
        if let Some(render_app) = app.get_sub_app_mut(RenderApp) {
            render_app.init_resource::<SpritePipeline>();
        }
    }
}

/// System calculating and inserting an [`Aabb`] component to entities with either:
/// - a `Mesh2d` component,
/// - a `Sprite` and `Handle<Image>` components,
///     and without a [`NoFrustumCulling`] component.
///
/// Used in system set [`VisibilitySystems::CalculateBounds`].
pub fn calculate_bounds_2d(
    mut commands: Commands,
    meshes: Res<Assets<Mesh>>,
    images: Res<Assets<Image>>,
    atlases: Res<Assets<TextureAtlasLayout>>,
    meshes_without_aabb: Query<(Entity, &Mesh2d), (Without<Aabb>, Without<NoFrustumCulling>)>,
    sprites_to_recalculate_aabb: Query<
        (Entity, &Sprite),
        (
            Or<(Without<Aabb>, Changed<Sprite>)>,
            Without<NoFrustumCulling>,
        ),
    >,
) {
    for (entity, mesh_handle) in &meshes_without_aabb {
        if let Some(mesh) = meshes.get(&mesh_handle.0) {
            if let Some(aabb) = mesh.compute_aabb() {
                commands.entity(entity).try_insert(aabb);
            }
        }
    }
    for (entity, sprite) in &sprites_to_recalculate_aabb {
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
                center: (-sprite.anchor.as_vec() * size).extend(0.0).into(),
                half_extents: (0.5 * size).extend(0.0).into(),
            };
            commands.entity(entity).try_insert(aabb);
        }
    }
}

#[cfg(test)]
mod test {

    use bevy_math::{Rect, Vec2, Vec3A};
    use bevy_utils::default;

    use super::*;

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
                ..default()
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
            .spawn(Sprite {
                rect: Some(Rect::new(0., 0., 0.5, 1.)),
                anchor: Anchor::TopRight,
                image: image_handle,
                ..default()
            })
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
