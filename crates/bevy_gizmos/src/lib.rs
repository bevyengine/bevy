#![cfg_attr(docsrs, feature(doc_auto_cfg))]
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

/// System set label for the systems handling the rendering of gizmos.
#[derive(SystemSet, Clone, Debug, Hash, PartialEq, Eq)]
pub enum GizmoRenderSystems {
    /// Adds gizmos to the [`Transparent2d`](bevy_core_pipeline::core_2d::Transparent2d) render phase
    #[cfg(feature = "bevy_sprite")]
    QueueLineGizmos2d,
    /// Adds gizmos to the [`Transparent3d`](bevy_core_pipeline::core_3d::Transparent3d) render phase
    #[cfg(feature = "bevy_pbr")]
    QueueLineGizmos3d,
}

/// Deprecated alias for [`GizmoRenderSystems`].
#[deprecated(since = "0.17.0", note = "Renamed to `GizmoRenderSystems`.")]
pub type GizmoRenderSystem = GizmoRenderSystems;

#[cfg(feature = "bevy_render")]
pub mod aabb;
pub mod arcs;
pub mod arrows;
pub mod circles;
pub mod config;
pub mod cross;
pub mod curves;
pub mod gizmos;
pub mod grid;
pub mod primitives;
pub mod retained;
pub mod rounded_box;

#[cfg(all(feature = "bevy_pbr", feature = "bevy_render"))]
pub mod light;

#[cfg(all(feature = "bevy_sprite", feature = "bevy_render"))]
mod pipeline_2d;
#[cfg(all(feature = "bevy_pbr", feature = "bevy_render"))]
mod pipeline_3d;

/// The gizmos prelude.
///
/// This includes the most common types in this crate, re-exported for your convenience.
pub mod prelude {
    #[cfg(feature = "bevy_render")]
    pub use crate::aabb::{AabbGizmoConfigGroup, ShowAabbGizmo};

    #[doc(hidden)]
    pub use crate::{
        config::{
            DefaultGizmoConfigGroup, GizmoConfig, GizmoConfigGroup, GizmoConfigStore,
            GizmoLineConfig, GizmoLineJoint, GizmoLineStyle,
        },
        gizmos::Gizmos,
        primitives::{dim2::GizmoPrimitive2d, dim3::GizmoPrimitive3d},
        retained::Gizmo,
        AppGizmoBuilder, GizmoAsset,
    };

    #[cfg(all(feature = "bevy_pbr", feature = "bevy_render"))]
    pub use crate::light::{LightGizmoColor, LightGizmoConfigGroup, ShowLightGizmo};
}

use bevy_app::{App, FixedFirst, FixedLast, Last, Plugin, RunFixedMainLoop};
use bevy_asset::{Asset, AssetApp, Assets, Handle};
use bevy_ecs::{
    resource::Resource,
    schedule::{IntoScheduleConfigs, SystemSet},
    system::{Res, ResMut},
};
use bevy_reflect::TypePath;

#[cfg(all(
    feature = "bevy_render",
    any(feature = "bevy_pbr", feature = "bevy_sprite")
))]
use {crate::config::GizmoMeshConfig, bevy_mesh::VertexBufferLayout};

use crate::{config::ErasedGizmoConfigGroup, gizmos::GizmoBuffer};

#[cfg(feature = "bevy_render")]
use {
    crate::retained::extract_linegizmos,
    bevy_asset::AssetId,
    bevy_ecs::{
        component::Component,
        entity::Entity,
        query::ROQueryItem,
        system::{
            lifetimeless::{Read, SRes},
            Commands, SystemParamItem,
        },
    },
    bevy_math::{Affine3, Affine3A, Vec4},
    bevy_render::{
        extract_component::{ComponentUniforms, DynamicUniformIndex, UniformComponentPlugin},
        render_asset::{PrepareAssetError, RenderAsset, RenderAssetPlugin, RenderAssets},
        render_phase::{PhaseItem, RenderCommand, RenderCommandResult, TrackedRenderPass},
        render_resource::{
            binding_types::uniform_buffer, BindGroup, BindGroupEntries, BindGroupLayout,
            BindGroupLayoutEntries, Buffer, BufferInitDescriptor, BufferUsages, ShaderStages,
            ShaderType, VertexFormat,
        },
        renderer::RenderDevice,
        sync_world::{MainEntity, TemporaryRenderEntity},
        Extract, ExtractSchedule, Render, RenderApp, RenderStartup, RenderSystems,
    },
    bytemuck::cast_slice,
};

