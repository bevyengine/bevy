use std::ops::Range;

use crate::{
    texture_atlas::{TextureAtlas, TextureAtlasSprite},
    Mesh2dViewKey, Sprite, SPRITE_SHADER_HANDLE,
};
use bevy_asset::{AssetEvent, AssetId, Assets, Handle};
use bevy_core_pipeline::core_2d::Transparent2d;
use bevy_ecs::{
    prelude::*,
    system::{lifetimeless::*, SystemParamItem, SystemState},
};
use bevy_math::{Affine3A, Quat, Rect, Vec2, Vec4};
use bevy_render::{
    color::Color,
    pipeline_keys::{PipelineKey, PipelineKeys},
    render_asset::RenderAssets,
    render_phase::{
        DrawFunctions, PhaseItem, RenderCommand, RenderCommandResult, RenderPhase, SetItemPipeline,
        TrackedRenderPass,
    },
    render_resource::{
        binding_types::{sampler, texture_2d, uniform_buffer},
        BindGroupEntries, *,
    },
    renderer::{RenderDevice, RenderQueue},
    texture::{DefaultImageSampler, GpuImage, Image, ImageSampler, TextureFormatPixelInfo},
    view::{ViewUniform, ViewUniformOffset, ViewUniforms, ViewVisibility, VisibleEntities},
    Extract,
};
use bevy_transform::components::GlobalTransform;
use bevy_utils::{EntityHashMap, FloatOrd, HashMap};
use bytemuck::{Pod, Zeroable};
use fixedbitset::FixedBitSet;

#[derive(Resource)]
pub struct SpritePipeline {
    view_layout: BindGroupLayout,
    material_layout: BindGroupLayout,
    pub dummy_white_gpu_image: GpuImage,
}

impl FromWorld for SpritePipeline {
    fn from_world(world: &mut World) -> Self {
        let mut system_state: SystemState<(
            Res<RenderDevice>,
            Res<DefaultImageSampler>,
            Res<RenderQueue>,
        )> = SystemState::new(world);
        let (render_device, default_sampler, render_queue) = system_state.get_mut(world);

        let view_layout = render_device.create_bind_group_layout(
            "sprite_view_layout",
            &BindGroupLayoutEntries::single(
                ShaderStages::VERTEX_FRAGMENT,
                uniform_buffer::<ViewUniform>(true),
            ),
        );

        let material_layout = render_device.create_bind_group_layout(
            "sprite_material_layout",
            &BindGroupLayoutEntries::sequential(
                ShaderStages::FRAGMENT,
                (
                    texture_2d(TextureSampleType::Float { filterable: true }),
                    sampler(SamplerBindingType::Filtering),
                ),
            ),
        );
        let dummy_white_gpu_image = {
            let image = Image::default();
            let texture = render_device.create_texture(&image.texture_descriptor);
            let sampler = match image.sampler {
                ImageSampler::Default => (**default_sampler).clone(),
                ImageSampler::Descriptor(ref descriptor) => {
                    render_device.create_sampler(&descriptor.as_wgpu())
                }
            };

            let format_size = image.texture_descriptor.format.pixel_size();
            render_queue.write_texture(
                texture.as_image_copy(),
                &image.data,
                ImageDataLayout {
                    offset: 0,
                    bytes_per_row: Some(image.width() * format_size as u32),
                    rows_per_image: None,
                },
                image.texture_descriptor.size,
            );
            let texture_view = texture.create_view(&TextureViewDescriptor::default());
            GpuImage {
                texture,
                texture_view,
                texture_format: image.texture_descriptor.format,
                sampler,
                size: image.size_f32(),
                mip_level_count: image.texture_descriptor.mip_level_count,
            }
        };

        SpritePipeline {
            view_layout,
            material_layout,
            dummy_white_gpu_image,
        }
    }
}

impl SpecializedRenderPipeline for SpritePipeline {
    type Key = Mesh2dViewKey;

