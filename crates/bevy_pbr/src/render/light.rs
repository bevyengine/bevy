use crate::{
    point_light_order, AmbientLight, Clusters, CubemapVisibleEntities, DirectionalLight,
    DirectionalLightShadowMap, DrawMesh, MeshPipeline, NotShadowCaster, PointLight,
    PointLightShadowMap, SetMeshBindGroup, VisiblePointLights,
    SHADOW_SHADER_HANDLE,
};
use bevy_asset::Handle;
use bevy_core::FloatOrd;
use bevy_core_pipeline::Transparent3d;
use bevy_ecs::{
    prelude::*,
    system::{lifetimeless::*, SystemParamItem},
};
use bevy_math::{const_vec3, Mat4, UVec3, UVec4, Vec2, Vec3, Vec4, Vec4Swizzles};
use bevy_render::{
    camera::{Camera, CameraProjection},
    color::Color,
    mesh::{Mesh, MeshVertexBufferLayout},
    render_asset::RenderAssets,
    render_graph::{Node, NodeRunError, RenderGraphContext, SlotInfo, SlotType},
    render_phase::{
        CachedPipelinePhaseItem, DrawFunctionId, DrawFunctions, EntityPhaseItem,
        EntityRenderCommand, PhaseItem, RenderCommandResult, RenderPhase, SetItemPipeline,
        TrackedRenderPass,
    },
    render_resource::{std140::AsStd140, *},
    renderer::{RenderContext, RenderDevice, RenderQueue},
    texture::*,
    view::{
        ExtractedView, ViewUniform, ViewUniformOffset, ViewUniforms, Visibility, VisibleEntities,
    },
};
use bevy_transform::components::GlobalTransform;
use bevy_utils::{
    tracing::{error, warn},
    HashMap,
};
use std::num::NonZeroU32;

#[derive(Debug, Hash, PartialEq, Eq, Clone, SystemLabel)]
pub enum RenderLightSystems {
    ExtractClusters,
    ExtractLights,
    PrepareClusters,
    PrepareLights,
    QueueShadows,
}

pub struct ExtractedAmbientLight {
    color: Color,
    brightness: f32,
}

#[derive(Component)]
pub struct ExtractedPointLight {
    color: Color,
    /// luminous intensity in lumens per steradian
    intensity: f32,
    range: f32,
    radius: f32,
    transform: GlobalTransform,
    shadows_enabled: bool,
    shadow_depth_bias: f32,
    shadow_normal_bias: f32,
}

pub type ExtractedPointLightShadowMap = PointLightShadowMap;

#[derive(Component)]
pub struct ExtractedDirectionalLight {
    color: Color,
    illuminance: f32,
    direction: Vec3,
    projection: Mat4,
    shadows_enabled: bool,
    shadow_depth_bias: f32,
    shadow_normal_bias: f32,
    near: f32,
    far: f32,
}

pub type ExtractedDirectionalLightShadowMap = DirectionalLightShadowMap;

#[repr(C)]
#[derive(Copy, Clone, AsStd140, Default, Debug)]
pub struct GpuPointLight {
    // The lower-right 2x2 values of the projection matrix 22 23 32 33
    projection_lr: Vec4,
    color_inverse_square_range: Vec4,
    position_radius: Vec4,
    flags: u32,
    shadow_depth_bias: f32,
    shadow_normal_bias: f32,
}

#[derive(AsStd140)]
pub struct GpuPointLights {
    data: [GpuPointLight; MAX_POINT_LIGHTS],
}

// NOTE: These must match the bit flags in bevy_pbr2/src/render/pbr.frag!
bitflags::bitflags! {
    #[repr(transparent)]
    struct PointLightFlags: u32 {
        const SHADOWS_ENABLED            = (1 << 0);
        const NONE                       = 0;
        const UNINITIALIZED              = 0xFFFF;
    }
}

#[repr(C)]
#[derive(Copy, Clone, AsStd140, Default, Debug)]
pub struct GpuDirectionalLight {
    view_projection: Mat4,
    color: Vec4,
    dir_to_light: Vec3,
    flags: u32,
    shadow_depth_bias: f32,
    shadow_normal_bias: f32,
}

// NOTE: These must match the bit flags in bevy_pbr2/src/render/pbr.frag!
bitflags::bitflags! {
    #[repr(transparent)]
    struct DirectionalLightFlags: u32 {
        const SHADOWS_ENABLED            = (1 << 0);
        const NONE                       = 0;
        const UNINITIALIZED              = 0xFFFF;
    }
}

#[repr(C)]
#[derive(Copy, Clone, Debug, AsStd140)]
pub struct GpuLights {
    // TODO: this comes first to work around a WGSL alignment issue. We need to solve this issue before releasing the renderer rework
    directional_lights: [GpuDirectionalLight; MAX_DIRECTIONAL_LIGHTS],
    ambient_color: Vec4,
    // xyz are x/y/z cluster dimensions and w is the number of clusters
    cluster_dimensions: UVec4,
    // xy are vec2<f32>(cluster_dimensions.xy) / vec2<f32>(view.width, view.height)
    // z is cluster_dimensions.z / log(far / near)
    // w is cluster_dimensions.z * log(near) / log(far / near)
    cluster_factors: Vec4,
    n_directional_lights: u32,
}