#[cfg(all(
    feature = "bevy_render",
    any(feature = "bevy_pbr", feature = "bevy_sprite"),
))]
use bevy_render::render_resource::{VertexAttribute, VertexStepMode};
use bevy_time::Fixed;
use bevy_utils::TypeIdMap;
#[cfg(feature = "bevy_render")]
use config::GizmoLineJoint;
use config::{DefaultGizmoConfigGroup, GizmoConfig, GizmoConfigGroup, GizmoConfigStore};
use core::{any::TypeId, marker::PhantomData, mem};
use gizmos::{GizmoStorage, Swap};
#[cfg(all(feature = "bevy_pbr", feature = "bevy_render"))]
use light::LightGizmoPlugin;

/// A [`Plugin`] that provides an immediate mode drawing api for visual debugging.
///
/// Requires to be loaded after [`PbrPlugin`](bevy_pbr::PbrPlugin) or [`SpritePlugin`](bevy_sprite::SpritePlugin).
#[derive(Default)]
pub struct GizmoPlugin;

impl Plugin for GizmoPlugin {
    fn build(&self, app: &mut App) {
        #[cfg(feature = "bevy_render")]
        {
            use bevy_asset::embedded_asset;
            embedded_asset!(app, "lines.wgsl");
            embedded_asset!(app, "line_joints.wgsl");
        }

        app.init_asset::<GizmoAsset>()
            .init_resource::<GizmoHandles>()
            // We insert the Resource GizmoConfigStore into the world implicitly here if it does not exist.
            .init_gizmo_group::<DefaultGizmoConfigGroup>();

        #[cfg(feature = "bevy_render")]
        app.add_plugins(aabb::AabbGizmoPlugin)
            .add_plugins(UniformComponentPlugin::<LineGizmoUniform>::default())
            .add_plugins(RenderAssetPlugin::<GpuLineGizmo>::default());

        #[cfg(all(feature = "bevy_pbr", feature = "bevy_render"))]
        app.add_plugins(LightGizmoPlugin);

        #[cfg(feature = "bevy_render")]
        if let Some(render_app) = app.get_sub_app_mut(RenderApp) {
            render_app.add_systems(RenderStartup, init_line_gizmo_uniform_bind_group_layout);

            render_app.add_systems(
                Render,
                prepare_line_gizmo_bind_group.in_set(RenderSystems::PrepareBindGroups),
            );

            render_app.add_systems(ExtractSchedule, (extract_gizmo_data, extract_linegizmos));

            #[cfg(feature = "bevy_sprite")]
            if app.is_plugin_added::<bevy_sprite::SpritePlugin>() {
                app.add_plugins(pipeline_2d::LineGizmo2dPlugin);
            } else {
                tracing::warn!("bevy_sprite feature is enabled but bevy_sprite::SpritePlugin was not detected. Are you sure you loaded GizmoPlugin after SpritePlugin?");
            }
            #[cfg(feature = "bevy_pbr")]
            if app.is_plugin_added::<bevy_pbr::PbrPlugin>() {
                app.add_plugins(pipeline_3d::LineGizmo3dPlugin);
            } else {
                tracing::warn!("bevy_pbr feature is enabled but bevy_pbr::PbrPlugin was not detected. Are you sure you loaded GizmoPlugin after PbrPlugin?");
            }
        } else {
            tracing::warn!("bevy_render feature is enabled but RenderApp was not detected. Are you sure you loaded GizmoPlugin after RenderPlugin?");
        }
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
struct GizmoHandles {
    handles: TypeIdMap<Option<Handle<GizmoAsset>>>,
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

/// Deprecated alias for [`GizmoMeshSystems`].
#[deprecated(since = "0.17.0", note = "Renamed to `GizmoMeshSystems`.")]
pub type UpdateGizmoMeshes = GizmoMeshSystems;

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
                    marker: PhantomData,
                },
            };

