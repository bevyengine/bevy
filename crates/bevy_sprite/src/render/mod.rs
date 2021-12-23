use std::{cmp::Ordering, ops::Range};

use crate::{
    texture_atlas::{TextureAtlas, TextureAtlasSprite},
    Rect, Sprite, SPRITE_SHADER_HANDLE,
};
use bevy_asset::{AssetEvent, Assets, Handle};
use bevy_core::FloatOrd;
use bevy_core_pipeline::Transparent2d;
use bevy_ecs::{
    prelude::*,
    system::{lifetimeless::*, SystemState},
};
use bevy_math::{const_vec3, Mat4, Vec2, Vec3, Vec4Swizzles};
use bevy_render::{
    color::Color,
    render_asset::RenderAssets,
    render_phase::{Draw, DrawFunctions, RenderPhase, TrackedRenderPass},
    render_resource::*,
    renderer::{RenderDevice, RenderQueue},
    texture::{BevyDefault, Image},
    view::{ComputedVisibility, ViewUniform, ViewUniformOffset, ViewUniforms},
    RenderWorld,
};
use bevy_transform::components::GlobalTransform;
use bevy_utils::HashMap;
use bytemuck::{Pod, Zeroable};
use crevice::std140::AsStd140;

pub struct SpritePipeline {
    view_layout: BindGroupLayout,
    material_layout: BindGroupLayout,
}

impl FromWorld for SpritePipeline {
    fn from_world(world: &mut World) -> Self {
        let world = world.cell();
        let render_device = world.get_resource::<RenderDevice>().unwrap();

        let view_layout = render_device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            entries: &[BindGroupLayoutEntry {
                binding: 0,
                visibility: ShaderStages::VERTEX | ShaderStages::FRAGMENT,
                ty: BindingType::Buffer {
                    ty: BufferBindingType::Uniform,
                    has_dynamic_offset: true,
                    min_binding_size: BufferSize::new(ViewUniform::std140_size_static() as u64),
                },
                count: None,
            }],
            label: Some("sprite_view_layout"),
        });

        let material_layout = render_device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            entries: &[
                BindGroupLayoutEntry {
                    binding: 0,
                    visibility: ShaderStages::FRAGMENT,
                    ty: BindingType::Texture {
                        multisampled: false,
                        sample_type: TextureSampleType::Float { filterable: true },
                        view_dimension: TextureViewDimension::D2,
                    },
                    count: None,
                },
                BindGroupLayoutEntry {
                    binding: 1,
                    visibility: ShaderStages::FRAGMENT,
                    ty: BindingType::Sampler(SamplerBindingType::Filtering),
                    count: None,
                },
            ],
            label: Some("sprite_material_layout"),
        });

        SpritePipeline {
            view_layout,
            material_layout,
        }
    }
}

#[derive(Clone, Copy, Hash, PartialEq, Eq)]
pub struct SpritePipelineKey {
    colored: bool,
}

impl SpecializedPipeline for SpritePipeline {
    type Key = SpritePipelineKey;

    fn specialize(&self, key: Self::Key) -> RenderPipelineDescriptor {
        let mut vertex_buffer_layout = VertexBufferLayout {
            array_stride: 20,
            step_mode: VertexStepMode::Vertex,
            attributes: vec![
                VertexAttribute {
                    format: VertexFormat::Float32x3,
                    offset: 0,
                    shader_location: 0,
                },
                VertexAttribute {
                    format: VertexFormat::Float32x2,
                    offset: 12,
                    shader_location: 1,
                },
            ],
        };
        let mut shader_defs = Vec::new();
        if key.colored {
            shader_defs.push("COLORED".to_string());
            vertex_buffer_layout.attributes.push(VertexAttribute {
                format: VertexFormat::Uint32,
                offset: 20,
                shader_location: 2,
            });
            vertex_buffer_layout.array_stride += 4;
        }

        RenderPipelineDescriptor {
            vertex: VertexState {
                shader: SPRITE_SHADER_HANDLE.typed::<Shader>(),
                entry_point: "vertex".into(),
                shader_defs: shader_defs.clone(),
                buffers: vec![vertex_buffer_layout],
            },
            fragment: Some(FragmentState {
                shader: SPRITE_SHADER_HANDLE.typed::<Shader>(),
                shader_defs,
                entry_point: "fragment".into(),
                targets: vec![ColorTargetState {
                    format: TextureFormat::bevy_default(),
                    blend: Some(BlendState::ALPHA_BLENDING),
                    write_mask: ColorWrites::ALL,
                }],
            }),
            layout: Some(vec![self.view_layout.clone(), self.material_layout.clone()]),
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
            label: Some("sprite_pipeline".into()),
        }
    }
}

