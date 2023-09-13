use super::{
    SolariGlobalIlluminationPipelines, SolariGlobalIlluminationSettings, WORLD_CACHE_SIZE,
};
use bevy_core::FrameCount;
use bevy_core_pipeline::prepass::{
    DepthPrepass, MotionVectorPrepass, NormalPrepass, ViewPrepassTextures, DEPTH_PREPASS_FORMAT,
    NORMAL_PREPASS_FORMAT,
};
use bevy_ecs::{
    component::Component,
    entity::Entity,
    query::With,
    system::{Commands, Query, Res, ResMut},
};
use bevy_math::UVec2;
use bevy_render::{
    camera::ExtractedCamera,
    render_resource::{
        BindGroup, BindGroupDescriptor, BindGroupEntry, BindGroupLayout, BindGroupLayoutDescriptor,
        BindGroupLayoutEntry, BindingResource, BindingType, BufferBindingType, BufferCache,
        BufferDescriptor, BufferUsages, CachedBuffer, Extent3d, ShaderStages, ShaderType,
        StorageTextureAccess, TextureDescriptor, TextureDimension, TextureFormat,
        TextureSampleType, TextureUsages, TextureViewDimension,
    },
    renderer::RenderDevice,
    texture::{CachedTexture, TextureCache},
    view::{ViewUniform, ViewUniforms},
};
use std::num::NonZeroU64;

#[derive(Component)]
pub struct SolariGlobalIlluminationViewResources {
    pub previous_depth_buffer: CachedTexture,
    pub previous_normals_buffer: CachedTexture,
    screen_probes_a: CachedTexture,
    screen_probes_b: CachedTexture,
    screen_probes_spherical_harmonics: CachedBuffer,
    diffuse_raw: CachedTexture,
    diffuse_denoiser_temporal_history: CachedTexture,
    diffuse_denoised_temporal: CachedTexture,
    pub diffuse_denoised_spatiotemporal: CachedTexture,
    world_cache_checksums: CachedBuffer,
    world_cache_life: CachedBuffer,
    world_cache_irradiance: CachedBuffer,
    world_cache_cell_data: CachedBuffer,
    world_cache_active_cells_new_irradiance: CachedBuffer,
    world_cache_a: CachedBuffer,
    world_cache_b: CachedBuffer,
    world_cache_active_cell_indices: CachedBuffer,
    world_cache_active_cells_count: CachedBuffer,
    pub world_cache_active_cells_dispatch: CachedBuffer,
}

