#![allow(clippy::type_complexity)]
#![warn(missing_docs)]

//! This crate adds an immediate mode drawing api to Bevy for visual debugging.
//!
//! # Example
//! ```
//! # use bevy_gizmos::prelude::*;
//! # use bevy_render::prelude::*;
//! # use bevy_math::prelude::*;
//! fn system(mut gizmos: Gizmos) {
//!     gizmos.line(Vec3::ZERO, Vec3::X, Color::GREEN);
//! }
//! # bevy_ecs::system::assert_is_system(system);
//! ```
//!
//! See the documentation on [`Gizmos`](crate::gizmos::Gizmos) for more examples.

use std::mem;

use bevy_app::{Last, Plugin};
use bevy_asset::{load_internal_asset, AddAsset, Assets, Handle, HandleUntyped};
use bevy_core::cast_slice;
use bevy_ecs::{
    prelude::{Component, DetectChanges},
    query::ROQueryItem,
    system::{
        lifetimeless::{Read, SRes},
        Commands, Res, ResMut, Resource, SystemParamItem,
    },
    world::{FromWorld, World},
};
use bevy_reflect::TypeUuid;
use bevy_render::{
    extract_component::UniformComponentPlugin,
    render_asset::{PrepareAssetError, RenderAsset, RenderAssetPlugin, RenderAssets},
    render_phase::{PhaseItem, RenderCommand, RenderCommandResult, TrackedRenderPass},
    render_resource::{
        Buffer, BufferInitDescriptor, BufferUsages, Shader, ShaderType, VertexAttribute,
        VertexBufferLayout, VertexFormat, VertexStepMode,
    },
    renderer::RenderDevice,
    Extract, ExtractSchedule, RenderApp,
};
use bevy_utils::default;

pub mod gizmos;

#[cfg(feature = "bevy_sprite")]
mod pipeline_2d;
#[cfg(feature = "bevy_pbr")]
mod pipeline_3d;

use gizmos::GizmoStorage;

/// The `bevy_gizmos` prelude.
pub mod prelude {
    #[doc(hidden)]
    pub use crate::{gizmos::Gizmos, GizmoConfig};
}

const LINE_SHADER_HANDLE: HandleUntyped =
    HandleUntyped::weak_from_u64(Shader::TYPE_UUID, 7414812689238026784);

/// A [`Plugin`] that provides an immediate mode drawing api for visual debugging.
pub struct GizmoPlugin;

impl Plugin for GizmoPlugin {
    fn build(&self, app: &mut bevy_app::App) {
        load_internal_asset!(app, LINE_SHADER_HANDLE, "lines.wgsl", Shader::from_wgsl);

        #[cfg(feature = "bevy_sprite")]
        app.add_plugin(pipeline_2d::LineGizmo2dPlugin);
        #[cfg(feature = "bevy_pbr")]
        app.add_plugin(pipeline_3d::LineGizmo3dPlugin);

        app.add_plugin(UniformComponentPlugin::<LineGizmoUniform>::default())
            .add_asset::<LineGizmo>()
            .add_plugin(RenderAssetPlugin::<LineGizmo>::default())
            .init_resource::<LineGizmoHandles>()
            .init_resource::<GizmoConfig>()
            .init_resource::<GizmoStorage>()
            .add_systems(Last, update_gizmo_meshes);

        let Ok(render_app) = app.get_sub_app_mut(RenderApp) else { return; };

        render_app.add_systems(ExtractSchedule, extract_gizmo_data);
    }
}

/// A [`Resource`] that stores configuration for gizmos.
#[derive(Resource, Clone, Copy)]
pub struct GizmoConfig {
    /// Set to `false` to stop drawing gizmos.
    ///
    /// Defaults to `true`.
    pub enabled: bool,
    /// Line width specified in pixels.
    ///
    /// If `line_perspective` is `true` then this is the size in pixels at the camera's near plane.
    ///
    /// Defaults to `2.0`.
    pub line_width: f32,
    /// Apply perspective to gizmo lines.
    ///
    /// This setting only affects 3D, non-orhographic cameras.
    ///
    /// Defaults to `false`.
    pub line_perspective: bool,
    /// How closer to the camera than real geometry the line should be.
    ///
    /// Value between -1 and 1 (inclusive).
    /// * 0 means that there is no change to the line position when rendering
    /// * 1 means it is furthest away from camera as possible
    /// * -1 means that it will always render in front of other things.
    ///
    /// This is typically useful if you are drawing wireframes on top of polygons
    /// and your wireframe is z-fighting (flickering on/off) with your main model.
    /// You would set this value to a negative number close to 0.0.
    pub depth_bias: f32,
}

impl Default for GizmoConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            line_width: 2.,
            line_perspective: false,
            depth_bias: 0.,
        }
    }
}

#[derive(Resource)]
struct LineGizmoHandles {
    list: Option<Handle<LineGizmo>>,
    strip: Option<Handle<LineGizmo>>,
}

impl FromWorld for LineGizmoHandles {
    fn from_world(_world: &mut World) -> Self {
        LineGizmoHandles {
            list: None,
            strip: None,
        }
    }
}