pub struct ExtractedSprite {
    pub transform: Mat4,
    pub color: Color,
    pub rect: Rect,
    pub handle: Handle<Image>,
    pub atlas_size: Option<Vec2>,
    pub flip_x: bool,
    pub flip_y: bool,
}

#[derive(Default)]
pub struct ExtractedSprites {
    pub sprites: Vec<ExtractedSprite>,
}

#[derive(Default)]
pub struct SpriteAssetEvents {
    pub images: Vec<AssetEvent<Image>>,
}

pub fn extract_sprite_events(
    mut render_world: ResMut<RenderWorld>,
    mut image_events: EventReader<AssetEvent<Image>>,
) {
    let mut events = render_world
        .get_resource_mut::<SpriteAssetEvents>()
        .unwrap();
    let SpriteAssetEvents { ref mut images } = *events;
    images.clear();

    for image in image_events.iter() {
        // AssetEvent: !Clone
        images.push(match image {
            AssetEvent::Created { handle } => AssetEvent::Created {
                handle: handle.clone_weak(),
            },
            AssetEvent::Modified { handle } => AssetEvent::Modified {
                handle: handle.clone_weak(),
            },
            AssetEvent::Removed { handle } => AssetEvent::Removed {
                handle: handle.clone_weak(),
            },
        });
    }
}

pub fn extract_sprites(
    mut render_world: ResMut<RenderWorld>,
    images: Res<Assets<Image>>,
    texture_atlases: Res<Assets<TextureAtlas>>,
    sprite_query: Query<(
        &ComputedVisibility,
        &Sprite,
        &GlobalTransform,
        &Handle<Image>,
    )>,
    atlas_query: Query<(
        &ComputedVisibility,
        &TextureAtlasSprite,
        &GlobalTransform,
        &Handle<TextureAtlas>,
    )>,
) {
    let mut extracted_sprites = render_world.get_resource_mut::<ExtractedSprites>().unwrap();
    extracted_sprites.sprites.clear();
    for (computed_visibility, sprite, transform, handle) in sprite_query.iter() {
        if !computed_visibility.is_visible {
            continue;
        }
        if let Some(image) = images.get(handle) {
            let size = image.texture_descriptor.size;

            extracted_sprites.sprites.push(ExtractedSprite {
                atlas_size: None,
                color: sprite.color,
                transform: transform.compute_matrix(),
                rect: Rect {
                    min: Vec2::ZERO,
                    max: sprite
                        .custom_size
                        .unwrap_or_else(|| Vec2::new(size.width as f32, size.height as f32)),
                },
                flip_x: sprite.flip_x,
                flip_y: sprite.flip_y,
                handle: handle.clone_weak(),
            });
        };
    }
    for (computed_visibility, atlas_sprite, transform, texture_atlas_handle) in atlas_query.iter() {
        if !computed_visibility.is_visible {
            continue;
        }
        if let Some(texture_atlas) = texture_atlases.get(texture_atlas_handle) {
            if images.contains(&texture_atlas.texture) {
                let rect = texture_atlas.textures[atlas_sprite.index as usize];
                extracted_sprites.sprites.push(ExtractedSprite {
                    atlas_size: Some(texture_atlas.size),
                    color: atlas_sprite.color,
                    transform: transform.compute_matrix(),
                    rect,
                    flip_x: atlas_sprite.flip_x,
                    flip_y: atlas_sprite.flip_y,
                    handle: texture_atlas.texture.clone_weak(),
                });
            }
        }
    }
}