    fn specialize(&self, key: PipelineKey<Self::Key>) -> RenderPipelineDescriptor {
        let shader_defs = key.shader_defs();

        let instance_rate_vertex_buffer_layout = VertexBufferLayout {
            array_stride: 80,
            step_mode: VertexStepMode::Instance,
            attributes: vec![
                // @location(0) i_model_transpose_col0: vec4<f32>,
                VertexAttribute {
                    format: VertexFormat::Float32x4,
                    offset: 0,
                    shader_location: 0,
                },
                // @location(1) i_model_transpose_col1: vec4<f32>,
                VertexAttribute {
                    format: VertexFormat::Float32x4,
                    offset: 16,
                    shader_location: 1,
                },
                // @location(2) i_model_transpose_col2: vec4<f32>,
                VertexAttribute {
                    format: VertexFormat::Float32x4,
                    offset: 32,
                    shader_location: 2,
                },
                // @location(3) i_color: vec4<f32>,
                VertexAttribute {
                    format: VertexFormat::Float32x4,
                    offset: 48,
                    shader_location: 3,
                },
                // @location(4) i_uv_offset_scale: vec4<f32>,
                VertexAttribute {
                    format: VertexFormat::Float32x4,
                    offset: 64,
                    shader_location: 4,
                },
            ],
        };

        RenderPipelineDescriptor {
            vertex: VertexState {
                shader: SPRITE_SHADER_HANDLE,
                entry_point: "vertex".into(),
                shader_defs: shader_defs.clone(),
                buffers: vec![instance_rate_vertex_buffer_layout],
            },
            fragment: Some(FragmentState {
                shader: SPRITE_SHADER_HANDLE,
                shader_defs,
                entry_point: "fragment".into(),
                targets: vec![Some(ColorTargetState {
                    format: key.texture_format.format(),
                    blend: Some(BlendState::ALPHA_BLENDING),
                    write_mask: ColorWrites::ALL,
                })],
            }),
            layout: vec![self.view_layout.clone(), self.material_layout.clone()],
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
                count: key.msaa.samples(),
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            label: Some("sprite_pipeline".into()),
            push_constant_ranges: Vec::new(),
        }
    }
}

pub struct ExtractedSprite {
    pub transform: GlobalTransform,
    pub color: Color,
    /// Select an area of the texture
    pub rect: Option<Rect>,
    /// Change the on-screen size of the sprite
    pub custom_size: Option<Vec2>,
    /// Asset ID of the [`Image`] of this sprite
    /// PERF: storing an `AssetId` instead of `Handle<Image>` enables some optimizations (`ExtractedSprite` becomes `Copy` and doesn't need to be dropped)
    pub image_handle_id: AssetId<Image>,
    pub flip_x: bool,
    pub flip_y: bool,
    pub anchor: Vec2,
    /// For cases where additional ExtractedSprites are created during extraction, this stores the
    /// entity that caused that creation for use in determining visibility.
    pub original_entity: Option<Entity>,
}

#[derive(Resource, Default)]
pub struct ExtractedSprites {
    pub sprites: EntityHashMap<Entity, ExtractedSprite>,
}

#[derive(Resource, Default)]
pub struct SpriteAssetEvents {
    pub images: Vec<AssetEvent<Image>>,
}

pub fn extract_sprite_events(
    mut events: ResMut<SpriteAssetEvents>,
    mut image_events: Extract<EventReader<AssetEvent<Image>>>,
) {
    let SpriteAssetEvents { ref mut images } = *events;
    images.clear();

    for event in image_events.read() {
        images.push(*event);
    }
}

pub fn extract_sprites(
    mut extracted_sprites: ResMut<ExtractedSprites>,
    texture_atlases: Extract<Res<Assets<TextureAtlas>>>,
    sprite_query: Extract<
        Query<(
            Entity,
            &ViewVisibility,
            &Sprite,
            &GlobalTransform,
            &Handle<Image>,
        )>,
    >,
    atlas_query: Extract<
        Query<(
            Entity,
            &ViewVisibility,
            &TextureAtlasSprite,
            &GlobalTransform,
            &Handle<TextureAtlas>,
        )>,
    >,
) {
    extracted_sprites.sprites.clear();

    for (entity, view_visibility, sprite, transform, handle) in sprite_query.iter() {
        if !view_visibility.get() {
            continue;
        }
        // PERF: we don't check in this function that the `Image` asset is ready, since it should be in most cases and hashing the handle is expensive
        extracted_sprites.sprites.insert(
            entity,
            ExtractedSprite {
                color: sprite.color,
                transform: *transform,
                rect: sprite.rect,
                // Pass the custom size
                custom_size: sprite.custom_size,
                flip_x: sprite.flip_x,
                flip_y: sprite.flip_y,
                image_handle_id: handle.id(),
                anchor: sprite.anchor.as_vec(),
                original_entity: None,
            },
        );
    }
    for (entity, view_visibility, atlas_sprite, transform, texture_atlas_handle) in
        atlas_query.iter()
    {
        if !view_visibility.get() {
            continue;
        }
        if let Some(texture_atlas) = texture_atlases.get(texture_atlas_handle) {
            let rect = Some(
                *texture_atlas
                    .textures
                    .get(atlas_sprite.index)
                    .unwrap_or_else(|| {
                        panic!(
                            "Sprite index {:?} does not exist for texture atlas handle {:?}.",
                            atlas_sprite.index,
                            texture_atlas_handle.id(),
                        )
                    }),
            );
            extracted_sprites.sprites.insert(
                entity,
                ExtractedSprite {
                    color: atlas_sprite.color,
                    transform: *transform,
                    // Select the area in the texture atlas
                    rect,
                    // Pass the custom size
                    custom_size: atlas_sprite.custom_size,
                    flip_x: atlas_sprite.flip_x,
                    flip_y: atlas_sprite.flip_y,
                    image_handle_id: texture_atlas.texture.id(),
                    anchor: atlas_sprite.anchor.as_vec(),
                    original_entity: None,
                },
            );
        }
    }
}