pub fn prepare_resources(
    views: Query<
        (Entity, &ExtractedCamera),
        (
            With<SolariGlobalIlluminationSettings>,
            With<DepthPrepass>,
            With<NormalPrepass>,
            With<MotionVectorPrepass>,
        ),
    >,
    mut commands: Commands,
    mut texture_cache: ResMut<TextureCache>,
    mut buffer_cache: ResMut<BufferCache>,
    render_device: Res<RenderDevice>,
    frame_count: Res<FrameCount>,
) {
    let texture = |label, format, size: UVec2| TextureDescriptor {
        label: Some(label),
        size: Extent3d {
            width: size.x,
            height: size.y,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: TextureDimension::D2,
        format,
        usage: TextureUsages::STORAGE_BINDING,
        view_formats: &[],
    };
    let texture2 = |label, format, size: UVec2| TextureDescriptor {
        label: Some(label),
        size: Extent3d {
            width: size.x,
            height: size.y,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: TextureDimension::D2,
        format,
        usage: TextureUsages::TEXTURE_BINDING | TextureUsages::COPY_DST,
        view_formats: &[],
    };
    let buffer = |label, size| BufferDescriptor {
        label: Some(label),
        size,
        usage: BufferUsages::STORAGE,
        mapped_at_creation: false,
    };

    for (entity, camera) in &views {
        let Some(viewport_size) = camera.physical_viewport_size else {
            continue;
        };
        let width_8 = round_up_to_multiple_of_8(viewport_size.x);
        let height_8 = round_up_to_multiple_of_8(viewport_size.y);
        let size_8 = UVec2::new(width_8, height_8);
        let probe_count = (width_8 as u64 * height_8 as u64) / 64;

        let previous_depth_buffer = texture2(
            "solari_previous_depth_buffer",
            DEPTH_PREPASS_FORMAT,
            viewport_size,
        );
        let previous_normals_buffer = texture2(
            "solari_previous_normals_buffer",
            NORMAL_PREPASS_FORMAT,
            viewport_size,
        );

        let mut screen_probes_a = texture(
            "solari_global_illumination_screen_probes_a",
            TextureFormat::Rgba16Float,
            size_8,
        );
        screen_probes_a.size.depth_or_array_layers = 4;
        let screen_probes_b = texture(
            "solari_global_illumination_screen_probes_b",
            TextureFormat::Rgba16Float,
            size_8,
        );
        let screen_probes_spherical_harmonics = buffer(
            "solari_global_illumination_screen_probes_spherical_harmonics",
            probe_count * 112,
        );
        let diffuse_raw = texture(
            "solari_global_illumination_diffuse_raw",
            TextureFormat::Rgba16Float,
            viewport_size,
        );
        let (diffuse_denoiser_temporal_history, diffuse_denoised_temporal) = {
            let mut t1 = texture(
                "solari_global_illumination_diffuse_temporal_denoise_1",
                TextureFormat::Rgba16Float,
                viewport_size,
            );
            t1.usage |= TextureUsages::TEXTURE_BINDING;
            let t2 = TextureDescriptor {
                label: Some("solari_global_illumination_diffuse_temporal_denoise_2"),
                ..t1
            };
            if frame_count.0 % 2 == 0 {
                (t1, t2)
            } else {
                (t2, t1)
            }
        };
        let mut diffuse_denoised_spatiotemporal = texture(
            "solari_global_illumination_diffuse_denoised_spatiotemporal",
            TextureFormat::Rgba16Float,
            viewport_size,
        );
        diffuse_denoised_spatiotemporal.usage |= TextureUsages::TEXTURE_BINDING;
        let world_cache_checksums = buffer(
            "solari_global_illumination_world_cache_checksums",
            4 * WORLD_CACHE_SIZE,
        );
        let world_cache_life = buffer(
            "solari_global_illumination_world_cache_life",
            4 * WORLD_CACHE_SIZE,
        );
        let world_cache_irradiance = buffer(
            "solari_global_illumination_world_cache_irradiance",
            16 * WORLD_CACHE_SIZE,
        );
        let world_cache_cell_data = buffer(
            "solari_global_illumination_world_cache_cell_data",
            32 * WORLD_CACHE_SIZE,
        );
        let world_cache_active_cells_new_irradiance = buffer(
            "solari_global_illumination_world_cache_active_cells_new_irradiance",
            16 * WORLD_CACHE_SIZE,
        );
        let world_cache_a = buffer(
            "solari_global_illumination_world_cache_a",
            4 * WORLD_CACHE_SIZE,
        );
        let world_cache_b = buffer("solari_global_illumination_world_cache_b", 4 * 1024);
        let world_cache_active_cell_indices = buffer(
            "solari_global_illumination_world_cache_active_cell_indices",
            4 * WORLD_CACHE_SIZE,
        );
        let world_cache_active_cells_count = buffer(
            "solari_global_illumination_world_cache_active_cells_count",
            4,
        );
        let world_cache_active_cells_dispatch = BufferDescriptor {
            label: Some("solari_global_illumination_world_cache_active_cells_dispatch"),
            size: 12,
            usage: BufferUsages::STORAGE | BufferUsages::INDIRECT,
            mapped_at_creation: false,
        };

        commands
            .entity(entity)
            .insert(SolariGlobalIlluminationViewResources {
                previous_depth_buffer: texture_cache.get(&render_device, previous_depth_buffer),
                previous_normals_buffer: texture_cache.get(&render_device, previous_normals_buffer),
                screen_probes_a: texture_cache.get(&render_device, screen_probes_a),
                screen_probes_b: texture_cache.get(&render_device, screen_probes_b),
                screen_probes_spherical_harmonics: buffer_cache
                    .get(&render_device, screen_probes_spherical_harmonics),
                diffuse_raw: texture_cache.get(&render_device, diffuse_raw),
                diffuse_denoiser_temporal_history: texture_cache
                    .get(&render_device, diffuse_denoiser_temporal_history),
                diffuse_denoised_temporal: texture_cache
                    .get(&render_device, diffuse_denoised_temporal),
                diffuse_denoised_spatiotemporal: texture_cache
                    .get(&render_device, diffuse_denoised_spatiotemporal),
                world_cache_checksums: buffer_cache.get(&render_device, world_cache_checksums),
                world_cache_life: buffer_cache.get(&render_device, world_cache_life),
                world_cache_irradiance: buffer_cache.get(&render_device, world_cache_irradiance),
                world_cache_cell_data: buffer_cache.get(&render_device, world_cache_cell_data),
                world_cache_active_cells_new_irradiance: buffer_cache
                    .get(&render_device, world_cache_active_cells_new_irradiance),
                world_cache_a: buffer_cache.get(&render_device, world_cache_a),
                world_cache_b: buffer_cache.get(&render_device, world_cache_b),
                world_cache_active_cell_indices: buffer_cache
                    .get(&render_device, world_cache_active_cell_indices),
                world_cache_active_cells_count: buffer_cache
                    .get(&render_device, world_cache_active_cells_count),
                world_cache_active_cells_dispatch: buffer_cache
                    .get(&render_device, world_cache_active_cells_dispatch),
            });
    }
}

pub fn create_bind_group_layouts(
    render_device: &RenderDevice,
) -> (BindGroupLayout, BindGroupLayout) {
    let mut entry_i = 0;
    let mut entry = |ty| {
        entry_i += 1;
        BindGroupLayoutEntry {
            binding: entry_i - 1,
            visibility: ShaderStages::COMPUTE,
            ty,
            count: None,
        }
    };

    let entries = &[
        // View
        entry(BindingType::Buffer {
            ty: BufferBindingType::Uniform,
            has_dynamic_offset: true,
            min_binding_size: Some(ViewUniform::min_size()),
        }),
        // Depth buffer
        entry(BindingType::Texture {
            sample_type: TextureSampleType::Depth,
            view_dimension: TextureViewDimension::D2,
            multisampled: false,
        }),
        // Normals buffer
        entry(BindingType::Texture {
            sample_type: TextureSampleType::Float { filterable: false },
            view_dimension: TextureViewDimension::D2,
            multisampled: false,
        }),
        // Motion vectors
        entry(BindingType::Texture {
            sample_type: TextureSampleType::Float { filterable: false },
            view_dimension: TextureViewDimension::D2,
            multisampled: false,
        }),
        // Depth buffer (previous)
        entry(BindingType::Texture {
            sample_type: TextureSampleType::Depth,
            view_dimension: TextureViewDimension::D2,
            multisampled: false,
        }),
        // Normals buffer (previous)
        entry(BindingType::Texture {
            sample_type: TextureSampleType::Float { filterable: false },
            view_dimension: TextureViewDimension::D2,
            multisampled: false,
        }),
        // Screen probes a
        entry(BindingType::StorageTexture {
            access: StorageTextureAccess::ReadWrite,
            format: TextureFormat::Rgba16Float,
            view_dimension: TextureViewDimension::D2Array,
        }),
        // Screen probes b
        entry(BindingType::StorageTexture {
            access: StorageTextureAccess::ReadWrite,
            format: TextureFormat::Rgba16Float,
            view_dimension: TextureViewDimension::D2,
        }),
        // Screen probe spherical harmonics
        entry(BindingType::Buffer {
            ty: BufferBindingType::Storage { read_only: false },
            has_dynamic_offset: false,
            min_binding_size: Some(unsafe { NonZeroU64::new_unchecked(112) }),
        }),
        // Diffuse raw
        entry(BindingType::StorageTexture {
            access: StorageTextureAccess::ReadWrite,
            format: TextureFormat::Rgba16Float,
            view_dimension: TextureViewDimension::D2,
        }),
        // Diffuse denoiser temporal history
        entry(BindingType::Texture {
            sample_type: TextureSampleType::Float { filterable: false },
            view_dimension: TextureViewDimension::D2,
            multisampled: false,
        }),
        // Diffuse denoised (temporal)
        entry(BindingType::StorageTexture {
            access: StorageTextureAccess::ReadWrite,
            format: TextureFormat::Rgba16Float,
            view_dimension: TextureViewDimension::D2,
        }),
        // Diffuse denoised (spatiotemporal)
        entry(BindingType::StorageTexture {
            access: StorageTextureAccess::ReadWrite,
            format: TextureFormat::Rgba16Float,
            view_dimension: TextureViewDimension::D2,
        }),
        // World cache checksums
        entry(BindingType::Buffer {
            ty: BufferBindingType::Storage { read_only: false },
            has_dynamic_offset: false,
            min_binding_size: Some(unsafe { NonZeroU64::new_unchecked(4) }),
        }),
        // World cache life
        entry(BindingType::Buffer {
            ty: BufferBindingType::Storage { read_only: false },
            has_dynamic_offset: false,
            min_binding_size: Some(unsafe { NonZeroU64::new_unchecked(4) }),
        }),
        // World cache irradiance
        entry(BindingType::Buffer {
            ty: BufferBindingType::Storage { read_only: false },
            has_dynamic_offset: false,
            min_binding_size: Some(unsafe { NonZeroU64::new_unchecked(16) }),
        }),
        // World cache cell data
        entry(BindingType::Buffer {
            ty: BufferBindingType::Storage { read_only: false },
            has_dynamic_offset: false,
            min_binding_size: Some(unsafe { NonZeroU64::new_unchecked(32) }),
        }),
        // World cache active cells new irradiance
        entry(BindingType::Buffer {
            ty: BufferBindingType::Storage { read_only: false },
            has_dynamic_offset: false,
            min_binding_size: Some(unsafe { NonZeroU64::new_unchecked(16) }),
        }),
        // World cache a
        entry(BindingType::Buffer {
            ty: BufferBindingType::Storage { read_only: false },
            has_dynamic_offset: false,
            min_binding_size: Some(unsafe { NonZeroU64::new_unchecked(4) }),
        }),
        // World cache b
        entry(BindingType::Buffer {
            ty: BufferBindingType::Storage { read_only: false },
            has_dynamic_offset: false,
            min_binding_size: Some(unsafe { NonZeroU64::new_unchecked(4) }),
        }),
        // World cache active cell indices
        entry(BindingType::Buffer {
            ty: BufferBindingType::Storage { read_only: false },
            has_dynamic_offset: false,
            min_binding_size: Some(unsafe { NonZeroU64::new_unchecked(4) }),
        }),
        // World cache active cells count
        entry(BindingType::Buffer {
            ty: BufferBindingType::Storage { read_only: false },
            has_dynamic_offset: false,
            min_binding_size: Some(unsafe { NonZeroU64::new_unchecked(4) }),
        }),
        // World cache active cells dispatch
        entry(BindingType::Buffer {
            ty: BufferBindingType::Storage { read_only: false },
            has_dynamic_offset: false,
            min_binding_size: Some(unsafe { NonZeroU64::new_unchecked(12) }),
        }),
    ];

    (
        render_device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            label: Some("solari_global_illumination_view_bind_group_layout"),
            entries: &entries[0..entries.len() - 1],
        }),
        render_device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            label: Some(
                "solari_global_illumination_view_with_world_cache_dispatch_bind_group_layout",
            ),
            entries,
        }),
    )
}

