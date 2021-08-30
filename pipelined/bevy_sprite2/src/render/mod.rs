use std::{mem, ops::Range};

use crate::{
    texture_atlas::{TextureAtlas, TextureAtlasSprite},
    Rect, Sprite,
};
use bevy_asset::{Assets, Handle};
use bevy_core_pipeline::Transparent2dPhase;
use bevy_ecs::{prelude::*, system::SystemState};
use bevy_math::{Mat4, Vec2};
use bevy_render2::{
    render_asset::RenderAssets,
    render_graph::{Node, NodeRunError, RenderGraphContext},
    render_phase::{Draw, DrawFunctions, Drawable, RenderPhase, TrackedRenderPass},
    render_resource::*,
    renderer::{RenderContext, RenderDevice},
    shader::Shader,
    texture::{BevyDefault, Image},
    view::{ViewMeta, ViewUniformOffset},
    RenderWorld,
};
use bevy_transform::components::GlobalTransform;
use bevy_utils::slab::{FrameSlabMap, FrameSlabMapKey};
use bytemuck::{Pod, Zeroable};

pub(crate) struct SpriteShaders {
    pipeline: RenderPipeline,
    view_layout: BindGroupLayout,
    material_layout: BindGroupLayout,
}

// TODO: this pattern for initializing the shaders / pipeline isn't ideal. this should be handled by the asset system
impl FromWorld for SpriteShaders {
    fn from_world(world: &mut World) -> Self {
        let render_device = world.get_resource::<RenderDevice>().unwrap();
        let shader = Shader::from_wgsl(include_str!("sprite.wgsl"));
        let shader_module = render_device.create_shader_module(&shader);

        let view_layout = render_device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            entries: &[BindGroupLayoutEntry {
                binding: 0,
                visibility: ShaderStage::VERTEX | ShaderStage::FRAGMENT,
                ty: BindingType::Buffer {
                    ty: BufferBindingType::Uniform,
                    has_dynamic_offset: true,
                    // TODO: change this to ViewUniform::std140_size_static once crevice fixes this!
                    // Context: https://github.com/LPGhatguy/crevice/issues/29
                    min_binding_size: BufferSize::new(144),
                },
                count: None,
            }],
            label: None,
        });

        let material_layout = render_device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            entries: &[
                BindGroupLayoutEntry {
                    binding: 0,
                    visibility: ShaderStage::FRAGMENT,
                    ty: BindingType::Texture {
                        multisampled: false,
                        sample_type: TextureSampleType::Float { filterable: false },
                        view_dimension: TextureViewDimension::D2,
                    },
                    count: None,
                },
                BindGroupLayoutEntry {
                    binding: 1,
                    visibility: ShaderStage::FRAGMENT,
                    ty: BindingType::Sampler {
                        comparison: false,
                        filtering: true,
                    },
                    count: None,
                },
            ],
            label: None,
        });

        let pipeline_layout = render_device.create_pipeline_layout(&PipelineLayoutDescriptor {
            label: None,
            push_constant_ranges: &[],
            bind_group_layouts: &[&view_layout, &material_layout],
        });

        let pipeline = render_device.create_render_pipeline(&RenderPipelineDescriptor {
            label: None,
            depth_stencil: None,
            vertex: VertexState {
                buffers: &[VertexBufferLayout {
                    array_stride: mem::size_of::<SpriteInstance>() as BufferAddress,
                    step_mode: InputStepMode::Instance,
                    attributes: &[
                        // sprite_transform_0
                        VertexAttribute {
                            format: VertexFormat::Float32x4,
                            // The offset for the first attribute is 0
                            offset: 0,
                            shader_location: 0,
                        },
                        // sprite_transform_1
                        VertexAttribute {
                            format: VertexFormat::Float32x4,
                            // All other offsets need to be the total size of all of the attributes before it
                            offset: VertexFormat::Float32x4.size(),
                            shader_location: 1,
                        },
                        // sprite_transform_2
                        VertexAttribute {
                            format: VertexFormat::Float32x4,
                            offset: VertexFormat::Float32x4.size() * 2,
                            shader_location: 2,
                        },
                        // sprite_transform_3
                        VertexAttribute {
                            format: VertexFormat::Float32x4,
                            offset: VertexFormat::Float32x4.size() * 3,
                            shader_location: 3,
                        },
                        // sprite_size
                        VertexAttribute {
                            format: VertexFormat::Float32x2,
                            offset: VertexFormat::Float32x4.size() * 4,
                            shader_location: 4,
                        },
                        // uv_min
                        VertexAttribute {
                            format: VertexFormat::Float32x2,
                            offset: VertexFormat::Float32x4.size() * 4
                                + VertexFormat::Float32x2.size(),
                            shader_location: 5,
                        },
                        // uv_size
                        VertexAttribute {
                            format: VertexFormat::Float32x2,
                            offset: VertexFormat::Float32x4.size() * 4
                                + VertexFormat::Float32x2.size() * 2,
                            shader_location: 6,
                        },
                    ],
                }],
                module: &shader_module,
                entry_point: "vertex",
            },
            fragment: Some(FragmentState {
                module: &shader_module,
                entry_point: "fragment",
                targets: &[ColorTargetState {
                    format: TextureFormat::bevy_default(),
                    blend: Some(BlendState {
                        color: BlendComponent {
                            src_factor: BlendFactor::SrcAlpha,
                            dst_factor: BlendFactor::OneMinusSrcAlpha,
                            operation: BlendOperation::Add,
                        },
                        alpha: BlendComponent {
                            src_factor: BlendFactor::One,
                            dst_factor: BlendFactor::One,
                            operation: BlendOperation::Add,
                        },
                    }),
                    write_mask: ColorWrite::ALL,
                }],
            }),
            layout: Some(&pipeline_layout),
            multisample: MultisampleState::default(),
            primitive: PrimitiveState {
                topology: PrimitiveTopology::TriangleStrip,
                strip_index_format: None,
                front_face: FrontFace::Ccw,
                cull_mode: None,
                polygon_mode: PolygonMode::Fill,
                clamp_depth: false,
                conservative: false,
            },
        });

        SpriteShaders {
            pipeline,
            view_layout,
            material_layout,
        }
    }
}

