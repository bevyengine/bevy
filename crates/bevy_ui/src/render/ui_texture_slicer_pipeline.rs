use std::{hash::Hash, marker::PhantomData, ops::Range};

use bevy_asset::*;
use bevy_color::{Alpha, Color, ColorToComponents, LinearRgba};
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
    texture::{BevyDefault, FallbackImage, GpuImage, Image, TRANSPARENT_IMAGE_HANDLE},
    view::*,
    Extract, ExtractSchedule, Render, RenderSet,
};
use bevy_sprite::SpriteAssetEvents;
use bevy_transform::prelude::GlobalTransform;
use bevy_window::{PrimaryWindow, Window};
use binding_types::{sampler, texture_2d};
use bytemuck::{Pod, Zeroable};
use texture_slice::UiSlicer;

use crate::*;

pub const UI_SLICER_SHADER_HANDLE: Handle<Shader> = Handle::weak_from_u128(11156288772117983964);

pub struct UiSlicerPlugin;

impl Plugin for UiSlicerPlugin {
    fn build(&self, app: &mut App) {
        load_internal_asset!(
            app,
            UI_SLICER_SHADER_HANDLE,
            "ui_texture_slicer.wgsl",
            Shader::from_wgsl
        );

        if let Some(render_app) = app.get_sub_app_mut(RenderApp) {
            render_app
                .add_render_command::<TransparentUi, DrawUiSlicer>()
                .init_resource::<ExtractedUiSlicers>()
                .init_resource::<UiSlicerMeta>()
                .init_resource::<SpecializedRenderPipelines<UiSlicerPipeline>>()
                .add_systems(
                    ExtractSchedule,
                    extract_ui_slicers.after(extract_uinode_images),
                )
                .add_systems(
                    Render,
                    (
                        queue_ui_slicers.in_set(RenderSet::Queue),
                        prepare_ui_slicers.in_set(RenderSet::PrepareBindGroups),
                    ),
                );
        }
    }

    fn finish(&self, app: &mut App) {
        if let Some(render_app) = app.get_sub_app_mut(RenderApp) {
            render_app.init_resource::<UiSlicerPipeline>();
        }
    }
}

#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
struct UiSliceVertex {
    pub position: [f32; 3],
    pub uv: [f32; 2],
    pub color: [f32; 4],
    pub slices: [f32; 4],
    pub insets: [f32; 4],
    pub repeat: [f32; 4],
}

