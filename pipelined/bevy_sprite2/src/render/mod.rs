use crate::Sprite;
use bevy_asset::{Assets, Handle};
use bevy_ecs::{prelude::*, system::ParamState};
use bevy_math::{Mat4, Vec2, Vec3, Vec4Swizzles};
use bevy_render2::{
    camera::{CameraUniforms, Cameras},
    main_pass::{Draw, Drawable, RenderPhase, TrackedRenderPass},
    mesh::{shape::Quad, Indices, Mesh, VertexAttributeValues},
    pipeline::*,
    render_graph::{Node, ResourceSlots},
    render_resource::{
        BindGroupBuilder, BindGroupId, BufferUsage, BufferVec, RenderResourceBinding, SamplerId,
        TextureId,
    },
    renderer::{RenderContext, RenderResources},
    shader::{Shader, ShaderStage, ShaderStages},
    texture::{Texture, TextureFormat},
};
use bevy_transform::components::GlobalTransform;
use bevy_utils::HashMap;
use bytemuck::{Pod, Zeroable};

pub struct SpriteShaders {
    pipeline: PipelineId,
    pipeline_descriptor: RenderPipelineDescriptor,
}

// TODO: this pattern for initializing the shaders / pipeline isn't ideal. this should be handled by the asset system
impl FromWorld for SpriteShaders {
    fn from_world(world: &mut World) -> Self {
        let render_resources = world.get_resource::<RenderResources>().unwrap();
        let vertex_shader = Shader::from_glsl(ShaderStage::Vertex, include_str!("sprite.vert"))
            .get_spirv_shader(None)
            .unwrap();
        let fragment_shader = Shader::from_glsl(ShaderStage::Fragment, include_str!("sprite.frag"))
            .get_spirv_shader(None)
            .unwrap();

        let vertex_layout = vertex_shader.reflect_layout(true).unwrap();
        let fragment_layout = fragment_shader.reflect_layout(true).unwrap();

        let mut pipeline_layout =
            PipelineLayout::from_shader_layouts(&mut [vertex_layout, fragment_layout]);

        let vertex = render_resources.create_shader_module(&vertex_shader);
        let fragment = render_resources.create_shader_module(&fragment_shader);

        pipeline_layout.vertex_buffer_descriptors = vec![VertexBufferLayout {
            stride: 20,
            name: "Vertex".into(),
            step_mode: InputStepMode::Vertex,
            attributes: vec![
                VertexAttribute {
                    name: "Vertex_Position".into(),
                    format: VertexFormat::Float32x3,
                    offset: 0,
                    shader_location: 0,
                },
                VertexAttribute {
                    name: "Vertex_Uv".into(),
                    format: VertexFormat::Float32x2,
                    offset: 12,
                    shader_location: 1,
                },
            ],
        }];

        pipeline_layout.bind_groups[0].bindings[0].set_dynamic(true);

        let pipeline_descriptor = RenderPipelineDescriptor {
            depth_stencil: None,
            color_target_states: vec![ColorTargetState {
                format: TextureFormat::default(),
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
            primitive: PrimitiveState {
                topology: PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: FrontFace::Ccw,
                cull_mode: None,
                polygon_mode: PolygonMode::Fill,
                clamp_depth: false,
                conservative: false,
            },
            ..RenderPipelineDescriptor::new(
                ShaderStages {
                    vertex,
                    fragment: Some(fragment),
                },
                pipeline_layout,
            )
        };

        let pipeline = render_resources.create_render_pipeline(&pipeline_descriptor);

        SpriteShaders {
            pipeline,
            pipeline_descriptor,
        }
    }
}

struct ExtractedSprite {
    transform: Mat4,
    size: Vec2,
    texture: TextureId,
    sampler: SamplerId,
}

pub struct ExtractedSprites {
    sprites: Vec<ExtractedSprite>,
}

pub fn extract_sprites(
    mut commands: Commands,
    textures: Res<Assets<Texture>>,
    query: Query<(&Sprite, &GlobalTransform, &Handle<Texture>)>,
) {
    let mut extracted_sprites = Vec::new();
    for (sprite, transform, handle) in query.iter() {
        if let Some(texture) = textures.get(handle) {
            if let Some(gpu_data) = &texture.gpu_data {
                extracted_sprites.push(ExtractedSprite {
                    transform: transform.compute_matrix(),
                    size: sprite.size,
                    texture: gpu_data.texture_id,
                    sampler: gpu_data.sampler_id,
                })
            }
        }
    }

    commands.insert_resource(ExtractedSprites {
        sprites: extracted_sprites,
    });
}

#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
struct SpriteVertex {
    pub position: [f32; 3],
    pub uv: [f32; 2],
}

pub struct SpriteBuffers {
    vertices: BufferVec<SpriteVertex>,
    indices: BufferVec<u32>,
    quad: Mesh,
    camera_bind_group: Option<BindGroupId>,
    texture_bind_groups: Vec<BindGroupId>,
    dynamic_uniforms: Vec<u32>,
}

impl Default for SpriteBuffers {
    fn default() -> Self {
        Self {
            vertices: BufferVec::new(BufferUsage::VERTEX),
            indices: BufferVec::new(BufferUsage::INDEX),
            camera_bind_group: None,
            texture_bind_groups: Vec::new(),
            dynamic_uniforms: Vec::new(),
            quad: Quad {
                size: Vec2::new(1.0, 1.0),
                ..Default::default()
            }
            .into(),
        }
    }
}

pub fn prepare_sprites(
    render_resources: Res<RenderResources>,
    mut sprite_buffers: ResMut<SpriteBuffers>,
    extracted_sprites: Res<ExtractedSprites>,
) {
    // dont create buffers when there are no sprites
    if extracted_sprites.sprites.len() == 0 {
        return;
    }

    let quad_vertex_positions = if let VertexAttributeValues::Float32x3(vertex_positions) =
        sprite_buffers
            .quad
            .attribute(Mesh::ATTRIBUTE_POSITION)
            .unwrap()
            .clone()
    {
        vertex_positions
    } else {
        panic!("expected vec3");
    };

    let quad_vertex_uvs = if let VertexAttributeValues::Float32x2(vertex_uvs) = sprite_buffers
        .quad
        .attribute(Mesh::ATTRIBUTE_UV_0)
        .unwrap()
        .clone()
    {
        vertex_uvs
    } else {
        panic!("expected vec2");
    };

    let quad_indices = if let Indices::U32(indices) = sprite_buffers.quad.indices().unwrap() {
        indices.clone()
    } else {
        panic!("expected u32 indices");
    };

    sprite_buffers.vertices.reserve_and_clear(
        extracted_sprites.sprites.len() * quad_vertex_positions.len(),
        &render_resources,
    );
    sprite_buffers.indices.reserve_and_clear(
        extracted_sprites.sprites.len() * quad_indices.len(),
        &render_resources,
    );

    for (i, extracted_sprite) in extracted_sprites.sprites.iter().enumerate() {
        for (vertex_position, vertex_uv) in quad_vertex_positions.iter().zip(quad_vertex_uvs.iter())
        {
            let mut final_position =
                Vec3::from(*vertex_position) * extracted_sprite.size.extend(1.0);
            final_position = (extracted_sprite.transform * final_position.extend(1.0)).xyz();
            sprite_buffers.vertices.push(SpriteVertex {
                position: final_position.into(),
                uv: *vertex_uv,
            });
        }

        for index in quad_indices.iter() {
            sprite_buffers
                .indices
                .push((i * quad_vertex_positions.len()) as u32 + *index);
        }
    }

    sprite_buffers
        .vertices
        .write_to_staging_buffer(&render_resources);
    sprite_buffers
        .indices
        .write_to_staging_buffer(&render_resources);
}

pub fn queue_sprites(
    render_resources: Res<RenderResources>,
    mut sprite_buffers: ResMut<SpriteBuffers>,
    sprite_shaders: Res<SpriteShaders>,
    mut transparent_phase: ResMut<RenderPhase>,
    extracted_sprites: Res<ExtractedSprites>,
    extracted_cameras: Res<Cameras>,
    camera_uniforms: Query<&CameraUniforms>,
) {
    let camera_2d = if let Some(camera_2d) = extracted_cameras
        .entities
        .get(bevy_render2::camera::CAMERA_2D)
    {
        *camera_2d
    } else {
        return;
    };

    let layout = &sprite_shaders.pipeline_descriptor.layout;

    if let Ok(camera_uniforms) = camera_uniforms.get(camera_2d) {
        let camera_bind_group = BindGroupBuilder::default()
            .add_binding(0, camera_uniforms.view_proj.clone())
            .finish();

        // TODO: this will only create the bind group if it isn't already created. this is a bit nasty
        render_resources.create_bind_group(layout.bind_groups[0].id, &camera_bind_group);
        sprite_buffers.camera_bind_group = Some(camera_bind_group.id);
        sprite_buffers.dynamic_uniforms.clear();
        if let RenderResourceBinding::Buffer {
            dynamic_index: Some(index),
            ..
        } = camera_uniforms.view_proj
        {
            sprite_buffers.dynamic_uniforms.push(index);
        }
    }

    // TODO: free old bind groups? clear_unused_bind_groups() currently does this for us? Moving to RAII would also do this for us?
    sprite_buffers.texture_bind_groups.clear();
    let mut texture_bind_group_indices = HashMap::default();

    for (i, sprite) in extracted_sprites.sprites.iter().enumerate() {
        let bind_group_index = *texture_bind_group_indices
            .entry(sprite.texture)
            .or_insert_with(|| {
                let index = sprite_buffers.texture_bind_groups.len();
                let bind_group = BindGroupBuilder::default()
                    .add_binding(0, sprite.texture)
                    // NOTE: this currently reuses the same sampler across all sprites using the same texture
                    .add_binding(1, sprite.sampler)
                    .finish();
                render_resources.create_bind_group(layout.bind_groups[1].id, &bind_group);
                sprite_buffers.texture_bind_groups.push(bind_group.id);
                index
            });
        transparent_phase.add(Drawable {
            draw_function: 0, // TODO: get this from DrawFunctions
            draw_key: i,
            sort_key: bind_group_index,
        });
    }

    // TODO: this shouldn't happen here
    transparent_phase.sort();
}

pub struct SpriteNode;

impl Node for SpriteNode {
    fn update(
        &mut self,
        world: &World,
        render_context: &mut dyn RenderContext,
        _input: &ResourceSlots,
        _output: &mut ResourceSlots,
    ) {
        let sprite_buffers = world.get_resource::<SpriteBuffers>().unwrap();
        sprite_buffers
            .vertices
            .write_to_uniform_buffer(render_context);
        sprite_buffers
            .indices
            .write_to_uniform_buffer(render_context);
    }
}

pub struct DrawSprite {
    param_state: ParamState<(Res<'static, SpriteShaders>, Res<'static, SpriteBuffers>)>,
}

impl DrawSprite {
    pub fn new(world: &mut World) -> Self {
        Self {
            param_state: ParamState::new(world),
        }
    }
}

impl Draw for DrawSprite {
    fn draw(
        &mut self,
        world: &World,
        pass: &mut TrackedRenderPass,
        draw_key: usize,
        sort_key: usize,
    ) {
        const INDICES: usize = 6;
        let (sprite_shaders, sprite_buffers) = self.param_state.get(world);
        let layout = &sprite_shaders.pipeline_descriptor.layout;
        pass.set_pipeline(sprite_shaders.pipeline);
        pass.set_vertex_buffer(0, sprite_buffers.vertices.buffer().unwrap(), 0);
        pass.set_index_buffer(
            sprite_buffers.indices.buffer().unwrap(),
            0,
            IndexFormat::Uint32,
        );
        pass.set_bind_group(
            0,
            layout.bind_groups[0].id,
            sprite_buffers.camera_bind_group.unwrap(),
            Some(&sprite_buffers.dynamic_uniforms),
        );
        pass.set_bind_group(
            1,
            layout.bind_groups[1].id,
            sprite_buffers.texture_bind_groups[sort_key],
            None,
        );

        pass.draw_indexed(
            (draw_key * INDICES) as u32..(draw_key * INDICES + INDICES) as u32,
            0,
            0..1,
        );
    }
}
