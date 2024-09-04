use std::{hash::Hash, ops::Range};

use bevy_asset::*;
use bevy_color::{Alpha, ColorToComponents, LinearRgba};
use bevy_ecs::{
    prelude::Component,
    storage::SparseSet,
    system::{
        lifetimeless::{Read, SRes},
        *,
    },
};
use bevy_math::{FloatOrd, Mat4, Rect, Vec2, Vec4Swizzles};
use bevy_render::{
    render_asset::RenderAssets,
    render_phase::*,
    render_resource::{binding_types::uniform_buffer, *},
    renderer::{RenderDevice, RenderQueue},
    texture::{BevyDefault, GpuImage, Image, TRANSPARENT_IMAGE_HANDLE},
    view::*,
    Extract, ExtractSchedule, Render, RenderSet,
};
use bevy_sprite::{
    ImageScaleMode, SliceScaleMode, SpriteAssetEvents, TextureAtlas, TextureAtlasLayout,
    TextureSlicer,
};
use bevy_transform::prelude::GlobalTransform;
use bevy_utils::HashMap;
use binding_types::{sampler, texture_2d};
use bytemuck::{Pod, Zeroable};

use crate::*;

pub const UI_SLICER_SHADER_HANDLE: Handle<Shader> = Handle::weak_from_u128(11156288772117983964);

pub struct UiTextureSlicerPlugin;

impl Plugin for UiTextureSlicerPlugin {
    fn build(&self, app: &mut App) {
        load_internal_asset!(
            app,
            UI_SLICER_SHADER_HANDLE,
            "ui_texture_slice.wgsl",
            Shader::from_wgsl
        );

        if let Some(render_app) = app.get_sub_app_mut(RenderApp) {
            render_app
                .add_render_command::<TransparentUi, DrawUiTextureSlices>()
                .init_resource::<ExtractedUiTextureSlices>()
                .init_resource::<UiTextureSliceMeta>()
                .init_resource::<UiTextureSliceImageBindGroups>()
                .init_resource::<SpecializedRenderPipelines<UiTextureSlicePipeline>>()
                .add_systems(
                    ExtractSchedule,
                    extract_ui_texture_slices.after(extract_uinode_images),
                )
                .add_systems(
                    Render,
                    (
                        queue_ui_slices.in_set(RenderSet::Queue),
                        prepare_ui_slices.in_set(RenderSet::PrepareBindGroups),
                    ),
                );
        }
    }

    fn finish(&self, app: &mut App) {
        if let Some(render_app) = app.get_sub_app_mut(RenderApp) {
            render_app.init_resource::<UiTextureSlicePipeline>();
        }
    }
}

#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
struct UiTextureSliceVertex {
    pub position: [f32; 3],
    pub uv: [f32; 2],
    pub color: [f32; 4],
    pub slices: [f32; 4],
    pub border: [f32; 4],
    pub repeat: [f32; 4],
    pub atlas: [f32; 4],
}

#[derive(Component)]
pub struct UiTextureSlicerBatch {
    pub range: Range<u32>,
    pub image: AssetId<Image>,
    pub camera: Entity,
}

#[derive(Resource)]
pub struct UiTextureSliceMeta {
    vertices: RawBufferVec<UiTextureSliceVertex>,
    indices: RawBufferVec<u32>,
    view_bind_group: Option<BindGroup>,
}

impl Default for UiTextureSliceMeta {
    fn default() -> Self {
        Self {
            vertices: RawBufferVec::new(BufferUsages::VERTEX),
            indices: RawBufferVec::new(BufferUsages::INDEX),
            view_bind_group: None,
        }
    }
}

#[derive(Resource, Default)]
pub struct UiTextureSliceImageBindGroups {
    pub values: HashMap<AssetId<Image>, BindGroup>,
}

#[derive(Resource)]
pub struct UiTextureSlicePipeline {
    pub view_layout: BindGroupLayout,
    pub image_layout: BindGroupLayout,
}

