#![cfg_attr(docsrs, feature(doc_cfg))]
#![doc(
    html_logo_url = "https://bevy.org/assets/icon.png",
    html_favicon_url = "https://bevy.org/assets/icon.png"
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

// Required to make proc macros work in bevy itself.
extern crate self as bevy_gizmos;

pub mod aabb;
pub mod arcs;
pub mod arrows;
pub mod circles;
pub mod config;
pub mod cross;
pub mod curves;
pub mod gizmos;
mod global;
pub mod grid;
pub mod primitives;
pub mod retained;
pub mod rounded_box;
pub mod text;

#[cfg(feature = "bevy_light")]
pub mod light;

/// The gizmos prelude.
///
/// This includes the most common types in this crate, re-exported for your convenience.
pub mod prelude {
    #[doc(hidden)]
    pub use crate::aabb::{AabbGizmoConfigGroup, ShowAabbGizmo};

    #[doc(hidden)]
    pub use crate::{
        config::{
            DefaultGizmoConfigGroup, GizmoConfig, GizmoConfigGroup, GizmoConfigStore,
            GizmoLineConfig, GizmoLineJoint, GizmoLineStyle,
        },
        gizmos::Gizmos,
        global::gizmo,
        primitives::{dim2::GizmoPrimitive2d, dim3::GizmoPrimitive3d},
        retained::Gizmo,
        AppGizmoBuilder, GizmoAsset,
    };

    #[doc(hidden)]
    #[cfg(feature = "bevy_light")]
    pub use crate::light::{LightGizmoColor, LightGizmoConfigGroup, ShowLightGizmo};
}

use bevy_app::{App, FixedFirst, FixedLast, Last, Plugin, PostUpdate, RunFixedMainLoop};
use bevy_asset::{Asset, AssetApp, Assets, Handle};
use bevy_ecs::{
    resource::Resource,
    schedule::{IntoScheduleConfigs, SystemSet},
    system::{Res, ResMut},
};
use bevy_reflect::TypePath;

use crate::{
    config::ErasedGizmoConfigGroup,
    gizmos::GizmoBuffer,
    text::{gizmo_text_system, GizmoText, GizmoTextBuffer},
};

use bevy_time::Fixed;
use bevy_utils::TypeIdMap;
use config::{DefaultGizmoConfigGroup, GizmoConfig, GizmoConfigGroup, GizmoConfigStore};
use core::{any::TypeId, marker::PhantomData, mem};
use gizmos::{GizmoStorage, Swap};
#[cfg(feature = "bevy_light")]
use light::LightGizmoPlugin;

/// A [`Plugin`] that provides an immediate mode drawing api for visual debugging.
#[derive(Default)]
pub struct GizmoPlugin;

impl Plugin for GizmoPlugin {
    fn build(&self, app: &mut App) {
        app.init_asset::<GizmoAsset>()
            .init_resource::<GizmoHandles>()
            // We insert the Resource GizmoConfigStore into the world implicitly here if it does not exist.
            .init_gizmo_group::<DefaultGizmoConfigGroup>();

        app.add_plugins((aabb::AabbGizmoPlugin, global::GlobalGizmosPlugin));

        #[cfg(feature = "bevy_light")]
        app.add_plugins(LightGizmoPlugin);
    }
}

/// A extension trait adding `App::init_gizmo_group` and `App::insert_gizmo_config`.
pub trait AppGizmoBuilder {
    /// Registers [`GizmoConfigGroup`] in the app enabling the use of [Gizmos&lt;Config&gt;](crate::gizmos::Gizmos).
    ///
    /// Configurations can be set using the [`GizmoConfigStore`] [`Resource`].
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
            .get_resource_or_init::<GizmoConfigStore>()
            .register::<Config>();

        let mut handles = self.world_mut().get_resource_or_init::<GizmoHandles>();

        handles.handles.insert(TypeId::of::<Config>(), None);

        // These handles are safe to mutate in any order
        self.allow_ambiguous_resource::<GizmoHandles>();

