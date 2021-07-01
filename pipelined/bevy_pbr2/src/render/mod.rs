mod light;
pub use light::*;

use bevy_asset::{Assets, Handle};
use bevy_ecs::{prelude::*, system::SystemState};
use bevy_math::Mat4;
use bevy_render2::{
    core_pipeline::Transparent3dPhase,
    mesh::Mesh,
    render_asset::RenderAssets,
    render_graph::{Node, NodeRunError, RenderGraphContext},
    render_phase::{Draw, DrawFunctions, Drawable, RenderPhase, TrackedRenderPass},
    render_resource::*,
    renderer::{RenderContext, RenderDevice, RenderQueue},
    shader::Shader,
    texture::{BevyDefault, GpuImage, Image, TextureFormatPixelInfo},
    view::{ExtractedView, ViewMeta, ViewUniformOffset},
};
use bevy_transform::components::GlobalTransform;
use bevy_utils::slab::{FrameSlabMap, FrameSlabMapKey};
use crevice::std140::AsStd140;
use wgpu::{
    Extent3d, ImageCopyTexture, ImageDataLayout, Origin3d, TextureDimension, TextureFormat,
    TextureViewDescriptor,
};

use crate::{StandardMaterial, StandardMaterialUniformData};

pub struct PbrShaders {
    pipeline: RenderPipeline,
    shader_module: ShaderModule,
    view_layout: BindGroupLayout,
    material_layout: BindGroupLayout,
    mesh_layout: BindGroupLayout,
    // This dummy white texture is to be used in place of optional StandardMaterial textures
    dummy_white_gpu_image: GpuImage,
}