impl FromWorld for UiTextureSlicePipeline {
    fn from_world(world: &mut World) -> Self {
        let render_device = world.resource::<RenderDevice>();

        let view_layout = render_device.create_bind_group_layout(
            "ui_texture_slice_view_layout",
            &BindGroupLayoutEntries::single(
                ShaderStages::VERTEX_FRAGMENT,
                uniform_buffer::<ViewUniform>(true),
            ),
        );

        let image_layout = render_device.create_bind_group_layout(
            "ui_texture_slice_image_layout",
            &BindGroupLayoutEntries::sequential(
                ShaderStages::FRAGMENT,
                (
                    texture_2d(TextureSampleType::Float { filterable: true }),
                    sampler(SamplerBindingType::Filtering),
                ),
            ),
        );

        UiTextureSlicePipeline {
            view_layout,
            image_layout,
        }
    }
}

#[derive(Clone, Copy, Hash, PartialEq, Eq)]
pub struct UiTextureSlicePipelineKey {
    pub hdr: bool,
}

impl SpecializedRenderPipeline for UiTextureSlicePipeline {
    type Key = UiTextureSlicePipelineKey;

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
                // normalized texture slicing lines (left, top, right, bottom)
                VertexFormat::Float32x4,
                // normalized target slicing lines (left, top, right, bottom)
                VertexFormat::Float32x4,
                // repeat values (horizontal side, vertical side, horizontal center, vertical center)
                VertexFormat::Float32x4,
                // normalized texture atlas rect (left, top, right, bottom)
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
            label: Some("ui_texture_slice_pipeline".into()),
        }
    }
}

pub struct ExtractedUiTextureSlice {
    pub stack_index: u32,
    pub transform: Mat4,
    pub rect: Rect,
    pub atlas_rect: Option<Rect>,
    pub image: AssetId<Image>,
    pub clip: Option<Rect>,
    pub camera_entity: Entity,
    pub color: LinearRgba,
    pub image_scale_mode: ImageScaleMode,
    pub flip_x: bool,
    pub flip_y: bool,
}

#[derive(Resource, Default)]
pub struct ExtractedUiTextureSlices {
    pub slices: SparseSet<Entity, ExtractedUiTextureSlice>,
}

pub fn extract_ui_texture_slices(
    mut commands: Commands,
    mut extracted_ui_slicers: ResMut<ExtractedUiTextureSlices>,
    default_ui_camera: Extract<DefaultUiCamera>,
    texture_atlases: Extract<Res<Assets<TextureAtlasLayout>>>,
    slicers_query: Extract<
        Query<(
            &Node,
            &GlobalTransform,
            &ViewVisibility,
            Option<&CalculatedClip>,
            Option<&TargetCamera>,
            &UiImage,
            &ImageScaleMode,
            Option<&TextureAtlas>,
        )>,
    >,
) {
    for (uinode, transform, view_visibility, clip, camera, image, image_scale_mode, atlas) in
        &slicers_query
    {
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

        let atlas_rect = atlas.and_then(|atlas| {
            texture_atlases
                .get(&atlas.layout)
                .map(|layout| layout.textures[atlas.index].as_rect())
        });

        extracted_ui_slicers.slices.insert(
            commands.spawn_empty().id(),
            ExtractedUiTextureSlice {
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
                image_scale_mode: image_scale_mode.clone(),
                atlas_rect,
                flip_x: image.flip_x,
                flip_y: image.flip_y,
            },
        );
    }
}