            *handle = Some(gizmo_assets.add(gizmo));
        }
    }
}

#[cfg(feature = "bevy_render")]
fn init_line_gizmo_uniform_bind_group_layout(
    mut commands: Commands,
    render_device: Res<RenderDevice>,
) {
    let line_layout = render_device.create_bind_group_layout(
        "LineGizmoUniform layout",
        &BindGroupLayoutEntries::single(
            ShaderStages::VERTEX,
            uniform_buffer::<LineGizmoUniform>(true),
        ),
    );

    commands.insert_resource(LineGizmoUniformBindgroupLayout {
        layout: line_layout,
    });
}

#[cfg(feature = "bevy_render")]
fn extract_gizmo_data(
    mut commands: Commands,
    handles: Extract<Res<GizmoHandles>>,
    config: Extract<Res<GizmoConfigStore>>,
) {
    use bevy_utils::once;
    use config::GizmoLineStyle;
    use tracing::warn;

    for (group_type_id, handle) in &handles.handles {
        let Some((config, _)) = config.get_config_dyn(group_type_id) else {
            continue;
        };

        if !config.enabled {
            continue;
        }

        let Some(handle) = handle else {
            continue;
        };

        let joints_resolution = if let GizmoLineJoint::Round(resolution) = config.line.joints {
            resolution
        } else {
            0
        };

        let (gap_scale, line_scale) = if let GizmoLineStyle::Dashed {
            gap_scale,
            line_scale,
        } = config.line.style
        {
            if gap_scale <= 0.0 {
                once!(warn!("When using gizmos with the line style `GizmoLineStyle::Dashed{{..}}` the gap scale should be greater than zero."));
            }
            if line_scale <= 0.0 {
                once!(warn!("When using gizmos with the line style `GizmoLineStyle::Dashed{{..}}` the line scale should be greater than zero."));
            }
            (gap_scale, line_scale)
        } else {
            (1.0, 1.0)
        };

        commands.spawn((
            LineGizmoUniform {
                world_from_local: Affine3::from(&Affine3A::IDENTITY).to_transpose(),
                line_width: config.line.width,
                depth_bias: config.depth_bias,
                joints_resolution,
                gap_scale,
                line_scale,
                #[cfg(feature = "webgl")]
                _padding: Default::default(),
            },
            #[cfg(any(feature = "bevy_pbr", feature = "bevy_sprite"))]
            GizmoMeshConfig {
                line_perspective: config.line.perspective,
                line_style: config.line.style,
                line_joints: config.line.joints,
                render_layers: config.render_layers.clone(),
                handle: handle.clone(),
            },
            // The immediate mode API does not have a main world entity to refer to,
            // but we do need MainEntity on this render entity for the systems to find it.
            MainEntity::from(Entity::PLACEHOLDER),
            TemporaryRenderEntity,
        ));
    }
}

#[cfg(feature = "bevy_render")]
#[derive(Component, ShaderType, Clone, Copy)]
struct LineGizmoUniform {
    world_from_local: [Vec4; 3],
    line_width: f32,
    depth_bias: f32,
    // Only used by gizmo line t if the current configs `line_joints` is set to `GizmoLineJoint::Round(_)`
    joints_resolution: u32,
    // Only used if the current configs `line_style` is set to `GizmoLineStyle::Dashed{_}`
    gap_scale: f32,
    line_scale: f32,
    /// WebGL2 structs must be 16 byte aligned.
    #[cfg(feature = "webgl")]
    _padding: bevy_math::Vec3,
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

#[cfg(feature = "bevy_render")]
#[derive(Debug, Clone)]
struct GpuLineGizmo {
    list_position_buffer: Buffer,
    list_color_buffer: Buffer,
    list_vertex_count: u32,
    strip_position_buffer: Buffer,
    strip_color_buffer: Buffer,
    strip_vertex_count: u32,
}

#[cfg(feature = "bevy_render")]
impl RenderAsset for GpuLineGizmo {
    type SourceAsset = GizmoAsset;
    type Param = SRes<RenderDevice>;

