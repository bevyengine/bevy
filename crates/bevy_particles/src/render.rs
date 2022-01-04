use crate::{
    material::{ParticleMaterial, ParticleMaterialUniformData},
    particles::Particles,
};
use bevy_app::prelude::*;
use bevy_asset::{Assets, Handle, HandleUntyped};
use bevy_core::FloatOrd;
use bevy_core_pipeline::{Transparent2d, Transparent3d};
use bevy_ecs::{
    prelude::*,
    system::{lifetimeless::*, SystemState},
};
use bevy_math::*;
use bevy_reflect::TypeUuid;
use bevy_render::{
    primitives::Aabb,
    render_asset::RenderAssets,
    render_phase::{Draw, DrawFunctions, RenderPhase, TrackedRenderPass},
    render_resource::{std140::AsStd140, *},
    renderer::{RenderDevice, RenderQueue},
    texture::{BevyDefault, GpuImage, Image, TextureFormatPixelInfo},
    view::{ComputedVisibility, ViewUniform, ViewUniformOffset, ViewUniforms, VisibilitySystems},
    RenderApp, RenderStage, RenderWorld,
};
use bevy_tasks::ComputeTaskPool;
use bytemuck::Pod;
use std::{collections::HashMap, num::NonZeroU64, ops::Range};
use wgpu::{
    BindGroupDescriptor, BindGroupEntry, BindGroupLayoutDescriptor, BindingResource, BufferBinding,
    FrontFace, ImageCopyTexture, ImageDataLayout, MultisampleState, Origin3d, PolygonMode,
    PrimitiveState, PrimitiveTopology,
};

pub const PARTICLE_SHADER_HANDLE: HandleUntyped =
    HandleUntyped::weak_from_u64(Shader::TYPE_UUID, 3032357527543835453);

pub(crate) struct ParticleRenderPlugin;

impl Plugin for ParticleRenderPlugin {
    fn build(&self, app: &mut App) {
        let particle_shader = Shader::from_wgsl(include_str!("particle.wgsl"));
        app.add_system_to_stage(
            CoreStage::PostUpdate,
            compute_particles_aabb.label(VisibilitySystems::CalculateBounds),
        );
        app.world
            .get_resource_mut::<Assets<Shader>>()
            .unwrap()
            .set_untracked(PARTICLE_SHADER_HANDLE, particle_shader);
        let render_app = app.get_sub_app_mut(RenderApp).unwrap();
        render_app
            .add_system_to_stage(RenderStage::Extract, extract_particles)
            .add_system_to_stage(RenderStage::Prepare, prepare_particles)
            .add_system_to_stage(RenderStage::Queue, queue_particles)
            .add_system_to_stage(RenderStage::Queue, queue_particle_bind_groups)
            .init_resource::<ParticlePipeline>()
            .init_resource::<ParticleMeta>()
            .init_resource::<ExtractedParticles>()
            .init_resource::<MaterialBindGroups>()
            .init_resource::<SpecializedPipelines<ParticlePipeline>>();

        let draw_2d = DrawParticle::new(&mut render_app.world);
        render_app
            .world
            .get_resource::<DrawFunctions<Transparent2d>>()
            .unwrap()
            .write()
            .add(draw_2d);

        let draw_3d = DrawParticle::new(&mut render_app.world);
        render_app
            .world
            .get_resource::<DrawFunctions<Transparent3d>>()
            .unwrap()
            .write()
            .add(draw_3d);
    }
}

struct ParticlePipeline {
    view_layout: BindGroupLayout,
    particle_layout: BindGroupLayout,
    material_layout: BindGroupLayout,

    // This dummy white texture is to be used in place of optional StandardMaterial textures
    dummy_white_gpu_image: GpuImage,
}