#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
struct SpriteInstance {
    // Affine 4x3 transposed to 3x4
    pub i_model_transpose: [Vec4; 3],
    pub i_color: [f32; 4],
    pub i_uv_offset_scale: [f32; 4],
}

impl SpriteInstance {
    #[inline]
    fn from(transform: &Affine3A, color: &Color, uv_offset_scale: &Vec4) -> Self {
        let transpose_model_3x3 = transform.matrix3.transpose();
        Self {
            i_model_transpose: [
                transpose_model_3x3.x_axis.extend(transform.translation.x),
                transpose_model_3x3.y_axis.extend(transform.translation.y),
                transpose_model_3x3.z_axis.extend(transform.translation.z),
            ],
            i_color: color.as_linear_rgba_f32(),
            i_uv_offset_scale: uv_offset_scale.to_array(),
        }
    }
}

#[derive(Resource)]
pub struct SpriteMeta {
    view_bind_group: Option<BindGroup>,
    sprite_index_buffer: BufferVec<u32>,
    sprite_instance_buffer: BufferVec<SpriteInstance>,
}

impl Default for SpriteMeta {
    fn default() -> Self {
        Self {
            view_bind_group: None,
            sprite_index_buffer: BufferVec::<u32>::new(BufferUsages::INDEX),
            sprite_instance_buffer: BufferVec::<SpriteInstance>::new(BufferUsages::VERTEX),
        }
    }
}

#[derive(Component, PartialEq, Eq, Clone)]
pub struct SpriteBatch {
    image_handle_id: AssetId<Image>,
    range: Range<u32>,
}

#[derive(Resource, Default)]
pub struct ImageBindGroups {
    values: HashMap<AssetId<Image>, BindGroup>,
}

#[allow(clippy::too_many_arguments)]
pub fn queue_sprites(
    mut view_entities: Local<FixedBitSet>,
    draw_functions: Res<DrawFunctions<Transparent2d>>,
    sprite_pipeline: Res<SpritePipeline>,
    mut pipelines: ResMut<SpecializedRenderPipelines<SpritePipeline>>,
    pipeline_cache: Res<PipelineCache>,
    extracted_sprites: Res<ExtractedSprites>,
    mut views: Query<(
        &mut RenderPhase<Transparent2d>,
        &VisibleEntities,
        &PipelineKeys,
    )>,
) {
    let draw_sprite_function = draw_functions.read().id::<DrawSprite>();

    for (mut transparent_phase, visible_entities, keys) in &mut views {
        let Some(view_key) = keys.get_packed_key::<Mesh2dViewKey>() else {
            continue;
        };

        let pipeline = pipelines.specialize(&pipeline_cache, &sprite_pipeline, view_key);

        view_entities.clear();
        view_entities.extend(visible_entities.entities.iter().map(|e| e.index() as usize));

        transparent_phase
            .items
            .reserve(extracted_sprites.sprites.len());

        for (entity, extracted_sprite) in extracted_sprites.sprites.iter() {
            let index = extracted_sprite.original_entity.unwrap_or(*entity).index();

            if !view_entities.contains(index as usize) {
                continue;
            }

            // These items will be sorted by depth with other phase items
            let sort_key = FloatOrd(extracted_sprite.transform.translation().z);

            // Add the item to the render phase
            transparent_phase.add(Transparent2d {
                draw_function: draw_sprite_function,
                pipeline,
                entity: *entity,
                sort_key,
                // batch_range and dynamic_offset will be calculated in prepare_sprites
                batch_range: 0..0,
                dynamic_offset: None,
            });
        }
    }
}