#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
struct SpriteVertex {
    pub position: [f32; 3],
    pub uv: [f32; 2],
}

#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
struct ColoredSpriteVertex {
    pub position: [f32; 3],
    pub uv: [f32; 2],
    pub color: u32,
}

pub struct SpriteMeta {
    vertices: BufferVec<SpriteVertex>,
    colored_vertices: BufferVec<ColoredSpriteVertex>,
    view_bind_group: Option<BindGroup>,
}

impl Default for SpriteMeta {
    fn default() -> Self {
        Self {
            vertices: BufferVec::new(BufferUsages::VERTEX),
            colored_vertices: BufferVec::new(BufferUsages::VERTEX),
            view_bind_group: None,
        }
    }
}

const QUAD_VERTEX_POSITIONS: &[Vec3] = &[
    const_vec3!([-0.5, -0.5, 0.0]),
    const_vec3!([0.5, 0.5, 0.0]),
    const_vec3!([-0.5, 0.5, 0.0]),
    const_vec3!([-0.5, -0.5, 0.0]),
    const_vec3!([0.5, -0.5, 0.0]),
    const_vec3!([0.5, 0.5, 0.0]),
];

#[derive(Component)]
pub struct SpriteBatch {
    range: Range<u32>,
    handle: Handle<Image>,
    z: f32,
    colored: bool,
}

