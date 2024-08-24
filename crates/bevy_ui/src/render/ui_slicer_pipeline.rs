use std::{hash::Hash, marker::PhantomData, ops::Range};

use bevy_asset::*;
use bevy_ecs::{
    prelude::Component,
    query::ROQueryItem,
    storage::SparseSet,
    system::lifetimeless::{Read, SRes},
    system::*,
};
use bevy_math::{FloatOrd, Mat4, Rect, Vec2, Vec4Swizzles};
use bevy_render::{
    camera::Camera,
    extract_component::ExtractComponentPlugin,
    globals::{GlobalsBuffer, GlobalsUniform},
    render_asset::{PrepareAssetError, RenderAsset, RenderAssetPlugin, RenderAssets},
    render_phase::*,
    render_resource::{binding_types::uniform_buffer, *},
    renderer::{RenderDevice, RenderQueue},
    texture::{BevyDefault, FallbackImage, GpuImage, Image},
    view::*,
    Extract, ExtractSchedule, Render, RenderSet,
};
use bevy_transform::prelude::GlobalTransform;
use bevy_window::{PrimaryWindow, Window};
use binding_types::{sampler, texture_2d};
use bytemuck::{Pod, Zeroable};

use crate::*;

pub const UI_SLICER_SHADER_HANDLE: Handle<Shader> = Handle::weak_from_u128(11156288772117983964);

pub struct UiSlicerPlugin;

impl Plugin for UiSlicerPlugin {
    fn build(&self, app: &mut App) {
        load_internal_asset!(
            app,
            UI_SLICER_SHADER_HANDLE,
            "ui_slicer.wgsl",
            Shader::from_wgsl
        );

        if let Some(render_app) = app.get_sub_app_mut(RenderApp) {
            render_app
                .add_render_command::<TransparentUi, DrawUiSlicer>()
                .init_resource::<ExtractedUiSlicers>()
                .init_resource::<UiSlicerMeta>()
                .init_resource::<SpecializedRenderPipelines<UiSlicerPipeline>>()
                .add_systems(ExtractSchedule, extract_ui_slicers)
                .add_systems(Render, (queue_ui_slicers, prepare_ui_slicers));
        }
    }
}

#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
struct UiSliceVertex {
    pub position: [f32; 3],
    pub uv: [f32; 2],
    pub color: [f32; 4],
}

#[derive(Resource)]
pub struct UiSlicerMeta {
    vertices: RawBufferVec<UiSliceVertex>,
    indices: RawBufferVec<u32>,
    view_bind_group: Option<BindGroup>,
}

impl Default for UiSlicerMeta {
    fn default() -> Self {
        Self {
            vertices: RawBufferVec::new(BufferUsages::VERTEX),
            indices: RawBufferVec::new(BufferUsages::INDEX),
            view_bind_group: None,
        }
    }
}

#[derive(Resource)]
pub struct UiSlicerPipeline {
    pub view_layout: BindGroupLayout,
    pub image_layout: BindGroupLayout,
}

impl FromWorld for UiSlicerPipeline {
    fn from_world(world: &mut World) -> Self {
        let render_device = world.resource::<RenderDevice>();

        let view_layout = render_device.create_bind_group_layout(
            "ui_slicer_view_layout",
            &BindGroupLayoutEntries::single(
                ShaderStages::VERTEX_FRAGMENT,
                uniform_buffer::<ViewUniform>(true),
            ),
        );

        let image_layout = render_device.create_bind_group_layout(
            "ui_slicer_image_layout",
            &BindGroupLayoutEntries::sequential(
                ShaderStages::FRAGMENT,
                (
                    texture_2d(TextureSampleType::Float { filterable: true }),
                    sampler(SamplerBindingType::Filtering),
                ),
            ),
        );

        UiSlicerPipeline {
            view_layout,
            image_layout,
        }
    }
}

#[derive(Clone, Copy, Hash, PartialEq, Eq)]
pub struct UiSlicerPipelineKey {
    pub hdr: bool,
}

impl SpecializedRenderPipeline for UiSlicerPipeline {
    type Key = UiSlicerPipelineKey;

    fn specialize(&self, key: Self::Key) -> RenderPipelineDescriptor {
        let vertex_layout = VertexBufferLayout::from_vertex_formats(
            VertexStepMode::Vertex,
            vec![
                // position
                VertexFormat::Float32x3,
                // uv
                VertexFormat::Float32x2,
                // color
                VertexFormat::Float32x4,
            ],
        );
        let shader_defs = Vec::new();

        RenderPipelineDescriptor {
            vertex: VertexState {
                shader: super::UI_SHADER_HANDLE,
                entry_point: "vertex".into(),
                shader_defs: shader_defs.clone(),
                buffers: vec![vertex_layout],
            },
            fragment: Some(FragmentState {
                shader: super::UI_SHADER_HANDLE,
                shader_defs,
                entry_point: "fragment".into(),
                targets: vec![Some(ColorTargetState {
                    format: if key.hdr {
                        ViewTarget::TEXTURE_FORMAT_HDR
                    } else {
                        TextureFormat::bevy_default()
                    },
                    blend: Some(BlendState::ALPHA_BLENDING),
                    write_mask: ColorWrites::ALL,
                })],
            }),
            layout: vec![self.view_layout.clone(), self.image_layout.clone()],
            push_constant_ranges: Vec::new(),
            primitive: PrimitiveState {
                front_face: FrontFace::Ccw,
                cull_mode: None,
                unclipped_depth: false,
                polygon_mode: PolygonMode::Fill,
                conservative: false,
                topology: PrimitiveTopology::TriangleList,
                strip_index_format: None,
            },
            depth_stencil: None,
            multisample: MultisampleState {
                count: 1,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            label: Some("ui_slicer_pipeline".into()),
        }
    }
}

pub struct ExtractedUiSlicer {
    pub stack_index: usize,
    pub transform: Mat4,
    pub rect: Rect,
    pub border: [f32; 4],
    pub image: AssetId<Image>,
    pub clip: Option<Rect>,
    pub camera: Entity,
}

#[derive(Resource, Default)]
pub struct ExtractedUiSlicers {
    pub slicers: SparseSet<Entity, ExtractedUiSlicer>,
}

pub struct UiSlicerBatch {
    pub range: Range<u32>,
    pub image: AssetId<Image>,
    pub camera: Entity,
}

pub fn extract_ui_slicers(
    mut extracted_ui_slicers: ResMut<ExtractedUiSlicers>,
    ui_scale: Extract<Res<UiScale>>,
    camera_query: Extract<Query<(Entity, &Camera)>>,
    default_ui_camera: Extract<DefaultUiCamera>,
    slicers_query: Extract<
        Query<(
            Entity,
            &GlobalTransform,
            &ViewVisibility,
            Option<&CalculatedClip>,
            Option<&TargetCamera>,
            &UiImage,
        )>,
    >,
) {
}

pub fn queue_ui_slicers() {}

pub fn prepare_ui_slicers() {}

pub type DrawUiSlicer = ();