impl FromWorld for ParticlePipeline {
    fn from_world(world: &mut World) -> Self {
        let render_device = world.get_resource::<RenderDevice>().unwrap();

        let view_layout = render_device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            entries: &[BindGroupLayoutEntry {
                binding: 0,
                visibility: ShaderStages::VERTEX | ShaderStages::FRAGMENT,
                ty: BindingType::Buffer {
                    ty: BufferBindingType::Uniform,
                    has_dynamic_offset: true,
                    min_binding_size: BufferSize::new(std::mem::size_of::<ViewUniform>() as u64),
                },
                count: None,
            }],
            label: None,
        });

        let particle_layout = render_device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            label: None,
            entries: &[
                // Positions/Rotations
                BindGroupLayoutEntry {
                    binding: 0,
                    visibility: ShaderStages::VERTEX,
                    ty: BindingType::Buffer {
                        ty: BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: BufferSize::new(std::mem::size_of::<Vec4>() as u64),
                    },
                    count: None,
                },
                // Sizes
                BindGroupLayoutEntry {
                    binding: 1,
                    visibility: ShaderStages::VERTEX,
                    ty: BindingType::Buffer {
                        ty: BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: BufferSize::new(std::mem::size_of::<f32>() as u64),
                    },
                    count: None,
                },
                // Colors
                BindGroupLayoutEntry {
                    binding: 2,
                    visibility: ShaderStages::VERTEX,
                    ty: BindingType::Buffer {
                        ty: BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: BufferSize::new(std::mem::size_of::<Vec4>() as u64),
                    },
                    count: None,
                },
            ],
        });

        let material_layout = render_device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            label: None,
            entries: &[
                BindGroupLayoutEntry {
                    binding: 0,
                    visibility: ShaderStages::FRAGMENT,
                    ty: BindingType::Buffer {
                        ty: BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: BufferSize::new(
                            ParticleMaterialUniformData::std140_size_static() as u64,
                        ),
                    },
                    count: None,
                },
                // Base Color Texture
                BindGroupLayoutEntry {
                    binding: 1,
                    visibility: ShaderStages::FRAGMENT,
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
                    visibility: ShaderStages::FRAGMENT,
                    ty: BindingType::Sampler(SamplerBindingType::Filtering),
                    count: None,
                },
            ],
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
                    aspect: TextureAspect::All,
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

        Self {
            view_layout,
            particle_layout,
            material_layout,

            dummy_white_gpu_image,
        }
    }
}

#[derive(Clone, Eq, Hash, PartialEq)]
struct ParticlePipelineKey;

impl SpecializedPipeline for ParticlePipeline {
    type Key = ParticlePipelineKey;