#[allow(clippy::too_many_arguments)]
pub fn prepare_sprites(
    mut commands: Commands,
    mut previous_len: Local<usize>,
    render_device: Res<RenderDevice>,
    render_queue: Res<RenderQueue>,
    mut sprite_meta: ResMut<SpriteMeta>,
    view_uniforms: Res<ViewUniforms>,
    sprite_pipeline: Res<SpritePipeline>,
    mut image_bind_groups: ResMut<ImageBindGroups>,
    gpu_images: Res<RenderAssets<Image>>,
    extracted_sprites: Res<ExtractedSprites>,
    mut phases: Query<&mut RenderPhase<Transparent2d>>,
    events: Res<SpriteAssetEvents>,
) {
    // If an image has changed, the GpuImage has (probably) changed
    for event in &events.images {
        match event {
            AssetEvent::Added {..} |
            // images don't have dependencies
            AssetEvent::LoadedWithDependencies { .. } => {}
            AssetEvent::Modified { id } | AssetEvent::Removed { id } => {
                image_bind_groups.values.remove(id);
            }
        };
    }

    if let Some(view_binding) = view_uniforms.uniforms.binding() {
        let mut batches: Vec<(Entity, SpriteBatch)> = Vec::with_capacity(*previous_len);

        // Clear the sprite instances
        sprite_meta.sprite_instance_buffer.clear();

        sprite_meta.view_bind_group = Some(render_device.create_bind_group(
            "sprite_view_bind_group",
            &sprite_pipeline.view_layout,
            &BindGroupEntries::single(view_binding),
        ));

        // Index buffer indices
        let mut index = 0;

        let image_bind_groups = &mut *image_bind_groups;

        for mut transparent_phase in &mut phases {
            let mut batch_item_index = 0;
            let mut batch_image_size = Vec2::ZERO;
            let mut batch_image_handle = AssetId::invalid();

            // Iterate through the phase items and detect when successive sprites that can be batched.
            // Spawn an entity with a `SpriteBatch` component for each possible batch.
            // Compatible items share the same entity.
            for item_index in 0..transparent_phase.items.len() {
                let item = &transparent_phase.items[item_index];
                let Some(extracted_sprite) = extracted_sprites.sprites.get(&item.entity) else {
                    // If there is a phase item that is not a sprite, then we must start a new
                    // batch to draw the other phase item(s) and to respect draw order. This can be
                    // done by invalidating the batch_image_handle
                    batch_image_handle = AssetId::invalid();
                    continue;
                };

                let batch_image_changed = batch_image_handle != extracted_sprite.image_handle_id;
                if batch_image_changed {
                    let Some(gpu_image) = gpu_images.get(extracted_sprite.image_handle_id) else {
                        continue;
                    };

                    batch_image_size = Vec2::new(gpu_image.size.x, gpu_image.size.y);
                    batch_image_handle = extracted_sprite.image_handle_id;
                    image_bind_groups
                        .values
                        .entry(batch_image_handle)
                        .or_insert_with(|| {
                            render_device.create_bind_group(
                                "sprite_material_bind_group",
                                &sprite_pipeline.material_layout,
                                &BindGroupEntries::sequential((
                                    &gpu_image.texture_view,
                                    &gpu_image.sampler,
                                )),
                            )
                        });
                }

                // By default, the size of the quad is the size of the texture
                let mut quad_size = batch_image_size;

                // Calculate vertex data for this item
                let mut uv_offset_scale: Vec4;

                // If a rect is specified, adjust UVs and the size of the quad
                if let Some(rect) = extracted_sprite.rect {
                    let rect_size = rect.size();
                    uv_offset_scale = Vec4::new(
                        rect.min.x / batch_image_size.x,
                        rect.max.y / batch_image_size.y,
                        rect_size.x / batch_image_size.x,
                        -rect_size.y / batch_image_size.y,
                    );
                    quad_size = rect_size;
                } else {
                    uv_offset_scale = Vec4::new(0.0, 1.0, 1.0, -1.0);
                }

                if extracted_sprite.flip_x {
                    uv_offset_scale.x += uv_offset_scale.z;
                    uv_offset_scale.z *= -1.0;
                }
                if extracted_sprite.flip_y {
                    uv_offset_scale.y += uv_offset_scale.w;
                    uv_offset_scale.w *= -1.0;
                }

                // Override the size if a custom one is specified
                if let Some(custom_size) = extracted_sprite.custom_size {
                    quad_size = custom_size;
                }
                let transform = extracted_sprite.transform.affine()
                    * Affine3A::from_scale_rotation_translation(
                        quad_size.extend(1.0),
                        Quat::IDENTITY,
                        (quad_size * (-extracted_sprite.anchor - Vec2::splat(0.5))).extend(0.0),
                    );

                // Store the vertex data and add the item to the render phase
                sprite_meta
                    .sprite_instance_buffer
                    .push(SpriteInstance::from(
                        &transform,
                        &extracted_sprite.color,
                        &uv_offset_scale,
                    ));

                if batch_image_changed {
                    batch_item_index = item_index;

                    batches.push((
                        item.entity,
                        SpriteBatch {
                            image_handle_id: batch_image_handle,
                            range: index..index,
                        },
                    ));
                }

                transparent_phase.items[batch_item_index]
                    .batch_range_mut()
                    .end += 1;
                batches.last_mut().unwrap().1.range.end += 1;
                index += 1;
            }
        }
        sprite_meta
            .sprite_instance_buffer
            .write_buffer(&render_device, &render_queue);

        if sprite_meta.sprite_index_buffer.len() != 6 {
            sprite_meta.sprite_index_buffer.clear();

            // NOTE: This code is creating 6 indices pointing to 4 vertices.
            // The vertices form the corners of a quad based on their two least significant bits.
            // 10   11
            //
            // 00   01
            // The sprite shader can then use the two least significant bits as the vertex index.
            // The rest of the properties to transform the vertex positions and UVs (which are
            // implicit) are baked into the instance transform, and UV offset and scale.
            // See bevy_sprite/src/render/sprite.wgsl for the details.
            sprite_meta.sprite_index_buffer.push(2);
            sprite_meta.sprite_index_buffer.push(0);
            sprite_meta.sprite_index_buffer.push(1);
            sprite_meta.sprite_index_buffer.push(1);
            sprite_meta.sprite_index_buffer.push(3);
            sprite_meta.sprite_index_buffer.push(2);

            sprite_meta
                .sprite_index_buffer
                .write_buffer(&render_device, &render_queue);
        }

        *previous_len = batches.len();
        commands.insert_or_spawn_batch(batches);
    }
}