pub fn queue_ui_slices(
    extracted_ui_slicers: ResMut<ExtractedUiTextureSlices>,
    ui_slicer_pipeline: Res<UiTextureSlicePipeline>,
    mut pipelines: ResMut<SpecializedRenderPipelines<UiTextureSlicePipeline>>,
    mut transparent_render_phases: ResMut<ViewSortedRenderPhases<TransparentUi>>,
    mut views: Query<(Entity, &ExtractedView)>,
    pipeline_cache: Res<PipelineCache>,
    draw_functions: Res<DrawFunctions<TransparentUi>>,
) {
    let draw_function = draw_functions.read().id::<DrawUiTextureSlices>();
    for (entity, extracted_slicer) in extracted_ui_slicers.slices.iter() {
        let Ok((view_entity, view)) = views.get_mut(extracted_slicer.camera_entity) else {
            continue;
        };

        let Some(transparent_phase) = transparent_render_phases.get_mut(&view_entity) else {
            continue;
        };

        let pipeline = pipelines.specialize(
            &pipeline_cache,
            &ui_slicer_pipeline,
            UiTextureSlicePipelineKey { hdr: view.hdr },
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

#[allow(clippy::too_many_arguments)]
pub fn prepare_ui_slices(
    mut commands: Commands,
    render_device: Res<RenderDevice>,
    render_queue: Res<RenderQueue>,
    mut ui_meta: ResMut<UiTextureSliceMeta>,
    mut extracted_slices: ResMut<ExtractedUiTextureSlices>,
    view_uniforms: Res<ViewUniforms>,
    texture_slicer_pipeline: Res<UiTextureSlicePipeline>,
    mut image_bind_groups: ResMut<UiTextureSliceImageBindGroups>,
    gpu_images: Res<RenderAssets<GpuImage>>,
    mut phases: ResMut<ViewSortedRenderPhases<TransparentUi>>,
    events: Res<SpriteAssetEvents>,
    mut previous_len: Local<usize>,
) {
    // If an image has changed, the GpuImage has (probably) changed
    for event in &events.images {
        match event {
            AssetEvent::Added { .. } |
            AssetEvent::Unused { .. } |
            // Images don't have dependencies
            AssetEvent::LoadedWithDependencies { .. } => {}
            AssetEvent::Modified { id } | AssetEvent::Removed { id } => {
                image_bind_groups.values.remove(id);
            }
        };
    }

    if let Some(view_binding) = view_uniforms.uniforms.binding() {
        let mut batches: Vec<(Entity, UiTextureSlicerBatch)> = Vec::with_capacity(*previous_len);

        ui_meta.vertices.clear();
        ui_meta.indices.clear();
        ui_meta.view_bind_group = Some(render_device.create_bind_group(
            "ui_texture_slice_view_bind_group",
            &texture_slicer_pipeline.view_layout,
            &BindGroupEntries::single(view_binding),
        ));

        // Buffer indexes
        let mut vertices_index = 0;
        let mut indices_index = 0;

        for ui_phase in phases.values_mut() {
            let mut batch_item_index = 0;
            let mut batch_image_handle = AssetId::invalid();
            let mut batch_image_size = Vec2::ZERO;

            for item_index in 0..ui_phase.items.len() {
                let item = &mut ui_phase.items[item_index];
                if let Some(texture_slices) = extracted_slices.slices.get(item.entity) {
                    let mut existing_batch = batches.last_mut();

                    if batch_image_handle == AssetId::invalid()
                        || existing_batch.is_none()
                        || (batch_image_handle != AssetId::default()
                            && texture_slices.image != AssetId::default()
                            && batch_image_handle != texture_slices.image)
                        || existing_batch.as_ref().map(|(_, b)| b.camera)
                            != Some(texture_slices.camera_entity)
                    {
                        if let Some(gpu_image) = gpu_images.get(texture_slices.image) {
                            batch_item_index = item_index;
                            batch_image_handle = texture_slices.image;
                            batch_image_size = gpu_image.size.as_vec2();

                            let new_batch = UiTextureSlicerBatch {
                                range: vertices_index..vertices_index,
                                image: texture_slices.image,
                                camera: texture_slices.camera_entity,
                            };

                            batches.push((item.entity, new_batch));

                            image_bind_groups
                                .values
                                .entry(batch_image_handle)
                                .or_insert_with(|| {
                                    render_device.create_bind_group(
                                        "ui_texture_slice_image_layout",
                                        &texture_slicer_pipeline.image_layout,
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
                        && texture_slices.image != AssetId::default()
                    {
                        if let Some(gpu_image) = gpu_images.get(texture_slices.image) {
                            batch_image_handle = texture_slices.image;
                            batch_image_size = gpu_image.size.as_vec2();
                            existing_batch.as_mut().unwrap().1.image = texture_slices.image;

                            image_bind_groups
                                .values
                                .entry(batch_image_handle)
                                .or_insert_with(|| {
                                    render_device.create_bind_group(
                                        "ui_texture_slice_image_layout",
                                        &texture_slicer_pipeline.image_layout,
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

                    let uinode_rect = texture_slices.rect;

                    let rect_size = uinode_rect.size().extend(1.0);

                    // Specify the corners of the node
                    let positions = QUAD_VERTEX_POSITIONS
                        .map(|pos| (texture_slices.transform * (pos * rect_size).extend(1.)).xyz());

                    // Calculate the effect of clipping
                    // Note: this won't work with rotation/scaling, but that's much more complex (may need more that 2 quads)
                    let positions_diff = if let Some(clip) = texture_slices.clip {
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
                        texture_slices.transform.transform_vector3(rect_size);

                    // Don't try to cull nodes that have a rotation
                    // In a rotation around the Z-axis, this value is 0.0 for an angle of 0.0 or π
                    // In those two cases, the culling check can proceed normally as corners will be on
                    // horizontal / vertical lines
                    // For all other angles, bypass the culling check
                    // This does not properly handles all rotations on all axis
                    if texture_slices.transform.x_axis[1] == 0.0 {
                        // Cull nodes that are completely clipped
                        if positions_diff[0].x - positions_diff[1].x >= transformed_rect_size.x
                            || positions_diff[1].y - positions_diff[2].y >= transformed_rect_size.y
                        {
                            continue;
                        }
                    }
                    let flags = if texture_slices.image != AssetId::default() {
                        shader_flags::TEXTURED
                    } else {
                        shader_flags::UNTEXTURED
                    };

                    let uvs = if flags == shader_flags::UNTEXTURED {
                        [Vec2::ZERO, Vec2::X, Vec2::ONE, Vec2::Y]
                    } else {
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

                    let color = texture_slices.color.to_f32_array();

                    let (image_size, mut atlas) = if let Some(atlas) = texture_slices.atlas_rect {
                        (
                            atlas.size(),
                            [
                                atlas.min.x / batch_image_size.x,
                                atlas.min.y / batch_image_size.y,
                                atlas.max.x / batch_image_size.x,
                                atlas.max.y / batch_image_size.y,
                            ],
                        )
                    } else {
                        (batch_image_size, [0., 0., 1., 1.])
                    };

                    if texture_slices.flip_x {
                        atlas.swap(0, 2);
                    }

                    if texture_slices.flip_y {
                        atlas.swap(1, 3);
                    }

                    let [slices, border, repeat] = compute_texture_slices(
                        image_size,
                        uinode_rect.size(),
                        &texture_slices.image_scale_mode,
                    );

                    for i in 0..4 {
                        ui_meta.vertices.push(UiTextureSliceVertex {
                            position: positions_clipped[i].into(),
                            uv: uvs[i].into(),
                            color,
                            slices,
                            border,
                            repeat,
                            atlas,
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
    extracted_slices.slices.clear();
}

pub type DrawUiTextureSlices = (
    SetItemPipeline,
    SetSlicerViewBindGroup<0>,
    SetSlicerTextureBindGroup<1>,
    DrawSlicer,
);

pub struct SetSlicerViewBindGroup<const I: usize>;
impl<P: PhaseItem, const I: usize> RenderCommand<P> for SetSlicerViewBindGroup<I> {
    type Param = SRes<UiTextureSliceMeta>;
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
            return RenderCommandResult::Failure;
        };
        pass.set_bind_group(I, view_bind_group, &[view_uniform.offset]);
        RenderCommandResult::Success
    }
}
pub struct SetSlicerTextureBindGroup<const I: usize>;
impl<P: PhaseItem, const I: usize> RenderCommand<P> for SetSlicerTextureBindGroup<I> {
    type Param = SRes<UiTextureSliceImageBindGroups>;
    type ViewQuery = ();
    type ItemQuery = Read<UiTextureSlicerBatch>;

    #[inline]
    fn render<'w>(
        _item: &P,
        _view: (),
        batch: Option<&'w UiTextureSlicerBatch>,
        image_bind_groups: SystemParamItem<'w, '_, Self::Param>,
        pass: &mut TrackedRenderPass<'w>,
    ) -> RenderCommandResult {
        let image_bind_groups = image_bind_groups.into_inner();
        let Some(batch) = batch else {
            return RenderCommandResult::Success;
        };

        pass.set_bind_group(I, image_bind_groups.values.get(&batch.image).unwrap(), &[]);
        RenderCommandResult::Success
    }
}
pub struct DrawSlicer;
impl<P: PhaseItem> RenderCommand<P> for DrawSlicer {
    type Param = SRes<UiTextureSliceMeta>;
    type ViewQuery = ();
    type ItemQuery = Read<UiTextureSlicerBatch>;

    #[inline]
    fn render<'w>(
        _item: &P,
        _view: (),
        batch: Option<&'w UiTextureSlicerBatch>,
        ui_meta: SystemParamItem<'w, '_, Self::Param>,
        pass: &mut TrackedRenderPass<'w>,
    ) -> RenderCommandResult {
        let Some(batch) = batch else {
            return RenderCommandResult::Success;
        };
        let ui_meta = ui_meta.into_inner();
        let Some(vertices) = ui_meta.vertices.buffer() else {
            return RenderCommandResult::Failure;
        };
        let Some(indices) = ui_meta.indices.buffer() else {
            return RenderCommandResult::Failure;
        };

        // Store the vertices
        pass.set_vertex_buffer(0, vertices.slice(..));
        // Define how to "connect" the vertices
        pass.set_index_buffer(indices.slice(..), 0, IndexFormat::Uint32);
        // Draw the vertices
        pass.draw_indexed(batch.range.clone(), 0, 0..1);
        RenderCommandResult::Success
    }
}

fn compute_texture_slices(
    image_size: Vec2,
    target_size: Vec2,
    image_scale_mode: &ImageScaleMode,
) -> [[f32; 4]; 3] {
    match image_scale_mode {
        ImageScaleMode::Sliced(TextureSlicer {
            border: border_rect,
            center_scale_mode,
            sides_scale_mode,
            max_corner_scale,
        }) => {
            let min_coeff = (target_size / image_size)
                .min_element()
                .min(*max_corner_scale);

            // calculate the normalized extents of the nine-patched image slices
            let slices = [
                border_rect.left / image_size.x,
                border_rect.top / image_size.y,
                1. - border_rect.right / image_size.x,
                1. - border_rect.bottom / image_size.y,
            ];

            // calculate the normalized extents of the target slices
            let border = [
                (border_rect.left / target_size.x) * min_coeff,
                (border_rect.top / target_size.y) * min_coeff,
                1. - (border_rect.right / target_size.x) * min_coeff,
                1. - (border_rect.bottom / target_size.y) * min_coeff,
            ];

            let image_side_width = image_size.x * (slices[2] - slices[0]);
            let image_side_height = image_size.y * (slices[2] - slices[1]);
            let target_side_height = target_size.x * (border[2] - border[0]);
            let target_side_width = target_size.y * (border[3] - border[1]);

            // compute the number of times to repeat the side and center slices when tiling along each axis
            // if the returned value is `1.` the slice will be stretched to fill the axis.
            let repeat_side_x =
                compute_tiled_subaxis(image_side_width, target_side_height, sides_scale_mode);
            let repeat_side_y =
                compute_tiled_subaxis(image_side_height, target_side_width, sides_scale_mode);
            let repeat_center_x =
                compute_tiled_subaxis(image_side_width, target_side_height, center_scale_mode);
            let repeat_center_y =
                compute_tiled_subaxis(image_side_height, target_side_width, center_scale_mode);

            [
                slices,
                border,
                [
                    repeat_side_x,
                    repeat_side_y,
                    repeat_center_x,
                    repeat_center_y,
                ],
            ]
        }
        ImageScaleMode::Tiled {
            tile_x,
            tile_y,
            stretch_value,
        } => {
            let rx = compute_tiled_axis(*tile_x, image_size.x, target_size.x, *stretch_value);
            let ry = compute_tiled_axis(*tile_y, image_size.y, target_size.y, *stretch_value);
            [[0., 0., 1., 1.], [0., 0., 1., 1.], [1., 1., rx, ry]]
        }
    }
}

fn compute_tiled_axis(tile: bool, image_extent: f32, target_extent: f32, stretch: f32) -> f32 {
    if tile {
        let s = image_extent * stretch;
        target_extent / s
    } else {
        1.
    }
}

fn compute_tiled_subaxis(image_extent: f32, target_extent: f32, mode: &SliceScaleMode) -> f32 {
    match mode {
        SliceScaleMode::Stretch => 1.,
        SliceScaleMode::Tile { stretch_value } => {
            let s = image_extent * *stretch_value;
            target_extent / s
        }
    }
}
