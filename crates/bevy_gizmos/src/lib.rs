#![cfg_attr(docsrs, feature(doc_auto_cfg))]
#![doc(
    html_logo_url = "https://bevyengine.org/assets/icon.png",
    html_favicon_url = "https://bevyengine.org/assets/icon.png"
)]

//! This crate adds an immediate mode drawing api to Bevy for visual debugging.
//!
//! # Example
//! ```
//! # use bevy_gizmos::prelude::*;
//! # use bevy_math::prelude::*;
//! # use bevy_color::palettes::basic::GREEN;
//! fn system(mut gizmos: Gizmos) {
//!     gizmos.line(Vec3::ZERO, Vec3::X, GREEN);
//! }
//! # bevy_ecs::system::assert_is_system(system);
//! ```
//!
//! See the documentation on [Gizmos](crate::gizmos::Gizmos) for more examples.

/// System set label for the systems handling the rendering of gizmos.
#[derive(SystemSet, Clone, Debug, Hash, PartialEq, Eq)]
pub enum GizmoRenderSystem {
    /// Adds gizmos to the [`Transparent2d`](bevy_core_pipeline::core_2d::Transparent2d) render phase
    #[cfg(feature = "bevy_sprite")]
    QueueGizmos2d,
    /// Adds gizmos to the [`Transparent3d`](bevy_core_pipeline::core_3d::Transparent3d) render phase
    #[cfg(feature = "bevy_pbr")]
    QueueGizmos3d,
}

mod billboard;
mod lines;

#[cfg(feature = "bevy_render")]
pub mod aabb;
pub mod arcs;
pub mod arrows;
pub mod circles;
pub mod config;
pub mod cross;
pub mod gizmos;
pub mod grid;
#[cfg(all(feature = "bevy_pbr", feature = "bevy_render"))]
pub mod light;
pub mod primitives;
pub mod rounded_box;

/// The `bevy_gizmos` prelude.
pub mod prelude {
    #[cfg(feature = "bevy_render")]
    pub use crate::aabb::{AabbGizmoConfigGroup, ShowAabbGizmo};
    #[doc(hidden)]
    pub use crate::{
        config::{
            DefaultGizmoConfigGroup, GizmoConfig, GizmoConfigGroup, GizmoConfigStore,
            GizmoLineJoint, GizmoLineStyle,
        },
        gizmos::Gizmos,
        primitives::{dim2::GizmoPrimitive2d, dim3::GizmoPrimitive3d},
        AppGizmoBuilder,
    };

    #[cfg(all(feature = "bevy_pbr", feature = "bevy_render"))]
    pub use crate::light::{LightGizmoColor, LightGizmoConfigGroup, ShowLightGizmo};
}

use bevy_app::{App, FixedFirst, FixedLast, Last, Plugin, RunFixedMainLoop};
use bevy_ecs::{
    schedule::{IntoSystemConfigs, IntoSystemSetConfigs, SystemSet},
    system::{Res, ResMut},
};
use bevy_render::{Render, RenderSet};
use bevy_time::Fixed;
use billboard::{AppBillboardGizmoBuilder, BillboardGizmoPlugin};
use config::{DefaultGizmoConfigGroup, GizmoConfig, GizmoConfigGroup, GizmoConfigStore};
use gizmos::{GizmoStorage, Swap};
#[cfg(all(feature = "bevy_pbr", feature = "bevy_render"))]
use light::LightGizmoPlugin;
use lines::{AppLineGizmoBuilder, LineGizmoPlugin};

/// A [`Plugin`] that provides an immediate mode drawing api for visual debugging.
///
/// Requires to be loaded after [`PbrPlugin`](bevy_pbr::PbrPlugin) or [`SpritePlugin`](bevy_sprite::SpritePlugin).
#[derive(Default)]
pub struct GizmoPlugin;

impl Plugin for GizmoPlugin {
    fn build(&self, app: &mut App) {
        #[cfg(feature = "bevy_pbr")]
        app.configure_sets(
            Render,
            GizmoRenderSystem::QueueGizmos3d
                .in_set(RenderSet::Queue)
                .ambiguous_with(bevy_pbr::queue_material_meshes::<bevy_pbr::StandardMaterial>),
        );

        #[cfg(feature = "bevy_sprite")]
        app.configure_sets(
            Render,
            GizmoRenderSystem::QueueGizmos2d
                .in_set(RenderSet::Queue)
                .ambiguous_with(bevy_sprite::queue_sprites)
                .ambiguous_with(bevy_sprite::queue_material2d_meshes::<bevy_sprite::ColorMaterial>),
        );

        app.add_plugins(LineGizmoPlugin)
            .add_plugins(BillboardGizmoPlugin)
            .register_type::<GizmoConfig>()
            .register_type::<GizmoConfigStore>()
            // We insert the Resource GizmoConfigStore into the world implicitly here if it does not exist.
            .init_gizmo_group::<DefaultGizmoConfigGroup>();

        #[cfg(feature = "bevy_render")]
        app.add_plugins(aabb::AabbGizmoPlugin);

        #[cfg(all(feature = "bevy_pbr", feature = "bevy_render"))]
        app.add_plugins(LightGizmoPlugin);
    }
}

