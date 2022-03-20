use crate::{
    GlobalLightMeta, GpuLights, LightMeta, NotShadowCaster, NotShadowReceiver, ShadowPipeline,
    ViewClusterBindings, ViewLightsUniformOffset, ViewShadowBindings,
};
use bevy_app::Plugin;
use bevy_asset::{load_internal_asset, Assets, Handle, HandleUntyped};
use bevy_ecs::{
    prelude::*,
    system::{lifetimeless::*, SystemParamItem},
};
use bevy_math::{Mat4, Size};
use bevy_reflect::TypeUuid;
use bevy_render::{
    mesh::{
        skinning::{SkinnedMesh, SkinnedMeshInverseBindposes},
        GpuBufferInfo, Mesh, MeshVertexBufferLayout,
    },
    render_asset::RenderAssets,
    render_component::{ComponentUniforms, DynamicUniformIndex, UniformComponentPlugin},
    render_phase::{EntityRenderCommand, RenderCommandResult, TrackedRenderPass},
    render_resource::{std140::AsStd140, *},
    renderer::{RenderDevice, RenderQueue},
    settings::WgpuSettings,
    texture::{BevyDefault, GpuImage, Image, TextureFormatPixelInfo},
    view::{ComputedVisibility, ViewUniform, ViewUniformOffset, ViewUniforms},
    RenderApp, RenderStage,
};
use bevy_transform::components::GlobalTransform;
use std::num::NonZeroU64;

#[derive(Default)]
pub struct MeshRenderPlugin;

const MAX_JOINTS: usize = 256;
const JOINT_SIZE: usize = std::mem::size_of::<Mat4>();
const JOINT_BUFFER_SIZE: usize = MAX_JOINTS * JOINT_SIZE;

pub const MESH_VIEW_BIND_GROUP_HANDLE: HandleUntyped =
    HandleUntyped::weak_from_u64(Shader::TYPE_UUID, 9076678235888822571);
pub const MESH_STRUCT_HANDLE: HandleUntyped =
    HandleUntyped::weak_from_u64(Shader::TYPE_UUID, 2506024101911992377);
pub const MESH_SHADER_HANDLE: HandleUntyped =
    HandleUntyped::weak_from_u64(Shader::TYPE_UUID, 3252377289100772450);
pub const SKINNING_HANDLE: HandleUntyped =
    HandleUntyped::weak_from_u64(Shader::TYPE_UUID, 13215291596265391738);

impl Plugin for MeshRenderPlugin {
    fn build(&self, app: &mut bevy_app::App) {
        load_internal_asset!(app, MESH_SHADER_HANDLE, "mesh.wgsl", Shader::from_wgsl);
        load_internal_asset!(
            app,
            MESH_STRUCT_HANDLE,
            "mesh_struct.wgsl",
            Shader::from_wgsl
        );
        load_internal_asset!(
            app,
            MESH_VIEW_BIND_GROUP_HANDLE,
            "mesh_view_bind_group.wgsl",
            Shader::from_wgsl
        );
        load_internal_asset!(app, SKINNING_HANDLE, "skinning.wgsl", Shader::from_wgsl);

        app.add_plugin(UniformComponentPlugin::<MeshUniform>::default());

        if let Ok(render_app) = app.get_sub_app_mut(RenderApp) {
            render_app
                .init_resource::<MeshPipeline>()
                .add_system_to_stage(RenderStage::Extract, extract_meshes)
                .add_system_to_stage(RenderStage::Extract, extract_skinned_meshes)
                .add_system_to_stage(RenderStage::Prepare, prepare_skinned_meshes)
                .add_system_to_stage(RenderStage::Queue, queue_mesh_bind_group)
                .add_system_to_stage(RenderStage::Queue, queue_mesh_view_bind_groups)
                .add_system_to_stage(RenderStage::Queue, queue_skinned_mesh_bind_group);
        }
    }
}

#[derive(Component, AsStd140, Clone)]
pub struct MeshUniform {
    pub transform: Mat4,
    pub inverse_transpose_model: Mat4,
    pub flags: u32,
}

// NOTE: These must match the bit flags in bevy_pbr2/src/render/mesh.wgsl!
bitflags::bitflags! {
    #[repr(transparent)]
    struct MeshFlags: u32 {
        const SHADOW_RECEIVER            = (1 << 0);
        const NONE                       = 0;
        const UNINITIALIZED              = 0xFFFF;
    }
}