pub fn prepare_sprites(
    mut commands: Commands,
    render_device: Res<RenderDevice>,
    render_queue: Res<RenderQueue>,
    mut sprite_meta: ResMut<SpriteMeta>,
    mut extracted_sprites: ResMut<ExtractedSprites>,
) {
    sprite_meta.vertices.clear();
    sprite_meta.colored_vertices.clear();

    // sort first by z and then by handle. this ensures that, when possible, batches span multiple z layers
    // batches won't span z-layers if there is another batch between them
    extracted_sprites.sprites.sort_by(|a, b| {
        match FloatOrd(a.transform.w_axis[2]).cmp(&FloatOrd(b.transform.w_axis[2])) {
            Ordering::Equal => a.handle.cmp(&b.handle),
            other => other,
        }
    });

    let mut start = 0;
    let mut end = 0;
    let mut colored_start = 0;
    let mut colored_end = 0;
    let mut current_batch_handle: Option<Handle<Image>> = None;
    let mut current_batch_colored = false;
    let mut last_z = 0.0;
    for extracted_sprite in extracted_sprites.sprites.iter() {
        let colored = extracted_sprite.color != Color::WHITE;
        if let Some(current_batch_handle) = &current_batch_handle {
            if *current_batch_handle != extracted_sprite.handle || current_batch_colored != colored
            {
                if current_batch_colored {
                    commands.spawn_bundle((SpriteBatch {
                        range: colored_start..colored_end,
                        handle: current_batch_handle.clone_weak(),
                        z: last_z,
                        colored: true,
                    },));
                    colored_start = colored_end;
                } else {
                    commands.spawn_bundle((SpriteBatch {
                        range: start..end,
                        handle: current_batch_handle.clone_weak(),
                        z: last_z,
                        colored: false,
                    },));
                    start = end;
                }
            }
        }
        current_batch_handle = Some(extracted_sprite.handle.clone_weak());
        current_batch_colored = colored;
        let sprite_rect = extracted_sprite.rect;

        // Specify the corners of the sprite
        let mut bottom_left = Vec2::new(sprite_rect.min.x, sprite_rect.max.y);
        let mut top_left = sprite_rect.min;
        let mut top_right = Vec2::new(sprite_rect.max.x, sprite_rect.min.y);
        let mut bottom_right = sprite_rect.max;

        if extracted_sprite.flip_x {
            bottom_left.x = sprite_rect.max.x;
            top_left.x = sprite_rect.max.x;
            bottom_right.x = sprite_rect.min.x;
            top_right.x = sprite_rect.min.x;
        }

        if extracted_sprite.flip_y {
            bottom_left.y = sprite_rect.min.y;
            bottom_right.y = sprite_rect.min.y;
            top_left.y = sprite_rect.max.y;
            top_right.y = sprite_rect.max.y;
        }

        let atlas_extent = extracted_sprite.atlas_size.unwrap_or(sprite_rect.max);
        bottom_left /= atlas_extent;
        bottom_right /= atlas_extent;
        top_left /= atlas_extent;
        top_right /= atlas_extent;

        let uvs: [[f32; 2]; 6] = [
            bottom_left.into(),
            top_right.into(),
            top_left.into(),
            bottom_left.into(),
            bottom_right.into(),
            top_right.into(),
        ];

        let rect_size = extracted_sprite.rect.size().extend(1.0);
        if current_batch_colored {
            let color = extracted_sprite.color.as_linear_rgba_f32();
            // encode color as a single u32 to save space
            let color = (color[0] * 255.0) as u32
                | ((color[1] * 255.0) as u32) << 8
                | ((color[2] * 255.0) as u32) << 16
                | ((color[3] * 255.0) as u32) << 24;
            for (index, vertex_position) in QUAD_VERTEX_POSITIONS.iter().enumerate() {
                let mut final_position = *vertex_position * rect_size;
                final_position = (extracted_sprite.transform * final_position.extend(1.0)).xyz();
                sprite_meta.colored_vertices.push(ColoredSpriteVertex {
                    position: final_position.into(),
                    uv: uvs[index],
                    color,
                });
            }
        } else {
            for (index, vertex_position) in QUAD_VERTEX_POSITIONS.iter().enumerate() {
                let mut final_position = *vertex_position * rect_size;
                final_position = (extracted_sprite.transform * final_position.extend(1.0)).xyz();
                sprite_meta.vertices.push(SpriteVertex {
                    position: final_position.into(),
                    uv: uvs[index],
                });
            }
        }

        last_z = extracted_sprite.transform.w_axis[2];
        if current_batch_colored {
            colored_end += QUAD_VERTEX_POSITIONS.len() as u32;
        } else {
            end += QUAD_VERTEX_POSITIONS.len() as u32;
        }
    }

    // if start != end, there is one last batch to process
    if start != end {
        if let Some(current_batch_handle) = current_batch_handle {
            commands.spawn_bundle((SpriteBatch {
                range: start..end,
                handle: current_batch_handle,
                colored: false,
                z: last_z,
            },));
        }
    } else if colored_start != colored_end {
        if let Some(current_batch_handle) = current_batch_handle {
            commands.spawn_bundle((SpriteBatch {
                range: colored_start..colored_end,
                handle: current_batch_handle,
                colored: true,
                z: last_z,
            },));
        }
    }

    sprite_meta
        .vertices
        .write_buffer(&render_device, &render_queue);
    sprite_meta
        .colored_vertices
        .write_buffer(&render_device, &render_queue);
}

#[derive(Default)]
pub struct ImageBindGroups {
    values: HashMap<Handle<Image>, BindGroup>,
}