#[derive(Debug, Clone)]
struct ExtractedSprite {
    depth: f32,
    transform: Mat4,
    rect: Rect,
    handle: Handle<Image>,
    atlas_size: Option<Vec2>,
    flip_x: bool,
    flip_y: bool,
}

#[derive(Default)]
pub(crate) struct ExtractedSprites {
    sprites: Vec<ExtractedSprite>,
}

pub(crate) fn extract_atlases(
    texture_atlases: Res<Assets<TextureAtlas>>,
    atlas_query: Query<(&TextureAtlasSprite, &GlobalTransform, &Handle<TextureAtlas>)>,
    mut render_world: ResMut<RenderWorld>,
) {
    let extracted_sprites = &mut render_world
        .get_resource_mut::<ExtractedSprites>()
        .unwrap()
        .sprites;
    for (atlas_sprite, transform, texture_atlas_handle) in atlas_query.iter() {
        if !texture_atlases.contains(texture_atlas_handle) {
            continue;
        }

        if let Some(texture_atlas) = texture_atlases.get(texture_atlas_handle) {
            let rect = texture_atlas.textures[atlas_sprite.index as usize];
            extracted_sprites.push(ExtractedSprite {
                atlas_size: Some(texture_atlas.size),
                transform: transform.compute_matrix(),
                rect,
                handle: texture_atlas.texture.clone_weak(),
                flip_x: atlas_sprite.flip_x,
                flip_y: atlas_sprite.flip_y,
                depth: transform.translation.z,
            });
        }
    }
}

pub(crate) fn extract_sprites(
    images: Res<Assets<Image>>,
    sprite_query: Query<(&Sprite, &GlobalTransform, &Handle<Image>)>,
    mut render_world: ResMut<RenderWorld>,
) {
    let extracted_sprites = &mut render_world
        .get_resource_mut::<ExtractedSprites>()
        .unwrap()
        .sprites;
    for (sprite, transform, handle) in sprite_query.iter() {
        let image = if let Some(image) = images.get(handle) {
            image
        } else {
            continue;
        };
        let size = image.texture_descriptor.size;

        extracted_sprites.push(ExtractedSprite {
            atlas_size: None,
            transform: transform.compute_matrix(),
            rect: Rect {
                min: Vec2::ZERO,
                max: sprite
                    .custom_size
                    .unwrap_or_else(|| Vec2::new(size.width as f32, size.height as f32)),
            },
            handle: handle.clone_weak(),
            flip_x: sprite.flip_x,
            flip_y: sprite.flip_y,
            depth: transform.translation.z,
        });
    }
}

#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
struct SpriteInstance {
    transform: [[f32; 4]; 4],
    sprite_size: [f32; 2],
    uv_min: [f32; 2],
    uv_size: [f32; 2],
}