    fn prepare_asset(
        gizmo: Self::SourceAsset,
        _: AssetId<Self::SourceAsset>,
        render_device: &mut SystemParamItem<Self::Param>,
        _: Option<&Self>,
    ) -> Result<Self, PrepareAssetError<Self::SourceAsset>> {
        let list_position_buffer = render_device.create_buffer_with_data(&BufferInitDescriptor {
            usage: BufferUsages::VERTEX,
            label: Some("LineGizmo Position Buffer"),
            contents: cast_slice(&gizmo.buffer.list_positions),
        });

        let list_color_buffer = render_device.create_buffer_with_data(&BufferInitDescriptor {
            usage: BufferUsages::VERTEX,
            label: Some("LineGizmo Color Buffer"),
            contents: cast_slice(&gizmo.buffer.list_colors),
        });

        let strip_position_buffer = render_device.create_buffer_with_data(&BufferInitDescriptor {
            usage: BufferUsages::VERTEX,
            label: Some("LineGizmo Strip Position Buffer"),
            contents: cast_slice(&gizmo.buffer.strip_positions),
        });

        let strip_color_buffer = render_device.create_buffer_with_data(&BufferInitDescriptor {
            usage: BufferUsages::VERTEX,
            label: Some("LineGizmo Strip Color Buffer"),
            contents: cast_slice(&gizmo.buffer.strip_colors),
        });

        Ok(GpuLineGizmo {
            list_position_buffer,
            list_color_buffer,
            list_vertex_count: gizmo.buffer.list_positions.len() as u32,
            strip_position_buffer,
            strip_color_buffer,
            strip_vertex_count: gizmo.buffer.strip_positions.len() as u32,
        })
    }
}

#[cfg(feature = "bevy_render")]
#[derive(Resource)]
struct LineGizmoUniformBindgroupLayout {
    layout: BindGroupLayout,
}

#[cfg(feature = "bevy_render")]
#[derive(Resource)]
struct LineGizmoUniformBindgroup {
    bindgroup: BindGroup,
}

#[cfg(feature = "bevy_render")]
fn prepare_line_gizmo_bind_group(
    mut commands: Commands,
    line_gizmo_uniform_layout: Res<LineGizmoUniformBindgroupLayout>,
    render_device: Res<RenderDevice>,
    line_gizmo_uniforms: Res<ComponentUniforms<LineGizmoUniform>>,
) {
    if let Some(binding) = line_gizmo_uniforms.uniforms().binding() {
        commands.insert_resource(LineGizmoUniformBindgroup {
            bindgroup: render_device.create_bind_group(
                "LineGizmoUniform bindgroup",
                &line_gizmo_uniform_layout.layout,
                &BindGroupEntries::single(binding),
            ),
        });
    }
}

#[cfg(feature = "bevy_render")]
struct SetLineGizmoBindGroup<const I: usize>;
#[cfg(feature = "bevy_render")]
impl<const I: usize, P: PhaseItem> RenderCommand<P> for SetLineGizmoBindGroup<I> {
    type Param = SRes<LineGizmoUniformBindgroup>;
    type ViewQuery = ();
    type ItemQuery = Read<DynamicUniformIndex<LineGizmoUniform>>;

    #[inline]
    fn render<'w>(
        _item: &P,
        _view: ROQueryItem<'w, '_, Self::ViewQuery>,
        uniform_index: Option<ROQueryItem<'w, '_, Self::ItemQuery>>,
        bind_group: SystemParamItem<'w, '_, Self::Param>,
        pass: &mut TrackedRenderPass<'w>,
    ) -> RenderCommandResult {
        let Some(uniform_index) = uniform_index else {
            return RenderCommandResult::Skip;
        };
        pass.set_bind_group(
            I,
            &bind_group.into_inner().bindgroup,
            &[uniform_index.index()],
        );
        RenderCommandResult::Success
    }
}