pub fn extract_meshes(
    mut commands: Commands,
    mut previous_caster_len: Local<usize>,
    mut previous_not_caster_len: Local<usize>,
    caster_query: Query<
        (
            Entity,
            &ComputedVisibility,
            &GlobalTransform,
            &Handle<Mesh>,
            Option<&NotShadowReceiver>,
        ),
        Without<NotShadowCaster>,
    >,
    not_caster_query: Query<
        (
            Entity,
            &ComputedVisibility,
            &GlobalTransform,
            &Handle<Mesh>,
            Option<&NotShadowReceiver>,
        ),
        With<NotShadowCaster>,
    >,
) {
    let mut caster_values = Vec::with_capacity(*previous_caster_len);
    for (entity, computed_visibility, transform, mesh, not_receiver) in caster_query.iter() {
        if !computed_visibility.is_visible {
            continue;
        }
        let transform = transform.compute_matrix();
        caster_values.push((
            entity,
            (
                mesh.clone_weak(),
                MeshUniform {
                    flags: if not_receiver.is_some() {
                        MeshFlags::empty().bits
                    } else {
                        MeshFlags::SHADOW_RECEIVER.bits
                    },
                    transform,
                    inverse_transpose_model: transform.inverse().transpose(),
                },
            ),
        ));
    }
    *previous_caster_len = caster_values.len();
    commands.insert_or_spawn_batch(caster_values);

    let mut not_caster_values = Vec::with_capacity(*previous_not_caster_len);
    for (entity, computed_visibility, transform, mesh, not_receiver) in not_caster_query.iter() {
        if !computed_visibility.is_visible {
            continue;
        }
        let transform = transform.compute_matrix();
        not_caster_values.push((
            entity,
            (
                mesh.clone_weak(),
                MeshUniform {
                    flags: if not_receiver.is_some() {
                        MeshFlags::empty().bits
                    } else {
                        MeshFlags::SHADOW_RECEIVER.bits
                    },
                    transform,
                    inverse_transpose_model: transform.inverse().transpose(),
                },
                NotShadowCaster,
            ),
        ));
    }
    *previous_not_caster_len = not_caster_values.len();
    commands.insert_or_spawn_batch(not_caster_values);
}

#[derive(Component)]
pub struct SkinnedMeshJoints {
    pub buffer: BufferVec<Mat4>,
}

impl SkinnedMeshJoints {
    #[inline]
    pub fn build(
        skin: &SkinnedMesh,
        inverse_bindposes: &Assets<SkinnedMeshInverseBindposes>,
        joints: &Query<&GlobalTransform>,
        max_joint_count: usize,
    ) -> Option<Self> {
        let inverse_bindposes = inverse_bindposes.get(&skin.inverse_bindposes)?;
        let bindposes = inverse_bindposes.iter();
        let skin_joints = skin.joints.iter();
        let mut buffer = BufferVec::new(BufferUsages::UNIFORM);
        for (inverse_bindpose, joint) in bindposes.zip(skin_joints).take(max_joint_count) {
            let joint_matrix = joints.get(*joint).ok()?.compute_matrix();
            buffer.push(joint_matrix * *inverse_bindpose);
        }
        Some(Self { buffer })
    }
}

pub fn extract_skinned_meshes(
    mut commands: Commands,
    mut previous_len: Local<usize>,
    query: Query<(Entity, &ComputedVisibility, &SkinnedMesh)>,
    inverse_bindposes: Res<Assets<SkinnedMeshInverseBindposes>>,
    joint_query: Query<&GlobalTransform>,
    wgpu_settings: Res<WgpuSettings>,
) {
    let mut values = Vec::with_capacity(*previous_len);
    let max_joint_count =
        (wgpu_settings.limits.max_uniform_buffer_binding_size as usize) / JOINT_SIZE;
    for (entity, computed_visibility, skin) in query.iter() {
        if !computed_visibility.is_visible {
            continue;
        }
        // TODO: This can be expensive, can we move this to prepare?
        if let Some(uniform) =
            SkinnedMeshJoints::build(skin, &inverse_bindposes, &joint_query, max_joint_count)
        {
            values.push((entity, (uniform,)));
        }
    }
    *previous_len = values.len();
    commands.insert_or_spawn_batch(values);
}

