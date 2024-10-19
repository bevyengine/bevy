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
    QueueLineGizmos2d,
    /// Adds gizmos to the [`Transparent3d`](bevy_core_pipeline::core_3d::Transparent3d) render phase
    #[cfg(feature = "bevy_pbr")]
    QueueLineGizmos3d,
}

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
use bevy_asset::{Asset, AssetApp, Assets, Handle};
use bevy_color::LinearRgba;
use bevy_ecs::{
    schedule::{IntoSystemConfigs, SystemSet},
    system::{Res, ResMut, Resource},
};
use bevy_math::Vec3;
use bevy_reflect::TypePath;

#[cfg(all(
    feature = "bevy_render",
    any(feature = "bevy_pbr", feature = "bevy_sprite")
))]
use crate::config::GizmoMeshConfig;

#[cfg(feature = "bevy_render")]
use {
    bevy_ecs::{
        component::Component,
        entity::Entity,
        query::ROQueryItem,
        system::{
            lifetimeless::{Read, SRes},
            Commands, SystemParamItem,
        },
    },
    bevy_render::{
        extract_component::{ComponentUniforms, DynamicUniformIndex, UniformComponentPlugin},
        render_asset::{PrepareAssetError, RenderAsset, RenderAssetPlugin, RenderAssets},
        render_phase::{PhaseItem, RenderCommand, RenderCommandResult, TrackedRenderPass},
        render_resource::{
            binding_types::uniform_buffer, BindGroup, BindGroupEntries, BindGroupLayout,
            BindGroupLayoutEntries, Buffer, BufferInitDescriptor, BufferUsages, Shader,
            ShaderStages, ShaderType, VertexFormat,
        },
        renderer::RenderDevice,
        sync_world::{MainEntity, TemporaryRenderEntity},
        Extract, ExtractSchedule, Render, RenderApp, RenderSet,
    },
    bytemuck::cast_slice,
};

#[cfg(all(
    feature = "bevy_render",
    any(feature = "bevy_pbr", feature = "bevy_sprite"),
))]
use bevy_render::render_resource::{VertexAttribute, VertexBufferLayout, VertexStepMode};
use bevy_time::Fixed;
use bevy_utils::TypeIdMap;
use config::{
    DefaultGizmoConfigGroup, GizmoConfig, GizmoConfigGroup, GizmoConfigStore, GizmoLineJoint,
};
use core::{any::TypeId, mem};
use gizmos::{GizmoStorage, Swap};
#[cfg(all(feature = "bevy_pbr", feature = "bevy_render"))]
use light::LightGizmoPlugin;

#[cfg(feature = "bevy_render")]
const LINE_SHADER_HANDLE: Handle<Shader> = Handle::weak_from_u128(7414812689238026784);
#[cfg(feature = "bevy_render")]
const LINE_JOINT_SHADER_HANDLE: Handle<Shader> = Handle::weak_from_u128(1162780797909187908);

/// A [`Plugin`] that provides an immediate mode drawing api for visual debugging.
///
/// Requires to be loaded after [`PbrPlugin`](bevy_pbr::PbrPlugin) or [`SpritePlugin`](bevy_sprite::SpritePlugin).
#[derive(Default)]
pub struct GizmoPlugin;