#[derive(Component)]
pub struct SolariGlobalIlluminationBindGroups {
    pub view_bind_group: BindGroup,
    pub view_with_world_cache_dispatch_bind_group: BindGroup,
}

pub fn prepare_bind_groups(
    views: Query<(
        Entity,
        &SolariGlobalIlluminationViewResources,
        &ViewPrepassTextures,
    )>,
    view_uniforms: Res<ViewUniforms>,
    pipelines: Res<SolariGlobalIlluminationPipelines>,
    mut commands: Commands,
    render_device: Res<RenderDevice>,
) {
    let Some(view_uniforms) = view_uniforms.uniforms.binding() else {
        return;
    };

    for (entity, solari_resources, prepass_textures) in &views {
        let mut entry_i = 0;
        let mut entry = |resource| {
            entry_i += 1;
            BindGroupEntry {
                binding: entry_i - 1,
                resource,
            }
        };

        let entries = &[
            entry(view_uniforms.clone()),
            entry(t(prepass_textures.depth.as_ref().unwrap())),
            entry(t(prepass_textures.normal.as_ref().unwrap())),
            entry(t(prepass_textures.motion_vectors.as_ref().unwrap())),
            entry(t(&solari_resources.previous_depth_buffer)),
            entry(t(&solari_resources.previous_normals_buffer)),
            entry(t(&solari_resources.screen_probes_a)),
            entry(t(&solari_resources.screen_probes_b)),
            entry(b(&solari_resources.screen_probes_spherical_harmonics)),
            entry(t(&solari_resources.diffuse_raw)),
            entry(t(&solari_resources.diffuse_denoiser_temporal_history)),
            entry(t(&solari_resources.diffuse_denoised_temporal)),
            entry(t(&solari_resources.diffuse_denoised_spatiotemporal)),
            entry(b(&solari_resources.world_cache_checksums)),
            entry(b(&solari_resources.world_cache_life)),
            entry(b(&solari_resources.world_cache_irradiance)),
            entry(b(&solari_resources.world_cache_cell_data)),
            entry(b(&solari_resources.world_cache_active_cells_new_irradiance)),
            entry(b(&solari_resources.world_cache_a)),
            entry(b(&solari_resources.world_cache_b)),
            entry(b(&solari_resources.world_cache_active_cell_indices)),
            entry(b(&solari_resources.world_cache_active_cells_count)),
            entry(b(&solari_resources.world_cache_active_cells_dispatch)),
        ];

        let bind_groups = SolariGlobalIlluminationBindGroups {
            view_bind_group: render_device.create_bind_group(&BindGroupDescriptor {
                label: Some("solari_global_illumination_view_bind_group"),
                layout: &pipelines.view_bind_group_layout,
                entries: &entries[0..entries.len() - 1],
            }),
            view_with_world_cache_dispatch_bind_group: render_device.create_bind_group(
                &BindGroupDescriptor {
                    label: Some(
                        "solari_global_illumination_view_with_world_cache_dispatch_bind_group",
                    ),
                    layout: &pipelines.view_with_world_cache_dispatch_bind_group_layout,
                    entries,
                },
            ),
        };
        commands.entity(entity).insert(bind_groups);
    }
}

fn round_up_to_multiple_of_8(x: u32) -> u32 {
    (x + 7) & !7
}

fn t(texture: &CachedTexture) -> BindingResource<'_> {
    BindingResource::TextureView(&texture.default_view)
}

fn b(buffer: &CachedBuffer) -> BindingResource<'_> {
    buffer.buffer.as_entire_binding()
}
