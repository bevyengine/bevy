mod light;
pub use light::*;

use bevy_asset::{Assets, Handle};
use bevy_ecs::{prelude::*, system::SystemState};
use bevy_math::Mat4;
use bevy_render2::{
    core_pipeline::Transparent3dPhase,
    mesh::Mesh,
    render_graph::{Node, NodeRunError, RenderGraphContext},
    render_phase::{Draw, DrawFunctions, Drawable, RenderPhase, TrackedRenderPass},
    render_resource::*,
    renderer::{RenderContext, RenderDevice},
    shader::Shader,
    texture::BevyDefault,
    view::{ViewMeta, ViewUniform, ViewUniformOffset},
};
use bevy_transform::components::GlobalTransform;
use bevy_utils::HashMap;
use crevice::std140::AsStd140;
use std::borrow::Cow;

use crate::{StandardMaterial, StandardMaterialUniformData};

pub struct PbrShaders {
    pipeline: RenderPipeline,
    vertex_shader_module: ShaderModule,
    view_layout: BindGroupLayout,
    material_layout: BindGroupLayout,
    mesh_layout: BindGroupLayout,
}

// TODO: this pattern for initializing the shaders / pipeline isn't ideal. this should be handled by the asset system
impl FromWorld for PbrShaders {
    fn from_world(world: &mut World) -> Self {
        let render_device = world.get_resource::<RenderDevice>().unwrap();
        let vertex_shader = Shader::from_glsl(ShaderStage::VERTEX, include_str!("pbr.vert"))
            .get_spirv_shader(None)
            .unwrap();
        let fragment_shader = Shader::from_glsl(ShaderStage::FRAGMENT, include_str!("pbr.frag"))
            .get_spirv_shader(None)
            .unwrap();

        let vertex_spirv = vertex_shader.get_spirv(None).unwrap();
        let fragment_spirv = fragment_shader.get_spirv(None).unwrap();

        let vertex_shader_module = render_device.create_shader_module(&ShaderModuleDescriptor {
            flags: ShaderFlags::default(),
            label: None,
            source: ShaderSource::SpirV(Cow::Borrowed(&vertex_spirv)),
        });
        let fragment_shader_module = render_device.create_shader_module(&ShaderModuleDescriptor {
            flags: ShaderFlags::default(),
            label: None,
            source: ShaderSource::SpirV(Cow::Borrowed(&fragment_spirv)),
        });

        // TODO: move this into ViewMeta?
        let view_layout = render_device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            entries: &[
                // View
                BindGroupLayoutEntry {
                    binding: 0,
                    visibility: ShaderStage::VERTEX | ShaderStage::FRAGMENT,
                    ty: BindingType::Buffer {
                        ty: BufferBindingType::Uniform,
                        has_dynamic_offset: true,
                        // TODO: verify this is correct
                        min_binding_size: BufferSize::new(ViewUniform::std140_size_static() as u64),
                    },
                    count: None,
                },
                // Lights
                BindGroupLayoutEntry {
                    binding: 1,
                    visibility: ShaderStage::FRAGMENT,
                    ty: BindingType::Buffer {
                        ty: BufferBindingType::Uniform,
                        has_dynamic_offset: true,
                        min_binding_size: BufferSize::new(GpuLights::std140_size_static() as u64),
                    },
                    count: None,
                },
                // Shadow Texture Array
                BindGroupLayoutEntry {
                    binding: 2,
                    visibility: ShaderStage::FRAGMENT,
                    ty: BindingType::Texture {
                        multisampled: false,
                        sample_type: TextureSampleType::Depth,
                        view_dimension: TextureViewDimension::D2Array,
                    },
                    count: None,
                },
                // Shadow Texture Array Sampler
                BindGroupLayoutEntry {
                    binding: 3,
                    visibility: ShaderStage::FRAGMENT,
                    ty: BindingType::Sampler {
                        comparison: true,
                        filtering: true,
                    },
                    count: None,
                },
            ],
            label: None,
        });

        let mesh_layout = render_device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            entries: &[BindGroupLayoutEntry {
                binding: 0,
                visibility: ShaderStage::VERTEX,
                ty: BindingType::Buffer {
                    ty: BufferBindingType::Uniform,
                    has_dynamic_offset: true,
                    min_binding_size: BufferSize::new(Mat4::std140_size_static() as u64),
                },
                count: None,
            }],
            label: None,
        });

        let material_layout = render_device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            entries: &[BindGroupLayoutEntry {
                binding: 0,
                visibility: ShaderStage::FRAGMENT,
                ty: BindingType::Buffer {
                    ty: BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: BufferSize::new(
                        StandardMaterialUniformData::std140_size_static() as u64,
                    ),
                },
                count: None,
            }],
            label: None,
        });

        let pipeline_layout = render_device.create_pipeline_layout(&PipelineLayoutDescriptor {
            label: None,
            push_constant_ranges: &[],
            bind_group_layouts: &[&view_layout, &mesh_layout, &material_layout],
        });

        let pipeline = render_device.create_render_pipeline(&RenderPipelineDescriptor {
            label: None,
            vertex: VertexState {
                buffers: &[VertexBufferLayout {
                    array_stride: 32,
                    step_mode: InputStepMode::Vertex,
                    attributes: &[
                        // Position (GOTCHA! Vertex_Position isn't first in the buffer due to how Mesh sorts attributes (alphabetically))
                        VertexAttribute {
                            format: VertexFormat::Float32x3,
                            offset: 12,
                            shader_location: 0,
                        },
                        // Normal
                        VertexAttribute {
                            format: VertexFormat::Float32x3,
                            offset: 0,
                            shader_location: 1,
                        },
                        // Uv
                        VertexAttribute {
                            format: VertexFormat::Float32x2,
                            offset: 24,
                            shader_location: 2,
                        },
                    ],
                }],
                module: &&vertex_shader_module,
                entry_point: "main",
            },
            fragment: Some(FragmentState {
                module: &&fragment_shader_module,
                entry_point: "main",
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
            depth_stencil: Some(DepthStencilState {
                format: TextureFormat::Depth32Float,
                depth_write_enabled: true,
                depth_compare: CompareFunction::Less,
                stencil: StencilState {
                    front: StencilFaceState::IGNORE,
                    back: StencilFaceState::IGNORE,
                    read_mask: 0,
                    write_mask: 0,
                },
                bias: DepthBiasState {
                    constant: 0,
                    slope_scale: 0.0,
                    clamp: 0.0,
                },
            }),
            layout: Some(&pipeline_layout),
            multisample: MultisampleState::default(),
            primitive: PrimitiveState {
                topology: PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: FrontFace::Ccw,
                cull_mode: Some(Face::Back),
                polygon_mode: PolygonMode::Fill,
                clamp_depth: false,
                conservative: false,
            },
        });

        PbrShaders {
            pipeline,
            view_layout,
            material_layout,
            mesh_layout,
            vertex_shader_module,
        }
    }
}