/// An extension trait adding `App::init_gizmo_group` and `App::insert_gizmo_config`.
pub trait AppGizmoBuilder {
    /// Registers [`GizmoConfigGroup`] in the app enabling the use of [Gizmos&lt;Config&gt;](crate::gizmos::Gizmos).
    ///
    /// Configurations can be set using the [`GizmoConfigStore`] [`Resource`](bevy_ecs::system::Resource).
    fn init_gizmo_group<Config: GizmoConfigGroup>(&mut self) -> &mut Self;

    /// Insert a [`GizmoConfig`] into a specific [`GizmoConfigGroup`].
    ///
    /// This method should be preferred over [`AppGizmoBuilder::init_gizmo_group`] if and only if you need to configure fields upon initialization.
    fn insert_gizmo_config<Config: GizmoConfigGroup>(
        &mut self,
        group: Config,
        config: GizmoConfig,
    ) -> &mut Self;
}

impl AppGizmoBuilder for App {
    fn init_gizmo_group<Config: GizmoConfigGroup>(&mut self) -> &mut Self {
        if self.world().contains_resource::<GizmoStorage<Config, ()>>() {
            return self;
        }

        self.world_mut()
            .get_resource_or_insert_with::<GizmoConfigStore>(Default::default)
            .register::<Config>();

        self.init_billboard_gizmo_group::<Config>();
        self.init_line_gizmo_group::<Config>();

        self.init_resource::<GizmoStorage<Config, ()>>()
            .init_resource::<GizmoStorage<Config, Fixed>>()
            .init_resource::<GizmoStorage<Config, Swap<Fixed>>>()
            .add_systems(
                RunFixedMainLoop,
                start_gizmo_context::<Config, Fixed>
                    .in_set(bevy_app::RunFixedMainLoopSystem::BeforeFixedMainLoop),
            )
            .add_systems(FixedFirst, clear_gizmo_context::<Config, Fixed>)
            .add_systems(FixedLast, collect_requested_gizmos::<Config, Fixed>)
            .add_systems(
                RunFixedMainLoop,
                end_gizmo_context::<Config, Fixed>
                    .in_set(bevy_app::RunFixedMainLoopSystem::AfterFixedMainLoop),
            )
            .add_systems(
                Last,
                (propagate_gizmos::<Config, Fixed>.before(UpdateGizmoMeshes),),
            );

        self
    }

    fn insert_gizmo_config<Config: GizmoConfigGroup>(
        &mut self,
        group: Config,
        config: GizmoConfig,
    ) -> &mut Self {
        self.init_gizmo_group::<Config>();

        self.world_mut()
            .get_resource_or_insert_with::<GizmoConfigStore>(Default::default)
            .insert(config, group);

        self
    }
}

/// Start a new gizmo clearing context.
///
/// Internally this pushes the parent default context into a swap buffer.
/// Gizmo contexts should be handled like a stack, so if you push a new context,
/// you must pop the context before the parent context ends.
pub fn start_gizmo_context<Config, Clear>(
    mut swap: ResMut<GizmoStorage<Config, Swap<Clear>>>,
    mut default: ResMut<GizmoStorage<Config, ()>>,
) where
    Config: GizmoConfigGroup,
    Clear: 'static + Send + Sync,
{
    default.swap(&mut *swap);
}

/// End this gizmo clearing context.
///
/// Pop the default gizmos context out of the [`Swap<Clear>`] gizmo storage.
///
/// This must be called before [`UpdateGizmoMeshes`] in the [`Last`] schedule.
pub fn end_gizmo_context<Config, Clear>(
    mut swap: ResMut<GizmoStorage<Config, Swap<Clear>>>,
    mut default: ResMut<GizmoStorage<Config, ()>>,
) where
    Config: GizmoConfigGroup,
    Clear: 'static + Send + Sync,
{
    default.clear();
    default.swap(&mut *swap);
}

/// Collect the requested gizmos into a specific clear context.
pub fn collect_requested_gizmos<Config, Clear>(
    mut update: ResMut<GizmoStorage<Config, ()>>,
    mut context: ResMut<GizmoStorage<Config, Clear>>,
) where
    Config: GizmoConfigGroup,
    Clear: 'static + Send + Sync,
{
    context.append_storage(&update);
    update.clear();
}

/// Clear out the contextual gizmos.
pub fn clear_gizmo_context<Config, Clear>(mut context: ResMut<GizmoStorage<Config, Clear>>)
where
    Config: GizmoConfigGroup,
    Clear: 'static + Send + Sync,
{
    context.clear();
}

/// Propagate the contextual gizmo into the `Update` storage for rendering.
///
/// This should be before [`UpdateGizmoMeshes`].
pub fn propagate_gizmos<Config, Clear>(
    mut update_storage: ResMut<GizmoStorage<Config, ()>>,
    contextual_storage: Res<GizmoStorage<Config, Clear>>,
) where
    Config: GizmoConfigGroup,
    Clear: 'static + Send + Sync,
{
    update_storage.append_storage(&*contextual_storage);
}

/// System set for updating the rendering meshes for drawing gizmos.
#[derive(SystemSet, Clone, Debug, PartialEq, Eq, Hash)]
pub struct UpdateGizmoMeshes;