    fn specialize(&self, _: Self::Key) -> RenderPipelineDescriptor {
        RenderPipelineDescriptor {
            label: Some("particle_render_pipeline".into()),
            vertex: VertexState {
                shader: PARTICLE_SHADER_HANDLE.typed::<Shader>(),
                entry_point: "vs_main".into(),
                buffers: vec![],
                shader_defs: vec![],
            },
            fragment: Some(FragmentState {
                shader: PARTICLE_SHADER_HANDLE.typed::<Shader>(),
                shader_defs: vec![],
                entry_point: "fs_main".into(),
                targets: vec![ColorTargetState {
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
                    write_mask: ColorWrites::ALL,
                }],
            }),
            depth_stencil: Some(DepthStencilState {
                format: TextureFormat::Depth32Float,
                depth_write_enabled: false,
                depth_compare: CompareFunction::Greater,
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
            layout: Some(vec![
                self.view_layout.clone(),
                self.particle_layout.clone(),
                self.material_layout.clone(),
            ]),
            multisample: MultisampleState::default(),
            primitive: PrimitiveState {
                topology: PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: FrontFace::Ccw,
                cull_mode: None,
                polygon_mode: PolygonMode::Fill,
                unclipped_depth: false,
                conservative: false,
            },
        }
    }
}

fn compute_particles_aabb(
    compute_task_pool: Res<ComputeTaskPool>,
    mut query: Query<(&mut Aabb, &Particles)>,
) {
    query.par_for_each_mut(&compute_task_pool, 8, |(mut aabb, particles)| {
        if let Some(bounding_box) = particles.compute_aabb() {
            *aabb = bounding_box;
        }
    });
}

struct ExtractedParticle {
    material: Handle<ParticleMaterial>,

    positions: Vec<Vec4>,
    sizes: Vec<f32>,
    colors: Vec<Vec4>,
}

#[derive(Default, Component)]
struct ExtractedParticles {
    particles: Vec<ExtractedParticle>,
}

fn extract_particles(
    mut render_world: ResMut<RenderWorld>,
    materials: Res<Assets<ParticleMaterial>>,
    images: Res<Assets<Image>>,
    query: Query<(&ComputedVisibility, &Particles, &Handle<ParticleMaterial>)>,
) {
    let mut extracted_particles = render_world
        .get_resource_mut::<ExtractedParticles>()
        .unwrap();
    extracted_particles.particles.clear();
    for (visible, particles, material_handle) in query.iter() {
        if !visible.is_visible {
            continue;
        }
        if let Some(material) = materials.get(material_handle) {
            if let Some(ref image) = material.base_color_texture {
                if !images.contains(image) {
                    continue;
                }
            }

            // TODO(james7132): Find a way to do this without clones.
            extracted_particles.particles.push(ExtractedParticle {
                material: material_handle.clone_weak(),
                positions: particles.positions.clone(),
                sizes: particles.sizes.clone(),
                colors: particles.colors.clone(),
            });
        }
    }
}

struct ParticleMeta {
    total_count: u64,
    view_bind_group: Option<BindGroup>,
    particle_bind_group: Option<BindGroup>,

    positions: BufferVec<Vec4>,
    sizes: BufferVec<f32>,
    colors: BufferVec<Vec4>,
}

impl Default for ParticleMeta {
    fn default() -> Self {
        ParticleMeta {
            total_count: 0,
            view_bind_group: None,
            particle_bind_group: None,

            positions: BufferVec::new(BufferUsages::STORAGE),
            sizes: BufferVec::new(BufferUsages::STORAGE),
            colors: BufferVec::new(BufferUsages::STORAGE),
        }
    }
}

fn prepare_particles(
    mut commands: Commands,
    mut particle_meta: ResMut<ParticleMeta>,
    mut extracted_particles: ResMut<ExtractedParticles>,
    render_device: Res<RenderDevice>,
    render_queue: Res<RenderQueue>,
) {
    particle_meta.positions.clear();
    particle_meta.sizes.clear();
    particle_meta.colors.clear();

    extracted_particles
        .particles
        .sort_by(|a, b| a.material.cmp(&b.material));

    let mut start: u32 = 0;
    let mut end: u32 = 0;
    let mut current_batch_handle: Option<Handle<ParticleMaterial>> = None;
    particle_meta.total_count = 0;
    for particle in extracted_particles.particles.iter() {
        batch_copy(&particle.positions, &mut particle_meta.positions);
        batch_copy(&particle.sizes, &mut particle_meta.sizes);
        batch_copy(&particle.colors, &mut particle_meta.colors);
        end += particle.positions.len() as u32;
        particle_meta.total_count += particle.positions.len() as u64;
        if let Some(current_batch_handle) = &current_batch_handle {
            if *current_batch_handle != particle.material {
                commands.spawn_bundle((ParticleBatch {
                    range: start..end,
                    handle: current_batch_handle.clone_weak(),
                },));
            }
            start = end;
        }
        current_batch_handle = Some(particle.material.clone_weak());
    }

    if start != end {
        if let Some(current_batch_handle) = &current_batch_handle {
            commands.spawn_bundle((ParticleBatch {
                range: start..end,
                handle: current_batch_handle.clone_weak(),
            },));
        }
    }

    if particle_meta.total_count == 0 {
        return;
    }

    particle_meta
        .positions
        .write_buffer(&render_device, &render_queue);
    particle_meta
        .sizes
        .write_buffer(&render_device, &render_queue);
    particle_meta
        .colors
        .write_buffer(&render_device, &render_queue);
}

fn batch_copy<T: Pod>(src: &[T], dst: &mut BufferVec<T>) {
    for item in src.iter() {
        dst.push(*item);
    }
}

fn bind_buffer<T: Pod>(buffer: &BufferVec<T>, count: u64) -> BindingResource {
    BindingResource::Buffer(BufferBinding {
        buffer: buffer.buffer().expect("missing buffer"),
        offset: 0,
        size: Some(NonZeroU64::new(std::mem::size_of::<T>() as u64 * count).unwrap()),
    })
}

fn image_handle_to_view_sampler<'a>(
    particle_pipeline: &'a ParticlePipeline,
    gpu_images: &'a RenderAssets<Image>,
    image_option: &Option<Handle<Image>>,
) -> (&'a TextureView, &'a Sampler) {
    image_option.as_ref().map_or(
        (
            &particle_pipeline.dummy_white_gpu_image.texture_view,
            &particle_pipeline.dummy_white_gpu_image.sampler,
        ),
        |image_handle| {
            let gpu_image = gpu_images
                .get(image_handle)
                .expect("only materials with valid textures should be drawn");
            (&gpu_image.texture_view, &gpu_image.sampler)
        },
    )
}

#[derive(Component)]
struct ParticleBatch {
    range: Range<u32>,
    handle: Handle<ParticleMaterial>,
}

#[derive(Default)]
struct MaterialBindGroups {
    values: HashMap<Handle<ParticleMaterial>, BindGroup>,
}

fn queue_particle_bind_groups(
    mut particle_meta: ResMut<ParticleMeta>,
    mut material_bind_groups: ResMut<MaterialBindGroups>,
    view_uniforms: Res<ViewUniforms>,
    render_device: Res<RenderDevice>,
    particle_batches: Query<&ParticleBatch>,
    particle_pipeline: Res<ParticlePipeline>,
    render_materials: Res<RenderAssets<ParticleMaterial>>,
    gpu_images: Res<RenderAssets<Image>>,
) {
    if view_uniforms.uniforms.is_empty() || particle_meta.total_count == 0 {
        return;
    }

    if let Some(view_bindings) = view_uniforms.uniforms.binding() {
        particle_meta.view_bind_group.get_or_insert_with(|| {
            render_device.create_bind_group(&BindGroupDescriptor {
                entries: &[BindGroupEntry {
                    binding: 0,
                    resource: view_bindings,
                }],
                label: Some("particle_view_bind_group"),
                layout: &particle_pipeline.view_layout,
            })
        });
    }

    // TODO(james7132): Find a way to cache this.
    particle_meta.particle_bind_group =
        Some(render_device.create_bind_group(&BindGroupDescriptor {
            entries: &[
                BindGroupEntry {
                    binding: 0,
                    resource: bind_buffer(&particle_meta.positions, particle_meta.total_count),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: bind_buffer(&particle_meta.sizes, particle_meta.total_count),
                },
                BindGroupEntry {
                    binding: 2,
                    resource: bind_buffer(&particle_meta.colors, particle_meta.total_count),
                },
            ],
            label: Some("particle_particle_bind_group"),
            layout: &particle_pipeline.particle_layout,
        }));

    for batch in particle_batches.iter() {
        let gpu_material = render_materials
            .get(&batch.handle)
            .expect("Failed to get ParticleMaterial PreparedAsset");

        if !material_bind_groups.values.contains_key(&batch.handle) {
            let (base_color_texture_view, base_color_sampler) = image_handle_to_view_sampler(
                &particle_pipeline,
                &gpu_images,
                &gpu_material.base_color_texture,
            );

            material_bind_groups.values.insert(
                batch.handle.clone_weak(),
                render_device.create_bind_group(&BindGroupDescriptor {
                    entries: &[
                        BindGroupEntry {
                            binding: 0,
                            resource: gpu_material.buffer.as_entire_binding(),
                        },
                        BindGroupEntry {
                            binding: 1,
                            resource: BindingResource::TextureView(base_color_texture_view),
                        },
                        BindGroupEntry {
                            binding: 2,
                            resource: BindingResource::Sampler(base_color_sampler),
                        },
                    ],
                    label: Some("particle_material_bind_group"),
                    layout: &particle_pipeline.material_layout,
                }),
            );
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn queue_particles(
    draw_functions_2d: Res<DrawFunctions<Transparent2d>>,
    draw_functions_3d: Res<DrawFunctions<Transparent2d>>,
    mut views_2d: Query<&mut RenderPhase<Transparent2d>>,
    mut views_3d: Query<&mut RenderPhase<Transparent3d>>,
    mut pipelines: ResMut<SpecializedPipelines<ParticlePipeline>>,
    mut pipeline_cache: ResMut<RenderPipelineCache>,
    particle_batches: Query<Entity, With<ParticleBatch>>,
    particle_pipeline: Res<ParticlePipeline>,
) {
    let draw_2d = draw_functions_2d.read().get_id::<DrawParticle>().unwrap();
    let draw_3d = draw_functions_3d.read().get_id::<DrawParticle>().unwrap();
    for entity in particle_batches.iter() {
        let pipeline =
            pipelines.specialize(&mut pipeline_cache, &particle_pipeline, ParticlePipelineKey);
        for mut phase in views_2d.iter_mut() {
            phase.add(Transparent2d {
                // TODO(james7132): properly compute this
                sort_key: FloatOrd(0.0),
                pipeline,
                entity,
                draw_function: draw_2d,
            });
        }
        for mut phase in views_3d.iter_mut() {
            phase.add(Transparent3d {
                // TODO(james7132): properly compute this
                distance: 10.0,
                pipeline,
                entity,
                draw_function: draw_3d,
            });
        }
    }
}

struct DrawParticle {
    params: SystemState<(
        SRes<ParticleMeta>,
        SRes<MaterialBindGroups>,
        SRes<RenderPipelineCache>,
        SQuery<Read<ViewUniformOffset>>,
        SQuery<Read<ParticleBatch>>,
    )>,
}

impl DrawParticle {
    fn new(world: &mut World) -> Self {
        Self {
            params: SystemState::new(world),
        }
    }

    #[inline(always)]
    fn draw_particles<'w>(
        &mut self,
        world: &'w World,
        view: Entity,
        pass: &mut TrackedRenderPass<'w>,
        entity: Entity,
        pipeline: CachedPipelineId,
    ) {
        let (particle_meta, material_bind_groups, pipelines, views, batches) =
            self.params.get(world);
        let view_uniform = views.get(view).unwrap();
        let material_bind_groups = material_bind_groups.into_inner();
        let particle_meta = particle_meta.into_inner();
        let batch = batches.get(entity).unwrap();

        if let Some(pipeline) = pipelines.into_inner().get(pipeline) {
            let vertex_range = (batch.range.start * 6)..(batch.range.end * 6);

            pass.set_render_pipeline(pipeline);
            pass.set_bind_group(
                0,
                particle_meta.view_bind_group.as_ref().unwrap(),
                &[view_uniform.offset],
            );
            pass.set_bind_group(1, particle_meta.particle_bind_group.as_ref().unwrap(), &[]);
            pass.set_bind_group(
                2,
                material_bind_groups.values.get(&batch.handle).unwrap(),
                &[],
            );
            pass.draw(vertex_range, 0..1);
        }
    }
}

impl Draw<Transparent2d> for DrawParticle {
    fn draw<'w>(
        &mut self,
        world: &'w World,
        pass: &mut TrackedRenderPass<'w>,
        view: Entity,
        item: &Transparent2d,
    ) {
        self.draw_particles(world, view, pass, item.entity, item.pipeline);
    }
}

impl Draw<Transparent3d> for DrawParticle {
    fn draw<'w>(
        &mut self,
        world: &'w World,
        pass: &mut TrackedRenderPass<'w>,
        view: Entity,
        item: &Transparent3d,
    ) {
        self.draw_particles(world, view, pass, item.entity, item.pipeline);
    }
}
