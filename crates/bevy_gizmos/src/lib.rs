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
//! # use bevy_render::prelude::*;
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

pub mod aabb;
pub mod arcs;
pub mod arrows;
pub mod circles;
pub mod config;
pub mod gizmos;
pub mod grid;
pub mod light;
pub mod primitives;

#[cfg(feature = "bevy_sprite")]
mod pipeline_2d;
#[cfg(feature = "bevy_pbr")]
mod pipeline_3d;

/// The `bevy_gizmos` prelude.
pub mod prelude {
    #[doc(hidden)]
    pub use crate::{
        aabb::{AabbGizmoConfigGroup, ShowAabbGizmo},
        config::{
            DefaultGizmoConfigGroup, GizmoConfig, GizmoConfigGroup, GizmoConfigStore,
            GizmoLineJoint, GizmoLineStyle,
        },
        gizmos::Gizmos,
        light::{LightGizmoColor, LightGizmoConfigGroup, ShowLightGizmo},
        primitives::{dim2::GizmoPrimitive2d, dim3::GizmoPrimitive3d},
        AppGizmoBuilder,
    };
}

use aabb::AabbGizmoPlugin;
use bevy_app::{App, Last, Plugin};
use bevy_asset::{load_internal_asset, Asset, AssetApp, Assets, Handle};
use bevy_color::LinearRgba;
use bevy_ecs::{
    component::Component,
    query::ROQueryItem,
    schedule::{IntoSystemConfigs, SystemSet},
    system::{
        lifetimeless::{Read, SRes},
        Commands, Res, ResMut, Resource, SystemParamItem,
    },
};
use bevy_math::Vec3;
use bevy_reflect::TypePath;
use bevy_render::{
    extract_component::{ComponentUniforms, DynamicUniformIndex, UniformComponentPlugin},
    render_asset::{
        PrepareAssetError, RenderAsset, RenderAssetPlugin, RenderAssetUsages, RenderAssets,
    },
    render_phase::{PhaseItem, RenderCommand, RenderCommandResult, TrackedRenderPass},
    render_resource::{
        binding_types::uniform_buffer, BindGroup, BindGroupEntries, BindGroupLayout,
        BindGroupLayoutEntries, Buffer, BufferInitDescriptor, BufferUsages, Shader, ShaderStages,
        ShaderType, VertexAttribute, VertexBufferLayout, VertexFormat, VertexStepMode,
    },
    renderer::RenderDevice,
    Extract, ExtractSchedule, Render, RenderApp, RenderSet,
};
use bevy_utils::TypeIdMap;
use bytemuck::cast_slice;
use config::{
    DefaultGizmoConfigGroup, GizmoConfig, GizmoConfigGroup, GizmoConfigStore, GizmoLineJoint,
    GizmoMeshConfig,
};
use gizmos::GizmoStorage;
use light::LightGizmoPlugin;
use std::{any::TypeId, mem};

const LINE_SHADER_HANDLE: Handle<Shader> = Handle::weak_from_u128(7414812689238026784);
const LINE_JOINT_SHADER_HANDLE: Handle<Shader> = Handle::weak_from_u128(1162780797909187908);

/// A [`Plugin`] that provides an immediate mode drawing api for visual debugging.
///
/// Requires to be loaded after [`PbrPlugin`](bevy_pbr::PbrPlugin) or [`SpritePlugin`](bevy_sprite::SpritePlugin).
pub struct GizmoPlugin;

impl Plugin for GizmoPlugin {
    fn build(&self, app: &mut bevy_app::App) {
        // Gizmos cannot work without either a 3D or 2D renderer.
        #[cfg(all(not(feature = "bevy_pbr"), not(feature = "bevy_sprite")))]
        bevy_utils::tracing::error!(
            "bevy_gizmos requires either bevy_pbr or bevy_sprite. Please enable one."
        );

        load_internal_asset!(app, LINE_SHADER_HANDLE, "lines.wgsl", Shader::from_wgsl);
        load_internal_asset!(
            app,
            LINE_JOINT_SHADER_HANDLE,
            "line_joints.wgsl",
            Shader::from_wgsl
        );

        app.register_type::<GizmoConfig>()
            .register_type::<GizmoConfigStore>()
            .add_plugins(UniformComponentPlugin::<LineGizmoUniform>::default())
            .init_asset::<LineGizmo>()
            .add_plugins(RenderAssetPlugin::<LineGizmo>::default())
            .init_resource::<LineGizmoHandles>()
            // We insert the Resource GizmoConfigStore into the world implicitly here if it does not exist.
            .init_gizmo_group::<DefaultGizmoConfigGroup>()
            .add_plugins(AabbGizmoPlugin)
            .add_plugins(LightGizmoPlugin);

        let Ok(render_app) = app.get_sub_app_mut(RenderApp) else {
            return;
        };

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
    }