#[cfg(feature = "bevy_render")]
struct DrawLineGizmo<const STRIP: bool>;
#[cfg(all(
    feature = "bevy_render",
    any(feature = "bevy_pbr", feature = "bevy_sprite")
))]
impl<P: PhaseItem, const STRIP: bool> RenderCommand<P> for DrawLineGizmo<STRIP> {
    type Param = SRes<RenderAssets<GpuLineGizmo>>;
    type ViewQuery = ();
    type ItemQuery = Read<GizmoMeshConfig>;

    #[inline]
    fn render<'w>(
        _item: &P,
        _view: ROQueryItem<'w, '_, Self::ViewQuery>,
        config: Option<ROQueryItem<'w, '_, Self::ItemQuery>>,
        line_gizmos: SystemParamItem<'w, '_, Self::Param>,
        pass: &mut TrackedRenderPass<'w>,
    ) -> RenderCommandResult {
        let Some(config) = config else {
            return RenderCommandResult::Skip;
        };
        let Some(line_gizmo) = line_gizmos.into_inner().get(&config.handle) else {
            return RenderCommandResult::Skip;
        };

        let vertex_count = if STRIP {
            line_gizmo.strip_vertex_count
        } else {
            line_gizmo.list_vertex_count
        };

        if vertex_count < 2 {
            return RenderCommandResult::Success;
        }

        let instances = if STRIP {
            let item_size = VertexFormat::Float32x3.size();
            let buffer_size = line_gizmo.strip_position_buffer.size() - item_size;

            pass.set_vertex_buffer(0, line_gizmo.strip_position_buffer.slice(..buffer_size));
            pass.set_vertex_buffer(1, line_gizmo.strip_position_buffer.slice(item_size..));

            let item_size = VertexFormat::Float32x4.size();
            let buffer_size = line_gizmo.strip_color_buffer.size() - item_size;

            pass.set_vertex_buffer(2, line_gizmo.strip_color_buffer.slice(..buffer_size));
            pass.set_vertex_buffer(3, line_gizmo.strip_color_buffer.slice(item_size..));

            vertex_count - 1
        } else {
            pass.set_vertex_buffer(0, line_gizmo.list_position_buffer.slice(..));
            pass.set_vertex_buffer(1, line_gizmo.list_color_buffer.slice(..));

            vertex_count / 2
        };

        pass.draw(0..6, 0..instances);

        RenderCommandResult::Success
    }
}

#[cfg(feature = "bevy_render")]
struct DrawLineJointGizmo;
#[cfg(all(
    feature = "bevy_render",
    any(feature = "bevy_pbr", feature = "bevy_sprite")
))]
impl<P: PhaseItem> RenderCommand<P> for DrawLineJointGizmo {
    type Param = SRes<RenderAssets<GpuLineGizmo>>;
    type ViewQuery = ();
    type ItemQuery = Read<GizmoMeshConfig>;