fn update_gizmo_meshes(
    mut line_gizmos: ResMut<Assets<LineGizmo>>,
    mut handles: ResMut<LineGizmoHandles>,
    mut storage: ResMut<GizmoStorage>,
) {
    if storage.list_positions.is_empty() {
        handles.list = None;
    } else if let Some(handle) = handles.list.as_ref() {
        let list = line_gizmos.get_mut(handle).unwrap();

        list.positions = mem::take(&mut storage.list_positions);
        list.colors = mem::take(&mut storage.list_colors);
    } else {
        let mut list = LineGizmo {
            strip: false,
            ..default()
        };

        list.positions = mem::take(&mut storage.list_positions);
        list.colors = mem::take(&mut storage.list_colors);

        handles.list = Some(line_gizmos.add(list));
    }

    if storage.strip_positions.is_empty() {
        handles.strip = None;
    } else if let Some(handle) = handles.strip.as_ref() {
        let strip = line_gizmos.get_mut(handle).unwrap();

        strip.positions = mem::take(&mut storage.strip_positions);
        strip.colors = mem::take(&mut storage.strip_colors);
    } else {
        let mut strip = LineGizmo {
            strip: true,
            ..default()
        };

        strip.positions = mem::take(&mut storage.strip_positions);
        strip.colors = mem::take(&mut storage.strip_colors);

        handles.strip = Some(line_gizmos.add(strip));
    }
}

fn extract_gizmo_data(
    mut commands: Commands,
    handles: Extract<Res<LineGizmoHandles>>,
    config: Extract<Res<GizmoConfig>>,
) {
    if config.is_changed() {
        commands.insert_resource(**config);
    }

    if !config.enabled {
        return;
    }

    for handle in [&handles.list, &handles.strip].into_iter().flatten() {
        commands.spawn((
            LineGizmoUniform {
                line_width: config.line_width,
                depth_bias: config.depth_bias,
            },
            handle.clone_weak(),
        ));
    }
}

#[derive(Component, ShaderType, Clone, Copy)]
struct LineGizmoUniform {
    line_width: f32,
    depth_bias: f32,
}

#[derive(Debug, Default, Component, Clone, TypeUuid)]
#[uuid = "02b99cbf-bb26-4713-829a-aee8e08dedc0"]
struct LineGizmo {
    positions: Vec<[f32; 3]>,
    colors: Vec<[f32; 4]>,
    /// Whether this gizmo's topology is a line-strip or line-list
    strip: bool,
}

#[derive(Debug, Clone)]
struct GpuLineGizmo {
    position_buffer: Buffer,
    color_buffer: Buffer,
    vertex_count: u32,
    strip: bool,
}

impl RenderAsset for LineGizmo {
    type ExtractedAsset = LineGizmo;

    type PreparedAsset = GpuLineGizmo;

    type Param = SRes<RenderDevice>;

    fn extract_asset(&self) -> Self::ExtractedAsset {
        self.clone()
    }

    fn prepare_asset(
        line_gizmo: Self::ExtractedAsset,
        render_device: &mut SystemParamItem<Self::Param>,
    ) -> Result<Self::PreparedAsset, PrepareAssetError<Self::ExtractedAsset>> {
        let position_buffer_data = cast_slice(&line_gizmo.positions);
        let position_buffer = render_device.create_buffer_with_data(&BufferInitDescriptor {
            usage: BufferUsages::VERTEX,
            label: Some("LineGizmo Position Buffer"),
            contents: position_buffer_data,
        });

        let color_buffer_data = cast_slice(&line_gizmo.colors);
        let color_buffer = render_device.create_buffer_with_data(&BufferInitDescriptor {
            usage: BufferUsages::VERTEX,
            label: Some("LineGizmo Color Buffer"),
            contents: color_buffer_data,
        });

        Ok(GpuLineGizmo {
            position_buffer,
            color_buffer,
            vertex_count: line_gizmo.positions.len() as u32,
            strip: line_gizmo.strip,
        })
    }
}

struct DrawLineGizmo;
impl<P: PhaseItem> RenderCommand<P> for DrawLineGizmo {
    type ViewWorldQuery = ();
    type ItemWorldQuery = Read<Handle<LineGizmo>>;
    type Param = SRes<RenderAssets<LineGizmo>>;

    #[inline]
    fn render<'w>(
        _item: &P,
        _view: ROQueryItem<'w, Self::ViewWorldQuery>,
        handle: ROQueryItem<'w, Self::ItemWorldQuery>,
        polylines: SystemParamItem<'w, '_, Self::Param>,
        pass: &mut TrackedRenderPass<'w>,
    ) -> RenderCommandResult {
        let Some(line_gizmo) = polylines.into_inner().get(handle) else {
            return RenderCommandResult::Failure;
        };

        pass.set_vertex_buffer(0, line_gizmo.position_buffer.slice(..));
        pass.set_vertex_buffer(1, line_gizmo.color_buffer.slice(..));

        let instances = if line_gizmo.strip {
            u32::max(line_gizmo.vertex_count, 1) - 1
        } else {
            line_gizmo.vertex_count / 2
        };

        pass.draw(0..6, 0..instances);

        RenderCommandResult::Success
    }
}

fn line_gizmo_vertex_buffer_layouts(strip: bool) -> Vec<VertexBufferLayout> {
    let stride_multiplier = if strip { 1 } else { 2 };
    vec![
        // Positions
        VertexBufferLayout {
            array_stride: 12 * stride_multiplier,
            step_mode: VertexStepMode::Instance,
            attributes: vec![
                VertexAttribute {
                    format: VertexFormat::Float32x3,
                    offset: 0,
                    shader_location: 0,
                },
                VertexAttribute {
                    format: VertexFormat::Float32x3,
                    offset: 12,
                    shader_location: 1,
                },
            ],
        },
        // Colors
        VertexBufferLayout {
            array_stride: 16 * stride_multiplier,
            step_mode: VertexStepMode::Instance,
            attributes: vec![
                VertexAttribute {
                    format: VertexFormat::Float32x4,
                    offset: 0,
                    shader_location: 2,
                },
                VertexAttribute {
                    format: VertexFormat::Float32x4,
                    offset: 16,
                    shader_location: 3,
                },
            ],
        },
    ]
}