// NOTE: this must be kept in sync with the same constants in pbr.frag
pub const MAX_POINT_LIGHTS: usize = 256;
// FIXME: How should we handle shadows for clustered forward? Limiting to maximum 10
// point light shadow maps for now
#[cfg(feature = "webgl")]
pub const MAX_POINT_LIGHT_SHADOW_MAPS: usize = 1;
#[cfg(not(feature = "webgl"))]
pub const MAX_POINT_LIGHT_SHADOW_MAPS: usize = 10;
pub const MAX_DIRECTIONAL_LIGHTS: usize = 1;
pub const POINT_SHADOW_LAYERS: u32 = (6 * MAX_POINT_LIGHT_SHADOW_MAPS) as u32;
pub const DIRECTIONAL_SHADOW_LAYERS: u32 = MAX_DIRECTIONAL_LIGHTS as u32;
pub const SHADOW_FORMAT: TextureFormat = TextureFormat::Depth32Float;

pub struct ShadowPipeline {
    pub view_layout: BindGroupLayout,
    pub mesh_layout: BindGroupLayout,
    pub point_light_sampler: Sampler,
    pub directional_light_sampler: Sampler,
}

// TODO: this pattern for initializing the shaders / pipeline isn't ideal. this should be handled by the asset system
impl FromWorld for ShadowPipeline {
    fn from_world(world: &mut World) -> Self {
        let world = world.cell();
        let render_device = world.get_resource::<RenderDevice>().unwrap();

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
            ],
            label: Some("shadow_view_layout"),
        });

        let mesh_pipeline = world.get_resource::<MeshPipeline>().unwrap();

        ShadowPipeline {
            view_layout,
            mesh_layout: mesh_pipeline.mesh_layout.clone(),
            point_light_sampler: render_device.create_sampler(&SamplerDescriptor {
                address_mode_u: AddressMode::ClampToEdge,
                address_mode_v: AddressMode::ClampToEdge,
                address_mode_w: AddressMode::ClampToEdge,
                mag_filter: FilterMode::Linear,
                min_filter: FilterMode::Linear,
                mipmap_filter: FilterMode::Nearest,
                compare: Some(CompareFunction::GreaterEqual),
                ..Default::default()
            }),
            directional_light_sampler: render_device.create_sampler(&SamplerDescriptor {
                address_mode_u: AddressMode::ClampToEdge,
                address_mode_v: AddressMode::ClampToEdge,
                address_mode_w: AddressMode::ClampToEdge,
                mag_filter: FilterMode::Linear,
                min_filter: FilterMode::Linear,
                mipmap_filter: FilterMode::Nearest,
                compare: Some(CompareFunction::GreaterEqual),
                ..Default::default()
            }),
        }
    }
}

bitflags::bitflags! {
    #[repr(transparent)]
    pub struct ShadowPipelineKey: u32 {
        const NONE               = 0;
        const PRIMITIVE_TOPOLOGY_RESERVED_BITS = ShadowPipelineKey::PRIMITIVE_TOPOLOGY_MASK_BITS << ShadowPipelineKey::PRIMITIVE_TOPOLOGY_SHIFT_BITS;
    }
}

impl ShadowPipelineKey {
    const PRIMITIVE_TOPOLOGY_MASK_BITS: u32 = 0b111;
    const PRIMITIVE_TOPOLOGY_SHIFT_BITS: u32 = 32 - 3;