    #[inline]
    fn render<'w>(
        _item: &P,
        _view: ROQueryItem<'w, '_, Self::ViewQuery>,
        config: Option<ROQueryItem<'w, '_, Self::ItemQuery>>,
        line_gizmos: SystemParamItem<'w, '_, Self::Param>,
        pass: &mut TrackedRenderPass<'w>,
    ) -> RenderCommandResult {
        let Some(config) = config else {
            return RenderCommandResult::Skip;
        };
        let Some(line_gizmo) = line_gizmos.into_inner().get(&config.handle) else {
            return RenderCommandResult::Skip;
        };

        if line_gizmo.strip_vertex_count <= 2 {
            return RenderCommandResult::Success;
        };

        if config.line_joints == GizmoLineJoint::None {
            return RenderCommandResult::Success;
        };

        let instances = {
            let item_size = VertexFormat::Float32x3.size();
            // position_a
            let buffer_size_a = line_gizmo.strip_position_buffer.size() - item_size * 2;
            pass.set_vertex_buffer(0, line_gizmo.strip_position_buffer.slice(..buffer_size_a));
            // position_b
            let buffer_size_b = line_gizmo.strip_position_buffer.size() - item_size;
            pass.set_vertex_buffer(
                1,
                line_gizmo
                    .strip_position_buffer
                    .slice(item_size..buffer_size_b),
            );
            // position_c
            pass.set_vertex_buffer(2, line_gizmo.strip_position_buffer.slice(item_size * 2..));

            // color
            let item_size = VertexFormat::Float32x4.size();
            let buffer_size = line_gizmo.strip_color_buffer.size() - item_size;
            // This corresponds to the color of position_b, hence starts from `item_size`
            pass.set_vertex_buffer(
                3,
                line_gizmo.strip_color_buffer.slice(item_size..buffer_size),
            );

            line_gizmo.strip_vertex_count - 2
        };

        let vertices = match config.line_joints {
            GizmoLineJoint::None => unreachable!(),
            GizmoLineJoint::Miter => 6,
            GizmoLineJoint::Round(resolution) => resolution * 3,
            GizmoLineJoint::Bevel => 3,
        };

        pass.draw(0..vertices, 0..instances);

        RenderCommandResult::Success
    }
}

#[cfg(all(
    feature = "bevy_render",
    any(feature = "bevy_pbr", feature = "bevy_sprite")
))]
fn line_gizmo_vertex_buffer_layouts(strip: bool) -> Vec<VertexBufferLayout> {
    use VertexFormat::*;
    let mut position_layout = VertexBufferLayout {
        array_stride: Float32x3.size(),
        step_mode: VertexStepMode::Instance,
        attributes: vec![VertexAttribute {
            format: Float32x3,
            offset: 0,
            shader_location: 0,
        }],
    };

    let mut color_layout = VertexBufferLayout {
        array_stride: Float32x4.size(),
        step_mode: VertexStepMode::Instance,
        attributes: vec![VertexAttribute {
            format: Float32x4,
            offset: 0,
            shader_location: 2,
        }],
    };

    if strip {
        vec![
            position_layout.clone(),
            {
                position_layout.attributes[0].shader_location = 1;
                position_layout
            },
            color_layout.clone(),
            {
                color_layout.attributes[0].shader_location = 3;
                color_layout
            },
        ]
    } else {
        position_layout.array_stride *= 2;
        position_layout.attributes.push(VertexAttribute {
            format: Float32x3,
            offset: Float32x3.size(),
            shader_location: 1,
        });

        color_layout.array_stride *= 2;
        color_layout.attributes.push(VertexAttribute {
            format: Float32x4,
            offset: Float32x4.size(),
            shader_location: 3,
        });

        vec![position_layout, color_layout]
    }
}

#[cfg(all(
    feature = "bevy_render",
    any(feature = "bevy_pbr", feature = "bevy_sprite")
))]
fn line_joint_gizmo_vertex_buffer_layouts() -> Vec<VertexBufferLayout> {
    use VertexFormat::*;
    let mut position_layout = VertexBufferLayout {
        array_stride: Float32x3.size(),
        step_mode: VertexStepMode::Instance,
        attributes: vec![VertexAttribute {
            format: Float32x3,
            offset: 0,
            shader_location: 0,
        }],
    };

    let color_layout = VertexBufferLayout {
        array_stride: Float32x4.size(),
        step_mode: VertexStepMode::Instance,
        attributes: vec![VertexAttribute {
            format: Float32x4,
            offset: 0,
            shader_location: 3,
        }],
    };

    vec![
        position_layout.clone(),
        {
            position_layout.attributes[0].shader_location = 1;
            position_layout.clone()
        },
        {
            position_layout.attributes[0].shader_location = 2;
            position_layout
        },
        color_layout.clone(),
    ]
}