#[derive(Clone)]
pub struct MeshPipeline {
    pub view_layout: BindGroupLayout,
    pub mesh_layout: BindGroupLayout,
    pub skinned_mesh_layout: BindGroupLayout,
    // This dummy white texture is to be used in place of optional StandardMaterial textures
    pub dummy_white_gpu_image: GpuImage,
}

impl FromWorld for MeshPipeline {
    fn from_world(world: &mut World) -> Self {
        let render_device = world.resource::<RenderDevice>();
        let view_layout = render_device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            entries: &[
                // View
                BindGroupLayoutEntry {
                    binding: 0,
                    visibility: ShaderStages::VERTEX | ShaderStages::FRAGMENT,
                    ty: BindingType::Buffer {
                        ty: BufferBindingType::Uniform,
                        has_dynamic_offset: true,
                        min_binding_size: BufferSize::new(ViewUniform::std140_size_static() as u64),
                    },
                    count: None,
                },
                // Lights
                BindGroupLayoutEntry {
                    binding: 1,
                    visibility: ShaderStages::FRAGMENT,
                    ty: BindingType::Buffer {
                        ty: BufferBindingType::Uniform,
                        has_dynamic_offset: true,
                        min_binding_size: BufferSize::new(GpuLights::std140_size_static() as u64),
                    },
                    count: None,
                },
                // Point Shadow Texture Cube Array
                BindGroupLayoutEntry {
                    binding: 2,
                    visibility: ShaderStages::FRAGMENT,
                    ty: BindingType::Texture {
                        multisampled: false,
                        sample_type: TextureSampleType::Depth,
                        #[cfg(not(feature = "webgl"))]
                        view_dimension: TextureViewDimension::CubeArray,
                        #[cfg(feature = "webgl")]
                        view_dimension: TextureViewDimension::Cube,
                    },
                    count: None,
                },
                // Point Shadow Texture Array Sampler
                BindGroupLayoutEntry {
                    binding: 3,
                    visibility: ShaderStages::FRAGMENT,
                    ty: BindingType::Sampler(SamplerBindingType::Comparison),
                    count: None,
                },
                // Directional Shadow Texture Array
                BindGroupLayoutEntry {
                    binding: 4,
                    visibility: ShaderStages::FRAGMENT,
                    ty: BindingType::Texture {
                        multisampled: false,
                        sample_type: TextureSampleType::Depth,
                        #[cfg(not(feature = "webgl"))]
                        view_dimension: TextureViewDimension::D2Array,
                        #[cfg(feature = "webgl")]
                        view_dimension: TextureViewDimension::D2,
                    },
                    count: None,
                },
                // Directional Shadow Texture Array Sampler
                BindGroupLayoutEntry {
                    binding: 5,
                    visibility: ShaderStages::FRAGMENT,
                    ty: BindingType::Sampler(SamplerBindingType::Comparison),
                    count: None,
                },
                // PointLights
                BindGroupLayoutEntry {
                    binding: 6,
                    visibility: ShaderStages::FRAGMENT,
                    ty: BindingType::Buffer {
                        ty: BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        // NOTE: Static size for uniform buffers. GpuPointLight has a padded
                        // size of 64 bytes, so 16384 / 64 = 256 point lights max
                        min_binding_size: BufferSize::new(16384),
                    },
                    count: None,
                },
                // ClusteredLightIndexLists
                BindGroupLayoutEntry {
                    binding: 7,
                    visibility: ShaderStages::FRAGMENT,
                    ty: BindingType::Buffer {
                        ty: BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        // NOTE: With 256 point lights max, indices need 8 bits so use u8
                        min_binding_size: BufferSize::new(16384),
                    },
                    count: None,
                },
                // ClusterOffsetsAndCounts
                BindGroupLayoutEntry {
                    binding: 8,
                    visibility: ShaderStages::FRAGMENT,
                    ty: BindingType::Buffer {
                        ty: BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        // NOTE: The offset needs to address 16384 indices, which needs 14 bits.
                        // The count can be at most all 256 lights so 8 bits.
                        // Pack the offset into the upper 24 bits and the count into the
                        // lower 8 bits.
                        min_binding_size: BufferSize::new(16384),
                    },
                    count: None,
                },
            ],
            label: Some("mesh_view_layout"),
        });

        let mesh_layout = render_device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            entries: &[BindGroupLayoutEntry {
                binding: 0,
                visibility: ShaderStages::VERTEX | ShaderStages::FRAGMENT,
                ty: BindingType::Buffer {
                    ty: BufferBindingType::Uniform,
                    has_dynamic_offset: true,
                    min_binding_size: BufferSize::new(MeshUniform::std140_size_static() as u64),
                },
                count: None,
            }],
            label: Some("mesh_layout"),
        });

        let skinned_mesh_layout =
            render_device.create_bind_group_layout(&BindGroupLayoutDescriptor {
                entries: &[BindGroupLayoutEntry {
                    binding: 0,
                    visibility: ShaderStages::VERTEX,
                    ty: BindingType::Buffer {
                        ty: BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: BufferSize::new(JOINT_BUFFER_SIZE as u64),
                    },
                    count: None,
                }],
                label: Some("mesh_layout"),
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
            let render_queue = world.resource_mut::<RenderQueue>();
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
                texture_format: image.texture_descriptor.format,
                sampler,
                size: Size::new(
                    image.texture_descriptor.size.width as f32,
                    image.texture_descriptor.size.height as f32,
                ),
            }
        };
        MeshPipeline {
            view_layout,
            mesh_layout,
            skinned_mesh_layout,
            dummy_white_gpu_image,
        }
    }
}