    fn finish(&self, app: &mut bevy_app::App) {
        let Ok(render_app) = app.get_sub_app_mut(RenderApp) else {
            return;
        };

        let render_device = render_app.world.resource::<RenderDevice>();
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

/// A trait adding `init_gizmo_group<T>()` to the app
pub trait AppGizmoBuilder {
    /// Registers [`GizmoConfigGroup`] `T` in the app enabling the use of [Gizmos&lt;T&gt;](crate::gizmos::Gizmos).
    ///
    /// Configurations can be set using the [`GizmoConfigStore`] [`Resource`].
    fn init_gizmo_group<T: GizmoConfigGroup + Default>(&mut self) -> &mut Self;

    /// Insert the [`GizmoConfigGroup`] in the app with the given value and [`GizmoConfig`].
    ///
    /// This method should be preferred over [`AppGizmoBuilder::init_gizmo_group`] if and only if you need to configure fields upon initialization.
    fn insert_gizmo_group<T: GizmoConfigGroup>(
        &mut self,
        group: T,
        config: GizmoConfig,
    ) -> &mut Self;
}

impl AppGizmoBuilder for App {
    fn init_gizmo_group<T: GizmoConfigGroup + Default>(&mut self) -> &mut Self {
        if self.world.contains_resource::<GizmoStorage<T>>() {
            return self;
        }

        let mut handles = self
            .world
            .get_resource_or_insert_with::<LineGizmoHandles>(Default::default);
        handles.list.insert(TypeId::of::<T>(), None);
        handles.strip.insert(TypeId::of::<T>(), None);

        self.init_resource::<GizmoStorage<T>>()
            .add_systems(Last, update_gizmo_meshes::<T>);

        self.world
            .get_resource_or_insert_with::<GizmoConfigStore>(Default::default)
            .register::<T>();

        self
    }

    fn insert_gizmo_group<T: GizmoConfigGroup>(
        &mut self,
        group: T,
        config: GizmoConfig,
    ) -> &mut Self {
        self.world
            .get_resource_or_insert_with::<GizmoConfigStore>(Default::default)
            .insert(config, group);

        if self.world.contains_resource::<GizmoStorage<T>>() {
            return self;
        }

        let mut handles = self
            .world
            .get_resource_or_insert_with::<LineGizmoHandles>(Default::default);
        handles.list.insert(TypeId::of::<T>(), None);
        handles.strip.insert(TypeId::of::<T>(), None);

        self.init_resource::<GizmoStorage<T>>()
            .add_systems(Last, update_gizmo_meshes::<T>);

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

fn update_gizmo_meshes<T: GizmoConfigGroup>(
    mut line_gizmos: ResMut<Assets<LineGizmo>>,
    mut handles: ResMut<LineGizmoHandles>,
    mut storage: ResMut<GizmoStorage<T>>,
    config_store: Res<GizmoConfigStore>,
) {
    if storage.list_positions.is_empty() {
        handles.list.insert(TypeId::of::<T>(), None);
    } else if let Some(handle) = handles.list.get_mut(&TypeId::of::<T>()) {
        if let Some(handle) = handle {
            let list = line_gizmos.get_mut(handle.id()).unwrap();

            list.positions = mem::take(&mut storage.list_positions);
            list.colors = mem::take(&mut storage.list_colors);
        } else {
            let mut list = LineGizmo {
                strip: false,
                ..Default::default()
            };

            list.positions = mem::take(&mut storage.list_positions);
            list.colors = mem::take(&mut storage.list_colors);

            *handle = Some(line_gizmos.add(list));
        }
    }

    let (config, _) = config_store.config::<T>();
    if storage.strip_positions.is_empty() {
        handles.strip.insert(TypeId::of::<T>(), None);
    } else if let Some(handle) = handles.strip.get_mut(&TypeId::of::<T>()) {
        if let Some(handle) = handle {
            let strip = line_gizmos.get_mut(handle.id()).unwrap();

            strip.positions = mem::take(&mut storage.strip_positions);
            strip.colors = mem::take(&mut storage.strip_colors);
            strip.joints = config.line_joints;
        } else {
            let mut strip = LineGizmo {
                strip: true,
                joints: config.line_joints,
                ..Default::default()
            };

            strip.positions = mem::take(&mut storage.strip_positions);
            strip.colors = mem::take(&mut storage.strip_colors);

            *handle = Some(line_gizmos.add(strip));
        }
    }
}

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
            (*handle).clone_weak(),
            GizmoMeshConfig::from(config),
        ));
    }
}

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

#[derive(Asset, Debug, Default, Clone, TypePath)]
struct LineGizmo {
    positions: Vec<Vec3>,
    colors: Vec<LinearRgba>,
    /// Whether this gizmo's topology is a line-strip or line-list
    strip: bool,
    /// Whether this gizmo should draw line joints. This is only applicable if the gizmo's topology is line-strip.
    joints: GizmoLineJoint,
}

#[derive(Debug, Clone)]
struct GpuLineGizmo {
    position_buffer: Buffer,
    color_buffer: Buffer,
    vertex_count: u32,
    strip: bool,
    joints: GizmoLineJoint,
}

impl RenderAsset for LineGizmo {
    type PreparedAsset = GpuLineGizmo;
    type Param = SRes<RenderDevice>;

    fn asset_usage(&self) -> RenderAssetUsages {
        RenderAssetUsages::MAIN_WORLD | RenderAssetUsages::RENDER_WORLD
    }