pub type DrawSprite = (
    SetItemPipeline,
    SetSpriteViewBindGroup<0>,
    SetSpriteTextureBindGroup<1>,
    DrawSpriteBatch,
);

pub struct SetSpriteViewBindGroup<const I: usize>;
impl<P: PhaseItem, const I: usize> RenderCommand<P> for SetSpriteViewBindGroup<I> {
    type Param = SRes<SpriteMeta>;
    type ViewWorldQuery = Read<ViewUniformOffset>;
    type ItemWorldQuery = ();

    fn render<'w>(
        _item: &P,
        view_uniform: &'_ ViewUniformOffset,
        _entity: (),
        sprite_meta: SystemParamItem<'w, '_, Self::Param>,
        pass: &mut TrackedRenderPass<'w>,
    ) -> RenderCommandResult {
        pass.set_bind_group(
            I,
            sprite_meta.into_inner().view_bind_group.as_ref().unwrap(),
            &[view_uniform.offset],
        );
        RenderCommandResult::Success
    }
}
pub struct SetSpriteTextureBindGroup<const I: usize>;
impl<P: PhaseItem, const I: usize> RenderCommand<P> for SetSpriteTextureBindGroup<I> {
    type Param = SRes<ImageBindGroups>;
    type ViewWorldQuery = ();
    type ItemWorldQuery = Read<SpriteBatch>;

    fn render<'w>(
        _item: &P,
        _view: (),
        batch: &'_ SpriteBatch,
        image_bind_groups: SystemParamItem<'w, '_, Self::Param>,
        pass: &mut TrackedRenderPass<'w>,
    ) -> RenderCommandResult {
        let image_bind_groups = image_bind_groups.into_inner();

        pass.set_bind_group(
            I,
            image_bind_groups
                .values
                .get(&batch.image_handle_id)
                .unwrap(),
            &[],
        );
        RenderCommandResult::Success
    }
}

pub struct DrawSpriteBatch;
impl<P: PhaseItem> RenderCommand<P> for DrawSpriteBatch {
    type Param = SRes<SpriteMeta>;
    type ViewWorldQuery = ();
    type ItemWorldQuery = Read<SpriteBatch>;

    fn render<'w>(
        _item: &P,
        _view: (),
        batch: &'_ SpriteBatch,
        sprite_meta: SystemParamItem<'w, '_, Self::Param>,
        pass: &mut TrackedRenderPass<'w>,
    ) -> RenderCommandResult {
        let sprite_meta = sprite_meta.into_inner();
        pass.set_index_buffer(
            sprite_meta.sprite_index_buffer.buffer().unwrap().slice(..),
            0,
            IndexFormat::Uint32,
        );
        pass.set_vertex_buffer(
            0,
            sprite_meta
                .sprite_instance_buffer
                .buffer()
                .unwrap()
                .slice(..),
        );
        pass.draw_indexed(0..6, 0, batch.range.clone());
        RenderCommandResult::Success
    }
}