impl MeshPipeline {
    pub fn get_image_texture<'a>(
        &'a self,
        gpu_images: &'a RenderAssets<Image>,
        handle_option: &Option<Handle<Image>>,
    ) -> Option<(&'a TextureView, &'a Sampler)> {
        if let Some(handle) = handle_option {
            let gpu_image = gpu_images.get(handle)?;
            Some((&gpu_image.texture_view, &gpu_image.sampler))
        } else {
            Some((
                &self.dummy_white_gpu_image.texture_view,
                &self.dummy_white_gpu_image.sampler,
            ))
        }
    }
}

bitflags::bitflags! {
    #[repr(transparent)]
    // NOTE: Apparently quadro drivers support up to 64x MSAA.
    /// MSAA uses the highest 6 bits for the MSAA sample count - 1 to support up to 64x MSAA.
    pub struct MeshPipelineKey: u32 {
        const NONE                        = 0;
        const TRANSPARENT_MAIN_PASS       = (1 << 0);
        const MSAA_RESERVED_BITS          = MeshPipelineKey::MSAA_MASK_BITS << MeshPipelineKey::MSAA_SHIFT_BITS;
        const PRIMITIVE_TOPOLOGY_RESERVED_BITS = MeshPipelineKey::PRIMITIVE_TOPOLOGY_MASK_BITS << MeshPipelineKey::PRIMITIVE_TOPOLOGY_SHIFT_BITS;
    }
}

impl MeshPipelineKey {
    const MSAA_MASK_BITS: u32 = 0b111111;
    const MSAA_SHIFT_BITS: u32 = 32 - 6;
    const PRIMITIVE_TOPOLOGY_MASK_BITS: u32 = 0b111;
    const PRIMITIVE_TOPOLOGY_SHIFT_BITS: u32 = Self::MSAA_SHIFT_BITS - 3;

    pub fn from_msaa_samples(msaa_samples: u32) -> Self {
        let msaa_bits = ((msaa_samples - 1) & Self::MSAA_MASK_BITS) << Self::MSAA_SHIFT_BITS;
        MeshPipelineKey::from_bits(msaa_bits).unwrap()
    }

    pub fn msaa_samples(&self) -> u32 {
        ((self.bits >> Self::MSAA_SHIFT_BITS) & Self::MSAA_MASK_BITS) + 1
    }

    pub fn from_primitive_topology(primitive_topology: PrimitiveTopology) -> Self {
        let primitive_topology_bits = ((primitive_topology as u32)
            & Self::PRIMITIVE_TOPOLOGY_MASK_BITS)
            << Self::PRIMITIVE_TOPOLOGY_SHIFT_BITS;
        MeshPipelineKey::from_bits(primitive_topology_bits).unwrap()
    }