        self.init_resource::<GizmoStorage<Config, ()>>()
            .init_resource::<GizmoStorage<Config, Fixed>>()
            .init_resource::<GizmoStorage<Config, Swap<Fixed>>>()
            .init_resource::<GizmoTextBuffer<Config, ()>>()
            .add_systems(
                RunFixedMainLoop,
                start_gizmo_context::<Config, Fixed>
                    .in_set(bevy_app::RunFixedMainLoopSystems::BeforeFixedMainLoop),
            )
            .add_systems(FixedFirst, clear_gizmo_context::<Config, Fixed>)
            .add_systems(FixedLast, collect_requested_gizmos::<Config, Fixed>)
            .add_systems(
                RunFixedMainLoop,
                end_gizmo_context::<Config, Fixed>
                    .in_set(bevy_app::RunFixedMainLoopSystems::AfterFixedMainLoop),
            )
            .add_systems(PostUpdate, gizmo_text_system::<Config, ()>)
            .add_systems(
                Last,
                (
                    propagate_gizmos::<Config, Fixed>.before(GizmoMeshSystems),
                    update_gizmo_meshes::<Config>.in_set(GizmoMeshSystems),
                ),
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
            .get_resource_or_init::<GizmoConfigStore>()
            .insert(config, group);

        self
    }
}

/// Holds handles to the line gizmos for each gizmo configuration group
// As `TypeIdMap` iteration order depends on the order of insertions and deletions, this uses
// `Option<Handle>` to be able to reserve the slot when creating the gizmo configuration group.
// That way iteration order is stable across executions and depends on the order of configuration
// group creation.
#[derive(Resource, Default)]
pub struct GizmoHandles {
    handles: TypeIdMap<Option<Handle<GizmoAsset>>>,
}

impl GizmoHandles {
    /// The handles to the gizmo assets of each gizmo configuration group.
    pub fn handles(&self) -> &TypeIdMap<Option<Handle<GizmoAsset>>> {
        &self.handles
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
/// This must be called before [`GizmoMeshSystems`] in the [`Last`] schedule.
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
/// This should be before [`GizmoMeshSystems`].
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
pub struct GizmoMeshSystems;

/// Prepare gizmos for rendering.
///
/// This also clears the default `GizmoStorage`.
fn update_gizmo_meshes<Config: GizmoConfigGroup>(
    mut gizmo_assets: ResMut<Assets<GizmoAsset>>,
    mut handles: ResMut<GizmoHandles>,
    mut storage: ResMut<GizmoStorage<Config, ()>>,
) {
    if storage.list_positions.is_empty() && storage.strip_positions.is_empty() {
        handles.handles.insert(TypeId::of::<Config>(), None);
    } else if let Some(handle) = handles.handles.get_mut(&TypeId::of::<Config>()) {
        if let Some(handle) = handle {
            let gizmo = gizmo_assets.get_mut(handle.id()).unwrap();

            gizmo.buffer.list_positions = mem::take(&mut storage.list_positions);
            gizmo.buffer.list_colors = mem::take(&mut storage.list_colors);
            gizmo.buffer.strip_positions = mem::take(&mut storage.strip_positions);
            gizmo.buffer.strip_colors = mem::take(&mut storage.strip_colors);
        } else {
            let gizmo = GizmoAsset {
                config_ty: TypeId::of::<Config>(),
                buffer: GizmoBuffer {
                    enabled: true,
                    list_positions: mem::take(&mut storage.list_positions),
                    list_colors: mem::take(&mut storage.list_colors),
                    strip_positions: mem::take(&mut storage.strip_positions),
                    strip_colors: mem::take(&mut storage.strip_colors),
                    glyph_vertices: mem::take(&mut storage.glyph_vertices),
                    glyph_uvs: mem::take(&mut storage.glyph_uvs),
                    glyph_colors: mem::take(&mut storage.glyph_colors),
                    marker: PhantomData,
                },
            };

            *handle = Some(gizmo_assets.add(gizmo));
        }
    }
}

/// A collection of gizmos.
///
/// Has the same gizmo drawing API as [`Gizmos`](crate::gizmos::Gizmos).
#[derive(Asset, Debug, Clone, TypePath)]
pub struct GizmoAsset {
    /// vertex buffers.
    buffer: GizmoBuffer<ErasedGizmoConfigGroup, ()>,
    config_ty: TypeId,
}

impl GizmoAsset {
    /// A reference to the gizmo's vertex buffer.
    pub fn buffer(&self) -> &GizmoBuffer<ErasedGizmoConfigGroup, ()> {
        &self.buffer
    }
}

impl GizmoAsset {
    /// Create a new [`GizmoAsset`].
    pub fn new() -> Self {
        GizmoAsset {
            buffer: GizmoBuffer::default(),
            config_ty: TypeId::of::<ErasedGizmoConfigGroup>(),
        }
    }

    /// The type of the gizmo's configuration group.
    pub fn config_typeid(&self) -> TypeId {
        self.config_ty
    }
}

impl Default for GizmoAsset {
    fn default() -> Self {
        GizmoAsset::new()
    }
}