struct ExtractedMesh {
    transform: Mat4,
    vertex_buffer: Buffer,
    index_info: Option<IndexInfo>,
    transform_binding_offset: u32,
    material_buffer: Buffer,
}

struct IndexInfo {
    buffer: Buffer,
    count: u32,
}

pub struct ExtractedMeshes {
    meshes: Vec<ExtractedMesh>,
}

pub fn extract_meshes(
    mut commands: Commands,
    meshes: Res<Assets<Mesh>>,
    materials: Res<Assets<StandardMaterial>>,
    query: Query<(&GlobalTransform, &Handle<Mesh>, &Handle<StandardMaterial>)>,
) {
    let mut extracted_meshes = Vec::new();
    for (transform, mesh_handle, material_handle) in query.iter() {
        if let Some(mesh) = meshes.get(mesh_handle) {
            if let Some(mesh_gpu_data) = &mesh.gpu_data() {
                if let Some(material) = materials.get(material_handle) {
                    if let Some(material_gpu_data) = &material.gpu_data() {
                        extracted_meshes.push(ExtractedMesh {
                            transform: transform.compute_matrix(),
                            vertex_buffer: mesh_gpu_data.vertex_buffer.clone(),
                            index_info: mesh_gpu_data.index_buffer.as_ref().map(|i| IndexInfo {
                                buffer: i.clone(),
                                count: mesh.indices().unwrap().len() as u32,
                            }),
                            transform_binding_offset: 0,
                            material_buffer: material_gpu_data.buffer.clone(),
                        });
                    }
                }
            }
        }
    }

    commands.insert_resource(ExtractedMeshes {
        meshes: extracted_meshes,
    });
}