pub(crate) struct SpriteMeta {
    /// The buffer containing sprite instance data
    instances: BufferVec<SpriteInstance>,
    /// The list of sprite batches
    drawables: Vec<DrawableSpriteBatch>,
    view_bind_group: Option<BindGroup>,
    texture_bind_groups: FrameSlabMap<Handle<Image>, BindGroup>,
}

/// A batch of sprites that share the same texture and can be drawn in the same call
struct DrawableSpriteBatch {
    texture_bind_group_key: FrameSlabMapKey<Handle<Image>, BindGroup>,
    instances: Range<u32>,
}

impl Default for SpriteMeta {
    fn default() -> Self {
        Self {
            instances: BufferVec::new(BufferUsage::VERTEX),
            texture_bind_groups: Default::default(),
            drawables: Default::default(),
            view_bind_group: None,
        }
    }
}

pub(crate) fn prepare_sprites(
    render_device: Res<RenderDevice>,
    mut sprite_meta: ResMut<SpriteMeta>,
    mut extracted_sprites: ResMut<ExtractedSprites>,
) {
    // dont create buffers when there are no sprites
    if extracted_sprites.sprites.is_empty() {
        return;
    }

    // Sort sprites first by their depth, then by their texture
    extracted_sprites.sprites.sort_unstable_by(|a, b| {
        let depth_diff = (a.depth - b.depth).abs();

        // If depths are essentially equal
        if depth_diff < f32::EPSILON {
            // Compare based on texture
            a.handle.cmp(&b.handle)

        // If the depths are unequal, return the comparison of their depths
        } else {
            b.depth
                .partial_cmp(&a.depth)
                .expect("Could not compare floats")
        }
    });

    // Reserve space in the instance buffer for the sprites
    sprite_meta
        .instances
        .reserve_and_clear(extracted_sprites.sprites.len(), &render_device);

    // Push an instance to the buffer for every sprite
    for extracted_sprite in extracted_sprites.sprites.iter() {
        let sprite_rect = extracted_sprite.rect;
        let size = sprite_rect.size().into();
        let transform = extracted_sprite.transform.to_cols_array_2d();
        let mut uv_min = sprite_rect.min / extracted_sprite.atlas_size.unwrap_or(sprite_rect.max);
        let uv_max = sprite_rect.max / extracted_sprite.atlas_size.unwrap_or(sprite_rect.max);
        let mut uv_size = uv_max - uv_min;

        // Flip the sprite UV along x and y axes if necessary
        if extracted_sprite.flip_x {
            uv_min.x += uv_size.x;
            uv_size.x = -uv_size.x;
        }
        if extracted_sprite.flip_y {
            uv_min.y += uv_size.y;
            uv_size.y = -uv_size.y;
        }

        sprite_meta.instances.push(SpriteInstance {
            transform,
            sprite_size: size,
            uv_min: uv_min.into(),
            uv_size: uv_size.into(),
        });
    }

    // Write buffer to staging area
    sprite_meta
        .instances
        .write_to_staging_buffer(&render_device);
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn queue_sprites(
    draw_functions: Res<DrawFunctions>,
    render_device: Res<RenderDevice>,
    mut sprite_meta: ResMut<SpriteMeta>,
    view_meta: Res<ViewMeta>,
    sprite_shaders: Res<SpriteShaders>,
    mut extracted_sprites: ResMut<ExtractedSprites>,
    gpu_images: Res<RenderAssets<Image>>,
    mut views: Query<&mut RenderPhase<Transparent2dPhase>>,
) {
    if view_meta.uniforms.is_empty() {
        return;
    }
    if extracted_sprites.sprites.is_empty() {
        return;
    }

    // TODO: define this without needing to check every frame
    sprite_meta.view_bind_group.get_or_insert_with(|| {
        render_device.create_bind_group(&BindGroupDescriptor {
            entries: &[BindGroupEntry {
                binding: 0,
                resource: view_meta.uniforms.binding(),
            }],
            label: None,
            layout: &sprite_shaders.view_layout,
        })
    });
    let sprite_meta = &mut *sprite_meta;
    let draw_sprite_function = draw_functions.read().get_id::<DrawSprite>().unwrap();
    sprite_meta.texture_bind_groups.next_frame();
    sprite_meta.drawables.clear();
    for mut transparent_phase in views.iter_mut() {
        let mut last_texture_bind_group_key = None;
        let mut current_batch_start = 0usize;
        let mut current_batch_len = 0usize;

        for (i, sprite) in extracted_sprites.sprites.iter().enumerate() {
            let texture_bind_group_key = sprite_meta.texture_bind_groups.get_or_insert_with(
                sprite.handle.clone_weak(),
                || {
                    let gpu_image = gpu_images.get(&sprite.handle).unwrap();
                    render_device.create_bind_group(&BindGroupDescriptor {
                        entries: &[
                            BindGroupEntry {
                                binding: 0,
                                resource: BindingResource::TextureView(&gpu_image.texture_view),
                            },
                            BindGroupEntry {
                                binding: 1,
                                resource: BindingResource::Sampler(&gpu_image.sampler),
                            },
                        ],
                        label: None,
                        layout: &sprite_shaders.material_layout,
                    })
                },
            );

            // If this sprite is in the same batch with the current batch
            if last_texture_bind_group_key == Some(texture_bind_group_key) {
                current_batch_len += 1;

            // If it is not in the same batch, but there was a previous batch
            } else if let Some(last_texture_bind_group_key) = last_texture_bind_group_key {
                // Push the previous batch to the drawables list
                let drawable_idx = sprite_meta.drawables.len();
                sprite_meta.drawables.push(DrawableSpriteBatch {
                    texture_bind_group_key: last_texture_bind_group_key,
                    instances: current_batch_start as u32
                        ..(current_batch_start + current_batch_len) as u32,
                });
                // Add the drawable to the render phase
                transparent_phase.add(Drawable {
                    draw_function: draw_sprite_function,
                    draw_key: drawable_idx,
                    sort_key: texture_bind_group_key.index(),
                });

                // Start a new batch
                current_batch_start = i;
                current_batch_len = 1;

            // If this is the very first sprite and there is not a current batch yet
            } else {
                current_batch_len += 1;
            };

            // Update the last bind group key with the current key
            last_texture_bind_group_key = Some(texture_bind_group_key);
        }

        // Add the last pending batch to the render pass
        let drawable_idx = sprite_meta.drawables.len();
        sprite_meta.drawables.push(DrawableSpriteBatch {
            texture_bind_group_key: last_texture_bind_group_key.unwrap(),
            instances: current_batch_start as u32..(current_batch_start + current_batch_len) as u32,
        });

        // Add the drawable to the render phase
        transparent_phase.add(Drawable {
            draw_function: draw_sprite_function,
            draw_key: drawable_idx,
            sort_key: last_texture_bind_group_key.unwrap().index(),
        });
    }

    extracted_sprites.sprites.clear();
}

// TODO: this logic can be moved to prepare_sprites once wgpu::Queue is exposed directly
pub(crate) struct SpriteNode;

impl Node for SpriteNode {
    fn run(
        &self,
        _graph: &mut RenderGraphContext,
        render_context: &mut RenderContext,
        world: &World,
    ) -> Result<(), NodeRunError> {
        let sprite_buffers = world.get_resource::<SpriteMeta>().unwrap();
        sprite_buffers
            .instances
            .write_to_buffer(&mut render_context.command_encoder);
        Ok(())
    }
}

type DrawSpriteQuery<'s, 'w> = (
    Res<'w, SpriteShaders>,
    Res<'w, SpriteMeta>,
    Query<'w, 's, &'w ViewUniformOffset>,
);
pub(crate) struct DrawSprite {
    params: SystemState<DrawSpriteQuery<'static, 'static>>,
}

impl DrawSprite {
    pub fn new(world: &mut World) -> Self {
        Self {
            params: SystemState::new(world),
        }
    }
}

impl Draw for DrawSprite {
    fn draw<'w>(
        &mut self,
        world: &'w World,
        pass: &mut TrackedRenderPass<'w>,
        view: Entity,
        draw_key: usize,
        _sort_key: usize,
    ) {
        let (sprite_shaders, sprite_meta, views) = self.params.get(world);
        let view_uniform = views.get(view).unwrap();
        let sprite_meta = sprite_meta.into_inner();
        let batch = &sprite_meta.drawables[draw_key];

        pass.set_render_pipeline(&sprite_shaders.into_inner().pipeline);
        pass.set_vertex_buffer(0, sprite_meta.instances.buffer().unwrap().slice(..));
        pass.set_bind_group(
            0,
            sprite_meta.view_bind_group.as_ref().unwrap(),
            &[view_uniform.offset],
        );
        pass.set_bind_group(
            1,
            &sprite_meta.texture_bind_groups[batch.texture_bind_group_key],
            &[],
        );

        pass.draw(0..4, batch.instances.clone());
    }
}