    fn prepare_asset(
        self,
        render_device: &mut SystemParamItem<Self::Param>,
    ) -> Result<Self::PreparedAsset, PrepareAssetError<Self>> {
        let position_buffer_data = cast_slice(&self.positions);
        let position_buffer = render_device.create_buffer_with_data(&BufferInitDescriptor {
            usage: BufferUsages::VERTEX,
            label: Some("LineGizmo Position Buffer"),
            contents: position_buffer_data,
        });

        let color_buffer_data = cast_slice(&self.colors);
        let color_buffer = render_device.create_buffer_with_data(&BufferInitDescriptor {
            usage: BufferUsages::VERTEX,
            label: Some("LineGizmo Color Buffer"),
            contents: color_buffer_data,
        });

        Ok(GpuLineGizmo {
            position_buffer,
            color_buffer,
            vertex_count: self.positions.len() as u32,
            strip: self.strip,
            joints: self.joints,
        })
    }
}

#[derive(Resource)]
struct LineGizmoUniformBindgroupLayout {
    layout: BindGroupLayout,
}

#[derive(Resource)]
struct LineGizmoUniformBindgroup {
    bindgroup: BindGroup,
}

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

struct SetLineGizmoBindGroup<const I: usize>;
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
            return RenderCommandResult::Failure;
        };
        pass.set_bind_group(
            I,
            &bind_group.into_inner().bindgroup,
            &[uniform_index.index()],
        );
        RenderCommandResult::Success
    }
}

struct DrawLineGizmo;
impl<P: PhaseItem> RenderCommand<P> for DrawLineGizmo {
    type Param = SRes<RenderAssets<LineGizmo>>;
    type ViewQuery = ();
    type ItemQuery = Read<Handle<LineGizmo>>;

    #[inline]
    fn render<'w>(
        _item: &P,
        _view: ROQueryItem<'w, Self::ViewQuery>,
        handle: Option<ROQueryItem<'w, Self::ItemQuery>>,
        line_gizmos: SystemParamItem<'w, '_, Self::Param>,
        pass: &mut TrackedRenderPass<'w>,
    ) -> RenderCommandResult {
        let Some(handle) = handle else {
            return RenderCommandResult::Failure;
        };
        let Some(line_gizmo) = line_gizmos.into_inner().get(handle) else {
            return RenderCommandResult::Failure;
        };

        if line_gizmo.vertex_count < 2 {
            return RenderCommandResult::Success;
        }

        let instances = if line_gizmo.strip {
            pass.set_vertex_buffer(0, line_gizmo.position_buffer.slice(..));
            pass.set_vertex_buffer(1, line_gizmo.position_buffer.slice(..));

            pass.set_vertex_buffer(2, line_gizmo.color_buffer.slice(..));
            pass.set_vertex_buffer(3, line_gizmo.color_buffer.slice(..));

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

struct DrawLineJointGizmo;
impl<P: PhaseItem> RenderCommand<P> for DrawLineJointGizmo {
    type Param = SRes<RenderAssets<LineGizmo>>;
    type ViewQuery = ();
    type ItemQuery = Read<Handle<LineGizmo>>;

    #[inline]
    fn render<'w>(
        _item: &P,
        _view: ROQueryItem<'w, Self::ViewQuery>,
        handle: Option<ROQueryItem<'w, Self::ItemQuery>>,
        line_gizmos: SystemParamItem<'w, '_, Self::Param>,
        pass: &mut TrackedRenderPass<'w>,
    ) -> RenderCommandResult {
        let Some(handle) = handle else {
            return RenderCommandResult::Failure;
        };
        let Some(line_gizmo) = line_gizmos.into_inner().get(handle) else {
            return RenderCommandResult::Failure;
        };

        if line_gizmo.vertex_count <= 2 || !line_gizmo.strip {
            return RenderCommandResult::Success;
        };

        if line_gizmo.joints == GizmoLineJoint::None {
            return RenderCommandResult::Success;
        };

        let instances = {
            pass.set_vertex_buffer(0, line_gizmo.position_buffer.slice(..));
            pass.set_vertex_buffer(1, line_gizmo.position_buffer.slice(..));
            pass.set_vertex_buffer(2, line_gizmo.position_buffer.slice(..));

            pass.set_vertex_buffer(3, line_gizmo.color_buffer.slice(..));

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
                position_layout.attributes[0].offset = Float32x3.size();
                position_layout
            },
            color_layout.clone(),
            {
                color_layout.attributes[0].shader_location = 3;
                color_layout.attributes[0].offset = Float32x4.size();
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
            offset: Float32x4.size(),
            shader_location: 3,
        }],
    };

    vec![
        position_layout.clone(),
        {
            position_layout.attributes[0].shader_location = 1;
            position_layout.attributes[0].offset = Float32x3.size();
            position_layout.clone()
        },
        {
            position_layout.attributes[0].shader_location = 2;
            position_layout.attributes[0].offset = 2 * Float32x3.size();
            position_layout
        },
        color_layout.clone(),
    ]
}