#[derive(Default)]
pub struct MeshMeta {
    transform_uniforms: DynamicUniformVec<Mat4>,
    mesh_transform_bind_group: Option<BindGroup>,
}

pub fn prepare_meshes(
    render_device: Res<RenderDevice>,
    mut mesh_meta: ResMut<MeshMeta>,
    mut extracted_meshes: ResMut<ExtractedMeshes>,
) {
    mesh_meta
        .transform_uniforms
        .reserve_and_clear(extracted_meshes.meshes.len(), &render_device);
    for extracted_mesh in extracted_meshes.meshes.iter_mut() {
        extracted_mesh.transform_binding_offset =
            mesh_meta.transform_uniforms.push(extracted_mesh.transform);
    }

    mesh_meta
        .transform_uniforms
        .write_to_staging_buffer(&render_device);
}
#[derive(Default)]
pub struct MaterialMeta {
    material_bind_groups: Vec<BindGroup>,
    material_bind_group_indices: HashMap<BufferId, usize>,
}

pub struct MeshViewBindGroups {
    view: BindGroup,
}

pub fn queue_meshes(
    mut commands: Commands,
    draw_functions: Res<DrawFunctions>,
    render_device: Res<RenderDevice>,
    pbr_shaders: Res<PbrShaders>,
    shadow_shaders: Res<ShadowShaders>,
    mesh_meta: ResMut<MeshMeta>,
    material_meta: ResMut<MaterialMeta>,
    mut light_meta: ResMut<LightMeta>,
    view_meta: Res<ViewMeta>,
    mut extracted_meshes: ResMut<ExtractedMeshes>,
    mut views: Query<(Entity, &ViewLights, &mut RenderPhase<Transparent3dPhase>)>,
    mut view_light_shadow_phases: Query<&mut RenderPhase<ShadowPhase>>,
) {
    let mesh_meta = mesh_meta.into_inner();
    let material_meta = material_meta.into_inner();

    light_meta.shadow_view_bind_group.get_or_insert_with(|| {
        render_device.create_bind_group(&BindGroupDescriptor {
            entries: &[BindGroupEntry {
                binding: 0,
                resource: view_meta.uniforms.binding(),
            }],
            label: None,
            layout: &shadow_shaders.view_layout,
        })
    });
    if extracted_meshes.meshes.is_empty() {
        return;
    }

    let transform_uniforms = &mesh_meta.transform_uniforms;
    mesh_meta.mesh_transform_bind_group.get_or_insert_with(|| {
        render_device.create_bind_group(&BindGroupDescriptor {
            entries: &[BindGroupEntry {
                binding: 0,
                resource: transform_uniforms.binding(),
            }],
            label: None,
            layout: &pbr_shaders.mesh_layout,
        })
    });
    for (entity, view_lights, mut transparent_phase) in views.iter_mut() {
        // TODO: cache this?
        let view_bind_group = render_device.create_bind_group(&BindGroupDescriptor {
            entries: &[
                BindGroupEntry {
                    binding: 0,
                    resource: view_meta.uniforms.binding(),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: light_meta.view_gpu_lights.binding(),
                },
                BindGroupEntry {
                    binding: 2,
                    resource: BindingResource::TextureView(&view_lights.light_depth_texture_view),
                },
                BindGroupEntry {
                    binding: 3,
                    resource: BindingResource::Sampler(&shadow_shaders.light_sampler),
                },
            ],
            label: None,
            layout: &pbr_shaders.view_layout,
        });

        commands.entity(entity).insert(MeshViewBindGroups {
            view: view_bind_group,
        });

        // TODO: free old bind groups after a few frames without use?

        let draw_pbr = draw_functions.read().get_id::<DrawPbr>().unwrap();
        let material_bind_groups = &mut material_meta.material_bind_groups;
        for (i, mesh) in extracted_meshes.meshes.iter_mut().enumerate() {
            let material_bind_group_index = *material_meta
                .material_bind_group_indices
                .entry(mesh.material_buffer.id())
                .or_insert_with(|| {
                    let index = material_bind_groups.len();
                    let material_bind_group =
                        render_device.create_bind_group(&BindGroupDescriptor {
                            entries: &[BindGroupEntry {
                                binding: 0,
                                resource: mesh.material_buffer.as_entire_binding(),
                            }],
                            label: None,
                            layout: &pbr_shaders.material_layout,
                        });
                    material_bind_groups.push(material_bind_group);
                    index
                });

            // TODO: currently there is only "transparent phase". this should pick transparent vs opaque according to the mesh material
            transparent_phase.add(Drawable {
                draw_function: draw_pbr,
                draw_key: i,
                sort_key: material_bind_group_index, // TODO: sort back-to-front, sorting by material for now
            });
        }

        // ultimately lights should check meshes for relevancy (ex: light views can "see" different meshes than the main view can)
        let draw_shadow_mesh = draw_functions.read().get_id::<DrawShadowMesh>().unwrap();
        for view_light_entity in view_lights.lights.iter().copied() {
            let mut shadow_phase = view_light_shadow_phases.get_mut(view_light_entity).unwrap();
            // TODO: this should only queue up meshes that are actually visible by each "light view"
            for i in 0..extracted_meshes.meshes.len() {
                shadow_phase.add(Drawable {
                    draw_function: draw_shadow_mesh,
                    draw_key: i,
                    sort_key: 0, // TODO: sort back-to-front
                })
            }
        }
    }
}