impl Plugin for GizmoPlugin {
    fn build(&self, app: &mut App) {
        #[cfg(feature = "bevy_render")]
        {
            use bevy_asset::load_internal_asset;
            load_internal_asset!(app, LINE_SHADER_HANDLE, "lines.wgsl", Shader::from_wgsl);
            load_internal_asset!(
                app,
                LINE_JOINT_SHADER_HANDLE,
                "line_joints.wgsl",
                Shader::from_wgsl
            );
        }

        app.register_type::<GizmoConfig>()
            .register_type::<GizmoConfigStore>()
            .init_asset::<LineGizmo>()
            .init_resource::<LineGizmoHandles>()
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
            render_app.add_systems(
                Render,
                prepare_line_gizmo_bind_group.in_set(RenderSet::PrepareBindGroups),
            );

            render_app.add_systems(ExtractSchedule, extract_gizmo_data);

            #[cfg(feature = "bevy_sprite")]
            if app.is_plugin_added::<bevy_sprite::SpritePlugin>() {
                app.add_plugins(pipeline_2d::LineGizmo2dPlugin);
            } else {
                bevy_utils::tracing::warn!("bevy_sprite feature is enabled but bevy_sprite::SpritePlugin was not detected. Are you sure you loaded GizmoPlugin after SpritePlugin?");
            }
            #[cfg(feature = "bevy_pbr")]
            if app.is_plugin_added::<bevy_pbr::PbrPlugin>() {
                app.add_plugins(pipeline_3d::LineGizmo3dPlugin);
            } else {
                bevy_utils::tracing::warn!("bevy_pbr feature is enabled but bevy_pbr::PbrPlugin was not detected. Are you sure you loaded GizmoPlugin after PbrPlugin?");
            }
        } else {
            bevy_utils::tracing::warn!("bevy_render feature is enabled but RenderApp was not detected. Are you sure you loaded GizmoPlugin after RenderPlugin?");
        }
    }

    #[cfg(feature = "bevy_render")]
    fn finish(&self, app: &mut App) {
        let Some(render_app) = app.get_sub_app_mut(RenderApp) else {
            return;
        };

        let render_device = render_app.world().resource::<RenderDevice>();
        let line_layout = render_device.create_bind_group_layout(
            "LineGizmoUniform layout",
            &BindGroupLayoutEntries::single(
                ShaderStages::VERTEX,
                uniform_buffer::<LineGizmoUniform>(true),
            ),
        );

        render_app.insert_resource(LineGizmoUniformBindgroupLayout {
            layout: line_layout,
        });
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

        let mut handles = self.world_mut().get_resource_or_init::<LineGizmoHandles>();

        handles.list.insert(TypeId::of::<Config>(), None);
        handles.strip.insert(TypeId::of::<Config>(), None);

        // These handles are safe to mutate in any order
        self.allow_ambiguous_resource::<LineGizmoHandles>();

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
                (
                    propagate_gizmos::<Config, Fixed>.before(UpdateGizmoMeshes),
                    update_gizmo_meshes::<Config>.in_set(UpdateGizmoMeshes),
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
struct LineGizmoHandles {
    list: TypeIdMap<Option<Handle<LineGizmo>>>,
    strip: TypeIdMap<Option<Handle<LineGizmo>>>,
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

/// Prepare gizmos for rendering.
///
/// This also clears the default `GizmoStorage`.
fn update_gizmo_meshes<Config: GizmoConfigGroup>(
    mut line_gizmos: ResMut<Assets<LineGizmo>>,
    mut handles: ResMut<LineGizmoHandles>,
    mut storage: ResMut<GizmoStorage<Config, ()>>,
    config_store: Res<GizmoConfigStore>,
) {
    if storage.list_positions.is_empty() {
        handles.list.insert(TypeId::of::<Config>(), None);
    } else if let Some(handle) = handles.list.get_mut(&TypeId::of::<Config>()) {
        if let Some(handle) = handle {
            let list = line_gizmos.get_mut(handle.id()).unwrap();

            list.positions = mem::take(&mut storage.list_positions);
            list.colors = mem::take(&mut storage.list_colors);
        } else {
            let list = LineGizmo {
                strip: false,
                config_ty: TypeId::of::<Config>(),
                positions: mem::take(&mut storage.list_positions),
                colors: mem::take(&mut storage.list_colors),
                joints: GizmoLineJoint::None,
            };

            *handle = Some(line_gizmos.add(list));
        }
    }

    let (config, _) = config_store.config::<Config>();
    if storage.strip_positions.is_empty() {
        handles.strip.insert(TypeId::of::<Config>(), None);
    } else if let Some(handle) = handles.strip.get_mut(&TypeId::of::<Config>()) {
        if let Some(handle) = handle {
            let strip = line_gizmos.get_mut(handle.id()).unwrap();

            strip.positions = mem::take(&mut storage.strip_positions);
            strip.colors = mem::take(&mut storage.strip_colors);
            strip.joints = config.line_joints;
        } else {
            let strip = LineGizmo {
                strip: true,
                joints: config.line_joints,
                config_ty: TypeId::of::<Config>(),
                positions: mem::take(&mut storage.strip_positions),
                colors: mem::take(&mut storage.strip_colors),
            };

            *handle = Some(line_gizmos.add(strip));
        }
    }
}

#[cfg(feature = "bevy_render")]
fn extract_gizmo_data(
    mut commands: Commands,
    handles: Extract<Res<LineGizmoHandles>>,
    config: Extract<Res<GizmoConfigStore>>,
) {
    for (group_type_id, handle) in handles.list.iter().chain(handles.strip.iter()) {
        let Some((config, _)) = config.get_config_dyn(group_type_id) else {
            continue;
        };

        if !config.enabled {
            continue;
        }

        let Some(handle) = handle else {
            continue;
        };

        let joints_resolution = if let GizmoLineJoint::Round(resolution) = config.line_joints {
            resolution
        } else {
            0
        };

        commands.spawn((
            LineGizmoUniform {
                line_width: config.line_width,
                depth_bias: config.depth_bias,
                joints_resolution,
                #[cfg(feature = "webgl")]
                _padding: Default::default(),
            },
            #[cfg(any(feature = "bevy_pbr", feature = "bevy_sprite"))]
            GizmoMeshConfig {
                line_perspective: config.line_perspective,
                line_style: config.line_style,
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
    line_width: f32,
    depth_bias: f32,
    // Only used by gizmo line t if the current configs `line_joints` is set to `GizmoLineJoint::Round(_)`
    joints_resolution: u32,
    /// WebGL2 structs must be 16 byte aligned.
    #[cfg(feature = "webgl")]
    _padding: f32,
}

/// A gizmo asset that represents a line.
#[derive(Asset, Debug, Clone, TypePath)]
pub struct LineGizmo {
    /// Positions of the gizmo's vertices
    pub positions: Vec<Vec3>,
    /// Colors of the gizmo's vertices
    pub colors: Vec<LinearRgba>,
    /// Whether this gizmo's topology is a line-strip or line-list
    pub strip: bool,
    /// Whether this gizmo should draw line joints. This is only applicable if the gizmo's topology is line-strip.
    pub joints: GizmoLineJoint,
    /// The type of the gizmo's configuration group
    pub config_ty: TypeId,
}

#[cfg(feature = "bevy_render")]
#[derive(Debug, Clone)]
struct GpuLineGizmo {
    position_buffer: Buffer,
    color_buffer: Buffer,
    vertex_count: u32,
    strip: bool,
    joints: GizmoLineJoint,
}

#[cfg(feature = "bevy_render")]
impl RenderAsset for GpuLineGizmo {
    type SourceAsset = LineGizmo;
    type Param = SRes<RenderDevice>;

    fn prepare_asset(
        gizmo: Self::SourceAsset,
        render_device: &mut SystemParamItem<Self::Param>,
    ) -> Result<Self, PrepareAssetError<Self::SourceAsset>> {
        let position_buffer_data = cast_slice(&gizmo.positions);
        let position_buffer = render_device.create_buffer_with_data(&BufferInitDescriptor {
            usage: BufferUsages::VERTEX,
            label: Some("LineGizmo Position Buffer"),
            contents: position_buffer_data,
        });

        let color_buffer_data = cast_slice(&gizmo.colors);
        let color_buffer = render_device.create_buffer_with_data(&BufferInitDescriptor {
            usage: BufferUsages::VERTEX,
            label: Some("LineGizmo Color Buffer"),
            contents: color_buffer_data,
        });

        Ok(GpuLineGizmo {
            position_buffer,
            color_buffer,
            vertex_count: gizmo.positions.len() as u32,
            strip: gizmo.strip,
            joints: gizmo.joints,
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
        _view: ROQueryItem<'w, Self::ViewQuery>,
        uniform_index: Option<ROQueryItem<'w, Self::ItemQuery>>,
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
struct DrawLineGizmo;
#[cfg(all(
    feature = "bevy_render",
    any(feature = "bevy_pbr", feature = "bevy_sprite")
))]
impl<P: PhaseItem> RenderCommand<P> for DrawLineGizmo {
    type Param = SRes<RenderAssets<GpuLineGizmo>>;
    type ViewQuery = ();
    type ItemQuery = Read<GizmoMeshConfig>;

    #[inline]
    fn render<'w>(
        _item: &P,
        _view: ROQueryItem<'w, Self::ViewQuery>,
        config: Option<ROQueryItem<'w, Self::ItemQuery>>,
        line_gizmos: SystemParamItem<'w, '_, Self::Param>,
        pass: &mut TrackedRenderPass<'w>,
    ) -> RenderCommandResult {
        let Some(config) = config else {
            return RenderCommandResult::Skip;
        };
        let Some(line_gizmo) = line_gizmos.into_inner().get(&config.handle) else {
            return RenderCommandResult::Skip;
        };

        if line_gizmo.vertex_count < 2 {
            return RenderCommandResult::Success;
        }

        let instances = if line_gizmo.strip {
            let item_size = VertexFormat::Float32x3.size();
            let buffer_size = line_gizmo.position_buffer.size() - item_size;

            pass.set_vertex_buffer(0, line_gizmo.position_buffer.slice(..buffer_size));
            pass.set_vertex_buffer(1, line_gizmo.position_buffer.slice(item_size..));

            let item_size = VertexFormat::Float32x4.size();
            let buffer_size = line_gizmo.color_buffer.size() - item_size;
            pass.set_vertex_buffer(2, line_gizmo.color_buffer.slice(..buffer_size));
            pass.set_vertex_buffer(3, line_gizmo.color_buffer.slice(item_size..));

            u32::max(line_gizmo.vertex_count, 1) - 1
        } else {
            pass.set_vertex_buffer(0, line_gizmo.position_buffer.slice(..));
            pass.set_vertex_buffer(1, line_gizmo.color_buffer.slice(..));

            line_gizmo.vertex_count / 2
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
        _view: ROQueryItem<'w, Self::ViewQuery>,
        config: Option<ROQueryItem<'w, Self::ItemQuery>>,
        line_gizmos: SystemParamItem<'w, '_, Self::Param>,
        pass: &mut TrackedRenderPass<'w>,
    ) -> RenderCommandResult {
        let Some(config) = config else {
            return RenderCommandResult::Skip;
        };
        let Some(line_gizmo) = line_gizmos.into_inner().get(&config.handle) else {
            return RenderCommandResult::Skip;
        };

        if line_gizmo.vertex_count <= 2 || !line_gizmo.strip {
            return RenderCommandResult::Success;
        };

        if line_gizmo.joints == GizmoLineJoint::None {
            return RenderCommandResult::Success;
        };

        let instances = {
            let item_size = VertexFormat::Float32x3.size();
            // position_a
            let buffer_size_a = line_gizmo.position_buffer.size() - item_size * 2;
            pass.set_vertex_buffer(0, line_gizmo.position_buffer.slice(..buffer_size_a));
            // position_b
            let buffer_size_b = line_gizmo.position_buffer.size() - item_size;
            pass.set_vertex_buffer(
                1,
                line_gizmo.position_buffer.slice(item_size..buffer_size_b),
            );
            // position_c
            pass.set_vertex_buffer(2, line_gizmo.position_buffer.slice(item_size * 2..));

            // color
            let item_size = VertexFormat::Float32x4.size();
            let buffer_size = line_gizmo.color_buffer.size() - item_size;
            // This corresponds to the color of position_b, hence starts from `item_size`
            pass.set_vertex_buffer(3, line_gizmo.color_buffer.slice(item_size..buffer_size));

            u32::max(line_gizmo.vertex_count, 2) - 2
        };

        let vertices = match line_gizmo.joints {
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