    pub fn primitive_topology(&self) -> PrimitiveTopology {
        let primitive_topology_bits =
            (self.bits >> Self::PRIMITIVE_TOPOLOGY_SHIFT_BITS) & Self::PRIMITIVE_TOPOLOGY_MASK_BITS;
        match primitive_topology_bits {
            x if x == PrimitiveTopology::PointList as u32 => PrimitiveTopology::PointList,
            x if x == PrimitiveTopology::LineList as u32 => PrimitiveTopology::LineList,
            x if x == PrimitiveTopology::LineStrip as u32 => PrimitiveTopology::LineStrip,
            x if x == PrimitiveTopology::TriangleList as u32 => PrimitiveTopology::TriangleList,
            x if x == PrimitiveTopology::TriangleStrip as u32 => PrimitiveTopology::TriangleStrip,
            _ => PrimitiveTopology::default(),
        }
    }
}

impl SpecializedMeshPipeline for MeshPipeline {
    type Key = MeshPipelineKey;

    fn specialize(
        &self,
        key: Self::Key,
        layout: &MeshVertexBufferLayout,
    ) -> Result<RenderPipelineDescriptor, SpecializedMeshPipelineError> {
        let mut vertex_attributes = vec![
            Mesh::ATTRIBUTE_POSITION.at_shader_location(0),
            Mesh::ATTRIBUTE_NORMAL.at_shader_location(1),
            Mesh::ATTRIBUTE_UV_0.at_shader_location(2),
        ];

        let mut bind_group_layout = vec![self.view_layout.clone(), self.mesh_layout.clone()];
        let mut shader_defs = Vec::new();
        if layout.contains(Mesh::ATTRIBUTE_TANGENT) {
            shader_defs.push(String::from("VERTEX_TANGENTS"));
            vertex_attributes.push(Mesh::ATTRIBUTE_TANGENT.at_shader_location(3));
        }

        if layout.contains(Mesh::ATTRIBUTE_JOINT_INDEX)
            && layout.contains(Mesh::ATTRIBUTE_JOINT_WEIGHT)
        {
            shader_defs.push(String::from("SKINNED"));
            vertex_attributes.push(Mesh::ATTRIBUTE_JOINT_INDEX.at_shader_location(4));
            vertex_attributes.push(Mesh::ATTRIBUTE_JOINT_WEIGHT.at_shader_location(5));
            bind_group_layout.push(self.skinned_mesh_layout.clone());
        }

        let vertex_buffer_layout = layout.get_layout(&vertex_attributes)?;

        let (label, blend, depth_write_enabled);
        if key.contains(MeshPipelineKey::TRANSPARENT_MAIN_PASS) {
            label = "transparent_mesh_pipeline".into();
            blend = Some(BlendState::ALPHA_BLENDING);
            // For the transparent pass, fragments that are closer will be alpha blended
            // but their depth is not written to the depth buffer
            depth_write_enabled = false;
        } else {
            label = "opaque_mesh_pipeline".into();
            blend = Some(BlendState::REPLACE);
            // For the opaque and alpha mask passes, fragments that are closer will replace
            // the current fragment value in the output and the depth is written to the
            // depth buffer
            depth_write_enabled = true;
        }

        #[cfg(feature = "webgl")]
        shader_defs.push(String::from("NO_ARRAY_TEXTURES_SUPPORT"));

        Ok(RenderPipelineDescriptor {
            vertex: VertexState {
                shader: MESH_SHADER_HANDLE.typed::<Shader>(),
                entry_point: "vertex".into(),
                shader_defs: shader_defs.clone(),
                buffers: vec![vertex_buffer_layout],
            },
            fragment: Some(FragmentState {
                shader: MESH_SHADER_HANDLE.typed::<Shader>(),
                shader_defs,
                entry_point: "fragment".into(),
                targets: vec![ColorTargetState {
                    format: TextureFormat::bevy_default(),
                    blend,
                    write_mask: ColorWrites::ALL,
                }],
            }),
            layout: Some(bind_group_layout),
            primitive: PrimitiveState {
                front_face: FrontFace::Ccw,
                cull_mode: Some(Face::Back),
                unclipped_depth: false,
                polygon_mode: PolygonMode::Fill,
                conservative: false,
                topology: key.primitive_topology(),
                strip_index_format: None,
            },
            depth_stencil: Some(DepthStencilState {
                format: TextureFormat::Depth32Float,
                depth_write_enabled,
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
            multisample: MultisampleState {
                count: key.msaa_samples(),
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            label: Some(label),
        })
    }
}

pub struct MeshBindGroup {
    pub value: BindGroup,
}

pub fn queue_mesh_bind_group(
    mut commands: Commands,
    mesh_pipeline: Res<MeshPipeline>,
    render_device: Res<RenderDevice>,
    mesh_uniforms: Res<ComponentUniforms<MeshUniform>>,
) {
    if let Some(binding) = mesh_uniforms.uniforms().binding() {
        commands.insert_resource(MeshBindGroup {
            value: render_device.create_bind_group(&BindGroupDescriptor {
                entries: &[BindGroupEntry {
                    binding: 0,
                    resource: binding,
                }],
                label: Some("mesh_bind_group"),
                layout: &mesh_pipeline.mesh_layout,
            }),
        });
    }
}

#[derive(Component)]
pub struct SkinnedMeshBindGroup {
    pub value: BindGroup,
}

pub fn prepare_skinned_meshes(
    render_device: Res<RenderDevice>,
    render_queue: Res<RenderQueue>,
    mut skinned_meshes: Query<&mut SkinnedMeshJoints>,
) {
    for mut joints in skinned_meshes.iter_mut() {
        joints.buffer.reserve(JOINT_BUFFER_SIZE, &render_device);
        joints.buffer.write_buffer(&render_device, &render_queue);
    }
}

pub fn queue_skinned_mesh_bind_group(
    mut prev_len: Local<usize>,
    mut commands: Commands,
    mesh_pipeline: Res<MeshPipeline>,
    render_device: Res<RenderDevice>,
    skinned_meshes: Query<(Entity, &SkinnedMeshJoints)>,
) {
    let mut values = Vec::with_capacity(*prev_len);
    for (entity, joints) in skinned_meshes.iter() {
        values.push((
            entity,
            (SkinnedMeshBindGroup {
                value: render_device.create_bind_group(&BindGroupDescriptor {
                    entries: &[BindGroupEntry {
                        binding: 0,
                        resource: BindingResource::Buffer(BufferBinding {
                            buffer: joints.buffer.buffer().unwrap(),
                            offset: 0,
                            size: Some(NonZeroU64::new(JOINT_BUFFER_SIZE as u64).unwrap()),
                        }),
                    }],
                    label: Some("skinned_mesh_bind_group"),
                    layout: &mesh_pipeline.skinned_mesh_layout,
                }),
            },),
        ));
    }
    *prev_len = values.len();
    commands.insert_or_spawn_batch(values);
}

#[derive(Component)]
pub struct MeshViewBindGroup {
    pub value: BindGroup,
}

#[allow(clippy::too_many_arguments)]
pub fn queue_mesh_view_bind_groups(
    mut commands: Commands,
    render_device: Res<RenderDevice>,
    mesh_pipeline: Res<MeshPipeline>,
    shadow_pipeline: Res<ShadowPipeline>,
    light_meta: Res<LightMeta>,
    global_light_meta: Res<GlobalLightMeta>,
    view_uniforms: Res<ViewUniforms>,
    views: Query<(Entity, &ViewShadowBindings, &ViewClusterBindings)>,
) {
    if let (Some(view_binding), Some(light_binding), Some(point_light_binding)) = (
        view_uniforms.uniforms.binding(),
        light_meta.view_gpu_lights.binding(),
        global_light_meta.gpu_point_lights.binding(),
    ) {
        for (entity, view_shadow_bindings, view_cluster_bindings) in views.iter() {
            let view_bind_group = render_device.create_bind_group(&BindGroupDescriptor {
                entries: &[
                    BindGroupEntry {
                        binding: 0,
                        resource: view_binding.clone(),
                    },
                    BindGroupEntry {
                        binding: 1,
                        resource: light_binding.clone(),
                    },
                    BindGroupEntry {
                        binding: 2,
                        resource: BindingResource::TextureView(
                            &view_shadow_bindings.point_light_depth_texture_view,
                        ),
                    },
                    BindGroupEntry {
                        binding: 3,
                        resource: BindingResource::Sampler(&shadow_pipeline.point_light_sampler),
                    },
                    BindGroupEntry {
                        binding: 4,
                        resource: BindingResource::TextureView(
                            &view_shadow_bindings.directional_light_depth_texture_view,
                        ),
                    },
                    BindGroupEntry {
                        binding: 5,
                        resource: BindingResource::Sampler(
                            &shadow_pipeline.directional_light_sampler,
                        ),
                    },
                    BindGroupEntry {
                        binding: 6,
                        resource: point_light_binding.clone(),
                    },
                    BindGroupEntry {
                        binding: 7,
                        resource: view_cluster_bindings
                            .cluster_light_index_lists
                            .binding()
                            .unwrap(),
                    },
                    BindGroupEntry {
                        binding: 8,
                        resource: view_cluster_bindings
                            .cluster_offsets_and_counts
                            .binding()
                            .unwrap(),
                    },
                ],
                label: Some("mesh_view_bind_group"),
                layout: &mesh_pipeline.view_layout,
            });

            commands.entity(entity).insert(MeshViewBindGroup {
                value: view_bind_group,
            });
        }
    }
}

pub struct SetMeshViewBindGroup<const I: usize>;
impl<const I: usize> EntityRenderCommand for SetMeshViewBindGroup<I> {
    type Param = SQuery<(
        Read<ViewUniformOffset>,
        Read<ViewLightsUniformOffset>,
        Read<MeshViewBindGroup>,
    )>;
    #[inline]
    fn render<'w>(
        view: Entity,
        _item: Entity,
        view_query: SystemParamItem<'w, '_, Self::Param>,
        pass: &mut TrackedRenderPass<'w>,
    ) -> RenderCommandResult {
        let (view_uniform, view_lights, mesh_view_bind_group) = view_query.get(view).unwrap();
        pass.set_bind_group(
            I,
            &mesh_view_bind_group.value,
            &[view_uniform.offset, view_lights.offset],
        );

        RenderCommandResult::Success
    }
}

pub struct SetMeshBindGroup<const I: usize>;
impl<const I: usize> EntityRenderCommand for SetMeshBindGroup<I> {
    type Param = (
        SRes<MeshBindGroup>,
        SQuery<Read<DynamicUniformIndex<MeshUniform>>>,
    );
    #[inline]
    fn render<'w>(
        _view: Entity,
        item: Entity,
        (mesh_bind_group, mesh_query): SystemParamItem<'w, '_, Self::Param>,
        pass: &mut TrackedRenderPass<'w>,
    ) -> RenderCommandResult {
        let mesh_index = mesh_query.get(item).unwrap();
        pass.set_bind_group(
            I,
            &mesh_bind_group.into_inner().value,
            &[mesh_index.index()],
        );
        RenderCommandResult::Success
    }
}

pub struct SetSkinnedMeshBindGroup<const I: usize>;
impl<const I: usize> EntityRenderCommand for SetSkinnedMeshBindGroup<I> {
    type Param = SQuery<Read<SkinnedMeshBindGroup>>;
    #[inline]
    fn render<'w>(
        _view: Entity,
        item: Entity,
        mesh_query: SystemParamItem<'w, '_, Self::Param>,
        pass: &mut TrackedRenderPass<'w>,
    ) -> RenderCommandResult {
        if let Ok(mesh_bind_group) = mesh_query.get(item) {
            pass.set_bind_group(I, &mesh_bind_group.value, &[]);
        }
        RenderCommandResult::Success
    }
}