// TODO: this logic can be moved to prepare_meshes once wgpu::Queue is exposed directly
pub struct PbrNode;

impl Node for PbrNode {
    fn run(
        &self,
        _graph: &mut RenderGraphContext,
        render_context: &mut RenderContext,
        world: &World,
    ) -> Result<(), NodeRunError> {
        let mesh_meta = world.get_resource::<MeshMeta>().unwrap();
        let light_meta = world.get_resource::<LightMeta>().unwrap();
        mesh_meta
            .transform_uniforms
            .write_to_uniform_buffer(&mut render_context.command_encoder);
        light_meta
            .view_gpu_lights
            .write_to_uniform_buffer(&mut render_context.command_encoder);
        Ok(())
    }
}

type DrawPbrParams<'s, 'w> = (
    Res<'w, PbrShaders>,
    Res<'w, MaterialMeta>,
    Res<'w, MeshMeta>,
    Res<'w, ExtractedMeshes>,
    Query<
        'w,
        's,
        (
            &'w ViewUniformOffset,
            &'w ViewLights,
            &'w MeshViewBindGroups,
        ),
    >,
);

pub struct DrawPbr {
    params: SystemState<DrawPbrParams<'static, 'static>>,
}

impl DrawPbr {
    pub fn new(world: &mut World) -> Self {
        Self {
            params: SystemState::new(world),
        }
    }
}

impl Draw for DrawPbr {
    fn draw<'w, 's>(
        &'s mut self,
        world: &'w World,
        pass: &mut TrackedRenderPass<'w>,
        view: Entity,
        draw_key: usize,
        sort_key: usize,
    ) {
        let (pbr_shaders, material_meta, mesh_meta, extracted_meshes, views) =
            self.params.get(world);
        let (view_uniforms, view_lights, mesh_view_bind_groups) = views.get(view).unwrap();
        let extracted_mesh = &extracted_meshes.into_inner().meshes[draw_key];
        pass.set_render_pipeline(&pbr_shaders.into_inner().pipeline);
        pass.set_bind_group(
            0,
            &mesh_view_bind_groups.view,
            &[view_uniforms.offset, view_lights.gpu_light_binding_index],
        );
        pass.set_bind_group(
            1,
            mesh_meta
                .into_inner()
                .mesh_transform_bind_group
                .as_ref()
                .unwrap(),
            &[extracted_mesh.transform_binding_offset],
        );
        pass.set_bind_group(
            2,
            &material_meta.into_inner().material_bind_groups[sort_key],
            &[],
        );
        pass.set_vertex_buffer(0, extracted_mesh.vertex_buffer.slice(..));
        if let Some(index_info) = &extracted_mesh.index_info {
            pass.set_index_buffer(index_info.buffer.slice(..), 0, IndexFormat::Uint32);
            pass.draw_indexed(0..index_info.count, 0, 0..1);
        } else {
            panic!("non-indexed drawing not supported yet")
        }
    }
}