#[derive(Component)]
pub struct UiSlicerBatch {
    pub range: Range<u32>,
    pub image: AssetId<Image>,
    pub camera: Entity,
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
                // slices (left, top, right, bottom)
                VertexFormat::Float32x4,
                // insets (left, top, right, bottom)
                VertexFormat::Float32x4,
                // repeat values (h_side, v_side, h_center, v_center)
                VertexFormat::Float32x4,
            ],
        );
        let shader_defs = Vec::new();

        RenderPipelineDescriptor {
            vertex: VertexState {
                shader: UI_SLICER_SHADER_HANDLE,
                entry_point: "vertex".into(),
                shader_defs: shader_defs.clone(),
                buffers: vec![vertex_layout],
            },
            fragment: Some(FragmentState {
                shader: UI_SLICER_SHADER_HANDLE,
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
    pub stack_index: u32,
    pub transform: Mat4,
    pub rect: Rect,
    pub image: AssetId<Image>,
    pub clip: Option<Rect>,
    pub camera_entity: Entity,
    pub color: LinearRgba,
}

#[derive(Resource, Default)]
pub struct ExtractedUiSlicers {
    pub slicers: SparseSet<Entity, ExtractedUiSlicer>,
}

pub fn extract_ui_slicers(
    mut commands: Commands,
    mut extracted_ui_slicers: ResMut<ExtractedUiSlicers>,
    default_ui_camera: Extract<DefaultUiCamera>,
    slicers_query: Extract<
        Query<
            (
                &Node,
                &GlobalTransform,
                &ViewVisibility,
                Option<&CalculatedClip>,
                Option<&TargetCamera>,
                &UiImage,
            ),
            With<UiSlicer>,
        >,
    >,
) {
    for (uinode, transform, view_visibility, clip, camera, image) in &slicers_query {
        let Some(camera_entity) = camera.map(TargetCamera::entity).or(default_ui_camera.get())
        else {
            continue;
        };

        // Skip invisible images
        if !view_visibility.get()
            || image.color.is_fully_transparent()
            || image.texture.id() == TRANSPARENT_IMAGE_HANDLE.id()
        {
            continue;
        }

        extracted_ui_slicers.slicers.insert(
            commands.spawn_empty().id(),
            ExtractedUiSlicer {
                stack_index: uinode.stack_index,
                transform: transform.compute_matrix(),
                color: image.color.into(),
                rect: Rect {
                    min: Vec2::ZERO,
                    max: uinode.calculated_size,
                },
                clip: clip.map(|clip| clip.clip),
                image: image.texture.id(),
                camera_entity,
            },
        );
    }
}

pub fn queue_ui_slicers(
    extracted_ui_slicers: ResMut<ExtractedUiSlicers>,
    ui_slicer_pipeline: Res<UiSlicerPipeline>,
    mut pipelines: ResMut<SpecializedRenderPipelines<UiSlicerPipeline>>,
    mut transparent_render_phases: ResMut<ViewSortedRenderPhases<TransparentUi>>,
    mut views: Query<(Entity, &ExtractedView)>,
    pipeline_cache: Res<PipelineCache>,
    draw_functions: Res<DrawFunctions<TransparentUi>>,
) {
    let draw_function = draw_functions.read().id::<DrawUiSlicer>();
    for (entity, extracted_slicer) in extracted_ui_slicers.slicers.iter() {
        let Ok((view_entity, view)) = views.get_mut(extracted_slicer.camera_entity) else {
            continue;
        };

        let Some(transparent_phase) = transparent_render_phases.get_mut(&view_entity) else {
            continue;
        };

        let pipeline = pipelines.specialize(
            &pipeline_cache,
            &ui_slicer_pipeline,
            UiSlicerPipelineKey { hdr: view.hdr },
        );

        transparent_phase.add(TransparentUi {
            draw_function,
            pipeline,
            entity: *entity,
            sort_key: (
                FloatOrd(extracted_slicer.stack_index as f32),
                entity.index(),
            ),
            batch_range: 0..0,
            extra_index: PhaseItemExtraIndex::NONE,
        });
    }
}

pub fn prepare_ui_slicers(
    mut commands: Commands,
    render_device: Res<RenderDevice>,
    render_queue: Res<RenderQueue>,
    mut ui_meta: ResMut<UiSlicerMeta>,
    mut extracted_slicers: ResMut<ExtractedUiSlicers>,
    view_uniforms: Res<ViewUniforms>,
    ui_pipeline: Res<UiSlicerPipeline>,
    mut image_bind_groups: ResMut<UiImageBindGroups>,
    gpu_images: Res<RenderAssets<GpuImage>>,
    mut phases: ResMut<ViewSortedRenderPhases<TransparentUi>>,
    events: Res<SpriteAssetEvents>,
    mut previous_len: Local<usize>,
) {
    if let Some(view_binding) = view_uniforms.uniforms.binding() {
        let mut batches: Vec<(Entity, UiSlicerBatch)> = Vec::with_capacity(*previous_len);

        ui_meta.vertices.clear();
        ui_meta.indices.clear();
        ui_meta.view_bind_group = Some(render_device.create_bind_group(
            "ui_view_bind_group",
            &ui_pipeline.view_layout,
            &BindGroupEntries::single(view_binding),
        ));

        // Buffer indexes
        let mut vertices_index = 0;
        let mut indices_index = 0;

        for ui_phase in phases.values_mut() {
            let mut batch_item_index = 0;
            let mut batch_image_handle = AssetId::invalid();

            for item_index in 0..ui_phase.items.len() {
                let item = &mut ui_phase.items[item_index];
                if let Some(extracted_slicer) = extracted_slicers.slicers.get(item.entity) {
                    let mut existing_batch = batches.last_mut();

                    if batch_image_handle == AssetId::invalid()
                        || existing_batch.is_none()
                        || (batch_image_handle != AssetId::default()
                            && extracted_slicer.image != AssetId::default()
                            && batch_image_handle != extracted_slicer.image)
                        || existing_batch.as_ref().map(|(_, b)| b.camera)
                            != Some(extracted_slicer.camera_entity)
                    {
                        if let Some(gpu_image) = gpu_images.get(extracted_slicer.image) {
                            batch_item_index = item_index;
                            batch_image_handle = extracted_slicer.image;

                            let new_batch = UiSlicerBatch {
                                range: vertices_index..vertices_index,
                                image: extracted_slicer.image,
                                camera: extracted_slicer.camera_entity,
                            };

                            batches.push((item.entity, new_batch));

                            image_bind_groups
                                .values
                                .entry(batch_image_handle)
                                .or_insert_with(|| {
                                    render_device.create_bind_group(
                                        "ui_material_bind_group",
                                        &ui_pipeline.image_layout,
                                        &BindGroupEntries::sequential((
                                            &gpu_image.texture_view,
                                            &gpu_image.sampler,
                                        )),
                                    )
                                });

                            existing_batch = batches.last_mut();
                        } else {
                            continue;
                        }
                    } else if batch_image_handle == AssetId::default()
                        && extracted_slicer.image != AssetId::default()
                    {
                        if let Some(gpu_image) = gpu_images.get(extracted_slicer.image) {
                            batch_image_handle = extracted_slicer.image;
                            existing_batch.as_mut().unwrap().1.image = extracted_slicer.image;

                            image_bind_groups
                                .values
                                .entry(batch_image_handle)
                                .or_insert_with(|| {
                                    render_device.create_bind_group(
                                        "ui_material_bind_group",
                                        &ui_pipeline.image_layout,
                                        &BindGroupEntries::sequential((
                                            &gpu_image.texture_view,
                                            &gpu_image.sampler,
                                        )),
                                    )
                                });
                        } else {
                            continue;
                        }
                    }

                    let mut uinode_rect = extracted_slicer.rect;

                    let rect_size = uinode_rect.size().extend(1.0);

                    // Specify the corners of the node
                    let positions = QUAD_VERTEX_POSITIONS.map(|pos| {
                        (extracted_slicer.transform * (pos * rect_size).extend(1.)).xyz()
                    });

                    // Calculate the effect of clipping
                    // Note: this won't work with rotation/scaling, but that's much more complex (may need more that 2 quads)
                    let mut positions_diff = if let Some(clip) = extracted_slicer.clip {
                        [
                            Vec2::new(
                                f32::max(clip.min.x - positions[0].x, 0.),
                                f32::max(clip.min.y - positions[0].y, 0.),
                            ),
                            Vec2::new(
                                f32::min(clip.max.x - positions[1].x, 0.),
                                f32::max(clip.min.y - positions[1].y, 0.),
                            ),
                            Vec2::new(
                                f32::min(clip.max.x - positions[2].x, 0.),
                                f32::min(clip.max.y - positions[2].y, 0.),
                            ),
                            Vec2::new(
                                f32::max(clip.min.x - positions[3].x, 0.),
                                f32::min(clip.max.y - positions[3].y, 0.),
                            ),
                        ]
                    } else {
                        [Vec2::ZERO; 4]
                    };

                    let positions_clipped = [
                        positions[0] + positions_diff[0].extend(0.),
                        positions[1] + positions_diff[1].extend(0.),
                        positions[2] + positions_diff[2].extend(0.),
                        positions[3] + positions_diff[3].extend(0.),
                    ];

                    let transformed_rect_size =
                        extracted_slicer.transform.transform_vector3(rect_size);

                    // Don't try to cull nodes that have a rotation
                    // In a rotation around the Z-axis, this value is 0.0 for an angle of 0.0 or π
                    // In those two cases, the culling check can proceed normally as corners will be on
                    // horizontal / vertical lines
                    // For all other angles, bypass the culling check
                    // This does not properly handles all rotations on all axis
                    if extracted_slicer.transform.x_axis[1] == 0.0 {
                        // Cull nodes that are completely clipped
                        if positions_diff[0].x - positions_diff[1].x >= transformed_rect_size.x
                            || positions_diff[1].y - positions_diff[2].y >= transformed_rect_size.y
                        {
                            continue;
                        }
                    }
                    let flags = if extracted_slicer.image != AssetId::default() {
                        shader_flags::TEXTURED
                    } else {
                        shader_flags::UNTEXTURED
                    };

                    let uvs = if flags == shader_flags::UNTEXTURED {
                        [Vec2::ZERO, Vec2::X, Vec2::ONE, Vec2::Y]
                    } else {
                        // let image = gpu_images
                        //     .get(extracted_slicer.image)
                        //     .expect("Image was checked during batching and should still exist");
                        // // Rescale atlases. This is done here because we need texture data that might not be available in Extract.
                        let atlas_extent = uinode_rect.max;
                        [
                            Vec2::new(
                                uinode_rect.min.x + positions_diff[0].x,
                                uinode_rect.min.y + positions_diff[0].y,
                            ),
                            Vec2::new(
                                uinode_rect.max.x + positions_diff[1].x,
                                uinode_rect.min.y + positions_diff[1].y,
                            ),
                            Vec2::new(
                                uinode_rect.max.x + positions_diff[2].x,
                                uinode_rect.max.y + positions_diff[2].y,
                            ),
                            Vec2::new(
                                uinode_rect.min.x + positions_diff[3].x,
                                uinode_rect.max.y + positions_diff[3].y,
                            ),
                        ]
                        .map(|pos| pos / atlas_extent)
                    };

                    let color = extracted_slicer.color.to_f32_array();

                    for i in 0..4 {
                        ui_meta.vertices.push(UiSliceVertex {
                            position: positions_clipped[i].into(),
                            uv: uvs[i].into(),
                            color,
                            slices: [1. / 3., 1. / 3., 2. / 3., 2. / 3.],
                            insets: [1. / 6., 1. / 6., 1. / 2., 1. / 2.],
                            repeat: [1.; 4],
                        });
                    }

                    for &i in &QUAD_INDICES {
                        ui_meta.indices.push(indices_index + i as u32);
                    }

                    vertices_index += 6;
                    indices_index += 4;

                    existing_batch.unwrap().1.range.end = vertices_index;
                    ui_phase.items[batch_item_index].batch_range_mut().end += 1;
                } else {
                    batch_image_handle = AssetId::invalid();
                }
            }
        }
        ui_meta.vertices.write_buffer(&render_device, &render_queue);
        ui_meta.indices.write_buffer(&render_device, &render_queue);
        *previous_len = batches.len();
        commands.insert_or_spawn_batch(batches);
    }
    extracted_slicers.slicers.clear();
}

pub type DrawUiSlicer = (
    SetItemPipeline,
    SetSlicerViewBindGroup<0>,
    SetSlicerTextureBindGroup<1>,
    DrawSlicer,
);

pub struct SetSlicerViewBindGroup<const I: usize>;
impl<P: PhaseItem, const I: usize> RenderCommand<P> for SetSlicerViewBindGroup<I> {
    type Param = SRes<UiSlicerMeta>;
    type ViewQuery = Read<ViewUniformOffset>;
    type ItemQuery = ();

    fn render<'w>(
        _item: &P,
        view_uniform: &'w ViewUniformOffset,
        _entity: Option<()>,
        ui_meta: SystemParamItem<'w, '_, Self::Param>,
        pass: &mut TrackedRenderPass<'w>,
    ) -> RenderCommandResult {
        let Some(view_bind_group) = ui_meta.into_inner().view_bind_group.as_ref() else {
            return RenderCommandResult::Failure("view_bind_group not available");
        };
        pass.set_bind_group(I, view_bind_group, &[view_uniform.offset]);
        RenderCommandResult::Success
    }
}
pub struct SetSlicerTextureBindGroup<const I: usize>;
impl<P: PhaseItem, const I: usize> RenderCommand<P> for SetSlicerTextureBindGroup<I> {
    type Param = SRes<UiImageBindGroups>;
    type ViewQuery = ();
    type ItemQuery = Read<UiSlicerBatch>;

    #[inline]
    fn render<'w>(
        _item: &P,
        _view: (),
        batch: Option<&'w UiSlicerBatch>,
        image_bind_groups: SystemParamItem<'w, '_, Self::Param>,
        pass: &mut TrackedRenderPass<'w>,
    ) -> RenderCommandResult {
        let image_bind_groups = image_bind_groups.into_inner();
        let Some(batch) = batch else {
            return RenderCommandResult::Skip;
        };

        pass.set_bind_group(I, image_bind_groups.values.get(&batch.image).unwrap(), &[]);
        RenderCommandResult::Success
    }
}
pub struct DrawSlicer;
impl<P: PhaseItem> RenderCommand<P> for DrawSlicer {
    type Param = SRes<UiSlicerMeta>;
    type ViewQuery = ();
    type ItemQuery = Read<UiSlicerBatch>;

    #[inline]
    fn render<'w>(
        _item: &P,
        _view: (),
        batch: Option<&'w UiSlicerBatch>,
        ui_meta: SystemParamItem<'w, '_, Self::Param>,
        pass: &mut TrackedRenderPass<'w>,
    ) -> RenderCommandResult {
        let Some(batch) = batch else {
            return RenderCommandResult::Skip;
        };
        let ui_meta = ui_meta.into_inner();
        let Some(vertices) = ui_meta.vertices.buffer() else {
            return RenderCommandResult::Failure("missing vertices to draw ui");
        };
        let Some(indices) = ui_meta.indices.buffer() else {
            return RenderCommandResult::Failure("missing indices to draw ui");
        };

        // Store the vertices
        pass.set_vertex_buffer(0, vertices.slice(..));
        // Define how to "connect" the vertices
        pass.set_index_buffer(
            indices.slice(..),
            0,
            bevy_render::render_resource::IndexFormat::Uint32,
        );
        // Draw the vertices
        pass.draw_indexed(batch.range.clone(), 0, 0..1);
        RenderCommandResult::Success
    }
}