    pub fn from_primitive_topology(primitive_topology: PrimitiveTopology) -> Self {
        let primitive_topology_bits = ((primitive_topology as u32)
            & Self::PRIMITIVE_TOPOLOGY_MASK_BITS)
            << Self::PRIMITIVE_TOPOLOGY_SHIFT_BITS;
        Self::from_bits(primitive_topology_bits).unwrap()
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

impl SpecializedMeshPipeline for ShadowPipeline {
    type Key = ShadowPipelineKey;

    fn specialize(
        &self,
        key: Self::Key,
        layout: &MeshVertexBufferLayout,
    ) -> Result<RenderPipelineDescriptor, SpecializedMeshPipelineError> {
        let vertex_buffer_layout =
            layout.get_layout(&[Mesh::ATTRIBUTE_POSITION.at_shader_location(0)])?;

        Ok(RenderPipelineDescriptor {
            vertex: VertexState {
                shader: SHADOW_SHADER_HANDLE.typed::<Shader>(),
                entry_point: "vertex".into(),
                shader_defs: vec![],
                buffers: vec![vertex_buffer_layout],
            },
            fragment: None,
            layout: Some(vec![self.view_layout.clone(), self.mesh_layout.clone()]),
            primitive: PrimitiveState {
                topology: key.primitive_topology(),
                strip_index_format: None,
                front_face: FrontFace::Ccw,
                cull_mode: None,
                unclipped_depth: false,
                polygon_mode: PolygonMode::Fill,
                conservative: false,
            },
            depth_stencil: Some(DepthStencilState {
                format: SHADOW_FORMAT,
                depth_write_enabled: true,
                depth_compare: CompareFunction::GreaterEqual,
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
            multisample: MultisampleState::default(),
            label: Some("shadow_pipeline".into()),
        })
    }
}

#[derive(Component)]
pub struct ExtractedClusterConfig {
    /// Special near value for cluster calculations
    near: f32,
    far: f32,
    /// Number of clusters in x / y / z in the view frustum
    axis_slices: UVec3,
}

#[derive(Component)]
pub struct ExtractedClustersPointLights {
    data: Vec<VisiblePointLights>,
}

pub fn extract_clusters(mut commands: Commands, views: Query<(Entity, &Clusters), With<Camera>>) {
    for (entity, clusters) in views.iter() {
        commands.get_or_spawn(entity).insert_bundle((
            ExtractedClustersPointLights {
                data: clusters.lights.clone(),
            },
            ExtractedClusterConfig {
                near: clusters.near,
                far: clusters.far,
                axis_slices: clusters.axis_slices,
            },
        ));
    }
}

pub fn extract_lights(
    mut commands: Commands,
    ambient_light: Res<AmbientLight>,
    point_light_shadow_map: Res<PointLightShadowMap>,
    directional_light_shadow_map: Res<DirectionalLightShadowMap>,
    global_point_lights: Res<VisiblePointLights>,
    // visible_point_lights: Query<&VisiblePointLights>,
    mut point_lights: Query<(&PointLight, &mut CubemapVisibleEntities, &GlobalTransform)>,
    mut directional_lights: Query<(
        Entity,
        &DirectionalLight,
        &mut VisibleEntities,
        &GlobalTransform,
        &Visibility,
    )>,
) {
    commands.insert_resource(ExtractedAmbientLight {
        color: ambient_light.color,
        brightness: ambient_light.brightness,
    });
    commands.insert_resource::<ExtractedPointLightShadowMap>(point_light_shadow_map.clone());
    commands.insert_resource::<ExtractedDirectionalLightShadowMap>(
        directional_light_shadow_map.clone(),
    );
    // This is the point light shadow map texel size for one face of the cube as a distance of 1.0
    // world unit from the light.
    // point_light_texel_size = 2.0 * 1.0 * tan(PI / 4.0) / cube face width in texels
    // PI / 4.0 is half the cube face fov, tan(PI / 4.0) = 1.0, so this simplifies to:
    // point_light_texel_size = 2.0 / cube face width in texels
    // NOTE: When using various PCF kernel sizes, this will need to be adjusted, according to:
    // https://catlikecoding.com/unity/tutorials/custom-srp/point-and-spot-shadows/
    let point_light_texel_size = 2.0 / point_light_shadow_map.size as f32;

    for entity in global_point_lights.iter().copied() {
        if let Ok((point_light, cubemap_visible_entities, transform)) = point_lights.get_mut(entity)
        {
            let render_cubemap_visible_entities =
                std::mem::take(cubemap_visible_entities.into_inner());
            commands.get_or_spawn(entity).insert_bundle((
                ExtractedPointLight {
                    color: point_light.color,
                    // NOTE: Map from luminous power in lumens to luminous intensity in lumens per steradian
                    // for a point light. See https://google.github.io/filament/Filament.html#mjx-eqn-pointLightLuminousPower
                    // for details.
                    intensity: point_light.intensity / (4.0 * std::f32::consts::PI),
                    range: point_light.range,
                    radius: point_light.radius,
                    transform: *transform,
                    shadows_enabled: point_light.shadows_enabled,
                    shadow_depth_bias: point_light.shadow_depth_bias,
                    // The factor of SQRT_2 is for the worst-case diagonal offset
                    shadow_normal_bias: point_light.shadow_normal_bias
                        * point_light_texel_size
                        * std::f32::consts::SQRT_2,
                },
                render_cubemap_visible_entities,
            ));
        }
    }

    for (entity, directional_light, visible_entities, transform, visibility) in
        directional_lights.iter_mut()
    {
        if !visibility.is_visible {
            continue;
        }

        // Calulate the directional light shadow map texel size using the largest x,y dimension of
        // the orthographic projection divided by the shadow map resolution
        // NOTE: When using various PCF kernel sizes, this will need to be adjusted, according to:
        // https://catlikecoding.com/unity/tutorials/custom-srp/directional-shadows/
        let largest_dimension = (directional_light.shadow_projection.right
            - directional_light.shadow_projection.left)
            .max(
                directional_light.shadow_projection.top
                    - directional_light.shadow_projection.bottom,
            );
        let directional_light_texel_size =
            largest_dimension / directional_light_shadow_map.size as f32;
        let render_visible_entities = std::mem::take(visible_entities.into_inner());
        commands.get_or_spawn(entity).insert_bundle((
            ExtractedDirectionalLight {
                color: directional_light.color,
                illuminance: directional_light.illuminance,
                direction: transform.forward(),
                projection: directional_light.shadow_projection.get_projection_matrix(),
                shadows_enabled: directional_light.shadows_enabled,
                shadow_depth_bias: directional_light.shadow_depth_bias,
                // The factor of SQRT_2 is for the worst-case diagonal offset
                shadow_normal_bias: directional_light.shadow_normal_bias
                    * directional_light_texel_size
                    * std::f32::consts::SQRT_2,
                near: directional_light.shadow_projection.near,
                far: directional_light.shadow_projection.far,
            },
            render_visible_entities,
        ));
    }
}

pub(crate) const POINT_LIGHT_NEAR_Z: f32 = 0.1f32;

// Can't do `Vec3::Y * -1.0` because mul isn't const
const NEGATIVE_X: Vec3 = const_vec3!([-1.0, 0.0, 0.0]);
const NEGATIVE_Y: Vec3 = const_vec3!([0.0, -1.0, 0.0]);
const NEGATIVE_Z: Vec3 = const_vec3!([0.0, 0.0, -1.0]);

pub(crate) struct CubeMapFace {
    pub(crate) target: Vec3,
    pub(crate) up: Vec3,
}

// see https://www.khronos.org/opengl/wiki/Cubemap_Texture
pub(crate) const CUBE_MAP_FACES: [CubeMapFace; 6] = [
    // 0 	GL_TEXTURE_CUBE_MAP_POSITIVE_X
    CubeMapFace {
        target: NEGATIVE_X,
        up: NEGATIVE_Y,
    },
    // 1 	GL_TEXTURE_CUBE_MAP_NEGATIVE_X
    CubeMapFace {
        target: Vec3::X,
        up: NEGATIVE_Y,
    },
    // 2 	GL_TEXTURE_CUBE_MAP_POSITIVE_Y
    CubeMapFace {
        target: NEGATIVE_Y,
        up: Vec3::Z,
    },
    // 3 	GL_TEXTURE_CUBE_MAP_NEGATIVE_Y
    CubeMapFace {
        target: Vec3::Y,
        up: NEGATIVE_Z,
    },
    // 4 	GL_TEXTURE_CUBE_MAP_POSITIVE_Z
    CubeMapFace {
        target: NEGATIVE_Z,
        up: NEGATIVE_Y,
    },
    // 5 	GL_TEXTURE_CUBE_MAP_NEGATIVE_Z
    CubeMapFace {
        target: Vec3::Z,
        up: NEGATIVE_Y,
    },
];

fn face_index_to_name(face_index: usize) -> &'static str {
    match face_index {
        0 => "+x",
        1 => "-x",
        2 => "+y",
        3 => "-y",
        4 => "+z",
        5 => "-z",
        _ => "invalid",
    }
}

#[derive(Component)]
pub struct ShadowView {
    pub depth_texture_view: TextureView,
    pub pass_name: String,
}

#[derive(Component)]
pub struct ViewShadowBindings {
    pub point_light_depth_texture: Texture,
    pub point_light_depth_texture_view: TextureView,
    pub directional_light_depth_texture: Texture,
    pub directional_light_depth_texture_view: TextureView,
}

#[derive(Component)]
pub struct ViewLightEntities {
    pub lights: Vec<Entity>,
}

#[derive(Component)]
pub struct ViewLightsUniformOffset {
    pub offset: u32,
}

#[derive(Default)]
pub struct GlobalLightMeta {
    pub gpu_point_lights: UniformVec<GpuPointLights>,
    pub entity_to_index: HashMap<Entity, usize>,
}

#[derive(Default)]
pub struct LightMeta {
    pub view_gpu_lights: DynamicUniformVec<GpuLights>,
    pub shadow_view_bind_group: Option<BindGroup>,
}

#[derive(Component)]
pub enum LightEntity {
    Directional {
        light_entity: Entity,
    },
    Point {
        light_entity: Entity,
        face_index: usize,
    },
}
pub fn calculate_cluster_factors(
    near: f32,
    far: f32,
    z_slices: f32,
    is_orthographic: bool,
) -> Vec2 {
    if is_orthographic {
        Vec2::new(-near, z_slices / (-far - -near))
    } else {
        let z_slices_of_ln_zfar_over_znear = (z_slices - 1.0) / (far / near).ln();
        Vec2::new(
            z_slices_of_ln_zfar_over_znear,
            near.ln() * z_slices_of_ln_zfar_over_znear,
        )
    }
}

#[allow(clippy::too_many_arguments)]
pub fn prepare_lights(
    mut commands: Commands,
    mut texture_cache: ResMut<TextureCache>,
    render_device: Res<RenderDevice>,
    render_queue: Res<RenderQueue>,
    mut global_light_meta: ResMut<GlobalLightMeta>,
    mut light_meta: ResMut<LightMeta>,
    views: Query<
        (Entity, &ExtractedView, &ExtractedClusterConfig),
        With<RenderPhase<Transparent3d>>,
    >,
    ambient_light: Res<ExtractedAmbientLight>,
    point_light_shadow_map: Res<ExtractedPointLightShadowMap>,
    directional_light_shadow_map: Res<ExtractedDirectionalLightShadowMap>,
    point_lights: Query<(Entity, &ExtractedPointLight)>,
    directional_lights: Query<(Entity, &ExtractedDirectionalLight)>,
) {
    light_meta.view_gpu_lights.clear();

    // Pre-calculate for PointLights
    let cube_face_projection =
        Mat4::perspective_infinite_reverse_rh(std::f32::consts::FRAC_PI_2, 1.0, POINT_LIGHT_NEAR_Z);
    let cube_face_rotations = CUBE_MAP_FACES
        .iter()
        .map(|CubeMapFace { target, up }| GlobalTransform::identity().looking_at(*target, *up))
        .collect::<Vec<_>>();

    global_light_meta.gpu_point_lights.clear();
    global_light_meta.entity_to_index.clear();

    let mut point_lights: Vec<_> = point_lights.iter().collect::<Vec<_>>();

    // Sort point lights with shadows enabled first, then by a stable key so that the index can be used
    // to render at most `MAX_POINT_LIGHT_SHADOW_MAPS` point light shadows.
    point_lights.sort_by(|(entity_1, light_1), (entity_2, light_2)| {
        point_light_order(
            (entity_1, &light_1.shadows_enabled),
            (entity_2, &light_2.shadows_enabled),
        )
    });

    if global_light_meta.entity_to_index.capacity() < point_lights.len() {
        global_light_meta
            .entity_to_index
            .reserve(point_lights.len());
    }

    let mut gpu_point_lights = [GpuPointLight::default(); MAX_POINT_LIGHTS];
    for (index, &(entity, light)) in point_lights.iter().enumerate() {
        let mut flags = PointLightFlags::NONE;
        // Lights are sorted, shadow enabled lights are first
        if light.shadows_enabled && index < MAX_POINT_LIGHT_SHADOW_MAPS {
            flags |= PointLightFlags::SHADOWS_ENABLED;
        }
        gpu_point_lights[index] = GpuPointLight {
            projection_lr: Vec4::new(
                cube_face_projection.z_axis.z,
                cube_face_projection.z_axis.w,
                cube_face_projection.w_axis.z,
                cube_face_projection.w_axis.w,
            ),
            // premultiply color by intensity
            // we don't use the alpha at all, so no reason to multiply only [0..3]
            color_inverse_square_range: (Vec4::from_slice(&light.color.as_linear_rgba_f32())
                * light.intensity)
                .xyz()
                .extend(1.0 / (light.range * light.range)),
            position_radius: light.transform.translation.extend(light.radius),
            flags: flags.bits,
            shadow_depth_bias: light.shadow_depth_bias,
            shadow_normal_bias: light.shadow_normal_bias,
        };
        global_light_meta.entity_to_index.insert(entity, index);
    }
    global_light_meta.gpu_point_lights.push(GpuPointLights {
        data: gpu_point_lights,
    });
    global_light_meta
        .gpu_point_lights
        .write_buffer(&render_device, &render_queue);

    // set up light data for each view
    for (entity, extracted_view, clusters) in views.iter() {
        let point_light_depth_texture = texture_cache.get(
            &render_device,
            TextureDescriptor {
                size: Extent3d {
                    width: point_light_shadow_map.size as u32,
                    height: point_light_shadow_map.size as u32,
                    depth_or_array_layers: POINT_SHADOW_LAYERS,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: TextureDimension::D2,
                format: SHADOW_FORMAT,
                label: Some("point_light_shadow_map_texture"),
                usage: TextureUsages::RENDER_ATTACHMENT | TextureUsages::TEXTURE_BINDING,
            },
        );
        let directional_light_depth_texture = texture_cache.get(
            &render_device,
            TextureDescriptor {
                size: Extent3d {
                    width: (directional_light_shadow_map.size as u32)
                        .min(render_device.limits().max_texture_dimension_2d),
                    height: (directional_light_shadow_map.size as u32)
                        .min(render_device.limits().max_texture_dimension_2d),
                    depth_or_array_layers: DIRECTIONAL_SHADOW_LAYERS,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: TextureDimension::D2,
                format: SHADOW_FORMAT,
                label: Some("directional_light_shadow_map_texture"),
                usage: TextureUsages::RENDER_ATTACHMENT | TextureUsages::TEXTURE_BINDING,
            },
        );
        let mut view_lights = Vec::new();

        let is_orthographic = extracted_view.projection.w_axis.w == 1.0;
        let cluster_factors_zw = calculate_cluster_factors(
            clusters.near,
            clusters.far,
            clusters.axis_slices.z as f32,
            is_orthographic,
        );

        let n_clusters = clusters.axis_slices.x * clusters.axis_slices.y * clusters.axis_slices.z;
        let mut gpu_lights = GpuLights {
            directional_lights: [GpuDirectionalLight::default(); MAX_DIRECTIONAL_LIGHTS],
            ambient_color: Vec4::from_slice(&ambient_light.color.as_linear_rgba_f32())
                * ambient_light.brightness,
            cluster_factors: Vec4::new(
                clusters.axis_slices.x as f32 / extracted_view.width as f32,
                clusters.axis_slices.y as f32 / extracted_view.height as f32,
                cluster_factors_zw.x,
                cluster_factors_zw.y,
            ),
            cluster_dimensions: clusters.axis_slices.extend(n_clusters),
            n_directional_lights: directional_lights.iter().len() as u32,
        };

        // TODO: this should select lights based on relevance to the view instead of the first ones that show up in a query
        for &(light_entity, light) in point_lights
            .iter()
            // Lights are sorted, shadow enabled lights are first
            .take(MAX_POINT_LIGHT_SHADOW_MAPS)
            .filter(|(_, light)| light.shadows_enabled)
        {
            let light_index = *global_light_meta
                .entity_to_index
                .get(&light_entity)
                .unwrap();
            // ignore scale because we don't want to effectively scale light radius and range
            // by applying those as a view transform to shadow map rendering of objects
            // and ignore rotation because we want the shadow map projections to align with the axes
            let view_translation = GlobalTransform::from_translation(light.transform.translation);

            for (face_index, view_rotation) in cube_face_rotations.iter().enumerate() {
                let depth_texture_view =
                    point_light_depth_texture
                        .texture
                        .create_view(&TextureViewDescriptor {
                            label: Some("point_light_shadow_map_texture_view"),
                            format: None,
                            dimension: Some(TextureViewDimension::D2),
                            aspect: TextureAspect::All,
                            base_mip_level: 0,
                            mip_level_count: None,
                            base_array_layer: (light_index * 6 + face_index) as u32,
                            array_layer_count: NonZeroU32::new(1),
                        });

                let view_light_entity = commands
                    .spawn()
                    .insert_bundle((
                        ShadowView {
                            depth_texture_view,
                            pass_name: format!(
                                "shadow pass point light {} {}",
                                light_index,
                                face_index_to_name(face_index)
                            ),
                        },
                        ExtractedView {
                            width: point_light_shadow_map.size as u32,
                            height: point_light_shadow_map.size as u32,
                            transform: view_translation * *view_rotation,
                            projection: cube_face_projection,
                            near: POINT_LIGHT_NEAR_Z,
                            far: light.range,
                        },
                        RenderPhase::<Shadow>::default(),
                        LightEntity::Point {
                            light_entity,
                            face_index,
                        },
                    ))
                    .id();
                view_lights.push(view_light_entity);
            }
        }

        for (i, (light_entity, light)) in directional_lights
            .iter()
            .enumerate()
            .take(MAX_DIRECTIONAL_LIGHTS)
        {
            // direction is negated to be ready for N.L
            let dir_to_light = -light.direction;

            // convert from illuminance (lux) to candelas
            //
            // exposure is hard coded at the moment but should be replaced
            // by values coming from the camera
            // see: https://google.github.io/filament/Filament.html#imagingpipeline/physicallybasedcamera/exposuresettings
            const APERTURE: f32 = 4.0;
            const SHUTTER_SPEED: f32 = 1.0 / 250.0;
            const SENSITIVITY: f32 = 100.0;
            let ev100 =
                f32::log2(APERTURE * APERTURE / SHUTTER_SPEED) - f32::log2(SENSITIVITY / 100.0);
            let exposure = 1.0 / (f32::powf(2.0, ev100) * 1.2);
            let intensity = light.illuminance * exposure;

            // NOTE: A directional light seems to have to have an eye position on the line along the direction of the light
            // through the world origin. I (Rob Swain) do not yet understand why it cannot be translated away from this.
            let view = Mat4::look_at_rh(Vec3::ZERO, light.direction, Vec3::Y);
            // NOTE: This orthographic projection defines the volume within which shadows from a directional light can be cast
            let projection = light.projection;

            let mut flags = DirectionalLightFlags::NONE;
            if light.shadows_enabled {
                flags |= DirectionalLightFlags::SHADOWS_ENABLED;
            }

            gpu_lights.directional_lights[i] = GpuDirectionalLight {
                // premultiply color by intensity
                // we don't use the alpha at all, so no reason to multiply only [0..3]
                color: Vec4::from_slice(&light.color.as_linear_rgba_f32()) * intensity,
                dir_to_light,
                // NOTE: * view is correct, it should not be view.inverse() here
                view_projection: projection * view,
                flags: flags.bits,
                shadow_depth_bias: light.shadow_depth_bias,
                shadow_normal_bias: light.shadow_normal_bias,
            };

            if light.shadows_enabled {
                let depth_texture_view =
                    directional_light_depth_texture
                        .texture
                        .create_view(&TextureViewDescriptor {
                            label: Some("directional_light_shadow_map_texture_view"),
                            format: None,
                            dimension: Some(TextureViewDimension::D2),
                            aspect: TextureAspect::All,
                            base_mip_level: 0,
                            mip_level_count: None,
                            base_array_layer: i as u32,
                            array_layer_count: NonZeroU32::new(1),
                        });

                let view_light_entity = commands
                    .spawn()
                    .insert_bundle((
                        ShadowView {
                            depth_texture_view,
                            pass_name: format!("shadow pass directional light {}", i),
                        },
                        ExtractedView {
                            width: directional_light_shadow_map.size as u32,
                            height: directional_light_shadow_map.size as u32,
                            transform: GlobalTransform::from_matrix(view.inverse()),
                            projection,
                            near: light.near,
                            far: light.far,
                        },
                        RenderPhase::<Shadow>::default(),
                        LightEntity::Directional { light_entity },
                    ))
                    .id();
                view_lights.push(view_light_entity);
            }
        }
        let point_light_depth_texture_view =
            point_light_depth_texture
                .texture
                .create_view(&TextureViewDescriptor {
                    label: Some("point_light_shadow_map_array_texture_view"),
                    format: None,
                    #[cfg(not(feature = "webgl"))]
                    dimension: Some(TextureViewDimension::CubeArray),
                    #[cfg(feature = "webgl")]
                    dimension: Some(TextureViewDimension::Cube),
                    aspect: TextureAspect::All,
                    base_mip_level: 0,
                    mip_level_count: None,
                    base_array_layer: 0,
                    array_layer_count: None,
                });
        let directional_light_depth_texture_view = directional_light_depth_texture
            .texture
            .create_view(&TextureViewDescriptor {
                label: Some("directional_light_shadow_map_array_texture_view"),
                format: None,
                #[cfg(not(feature = "webgl"))]
                dimension: Some(TextureViewDimension::D2Array),
                #[cfg(feature = "webgl")]
                dimension: Some(TextureViewDimension::D2),
                aspect: TextureAspect::All,
                base_mip_level: 0,
                mip_level_count: None,
                base_array_layer: 0,
                array_layer_count: None,
            });

        commands.entity(entity).insert_bundle((
            ViewShadowBindings {
                point_light_depth_texture: point_light_depth_texture.texture,
                point_light_depth_texture_view,
                directional_light_depth_texture: directional_light_depth_texture.texture,
                directional_light_depth_texture_view,
            },
            ViewLightEntities {
                lights: view_lights,
            },
            ViewLightsUniformOffset {
                offset: light_meta.view_gpu_lights.push(gpu_lights),
            },
        ));
    }

    light_meta
        .view_gpu_lights
        .write_buffer(&render_device, &render_queue);
}

// this must match CLUSTER_COUNT_SIZE in pbr.wgsl
// and must be large enough to contain MAX_POINT_LIGHTS
const CLUSTER_COUNT_SIZE: u32 = 13;

const CLUSTER_OFFSET_MASK: u32 = (1 << (32 - CLUSTER_COUNT_SIZE)) - 1;
const CLUSTER_COUNT_MASK: u32 = (1 << CLUSTER_COUNT_SIZE) - 1;
const POINT_LIGHT_INDEX_MASK: u32 = (1 << 8) - 1;

// NOTE: With uniform buffer max binding size as 16384 bytes
// that means we can fit say 256 point lights in one uniform
// buffer, which means the count can be at most 256 so it
// needs 9 bits.
// The array of indices can also use u8 and that means the
// offset in to the array of indices needs to be able to address
// 16384 values. log2(16384) = 14 bits.
// We use 32 bits to store the pair, so we choose to divide the
// remaining 9 bits proportionally to give some future room.
// This means we can pack the offset into the upper 19 bits of a u32
// and the count into the lower 13 bits.
// NOTE: This assumes CPU and GPU endianness are the same which is true
// for all common and tested x86/ARM CPUs and AMD/NVIDIA/Intel/Apple/etc GPUs
fn pack_offset_and_count(offset: usize, count: usize) -> u32 {
    ((offset as u32 & CLUSTER_OFFSET_MASK) << CLUSTER_COUNT_SIZE)
        | (count as u32 & CLUSTER_COUNT_MASK)
}

#[derive(Component, Default)]
pub struct ViewClusterBindings {
    n_indices: usize,
    // NOTE: UVec4 is because all arrays in Std140 layout have 16-byte alignment
    pub cluster_light_index_lists: UniformVec<[UVec4; Self::MAX_UNIFORM_ITEMS]>,
    n_offsets: usize,
    // NOTE: UVec4 is because all arrays in Std140 layout have 16-byte alignment
    pub cluster_offsets_and_counts: UniformVec<[UVec4; Self::MAX_UNIFORM_ITEMS]>,
}

impl ViewClusterBindings {
    pub const MAX_OFFSETS: usize = 16384 / 4;
    const MAX_UNIFORM_ITEMS: usize = Self::MAX_OFFSETS / 4;
    pub const MAX_INDICES: usize = 16384;

    pub fn reserve_and_clear(&mut self) {
        self.cluster_light_index_lists.clear();
        self.cluster_light_index_lists
            .push([UVec4::ZERO; Self::MAX_UNIFORM_ITEMS]);
        self.cluster_offsets_and_counts.clear();
        self.cluster_offsets_and_counts
            .push([UVec4::ZERO; Self::MAX_UNIFORM_ITEMS]);
    }

    pub fn push_offset_and_count(&mut self, offset: usize, count: usize) {
        let array_index = self.n_offsets >> 2; // >> 2 is equivalent to / 4
        if array_index >= Self::MAX_UNIFORM_ITEMS {
            warn!("cluster offset and count out of bounds!");
            return;
        }
        let component = self.n_offsets & ((1 << 2) - 1);
        let packed = pack_offset_and_count(offset, count);

        self.cluster_offsets_and_counts.get_mut(0)[array_index][component] = packed;

        self.n_offsets += 1;
    }

    pub fn n_indices(&self) -> usize {
        self.n_indices
    }

    pub fn push_index(&mut self, index: usize) {
        let array_index = self.n_indices >> 4; // >> 4 is equivalent to / 16
        let component = (self.n_indices >> 2) & ((1 << 2) - 1);
        let sub_index = self.n_indices & ((1 << 2) - 1);
        let index = index as u32 & POINT_LIGHT_INDEX_MASK;

        self.cluster_light_index_lists.get_mut(0)[array_index][component] |=
            index << (8 * sub_index);

        self.n_indices += 1;
    }
}

pub fn prepare_clusters(
    mut commands: Commands,
    render_device: Res<RenderDevice>,
    render_queue: Res<RenderQueue>,
    global_light_meta: Res<GlobalLightMeta>,
    views: Query<
        (
            Entity,
            &ExtractedClusterConfig,
            &ExtractedClustersPointLights,
        ),
        With<RenderPhase<Transparent3d>>,
    >,
) {
    for (entity, cluster_config, extracted_clusters) in views.iter() {
        let mut view_clusters_bindings = ViewClusterBindings::default();
        view_clusters_bindings.reserve_and_clear();

        let mut indices_full = false;

        let mut cluster_index = 0;
        for _y in 0..cluster_config.axis_slices.y {
            for _x in 0..cluster_config.axis_slices.x {
                for _z in 0..cluster_config.axis_slices.z {
                    let offset = view_clusters_bindings.n_indices();
                    let cluster_lights = &extracted_clusters.data[cluster_index];
                    let count = cluster_lights.len();
                    view_clusters_bindings.push_offset_and_count(offset, count);

                    if !indices_full {
                        for entity in cluster_lights.iter() {
                            if let Some(light_index) = global_light_meta.entity_to_index.get(entity)
                            {
                                if view_clusters_bindings.n_indices()
                                    >= ViewClusterBindings::MAX_INDICES
                                {
                                    warn!("Cluster light index lists is full! The PointLights in the view are affecting too many clusters.");
                                    indices_full = true;
                                    break;
                                }
                                view_clusters_bindings.push_index(*light_index);
                            }
                        }
                    }

                    cluster_index += 1;
                }
            }
        }

        view_clusters_bindings
            .cluster_light_index_lists
            .write_buffer(&render_device, &render_queue);
        view_clusters_bindings
            .cluster_offsets_and_counts
            .write_buffer(&render_device, &render_queue);

        commands.get_or_spawn(entity).insert(view_clusters_bindings);
    }
}

pub fn queue_shadow_view_bind_group(
    render_device: Res<RenderDevice>,
    shadow_pipeline: Res<ShadowPipeline>,
    mut light_meta: ResMut<LightMeta>,
    view_uniforms: Res<ViewUniforms>,
) {
    if let Some(view_binding) = view_uniforms.uniforms.binding() {
        light_meta.shadow_view_bind_group =
            Some(render_device.create_bind_group(&BindGroupDescriptor {
                entries: &[BindGroupEntry {
                    binding: 0,
                    resource: view_binding,
                }],
                label: Some("shadow_view_bind_group"),
                layout: &shadow_pipeline.view_layout,
            }));
    }
}

#[allow(clippy::too_many_arguments)]
pub fn queue_shadows(
    shadow_draw_functions: Res<DrawFunctions<Shadow>>,
    shadow_pipeline: Res<ShadowPipeline>,
    casting_meshes: Query<&Handle<Mesh>, Without<NotShadowCaster>>,
    render_meshes: Res<RenderAssets<Mesh>>,
    mut pipelines: ResMut<SpecializedMeshPipelines<ShadowPipeline>>,
    mut pipeline_cache: ResMut<RenderPipelineCache>,
    view_lights: Query<&ViewLightEntities>,
    mut view_light_shadow_phases: Query<(&LightEntity, &mut RenderPhase<Shadow>)>,
    point_light_entities: Query<&CubemapVisibleEntities, With<ExtractedPointLight>>,
    directional_light_entities: Query<&VisibleEntities, With<ExtractedDirectionalLight>>,
) {
    for view_lights in view_lights.iter() {
        let draw_shadow_mesh = shadow_draw_functions
            .read()
            .get_id::<DrawShadowMesh>()
            .unwrap();
        for view_light_entity in view_lights.lights.iter().copied() {
            let (light_entity, mut shadow_phase) =
                view_light_shadow_phases.get_mut(view_light_entity).unwrap();
            let visible_entities = match light_entity {
                LightEntity::Directional { light_entity } => directional_light_entities
                    .get(*light_entity)
                    .expect("Failed to get directional light visible entities"),
                LightEntity::Point {
                    light_entity,
                    face_index,
                } => point_light_entities
                    .get(*light_entity)
                    .expect("Failed to get point light visible entities")
                    .get(*face_index),
            };
            // NOTE: Lights with shadow mapping disabled will have no visible entities
            // so no meshes will be queued
            for entity in visible_entities.iter().copied() {
                if let Ok(mesh_handle) = casting_meshes.get(entity) {
                    if let Some(mesh) = render_meshes.get(mesh_handle) {
                        let key =
                            ShadowPipelineKey::from_primitive_topology(mesh.primitive_topology);
                        let pipeline_id = pipelines.specialize(
                            &mut pipeline_cache,
                            &shadow_pipeline,
                            key,
                            &mesh.layout,
                        );

                        let pipeline_id = match pipeline_id {
                            Ok(id) => id,
                            Err(err) => {
                                error!("{}", err);
                                continue;
                            }
                        };

                        shadow_phase.add(Shadow {
                            draw_function: draw_shadow_mesh,
                            pipeline: pipeline_id,
                            entity,
                            distance: 0.0, // TODO: sort back-to-front
                        });
                    }
                }
            }
        }
    }
}

pub struct Shadow {
    pub distance: f32,
    pub entity: Entity,
    pub pipeline: CachedPipelineId,
    pub draw_function: DrawFunctionId,
}

impl PhaseItem for Shadow {
    type SortKey = FloatOrd;

    #[inline]
    fn sort_key(&self) -> Self::SortKey {
        FloatOrd(self.distance)
    }

    #[inline]
    fn draw_function(&self) -> DrawFunctionId {
        self.draw_function
    }
}

impl EntityPhaseItem for Shadow {
    fn entity(&self) -> Entity {
        self.entity
    }
}

impl CachedPipelinePhaseItem for Shadow {
    #[inline]
    fn cached_pipeline(&self) -> CachedPipelineId {
        self.pipeline
    }
}

pub struct ShadowPassNode {
    main_view_query: QueryState<&'static ViewLightEntities>,
    view_light_query: QueryState<(&'static ShadowView, &'static RenderPhase<Shadow>)>,
}

impl ShadowPassNode {
    pub const IN_VIEW: &'static str = "view";

    pub fn new(world: &mut World) -> Self {
        Self {
            main_view_query: QueryState::new(world),
            view_light_query: QueryState::new(world),
        }
    }
}

impl Node for ShadowPassNode {
    fn input(&self) -> Vec<SlotInfo> {
        vec![SlotInfo::new(ShadowPassNode::IN_VIEW, SlotType::Entity)]
    }

    fn update(&mut self, world: &mut World) {
        self.main_view_query.update_archetypes(world);
        self.view_light_query.update_archetypes(world);
    }

    fn run(
        &self,
        graph: &mut RenderGraphContext,
        render_context: &mut RenderContext,
        world: &World,
    ) -> Result<(), NodeRunError> {
        let view_entity = graph.get_input_entity(Self::IN_VIEW)?;
        if let Ok(view_lights) = self.main_view_query.get_manual(world, view_entity) {
            for view_light_entity in view_lights.lights.iter().copied() {
                let (view_light, shadow_phase) = self
                    .view_light_query
                    .get_manual(world, view_light_entity)
                    .unwrap();
                let pass_descriptor = RenderPassDescriptor {
                    label: Some(&view_light.pass_name),
                    color_attachments: &[],
                    depth_stencil_attachment: Some(RenderPassDepthStencilAttachment {
                        view: &view_light.depth_texture_view,
                        depth_ops: Some(Operations {
                            load: LoadOp::Clear(0.0),
                            store: true,
                        }),
                        stencil_ops: None,
                    }),
                };

                let draw_functions = world.resource::<DrawFunctions<Shadow>>();
                let render_pass = render_context
                    .command_encoder
                    .begin_render_pass(&pass_descriptor);
                let mut draw_functions = draw_functions.write();
                let mut tracked_pass = TrackedRenderPass::new(render_pass);
                for item in &shadow_phase.items {
                    let draw_function = draw_functions.get_mut(item.draw_function).unwrap();
                    draw_function.draw(world, &mut tracked_pass, view_light_entity, item);
                }
            }
        }

        Ok(())
    }
}

pub type DrawShadowMesh = (
    SetItemPipeline,
    SetShadowViewBindGroup<0>,
    SetMeshBindGroup<1>,
    DrawMesh,
);

pub struct SetShadowViewBindGroup<const I: usize>;
impl<const I: usize> EntityRenderCommand for SetShadowViewBindGroup<I> {
    type Param = (SRes<LightMeta>, SQuery<Read<ViewUniformOffset>>);
    #[inline]
    fn render<'w>(
        view: Entity,
        _item: Entity,
        (light_meta, view_query): SystemParamItem<'w, '_, Self::Param>,
        pass: &mut TrackedRenderPass<'w>,
    ) -> RenderCommandResult {
        let view_uniform_offset = view_query.get(view).unwrap();
        pass.set_bind_group(
            I,
            light_meta
                .into_inner()
                .shadow_view_bind_group
                .as_ref()
                .unwrap(),
            &[view_uniform_offset.offset],
        );

        RenderCommandResult::Success
    }
}