#[allow(clippy::too_many_arguments)]
pub fn queue_sprites(
    draw_functions: Res<DrawFunctions<Transparent2d>>,
    render_device: Res<RenderDevice>,
    mut sprite_meta: ResMut<SpriteMeta>,
    view_uniforms: Res<ViewUniforms>,
    sprite_pipeline: Res<SpritePipeline>,
    mut pipelines: ResMut<SpecializedPipelines<SpritePipeline>>,
    mut pipeline_cache: ResMut<RenderPipelineCache>,
    mut image_bind_groups: ResMut<ImageBindGroups>,
    gpu_images: Res<RenderAssets<Image>>,
    mut sprite_batches: Query<(Entity, &SpriteBatch)>,
    mut views: Query<&mut RenderPhase<Transparent2d>>,
    events: Res<SpriteAssetEvents>,
) {
    // If an image has changed, the GpuImage has (probably) changed
    for event in &events.images {
        match event {
            AssetEvent::Created { .. } => None,
            AssetEvent::Modified { handle } => image_bind_groups.values.remove(handle),
            AssetEvent::Removed { handle } => image_bind_groups.values.remove(handle),
        };
    }

    if let Some(view_binding) = view_uniforms.uniforms.binding() {
        sprite_meta.view_bind_group = Some(render_device.create_bind_group(&BindGroupDescriptor {
            entries: &[BindGroupEntry {
                binding: 0,
                resource: view_binding,
            }],
            label: Some("sprite_view_bind_group"),
            layout: &sprite_pipeline.view_layout,
        }));
        let draw_sprite_function = draw_functions.read().get_id::<DrawSprite>().unwrap();
        let pipeline = pipelines.specialize(
            &mut pipeline_cache,
            &sprite_pipeline,
            SpritePipelineKey { colored: false },
        );
        let colored_pipeline = pipelines.specialize(
            &mut pipeline_cache,
            &sprite_pipeline,
            SpritePipelineKey { colored: true },
        );
        for mut transparent_phase in views.iter_mut() {
            for (entity, batch) in sprite_batches.iter_mut() {
                image_bind_groups
                    .values
                    .entry(batch.handle.clone_weak())
                    .or_insert_with(|| {
                        let gpu_image = gpu_images.get(&batch.handle).unwrap();
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
                            label: Some("sprite_material_bind_group"),
                            layout: &sprite_pipeline.material_layout,
                        })
                    });
                transparent_phase.add(Transparent2d {
                    draw_function: draw_sprite_function,
                    pipeline: if batch.colored {
                        colored_pipeline
                    } else {
                        pipeline
                    },
                    entity,
                    sort_key: FloatOrd(batch.z),
                });
            }
        }
    }
}

pub struct DrawSprite {
    params: SystemState<(
        SRes<SpriteMeta>,
        SRes<ImageBindGroups>,
        SRes<RenderPipelineCache>,
        SQuery<Read<ViewUniformOffset>>,
        SQuery<Read<SpriteBatch>>,
    )>,
}

impl DrawSprite {
    pub fn new(world: &mut World) -> Self {
        Self {
            params: SystemState::new(world),
        }
    }
}

impl Draw<Transparent2d> for DrawSprite {
    fn draw<'w>(
        &mut self,
        world: &'w World,
        pass: &mut TrackedRenderPass<'w>,
        view: Entity,
        item: &Transparent2d,
    ) {
        let (sprite_meta, image_bind_groups, pipelines, views, sprites) = self.params.get(world);
        let view_uniform = views.get(view).unwrap();
        let sprite_meta = sprite_meta.into_inner();
        let image_bind_groups = image_bind_groups.into_inner();
        let sprite_batch = sprites.get(item.entity).unwrap();
        if let Some(pipeline) = pipelines.into_inner().get(item.pipeline) {
            pass.set_render_pipeline(pipeline);
            if sprite_batch.colored {
                pass.set_vertex_buffer(0, sprite_meta.colored_vertices.buffer().unwrap().slice(..));
            } else {
                pass.set_vertex_buffer(0, sprite_meta.vertices.buffer().unwrap().slice(..));
            }
            pass.set_bind_group(
                0,
                sprite_meta.view_bind_group.as_ref().unwrap(),
                &[view_uniform.offset],
            );
            pass.set_bind_group(
                1,
                image_bind_groups.values.get(&sprite_batch.handle).unwrap(),
                &[],
            );

            pass.draw(sprite_batch.range.clone(), 0..1);
        }
    }
}