// TODO: this pattern for initializing the shaders / pipeline isn't ideal. this should be handled by the asset system
impl FromWorld for PbrShaders {
    fn from_world(world: &mut World) -> Self {
        let render_device = world.get_resource::<RenderDevice>().unwrap();
        let shader = Shader::from_wgsl(include_str!("pbr.wgsl"));
        let shader_module = render_device.create_shader_module(&shader);

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
                        // TODO: change this to ViewUniform::std140_size_static once crevice fixes this!
                        // Context: https://github.com/LPGhatguy/crevice/issues/29
                        min_binding_size: BufferSize::new(80),
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
                        // TODO: change this to ViewUniform::std140_size_static once crevice fixes this!
                        // Context: https://github.com/LPGhatguy/crevice/issues/29
                        min_binding_size: BufferSize::new(1264),
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
            entries: &[
                BindGroupLayoutEntry {
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
                },
                // Base Color Texture
                BindGroupLayoutEntry {
                    binding: 1,
                    visibility: ShaderStage::FRAGMENT,
                    ty: BindingType::Texture {
                        multisampled: false,
                        sample_type: TextureSampleType::Float { filterable: true },
                        view_dimension: TextureViewDimension::D2,
                    },
                    count: None,
                },
                // Base Color Texture Sampler
                BindGroupLayoutEntry {
                    binding: 2,
                    visibility: ShaderStage::FRAGMENT,
                    ty: BindingType::Sampler {
                        comparison: false,
                        filtering: true,
                    },
                    count: None,
                },
                // Emissive Texture
                BindGroupLayoutEntry {
                    binding: 3,
                    visibility: ShaderStage::FRAGMENT,
                    ty: BindingType::Texture {
                        multisampled: false,
                        sample_type: TextureSampleType::Float { filterable: true },
                        view_dimension: TextureViewDimension::D2,
                    },
                    count: None,
                },
                // Emissive Texture Sampler
                BindGroupLayoutEntry {
                    binding: 4,
                    visibility: ShaderStage::FRAGMENT,
                    ty: BindingType::Sampler {
                        comparison: false,
                        filtering: true,
                    },
                    count: None,
                },
                // Metallic Roughness Texture
                BindGroupLayoutEntry {
                    binding: 5,
                    visibility: ShaderStage::FRAGMENT,
                    ty: BindingType::Texture {
                        multisampled: false,
                        sample_type: TextureSampleType::Float { filterable: true },
                        view_dimension: TextureViewDimension::D2,
                    },
                    count: None,
                },
                // Metallic Roughness Texture Sampler
                BindGroupLayoutEntry {
                    binding: 6,
                    visibility: ShaderStage::FRAGMENT,
                    ty: BindingType::Sampler {
                        comparison: false,
                        filtering: true,
                    },
                    count: None,
                },
                // Occlusion Texture
                BindGroupLayoutEntry {
                    binding: 7,
                    visibility: ShaderStage::FRAGMENT,
                    ty: BindingType::Texture {
                        multisampled: false,
                        sample_type: TextureSampleType::Float { filterable: true },
                        view_dimension: TextureViewDimension::D2,
                    },
                    count: None,
                },
                // Occlusion Texture Sampler
                BindGroupLayoutEntry {
                    binding: 8,
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

        // A 1x1x1 'all 1.0' texture to use as a dummy texture to use in place of optional StandardMaterial textures
        let dummy_white_gpu_image = {
            let image = Image::new_fill(
                Extent3d::default(),
                TextureDimension::D2,
                &[255u8; 4],
                TextureFormat::bevy_default(),
            );
            let texture = render_device.create_texture(&image.texture_descriptor);
            let sampler = render_device.create_sampler(&image.sampler_descriptor);

            let format_size = image.texture_descriptor.format.pixel_size();
            let render_queue = world.get_resource_mut::<RenderQueue>().unwrap();
            render_queue.write_texture(
                ImageCopyTexture {
                    texture: &texture,
                    mip_level: 0,
                    origin: Origin3d::ZERO,
                },
                &image.data,
                ImageDataLayout {
                    offset: 0,
                    bytes_per_row: Some(
                        std::num::NonZeroU32::new(
                            image.texture_descriptor.size.width * format_size as u32,
                        )
                        .unwrap(),
                    ),
                    rows_per_image: None,
                },
                image.texture_descriptor.size,
            );

            let texture_view = texture.create_view(&TextureViewDescriptor::default());
            GpuImage {
                texture,
                texture_view,
                sampler,
            }
        };
        PbrShaders {
            pipeline,
            view_layout,
            material_layout,
            mesh_layout,
            shader_module,
            dummy_white_gpu_image,
        }
    }
}

struct ExtractedMesh {
    transform: Mat4,
    mesh: Handle<Mesh>,
    transform_binding_offset: u32,
    material_handle: Handle<StandardMaterial>,
}

pub struct ExtractedMeshes {
    meshes: Vec<ExtractedMesh>,
}

pub fn extract_meshes(
    mut commands: Commands,
    meshes: Res<Assets<Mesh>>,
    materials: Res<Assets<StandardMaterial>>,
    images: Res<Assets<Image>>,
    query: Query<(&GlobalTransform, &Handle<Mesh>, &Handle<StandardMaterial>)>,
) {
    let mut extracted_meshes = Vec::new();
    for (transform, mesh_handle, material_handle) in query.iter() {
        if !meshes.contains(mesh_handle) {
            continue;
        }

        if let Some(material) = materials.get(material_handle) {
            if let Some(ref image) = material.base_color_texture {
                if !images.contains(image) {
                    continue;
                }
            }

            if let Some(ref image) = material.emissive_texture {
                if !images.contains(image) {
                    continue;
                }
            }
            if let Some(ref image) = material.metallic_roughness_texture {
                if !images.contains(image) {
                    continue;
                }
            }
            if let Some(ref image) = material.occlusion_texture {
                if !images.contains(image) {
                    continue;
                }
            }
            extracted_meshes.push(ExtractedMesh {
                transform: transform.compute_matrix(),
                mesh: mesh_handle.clone_weak(),
                transform_binding_offset: 0,
                material_handle: material_handle.clone_weak(),
            });
        } else {
            continue;
        }
    }

    commands.insert_resource(ExtractedMeshes {
        meshes: extracted_meshes,
    });
}

struct MeshDrawInfo {
    // TODO: compare cost of doing this vs cloning the BindGroup?
    material_bind_group_key: FrameSlabMapKey<BufferId, BindGroup>,
}

#[derive(Default)]
pub struct MeshMeta {
    transform_uniforms: DynamicUniformVec<Mat4>,
    material_bind_groups: FrameSlabMap<BufferId, BindGroup>,
    mesh_transform_bind_group: FrameSlabMap<BufferId, BindGroup>,
    mesh_transform_bind_group_key: Option<FrameSlabMapKey<BufferId, BindGroup>>,
    mesh_draw_info: Vec<MeshDrawInfo>,
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

pub struct MeshViewBindGroups {
    view: BindGroup,
}

fn image_handle_to_view_sampler<'a>(
    pbr_shaders: &'a PbrShaders,
    gpu_images: &'a RenderAssets<Image>,
    image_option: &Option<Handle<Image>>,
) -> (&'a TextureView, &'a Sampler) {
    image_option.as_ref().map_or(
        (
            &pbr_shaders.dummy_white_gpu_image.texture_view,
            &pbr_shaders.dummy_white_gpu_image.sampler,
        ),
        |image_handle| {
            let gpu_image = gpu_images
                .get(image_handle)
                .expect("only materials with valid textures should be drawn");
            (&gpu_image.texture_view, &gpu_image.sampler)
        },
    )
}

pub fn queue_meshes(
    mut commands: Commands,
    draw_functions: Res<DrawFunctions>,
    render_device: Res<RenderDevice>,
    pbr_shaders: Res<PbrShaders>,
    shadow_shaders: Res<ShadowShaders>,
    mesh_meta: ResMut<MeshMeta>,
    mut light_meta: ResMut<LightMeta>,
    view_meta: Res<ViewMeta>,
    mut extracted_meshes: ResMut<ExtractedMeshes>,
    gpu_images: Res<RenderAssets<Image>>,
    render_materials: Res<RenderAssets<StandardMaterial>>,
    mut views: Query<(
        Entity,
        &ExtractedView,
        &ViewLights,
        &mut RenderPhase<Transparent3dPhase>,
    )>,
    mut view_light_shadow_phases: Query<&mut RenderPhase<ShadowPhase>>,
) {
    let mesh_meta = mesh_meta.into_inner();

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
    mesh_meta.mesh_transform_bind_group.next_frame();
    mesh_meta.mesh_transform_bind_group_key =
        Some(mesh_meta.mesh_transform_bind_group.get_or_insert_with(
            transform_uniforms.uniform_buffer().unwrap().id(),
            || {
                render_device.create_bind_group(&BindGroupDescriptor {
                    entries: &[BindGroupEntry {
                        binding: 0,
                        resource: transform_uniforms.binding(),
                    }],
                    label: None,
                    layout: &pbr_shaders.mesh_layout,
                })
            },
        ));
    for (entity, view, view_lights, mut transparent_phase) in views.iter_mut() {
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

        let draw_pbr = draw_functions.read().get_id::<DrawPbr>().unwrap();
        mesh_meta.mesh_draw_info.clear();
        mesh_meta.material_bind_groups.next_frame();

        let view_matrix = view.transform.compute_matrix();
        let view_row_2 = view_matrix.row(2);
        for (i, mesh) in extracted_meshes.meshes.iter_mut().enumerate() {
            let gpu_material = &render_materials
                .get(&mesh.material_handle)
                .expect("Failed to get StandardMaterial PreparedAsset");
            let material_bind_group_key =
                mesh_meta
                    .material_bind_groups
                    .get_or_insert_with(gpu_material.buffer.id(), || {
                        let (base_color_texture_view, base_color_sampler) =
                            image_handle_to_view_sampler(
                                &pbr_shaders,
                                &gpu_images,
                                &gpu_material.base_color_texture,
                            );

                        let (emissive_texture_view, emissive_sampler) =
                            image_handle_to_view_sampler(
                                &pbr_shaders,
                                &gpu_images,
                                &gpu_material.emissive_texture,
                            );

                        let (metallic_roughness_texture_view, metallic_roughness_sampler) =
                            image_handle_to_view_sampler(
                                &pbr_shaders,
                                &gpu_images,
                                &gpu_material.metallic_roughness_texture,
                            );
                        let (occlusion_texture_view, occlusion_sampler) =
                            image_handle_to_view_sampler(
                                &pbr_shaders,
                                &gpu_images,
                                &gpu_material.occlusion_texture,
                            );
                        render_device.create_bind_group(&BindGroupDescriptor {
                            entries: &[
                                BindGroupEntry {
                                    binding: 0,
                                    resource: gpu_material.buffer.as_entire_binding(),
                                },
                                BindGroupEntry {
                                    binding: 1,
                                    resource: BindingResource::TextureView(
                                        &base_color_texture_view,
                                    ),
                                },
                                BindGroupEntry {
                                    binding: 2,
                                    resource: BindingResource::Sampler(&base_color_sampler),
                                },
                                BindGroupEntry {
                                    binding: 3,
                                    resource: BindingResource::TextureView(&emissive_texture_view),
                                },
                                BindGroupEntry {
                                    binding: 4,
                                    resource: BindingResource::Sampler(&emissive_sampler),
                                },
                                BindGroupEntry {
                                    binding: 5,
                                    resource: BindingResource::TextureView(
                                        &metallic_roughness_texture_view,
                                    ),
                                },
                                BindGroupEntry {
                                    binding: 6,
                                    resource: BindingResource::Sampler(&metallic_roughness_sampler),
                                },
                                BindGroupEntry {
                                    binding: 7,
                                    resource: BindingResource::TextureView(&occlusion_texture_view),
                                },
                                BindGroupEntry {
                                    binding: 8,
                                    resource: BindingResource::Sampler(&occlusion_sampler),
                                },
                            ],
                            label: None,
                            layout: &pbr_shaders.material_layout,
                        })
                    });

            mesh_meta.mesh_draw_info.push(MeshDrawInfo {
                material_bind_group_key,
            });

            // NOTE: row 2 of the view matrix dotted with column 3 of the model matrix
            //       gives the z component of translation of the mesh in view space
            let mesh_z = view_row_2.dot(mesh.transform.col(3));
            // FIXME: Switch from usize to u64 for portability and use sort key encoding
            //        similar to https://realtimecollisiondetection.net/blog/?p=86 as appropriate
            // FIXME: What is the best way to map from view space z to a number of bits of unsigned integer?
            let sort_key = (((mesh_z * 1000.0) as usize) << 10)
                | (material_bind_group_key.index() & ((1 << 10) - 1));
            // TODO: currently there is only "transparent phase". this should pick transparent vs opaque according to the mesh material
            transparent_phase.add(Drawable {
                draw_function: draw_pbr,
                draw_key: i,
                sort_key,
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
    Res<'w, MeshMeta>,
    Res<'w, ExtractedMeshes>,
    Res<'w, RenderAssets<Mesh>>,
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
        _sort_key: usize,
    ) {
        let (pbr_shaders, mesh_meta, extracted_meshes, meshes, views) = self.params.get(world);
        let (view_uniforms, view_lights, mesh_view_bind_groups) = views.get(view).unwrap();
        let extracted_mesh = &extracted_meshes.into_inner().meshes[draw_key];
        let mesh_meta = mesh_meta.into_inner();
        pass.set_render_pipeline(&pbr_shaders.into_inner().pipeline);
        pass.set_bind_group(
            0,
            &mesh_view_bind_groups.view,
            &[view_uniforms.offset, view_lights.gpu_light_binding_index],
        );
        pass.set_bind_group(
            1,
            mesh_meta
                .mesh_transform_bind_group
                .get_value(mesh_meta.mesh_transform_bind_group_key.unwrap())
                .unwrap(),
            &[extracted_mesh.transform_binding_offset],
        );
        let mesh_draw_info = &mesh_meta.mesh_draw_info[draw_key];
        pass.set_bind_group(
            2,
            // &mesh_meta.material_bind_groups[sort_key & ((1 << 10) - 1)],
            &mesh_meta.material_bind_groups[mesh_draw_info.material_bind_group_key],
            &[],
        );

        let gpu_mesh = meshes.into_inner().get(&extracted_mesh.mesh).unwrap();
        pass.set_vertex_buffer(0, gpu_mesh.vertex_buffer.slice(..));
        if let Some(index_info) = &gpu_mesh.index_info {
            pass.set_index_buffer(index_info.buffer.slice(..), 0, IndexFormat::Uint32);
            pass.draw_indexed(0..index_info.count, 0, 0..1);
        } else {
            panic!("non-indexed drawing not supported yet")
        }
    }
}
