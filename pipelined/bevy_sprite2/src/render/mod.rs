use crate::{
    texture_atlas::{TextureAtlas, TextureAtlasSprite},
    Rect, Sprite,
};
use bevy_asset::{Assets, Handle};
use bevy_core_pipeline::Transparent2d;
use bevy_ecs::{
    prelude::*,
    system::{lifetimeless::*, SystemState},
};
use bevy_math::{Mat4, Vec2, Vec3, Vec4Swizzles};
use bevy_render2::{
    mesh::{shape::Quad, Indices, Mesh, VertexAttributeValues},
    render_asset::RenderAssets,
    render_graph::{Node, NodeRunError, RenderGraphContext},
    render_phase::{Draw, DrawFunctions, RenderPhase, TrackedRenderPass},
    render_resource::*,
    renderer::{RenderContext, RenderDevice},
    shader::Shader,
    texture::{BevyDefault, Image},
    view::{ViewUniformOffset, ViewUniforms},
};
use bevy_transform::components::GlobalTransform;
use bevy_utils::HashMap;
use bytemuck::{Pod, Zeroable};

pub struct SpriteShaders {
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
            label: Some("sprite_view_layout"),
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
            label: Some("sprite_material_layout"),
        });

        let pipeline_layout = render_device.create_pipeline_layout(&PipelineLayoutDescriptor {
            label: Some("sprite_pipeline_layout"),
            push_constant_ranges: &[],
            bind_group_layouts: &[&view_layout, &material_layout],
        });

        let pipeline = render_device.create_render_pipeline(&RenderPipelineDescriptor {
            label: Some("sprite_pipeline"),
            depth_stencil: None,
            vertex: VertexState {
                buffers: &[VertexBufferLayout {
                    array_stride: 20,
                    step_mode: InputStepMode::Vertex,
                    attributes: &[
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
                topology: PrimitiveTopology::TriangleList,
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

pub struct ExtractedSprite {
    transform: Mat4,
    rect: Rect,
    handle: Handle<Image>,
    atlas_size: Option<Vec2>,
    vertex_index: usize,
}

pub fn extract_atlases(
    mut commands: Commands,
    texture_atlases: Res<Assets<TextureAtlas>>,
    atlas_query: Query<(
        Entity,
        &TextureAtlasSprite,
        &GlobalTransform,
        &Handle<TextureAtlas>,
    )>,
) {
    let mut sprites = Vec::new();
    for (entity, atlas_sprite, transform, texture_atlas_handle) in atlas_query.iter() {
        if let Some(texture_atlas) = texture_atlases.get(texture_atlas_handle) {
            let rect = texture_atlas.textures[atlas_sprite.index as usize];
            sprites.push((
                entity,
                (ExtractedSprite {
                    atlas_size: Some(texture_atlas.size),
                    transform: transform.compute_matrix(),
                    rect,
                    handle: texture_atlas.texture.clone_weak(),
                    vertex_index: 0,
                },),
            ));
        }
    }
    commands.insert_or_spawn_batch(sprites);
}

pub fn extract_sprites(
    mut commands: Commands,
    images: Res<Assets<Image>>,
    sprite_query: Query<(Entity, &Sprite, &GlobalTransform, &Handle<Image>)>,
) {
    let mut sprites = Vec::new();
    for (entity, sprite, transform, handle) in sprite_query.iter() {
        if let Some(image) = images.get(handle) {
            let size = image.texture_descriptor.size;

            sprites.push((
                entity,
                (ExtractedSprite {
                    atlas_size: None,
                    transform: transform.compute_matrix(),
                    rect: Rect {
                        min: Vec2::ZERO,
                        max: sprite
                            .custom_size
                            .unwrap_or_else(|| Vec2::new(size.width as f32, size.height as f32)),
                    },
                    handle: handle.clone_weak(),
                    vertex_index: 0,
                },),
            ));
        };
    }
    commands.insert_or_spawn_batch(sprites);
}

#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
struct SpriteVertex {
    pub position: [f32; 3],
    pub uv: [f32; 2],
}

pub struct SpriteMeta {
    vertices: BufferVec<SpriteVertex>,
    indices: BufferVec<u32>,
    quad: Mesh,
    view_bind_group: Option<BindGroup>,
}

impl Default for SpriteMeta {
    fn default() -> Self {
        Self {
            vertices: BufferVec::new(BufferUsage::VERTEX),
            indices: BufferVec::new(BufferUsage::INDEX),
            view_bind_group: None,
            quad: Quad {
                size: Vec2::new(1.0, 1.0),
                ..Default::default()
            }
            .into(),
        }
    }
}

pub fn prepare_sprites(
    render_device: Res<RenderDevice>,
    mut sprite_meta: ResMut<SpriteMeta>,
    mut extracted_sprites: Query<&mut ExtractedSprite>,
) {
    let extracted_sprite_len = extracted_sprites.iter_mut().len();
    // dont create buffers when there are no sprites
    if extracted_sprite_len == 0 {
        return;
    }

    let quad_vertex_positions = if let VertexAttributeValues::Float32x3(vertex_positions) =
        sprite_meta
            .quad
            .attribute(Mesh::ATTRIBUTE_POSITION)
            .unwrap()
            .clone()
    {
        vertex_positions
    } else {
        panic!("expected vec3");
    };

    let quad_indices = if let Indices::U32(indices) = sprite_meta.quad.indices().unwrap() {
        indices.clone()
    } else {
        panic!("expected u32 indices");
    };

    sprite_meta.vertices.reserve_and_clear(
        extracted_sprite_len * quad_vertex_positions.len(),
        &render_device,
    );
    sprite_meta
        .indices
        .reserve_and_clear(extracted_sprite_len * quad_indices.len(), &render_device);

    for (i, mut extracted_sprite) in extracted_sprites.iter_mut().enumerate() {
        let sprite_rect = extracted_sprite.rect;

        // Specify the corners of the sprite
        let bottom_left = Vec2::new(sprite_rect.min.x, sprite_rect.max.y);
        let top_left = sprite_rect.min;
        let top_right = Vec2::new(sprite_rect.max.x, sprite_rect.min.y);
        let bottom_right = sprite_rect.max;

        let atlas_positions: [Vec2; 4] = [bottom_left, top_left, top_right, bottom_right];

        extracted_sprite.vertex_index = i;
        for (index, vertex_position) in quad_vertex_positions.iter().enumerate() {
            let mut final_position =
                Vec3::from(*vertex_position) * extracted_sprite.rect.size().extend(1.0);
            final_position = (extracted_sprite.transform * final_position.extend(1.0)).xyz();
            sprite_meta.vertices.push(SpriteVertex {
                position: final_position.into(),
                uv: (atlas_positions[index]
                    / extracted_sprite.atlas_size.unwrap_or(sprite_rect.max))
                .into(),
            });
        }

        for index in quad_indices.iter() {
            sprite_meta
                .indices
                .push((i * quad_vertex_positions.len()) as u32 + *index);
        }
    }

    sprite_meta.vertices.write_to_staging_buffer(&render_device);
    sprite_meta.indices.write_to_staging_buffer(&render_device);
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
    sprite_shaders: Res<SpriteShaders>,
    mut image_bind_groups: ResMut<ImageBindGroups>,
    gpu_images: Res<RenderAssets<Image>>,
    mut extracted_sprites: Query<(Entity, &ExtractedSprite)>,
    mut views: Query<&mut RenderPhase<Transparent2d>>,
) {
    if let Some(view_binding) = view_uniforms.uniforms.binding() {
        sprite_meta.view_bind_group = Some(render_device.create_bind_group(&BindGroupDescriptor {
            entries: &[BindGroupEntry {
                binding: 0,
                resource: view_binding,
            }],
            label: Some("sprite_view_bind_group"),
            layout: &sprite_shaders.view_layout,
        }));
        let draw_sprite_function = draw_functions.read().get_id::<DrawSprite>().unwrap();
        for mut transparent_phase in views.iter_mut() {
            for (entity, sprite) in extracted_sprites.iter_mut() {
                image_bind_groups
                    .values
                    .entry(sprite.handle.clone_weak())
                    .or_insert_with(|| {
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
                            label: Some("sprite_material_bind_group"),
                            layout: &sprite_shaders.material_layout,
                        })
                    });
                transparent_phase.add(Transparent2d {
                    draw_function: draw_sprite_function,
                    entity,
                    sort_key: sprite.handle.clone_weak(),
                });
            }
        }
    }
}

// TODO: this logic can be moved to prepare_sprites once wgpu::Queue is exposed directly
pub struct SpriteNode;

impl Node for SpriteNode {
    fn run(
        &self,
        _graph: &mut RenderGraphContext,
        render_context: &mut RenderContext,
        world: &World,
    ) -> Result<(), NodeRunError> {
        let sprite_buffers = world.get_resource::<SpriteMeta>().unwrap();
        sprite_buffers
            .vertices
            .write_to_buffer(&mut render_context.command_encoder);
        sprite_buffers
            .indices
            .write_to_buffer(&mut render_context.command_encoder);
        Ok(())
    }
}

pub struct DrawSprite {
    params: SystemState<(
        SRes<SpriteShaders>,
        SRes<SpriteMeta>,
        SRes<ImageBindGroups>,
        SQuery<Read<ViewUniformOffset>>,
        SQuery<Read<ExtractedSprite>>,
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
        const INDICES: usize = 6;
        let (sprite_shaders, sprite_meta, image_bind_groups, views, sprites) =
            self.params.get(world);
        let view_uniform = views.get(view).unwrap();
        let sprite_meta = sprite_meta.into_inner();
        let image_bind_groups = image_bind_groups.into_inner();
        let extracted_sprite = sprites.get(item.entity).unwrap();
        pass.set_render_pipeline(&sprite_shaders.into_inner().pipeline);
        pass.set_vertex_buffer(0, sprite_meta.vertices.buffer().unwrap().slice(..));
        pass.set_index_buffer(
            sprite_meta.indices.buffer().unwrap().slice(..),
            0,
            IndexFormat::Uint32,
        );
        pass.set_bind_group(
            0,
            sprite_meta.view_bind_group.as_ref().unwrap(),
            &[view_uniform.offset],
        );
        pass.set_bind_group(
            1,
            image_bind_groups
                .values
                .get(&extracted_sprite.handle)
                .unwrap(),
            &[],
        );

        pass.draw_indexed(
            (extracted_sprite.vertex_index * INDICES) as u32
                ..(extracted_sprite.vertex_index * INDICES + INDICES) as u32,
            0,
            0..1,
        );
    }
}