pub struct DrawMesh;
impl EntityRenderCommand for DrawMesh {
    type Param = (SRes<RenderAssets<Mesh>>, SQuery<Read<Handle<Mesh>>>);
    #[inline]
    fn render<'w>(
        _view: Entity,
        item: Entity,
        (meshes, mesh_query): SystemParamItem<'w, '_, Self::Param>,
        pass: &mut TrackedRenderPass<'w>,
    ) -> RenderCommandResult {
        let mesh_handle = mesh_query.get(item).unwrap();
        if let Some(gpu_mesh) = meshes.into_inner().get(mesh_handle) {
            pass.set_vertex_buffer(0, gpu_mesh.vertex_buffer.slice(..));
            match &gpu_mesh.buffer_info {
                GpuBufferInfo::Indexed {
                    buffer,
                    index_format,
                    count,
                } => {
                    pass.set_index_buffer(buffer.slice(..), 0, *index_format);
                    pass.draw_indexed(0..*count, 0, 0..1);
                }
                GpuBufferInfo::NonIndexed { vertex_count } => {
                    pass.draw(0..*vertex_count, 0..1);
                }
            }
            RenderCommandResult::Success
        } else {
            RenderCommandResult::Failure
        }
    }
}

#[cfg(test)]
mod tests {
    use super::MeshPipelineKey;
    #[test]
    fn mesh_key_msaa_samples() {
        for i in 1..=64 {
            assert_eq!(MeshPipelineKey::from_msaa_samples(i).msaa_samples(), i);
        }
    }
}
