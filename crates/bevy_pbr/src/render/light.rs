use crate::{
    directional_light_order, point_light_order, AlphaMode, AmbientLight, Cascade,
    CascadeShadowConfig, Cascades, CascadesVisibleEntities, Clusters, CubemapVisibleEntities,
    DirectionalLight, DirectionalLightShadowMap, DrawPrepass, EnvironmentMapLight,
    GlobalVisiblePointLights, Material, MaterialPipelineKey, MeshPipeline, MeshPipelineKey,
    PointLight, PointLightShadowMap, PrepassPipeline, RenderMaterialInstances, RenderMaterials,
    RenderMeshInstances, SpotLight, VisiblePointLights,
};
use bevy_core_pipeline::core_3d::Transparent3d;
use bevy_ecs::prelude::*;
use bevy_math::{Mat4, UVec3, UVec4, Vec2, Vec3, Vec3Swizzles, Vec4, Vec4Swizzles};
use bevy_render::{
    camera::Camera,
    color::Color,
    mesh::Mesh,
    render_asset::RenderAssets,
    render_graph::{Node, NodeRunError, RenderGraphContext},
    render_phase::{
        CachedRenderPipelinePhaseItem, DrawFunctionId, DrawFunctions, PhaseItem, RenderPhase,
    },
    render_resource::*,
    renderer::{RenderContext, RenderDevice, RenderQueue},
    texture::*,
    view::{ExtractedView, ViewVisibility, VisibleEntities},
    Extract,
};
use bevy_transform::{components::GlobalTransform, prelude::Transform};
use bevy_utils::{
    nonmax::NonMaxU32,
    tracing::{error, warn},
    HashMap,
};
use std::{hash::Hash, num::NonZeroU64, ops::Range};

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
    spot_light_angles: Option<(f32, f32)>,
}

#[derive(Component, Debug)]
pub struct ExtractedDirectionalLight {
    color: Color,
    illuminance: f32,
    transform: GlobalTransform,
    shadows_enabled: bool,
    shadow_depth_bias: f32,
    shadow_normal_bias: f32,
    cascade_shadow_config: CascadeShadowConfig,
    cascades: HashMap<Entity, Vec<Cascade>>,
}

#[derive(Copy, Clone, ShaderType, Default, Debug)]
pub struct GpuPointLight {
    // For point lights: the lower-right 2x2 values of the projection matrix [2][2] [2][3] [3][2] [3][3]
    // For spot lights: 2 components of the direction (x,z), spot_scale and spot_offset
    light_custom_data: Vec4,
    color_inverse_square_range: Vec4,
    position_radius: Vec4,
    flags: u32,
    shadow_depth_bias: f32,
    shadow_normal_bias: f32,
    spot_light_tan_angle: f32,
}

#[derive(ShaderType)]
pub struct GpuPointLightsUniform {
    data: Box<[GpuPointLight; MAX_UNIFORM_BUFFER_POINT_LIGHTS]>,
}

impl Default for GpuPointLightsUniform {
    fn default() -> Self {
        Self {
            data: Box::new([GpuPointLight::default(); MAX_UNIFORM_BUFFER_POINT_LIGHTS]),
        }
    }
}

#[derive(ShaderType, Default)]
pub struct GpuPointLightsStorage {
    #[size(runtime)]
    data: Vec<GpuPointLight>,
}

pub enum GpuPointLights {
    Uniform(UniformBuffer<GpuPointLightsUniform>),
    Storage(StorageBuffer<GpuPointLightsStorage>),
}

impl GpuPointLights {
    fn new(buffer_binding_type: BufferBindingType) -> Self {
        match buffer_binding_type {
            BufferBindingType::Storage { .. } => Self::storage(),
            BufferBindingType::Uniform => Self::uniform(),
        }
    }

    fn uniform() -> Self {
        Self::Uniform(UniformBuffer::default())
    }

    fn storage() -> Self {
        Self::Storage(StorageBuffer::default())
    }

    fn set(&mut self, mut lights: Vec<GpuPointLight>) {
        match self {
            GpuPointLights::Uniform(buffer) => {
                let len = lights.len().min(MAX_UNIFORM_BUFFER_POINT_LIGHTS);
                let src = &lights[..len];
                let dst = &mut buffer.get_mut().data[..len];
                dst.copy_from_slice(src);
            }
            GpuPointLights::Storage(buffer) => {
                buffer.get_mut().data.clear();
                buffer.get_mut().data.append(&mut lights);
            }
        }
    }

    fn write_buffer(&mut self, render_device: &RenderDevice, render_queue: &RenderQueue) {
        match self {
            GpuPointLights::Uniform(buffer) => buffer.write_buffer(render_device, render_queue),
            GpuPointLights::Storage(buffer) => buffer.write_buffer(render_device, render_queue),
        }
    }

    pub fn binding(&self) -> Option<BindingResource> {
        match self {
            GpuPointLights::Uniform(buffer) => buffer.binding(),
            GpuPointLights::Storage(buffer) => buffer.binding(),
        }
    }

    pub fn min_size(buffer_binding_type: BufferBindingType) -> NonZeroU64 {
        match buffer_binding_type {
            BufferBindingType::Storage { .. } => GpuPointLightsStorage::min_size(),
            BufferBindingType::Uniform => GpuPointLightsUniform::min_size(),
        }
    }
}

// NOTE: These must match the bit flags in bevy_pbr/src/render/mesh_view_types.wgsl!
bitflags::bitflags! {
    #[repr(transparent)]
    struct PointLightFlags: u32 {
        const SHADOWS_ENABLED            = (1 << 0);
        const SPOT_LIGHT_Y_NEGATIVE      = (1 << 1);
        const NONE                       = 0;
        const UNINITIALIZED              = 0xFFFF;
    }
}

#[derive(Copy, Clone, ShaderType, Default, Debug)]
pub struct GpuDirectionalCascade {
    view_projection: Mat4,
    texel_size: f32,
    far_bound: f32,
}

#[derive(Copy, Clone, ShaderType, Default, Debug)]
pub struct GpuDirectionalLight {
    cascades: [GpuDirectionalCascade; MAX_CASCADES_PER_LIGHT],
    color: Vec4,
    dir_to_light: Vec3,
    flags: u32,
    shadow_depth_bias: f32,
    shadow_normal_bias: f32,
    num_cascades: u32,
    cascades_overlap_proportion: f32,
    depth_texture_base_index: u32,
}

// NOTE: These must match the bit flags in bevy_pbr/src/render/mesh_view_types.wgsl!
bitflags::bitflags! {
    #[repr(transparent)]
    struct DirectionalLightFlags: u32 {
        const SHADOWS_ENABLED            = (1 << 0);
        const NONE                       = 0;
        const UNINITIALIZED              = 0xFFFF;
    }
}

#[derive(Copy, Clone, Debug, ShaderType)]
pub struct GpuLights {
    directional_lights: [GpuDirectionalLight; MAX_DIRECTIONAL_LIGHTS],
    ambient_color: Vec4,
    // xyz are x/y/z cluster dimensions and w is the number of clusters
    cluster_dimensions: UVec4,
    // xy are vec2<f32>(cluster_dimensions.xy) / vec2<f32>(view.width, view.height)
    // z is cluster_dimensions.z / log(far / near)
    // w is cluster_dimensions.z * log(near) / log(far / near)
    cluster_factors: Vec4,
    n_directional_lights: u32,
    // offset from spot light's light index to spot light's shadow map index
    spot_light_shadowmap_offset: i32,
    environment_map_smallest_specular_mip_level: u32,
}

// NOTE: this must be kept in sync with the same constants in pbr.frag
pub const MAX_UNIFORM_BUFFER_POINT_LIGHTS: usize = 256;

//NOTE: When running bevy on Adreno GPU chipsets in WebGL, any value above 1 will result in a crash
// when loading the wgsl "pbr_functions.wgsl" in the function apply_fog.
#[cfg(all(feature = "webgl", target_arch = "wasm32"))]
pub const MAX_DIRECTIONAL_LIGHTS: usize = 1;
#[cfg(any(not(feature = "webgl"), not(target_arch = "wasm32")))]
pub const MAX_DIRECTIONAL_LIGHTS: usize = 10;
#[cfg(any(not(feature = "webgl"), not(target_arch = "wasm32")))]
pub const MAX_CASCADES_PER_LIGHT: usize = 4;
#[cfg(all(feature = "webgl", target_arch = "wasm32"))]
pub const MAX_CASCADES_PER_LIGHT: usize = 1;
pub const SHADOW_FORMAT: TextureFormat = TextureFormat::Depth32Float;

#[derive(Resource, Clone)]
pub struct ShadowSamplers {
    pub point_light_sampler: Sampler,
    pub directional_light_sampler: Sampler,
}

// TODO: this pattern for initializing the shaders / pipeline isn't ideal. this should be handled by the asset system
impl FromWorld for ShadowSamplers {
    fn from_world(world: &mut World) -> Self {
        let render_device = world.resource::<RenderDevice>();

        ShadowSamplers {
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

#[derive(Component)]
pub struct ExtractedClusterConfig {
    /// Special near value for cluster calculations
    near: f32,
    far: f32,
    /// Number of clusters in `X` / `Y` / `Z` in the view frustum
    dimensions: UVec3,
}

#[derive(Component)]
pub struct ExtractedClustersPointLights {
    data: Vec<VisiblePointLights>,
}

pub fn extract_clusters(
    mut commands: Commands,
    views: Extract<Query<(Entity, &Clusters), With<Camera>>>,
) {
    for (entity, clusters) in &views {
        commands.get_or_spawn(entity).insert((
            ExtractedClustersPointLights {
                data: clusters.lights.clone(),
            },
            ExtractedClusterConfig {
                near: clusters.near,
                far: clusters.far,
                dimensions: clusters.dimensions,
            },
        ));
    }
}

#[allow(clippy::too_many_arguments)]
pub fn extract_lights(
    mut commands: Commands,
    point_light_shadow_map: Extract<Res<PointLightShadowMap>>,
    directional_light_shadow_map: Extract<Res<DirectionalLightShadowMap>>,
    global_point_lights: Extract<Res<GlobalVisiblePointLights>>,
    point_lights: Extract<
        Query<(
            &PointLight,
            &CubemapVisibleEntities,
            &GlobalTransform,
            &ViewVisibility,
        )>,
    >,
    spot_lights: Extract<
        Query<(
            &SpotLight,
            &VisibleEntities,
            &GlobalTransform,
            &ViewVisibility,
        )>,
    >,
    directional_lights: Extract<
        Query<
            (
                Entity,
                &DirectionalLight,
                &CascadesVisibleEntities,
                &Cascades,
                &CascadeShadowConfig,
                &GlobalTransform,
                &ViewVisibility,
            ),
            Without<SpotLight>,
        >,
    >,
    mut previous_point_lights_len: Local<usize>,
    mut previous_spot_lights_len: Local<usize>,
) {
    // NOTE: These shadow map resources are extracted here as they are used here too so this avoids
    // races between scheduling of ExtractResourceSystems and this system.
    if point_light_shadow_map.is_changed() {
        commands.insert_resource(point_light_shadow_map.clone());
    }
    if directional_light_shadow_map.is_changed() {
        commands.insert_resource(directional_light_shadow_map.clone());
    }
    // This is the point light shadow map texel size for one face of the cube as a distance of 1.0
    // world unit from the light.
    // point_light_texel_size = 2.0 * 1.0 * tan(PI / 4.0) / cube face width in texels
    // PI / 4.0 is half the cube face fov, tan(PI / 4.0) = 1.0, so this simplifies to:
    // point_light_texel_size = 2.0 / cube face width in texels
    // NOTE: When using various PCF kernel sizes, this will need to be adjusted, according to:
    // https://catlikecoding.com/unity/tutorials/custom-srp/point-and-spot-shadows/
    let point_light_texel_size = 2.0 / point_light_shadow_map.size as f32;

    let mut point_lights_values = Vec::with_capacity(*previous_point_lights_len);
    for entity in global_point_lights.iter().copied() {
        let Ok((point_light, cubemap_visible_entities, transform, view_visibility)) =
            point_lights.get(entity)
        else {
            continue;
        };
        if !view_visibility.get() {
            continue;
        }
        // TODO: This is very much not ideal. We should be able to re-use the vector memory.
        // However, since exclusive access to the main world in extract is ill-advised, we just clone here.
        let render_cubemap_visible_entities = cubemap_visible_entities.clone();
        let extracted_point_light = ExtractedPointLight {
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
            spot_light_angles: None,
        };
        point_lights_values.push((
            entity,
            (extracted_point_light, render_cubemap_visible_entities),
        ));
    }
    *previous_point_lights_len = point_lights_values.len();
    commands.insert_or_spawn_batch(point_lights_values);

    let mut spot_lights_values = Vec::with_capacity(*previous_spot_lights_len);
    for entity in global_point_lights.iter().copied() {
        if let Ok((spot_light, visible_entities, transform, view_visibility)) =
            spot_lights.get(entity)
        {
            if !view_visibility.get() {
                continue;
            }
            // TODO: This is very much not ideal. We should be able to re-use the vector memory.
            // However, since exclusive access to the main world in extract is ill-advised, we just clone here.
            let render_visible_entities = visible_entities.clone();
            let texel_size =
                2.0 * spot_light.outer_angle.tan() / directional_light_shadow_map.size as f32;

            spot_lights_values.push((
                entity,
                (
                    ExtractedPointLight {
                        color: spot_light.color,
                        // NOTE: Map from luminous power in lumens to luminous intensity in lumens per steradian
                        // for a point light. See https://google.github.io/filament/Filament.html#mjx-eqn-pointLightLuminousPower
                        // for details.
                        // Note: Filament uses a divisor of PI for spot lights. We choose to use the same 4*PI divisor
                        // in both cases so that toggling between point light and spot light keeps lit areas lit equally,
                        // which seems least surprising for users
                        intensity: spot_light.intensity / (4.0 * std::f32::consts::PI),
                        range: spot_light.range,
                        radius: spot_light.radius,
                        transform: *transform,
                        shadows_enabled: spot_light.shadows_enabled,
                        shadow_depth_bias: spot_light.shadow_depth_bias,
                        // The factor of SQRT_2 is for the worst-case diagonal offset
                        shadow_normal_bias: spot_light.shadow_normal_bias
                            * texel_size
                            * std::f32::consts::SQRT_2,
                        spot_light_angles: Some((spot_light.inner_angle, spot_light.outer_angle)),
                    },
                    render_visible_entities,
                ),
            ));
        }
    }
    *previous_spot_lights_len = spot_lights_values.len();
    commands.insert_or_spawn_batch(spot_lights_values);

    for (
        entity,
        directional_light,
        visible_entities,
        cascades,
        cascade_config,
        transform,
        view_visibility,
    ) in &directional_lights
    {
        if !view_visibility.get() {
            continue;
        }

        // TODO: As above
        let render_visible_entities = visible_entities.clone();
        commands.get_or_spawn(entity).insert((
            ExtractedDirectionalLight {
                color: directional_light.color,
                illuminance: directional_light.illuminance,
                transform: *transform,
                shadows_enabled: directional_light.shadows_enabled,
                shadow_depth_bias: directional_light.shadow_depth_bias,
                // The factor of SQRT_2 is for the worst-case diagonal offset
                shadow_normal_bias: directional_light.shadow_normal_bias * std::f32::consts::SQRT_2,
                cascade_shadow_config: cascade_config.clone(),
                cascades: cascades.cascades.clone(),
            },
            render_visible_entities,
        ));
    }
}

pub(crate) const POINT_LIGHT_NEAR_Z: f32 = 0.1f32;

pub(crate) struct CubeMapFace {
    pub(crate) target: Vec3,
    pub(crate) up: Vec3,
}

// Cubemap faces are [+X, -X, +Y, -Y, +Z, -Z], per https://www.w3.org/TR/webgpu/#texture-view-creation
// Note: Cubemap coordinates are left-handed y-up, unlike the rest of Bevy.
// See https://registry.khronos.org/vulkan/specs/1.2/html/chap16.html#_cube_map_face_selection
//
// For each cubemap face, we take care to specify the appropriate target/up axis such that the rendered
// texture using Bevy's right-handed y-up coordinate space matches the expected cubemap face in
// left-handed y-up cubemap coordinates.
pub(crate) const CUBE_MAP_FACES: [CubeMapFace; 6] = [
    // +X
    CubeMapFace {
        target: Vec3::X,
        up: Vec3::Y,
    },
    // -X
    CubeMapFace {
        target: Vec3::NEG_X,
        up: Vec3::Y,
    },
    // +Y
    CubeMapFace {
        target: Vec3::Y,
        up: Vec3::Z,
    },
    // -Y
    CubeMapFace {
        target: Vec3::NEG_Y,
        up: Vec3::NEG_Z,
    },
    // +Z (with left-handed conventions, pointing forwards)
    CubeMapFace {
        target: Vec3::NEG_Z,
        up: Vec3::Y,
    },
    // -Z (with left-handed conventions, pointing backwards)
    CubeMapFace {
        target: Vec3::Z,
        up: Vec3::Y,
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

// NOTE: Clustered-forward rendering requires 3 storage buffer bindings so check that
// at least that many are supported using this constant and SupportedBindingType::from_device()
pub const CLUSTERED_FORWARD_STORAGE_BUFFER_COUNT: u32 = 3;

#[derive(Resource)]
pub struct GlobalLightMeta {
    pub gpu_point_lights: GpuPointLights,
    pub entity_to_index: HashMap<Entity, usize>,
}

impl FromWorld for GlobalLightMeta {
    fn from_world(world: &mut World) -> Self {
        Self::new(
            world
                .resource::<RenderDevice>()
                .get_supported_read_only_binding_type(CLUSTERED_FORWARD_STORAGE_BUFFER_COUNT),
        )
    }
}

impl GlobalLightMeta {
    pub fn new(buffer_binding_type: BufferBindingType) -> Self {
        Self {
            gpu_point_lights: GpuPointLights::new(buffer_binding_type),
            entity_to_index: HashMap::default(),
        }
    }
}

#[derive(Resource, Default)]
pub struct LightMeta {
    pub view_gpu_lights: DynamicUniformBuffer<GpuLights>,
}

#[derive(Component)]
pub enum LightEntity {
    Directional {
        light_entity: Entity,
        cascade_index: usize,
    },
    Point {
        light_entity: Entity,
        face_index: usize,
    },
    Spot {
        light_entity: Entity,
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

// this method of constructing a basis from a vec3 is used by glam::Vec3::any_orthonormal_pair
// we will also construct it in the fragment shader and need our implementations to match,
// so we reproduce it here to avoid a mismatch if glam changes. we also switch the handedness
// could move this onto transform but it's pretty niche
pub(crate) fn spot_light_view_matrix(transform: &GlobalTransform) -> Mat4 {
    // the matrix z_local (opposite of transform.forward())
    let fwd_dir = transform.back().extend(0.0);

    let sign = 1f32.copysign(fwd_dir.z);
    let a = -1.0 / (fwd_dir.z + sign);
    let b = fwd_dir.x * fwd_dir.y * a;
    let up_dir = Vec4::new(
        1.0 + sign * fwd_dir.x * fwd_dir.x * a,
        sign * b,
        -sign * fwd_dir.x,
        0.0,
    );
    let right_dir = Vec4::new(-b, -sign - fwd_dir.y * fwd_dir.y * a, fwd_dir.y, 0.0);

    Mat4::from_cols(
        right_dir,
        up_dir,
        fwd_dir,
        transform.translation().extend(1.0),
    )
}

pub(crate) fn spot_light_projection_matrix(angle: f32) -> Mat4 {
    // spot light projection FOV is 2x the angle from spot light center to outer edge
    Mat4::perspective_infinite_reverse_rh(angle * 2.0, 1.0, POINT_LIGHT_NEAR_Z)
}

#[allow(clippy::too_many_arguments)]
pub fn prepare_lights(
    mut commands: Commands,
    mut texture_cache: ResMut<TextureCache>,
    images: Res<RenderAssets<Image>>,
    render_device: Res<RenderDevice>,
    render_queue: Res<RenderQueue>,
    mut global_light_meta: ResMut<GlobalLightMeta>,
    mut light_meta: ResMut<LightMeta>,
    views: Query<
        (
            Entity,
            &ExtractedView,
            &ExtractedClusterConfig,
            Option<&EnvironmentMapLight>,
        ),
        With<RenderPhase<Transparent3d>>,
    >,
    ambient_light: Res<AmbientLight>,
    point_light_shadow_map: Res<PointLightShadowMap>,
    directional_light_shadow_map: Res<DirectionalLightShadowMap>,
    mut max_directional_lights_warning_emitted: Local<bool>,
    mut max_cascades_per_light_warning_emitted: Local<bool>,
    point_lights: Query<(Entity, &ExtractedPointLight)>,
    directional_lights: Query<(Entity, &ExtractedDirectionalLight)>,
) {
    let views_iter = views.iter();
    let views_count = views_iter.len();
    let Some(mut view_gpu_lights_writer) =
        light_meta
            .view_gpu_lights
            .get_writer(views_count, &render_device, &render_queue)
    else {
        return;
    };

    // Pre-calculate for PointLights
    let cube_face_projection =
        Mat4::perspective_infinite_reverse_rh(std::f32::consts::FRAC_PI_2, 1.0, POINT_LIGHT_NEAR_Z);
    let cube_face_rotations = CUBE_MAP_FACES
        .iter()
        .map(|CubeMapFace { target, up }| Transform::IDENTITY.looking_at(*target, *up))
        .collect::<Vec<_>>();

    global_light_meta.entity_to_index.clear();

    let mut point_lights: Vec<_> = point_lights.iter().collect::<Vec<_>>();
    let mut directional_lights: Vec<_> = directional_lights.iter().collect::<Vec<_>>();

    #[cfg(any(not(feature = "webgl"), not(target_arch = "wasm32")))]
    let max_texture_array_layers = render_device.limits().max_texture_array_layers as usize;
    #[cfg(any(not(feature = "webgl"), not(target_arch = "wasm32")))]
    let max_texture_cubes = max_texture_array_layers / 6;
    #[cfg(all(feature = "webgl", target_arch = "wasm32"))]
    let max_texture_array_layers = 1;
    #[cfg(all(feature = "webgl", target_arch = "wasm32"))]
    let max_texture_cubes = 1;

    if !*max_directional_lights_warning_emitted && directional_lights.len() > MAX_DIRECTIONAL_LIGHTS
    {
        warn!(
            "The amount of directional lights of {} is exceeding the supported limit of {}.",
            directional_lights.len(),
            MAX_DIRECTIONAL_LIGHTS
        );
        *max_directional_lights_warning_emitted = true;
    }

    if !*max_cascades_per_light_warning_emitted
        && directional_lights
            .iter()
            .any(|(_, light)| light.cascade_shadow_config.bounds.len() > MAX_CASCADES_PER_LIGHT)
    {
        warn!(
            "The number of cascades configured for a directional light exceeds the supported limit of {}.",
            MAX_CASCADES_PER_LIGHT
        );
        *max_cascades_per_light_warning_emitted = true;
    }

    let point_light_count = point_lights
        .iter()
        .filter(|light| light.1.spot_light_angles.is_none())
        .count();

    let point_light_shadow_maps_count = point_lights
        .iter()
        .filter(|light| light.1.shadows_enabled && light.1.spot_light_angles.is_none())
        .count()
        .min(max_texture_cubes);

    let directional_shadow_enabled_count = directional_lights
        .iter()
        .take(MAX_DIRECTIONAL_LIGHTS)
        .filter(|(_, light)| light.shadows_enabled)
        .count()
        .min(max_texture_array_layers / MAX_CASCADES_PER_LIGHT);

    let spot_light_shadow_maps_count = point_lights
        .iter()
        .filter(|(_, light)| light.shadows_enabled && light.spot_light_angles.is_some())
        .count()
        .min(max_texture_array_layers - directional_shadow_enabled_count * MAX_CASCADES_PER_LIGHT);

    // Sort lights by
    // - point-light vs spot-light, so that we can iterate point lights and spot lights in contiguous blocks in the fragment shader,
    // - then those with shadows enabled first, so that the index can be used to render at most `point_light_shadow_maps_count`
    //   point light shadows and `spot_light_shadow_maps_count` spot light shadow maps,
    // - then by entity as a stable key to ensure that a consistent set of lights are chosen if the light count limit is exceeded.
    point_lights.sort_by(|(entity_1, light_1), (entity_2, light_2)| {
        point_light_order(
            (
                entity_1,
                &light_1.shadows_enabled,
                &light_1.spot_light_angles.is_some(),
            ),
            (
                entity_2,
                &light_2.shadows_enabled,
                &light_2.spot_light_angles.is_some(),
            ),
        )
    });

    // Sort lights by
    // - those with shadows enabled first, so that the index can be used to render at most `directional_light_shadow_maps_count`
    //   directional light shadows
    // - then by entity as a stable key to ensure that a consistent set of lights are chosen if the light count limit is exceeded.
    directional_lights.sort_by(|(entity_1, light_1), (entity_2, light_2)| {
        directional_light_order(
            (entity_1, &light_1.shadows_enabled),
            (entity_2, &light_2.shadows_enabled),
        )
    });

    if global_light_meta.entity_to_index.capacity() < point_lights.len() {
        global_light_meta
            .entity_to_index
            .reserve(point_lights.len());
    }

    let mut gpu_point_lights = Vec::new();
    for (index, &(entity, light)) in point_lights.iter().enumerate() {
        let mut flags = PointLightFlags::NONE;

        // Lights are sorted, shadow enabled lights are first
        if light.shadows_enabled
            && (index < point_light_shadow_maps_count
                || (light.spot_light_angles.is_some()
                    && index - point_light_count < spot_light_shadow_maps_count))
        {
            flags |= PointLightFlags::SHADOWS_ENABLED;
        }

        let (light_custom_data, spot_light_tan_angle) = match light.spot_light_angles {
            Some((inner, outer)) => {
                let light_direction = light.transform.forward();
                if light_direction.y.is_sign_negative() {
                    flags |= PointLightFlags::SPOT_LIGHT_Y_NEGATIVE;
                }

                let cos_outer = outer.cos();
                let spot_scale = 1.0 / f32::max(inner.cos() - cos_outer, 1e-4);
                let spot_offset = -cos_outer * spot_scale;

                (
                    // For spot lights: the direction (x,z), spot_scale and spot_offset
                    light_direction.xz().extend(spot_scale).extend(spot_offset),
                    outer.tan(),
                )
            }
            None => {
                (
                    // For point lights: the lower-right 2x2 values of the projection matrix [2][2] [2][3] [3][2] [3][3]
                    Vec4::new(
                        cube_face_projection.z_axis.z,
                        cube_face_projection.z_axis.w,
                        cube_face_projection.w_axis.z,
                        cube_face_projection.w_axis.w,
                    ),
                    // unused
                    0.0,
                )
            }
        };

        gpu_point_lights.push(GpuPointLight {
            light_custom_data,
            // premultiply color by intensity
            // we don't use the alpha at all, so no reason to multiply only [0..3]
            color_inverse_square_range: (Vec4::from_slice(&light.color.as_linear_rgba_f32())
                * light.intensity)
                .xyz()
                .extend(1.0 / (light.range * light.range)),
            position_radius: light.transform.translation().extend(light.radius),
            flags: flags.bits(),
            shadow_depth_bias: light.shadow_depth_bias,
            shadow_normal_bias: light.shadow_normal_bias,
            spot_light_tan_angle,
        });
        global_light_meta.entity_to_index.insert(entity, index);
    }

    let mut gpu_directional_lights = [GpuDirectionalLight::default(); MAX_DIRECTIONAL_LIGHTS];
    let mut num_directional_cascades_enabled = 0usize;
    for (index, (_light_entity, light)) in directional_lights
        .iter()
        .enumerate()
        .take(MAX_DIRECTIONAL_LIGHTS)
    {
        let mut flags = DirectionalLightFlags::NONE;

        // Lights are sorted, shadow enabled lights are first
        if light.shadows_enabled && (index < directional_shadow_enabled_count) {
            flags |= DirectionalLightFlags::SHADOWS_ENABLED;
        }

        // convert from illuminance (lux) to candelas
        //
        // exposure is hard coded at the moment but should be replaced
        // by values coming from the camera
        // see: https://google.github.io/filament/Filament.html#imagingpipeline/physicallybasedcamera/exposuresettings
        const APERTURE: f32 = 4.0;
        const SHUTTER_SPEED: f32 = 1.0 / 250.0;
        const SENSITIVITY: f32 = 100.0;
        let ev100 = f32::log2(APERTURE * APERTURE / SHUTTER_SPEED) - f32::log2(SENSITIVITY / 100.0);
        let exposure = 1.0 / (f32::powf(2.0, ev100) * 1.2);
        let intensity = light.illuminance * exposure;

        let num_cascades = light
            .cascade_shadow_config
            .bounds
            .len()
            .min(MAX_CASCADES_PER_LIGHT);
        gpu_directional_lights[index] = GpuDirectionalLight {
            // Filled in later.
            cascades: [GpuDirectionalCascade::default(); MAX_CASCADES_PER_LIGHT],
            // premultiply color by intensity
            // we don't use the alpha at all, so no reason to multiply only [0..3]
            color: Vec4::from_slice(&light.color.as_linear_rgba_f32()) * intensity,
            // direction is negated to be ready for N.L
            dir_to_light: light.transform.back(),
            flags: flags.bits(),
            shadow_depth_bias: light.shadow_depth_bias,
            shadow_normal_bias: light.shadow_normal_bias,
            num_cascades: num_cascades as u32,
            cascades_overlap_proportion: light.cascade_shadow_config.overlap_proportion,
            depth_texture_base_index: num_directional_cascades_enabled as u32,
        };
        if index < directional_shadow_enabled_count {
            num_directional_cascades_enabled += num_cascades;
        }
    }

    global_light_meta.gpu_point_lights.set(gpu_point_lights);
    global_light_meta
        .gpu_point_lights
        .write_buffer(&render_device, &render_queue);

    // set up light data for each view
    for (entity, extracted_view, clusters, environment_map) in &views {
        let point_light_depth_texture = texture_cache.get(
            &render_device,
            TextureDescriptor {
                size: Extent3d {
                    width: point_light_shadow_map.size as u32,
                    height: point_light_shadow_map.size as u32,
                    depth_or_array_layers: point_light_shadow_maps_count.max(1) as u32 * 6,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: TextureDimension::D2,
                format: SHADOW_FORMAT,
                label: Some("point_light_shadow_map_texture"),
                usage: TextureUsages::RENDER_ATTACHMENT | TextureUsages::TEXTURE_BINDING,
                view_formats: &[],
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
                    depth_or_array_layers: (num_directional_cascades_enabled
                        + spot_light_shadow_maps_count)
                        .max(1) as u32,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: TextureDimension::D2,
                format: SHADOW_FORMAT,
                label: Some("directional_light_shadow_map_texture"),
                usage: TextureUsages::RENDER_ATTACHMENT | TextureUsages::TEXTURE_BINDING,
                view_formats: &[],
            },
        );
        let mut view_lights = Vec::new();

        let is_orthographic = extracted_view.projection.w_axis.w == 1.0;
        let cluster_factors_zw = calculate_cluster_factors(
            clusters.near,
            clusters.far,
            clusters.dimensions.z as f32,
            is_orthographic,
        );

        let n_clusters = clusters.dimensions.x * clusters.dimensions.y * clusters.dimensions.z;
        let mut gpu_lights = GpuLights {
            directional_lights: gpu_directional_lights,
            ambient_color: Vec4::from_slice(&ambient_light.color.as_linear_rgba_f32())
                * ambient_light.brightness,
            cluster_factors: Vec4::new(
                clusters.dimensions.x as f32 / extracted_view.viewport.z as f32,
                clusters.dimensions.y as f32 / extracted_view.viewport.w as f32,
                cluster_factors_zw.x,
                cluster_factors_zw.y,
            ),
            cluster_dimensions: clusters.dimensions.extend(n_clusters),
            n_directional_lights: directional_lights.iter().len() as u32,
            // spotlight shadow maps are stored in the directional light array, starting at num_directional_cascades_enabled.
            // the spot lights themselves start in the light array at point_light_count. so to go from light
            // index to shadow map index, we need to subtract point light count and add directional shadowmap count.
            spot_light_shadowmap_offset: num_directional_cascades_enabled as i32
                - point_light_count as i32,
            environment_map_smallest_specular_mip_level: environment_map
                .and_then(|env_map| images.get(&env_map.specular_map))
                .map(|specular_map| specular_map.mip_level_count - 1)
                .unwrap_or(0),
        };

        // TODO: this should select lights based on relevance to the view instead of the first ones that show up in a query
        for &(light_entity, light) in point_lights
            .iter()
            // Lights are sorted, shadow enabled lights are first
            .take(point_light_shadow_maps_count)
            .filter(|(_, light)| light.shadows_enabled)
        {
            let light_index = *global_light_meta
                .entity_to_index
                .get(&light_entity)
                .unwrap();
            // ignore scale because we don't want to effectively scale light radius and range
            // by applying those as a view transform to shadow map rendering of objects
            // and ignore rotation because we want the shadow map projections to align with the axes
            let view_translation = GlobalTransform::from_translation(light.transform.translation());

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
                            array_layer_count: Some(1u32),
                        });

                let view_light_entity = commands
                    .spawn((
                        ShadowView {
                            depth_texture_view,
                            pass_name: format!(
                                "shadow pass point light {} {}",
                                light_index,
                                face_index_to_name(face_index)
                            ),
                        },
                        ExtractedView {
                            viewport: UVec4::new(
                                0,
                                0,
                                point_light_shadow_map.size as u32,
                                point_light_shadow_map.size as u32,
                            ),
                            transform: view_translation * *view_rotation,
                            view_projection: None,
                            projection: cube_face_projection,
                            hdr: false,
                            color_grading: Default::default(),
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

        // spot lights
        for (light_index, &(light_entity, light)) in point_lights
            .iter()
            .skip(point_light_count)
            .take(spot_light_shadow_maps_count)
            .enumerate()
        {
            let spot_view_matrix = spot_light_view_matrix(&light.transform);
            let spot_view_transform = spot_view_matrix.into();

            let angle = light.spot_light_angles.expect("lights should be sorted so that \
                [point_light_count..point_light_count + spot_light_shadow_maps_count] are spot lights").1;
            let spot_projection = spot_light_projection_matrix(angle);

            let depth_texture_view =
                directional_light_depth_texture
                    .texture
                    .create_view(&TextureViewDescriptor {
                        label: Some("spot_light_shadow_map_texture_view"),
                        format: None,
                        dimension: Some(TextureViewDimension::D2),
                        aspect: TextureAspect::All,
                        base_mip_level: 0,
                        mip_level_count: None,
                        base_array_layer: (num_directional_cascades_enabled + light_index) as u32,
                        array_layer_count: Some(1u32),
                    });

            let view_light_entity = commands
                .spawn((
                    ShadowView {
                        depth_texture_view,
                        pass_name: format!("shadow pass spot light {light_index}"),
                    },
                    ExtractedView {
                        viewport: UVec4::new(
                            0,
                            0,
                            directional_light_shadow_map.size as u32,
                            directional_light_shadow_map.size as u32,
                        ),
                        transform: spot_view_transform,
                        projection: spot_projection,
                        view_projection: None,
                        hdr: false,
                        color_grading: Default::default(),
                    },
                    RenderPhase::<Shadow>::default(),
                    LightEntity::Spot { light_entity },
                ))
                .id();

            view_lights.push(view_light_entity);
        }

        // directional lights
        let mut directional_depth_texture_array_index = 0u32;
        for (light_index, &(light_entity, light)) in directional_lights
            .iter()
            .enumerate()
            .take(directional_shadow_enabled_count)
        {
            for (cascade_index, (cascade, bound)) in light
                .cascades
                .get(&entity)
                .unwrap()
                .iter()
                .take(MAX_CASCADES_PER_LIGHT)
                .zip(&light.cascade_shadow_config.bounds)
                .enumerate()
            {
                gpu_lights.directional_lights[light_index].cascades[cascade_index] =
                    GpuDirectionalCascade {
                        view_projection: cascade.view_projection,
                        texel_size: cascade.texel_size,
                        far_bound: *bound,
                    };

                let depth_texture_view =
                    directional_light_depth_texture
                        .texture
                        .create_view(&TextureViewDescriptor {
                            label: Some("directional_light_shadow_map_array_texture_view"),
                            format: None,
                            dimension: Some(TextureViewDimension::D2),
                            aspect: TextureAspect::All,
                            base_mip_level: 0,
                            mip_level_count: None,
                            base_array_layer: directional_depth_texture_array_index,
                            array_layer_count: Some(1u32),
                        });
                directional_depth_texture_array_index += 1;

                let view_light_entity = commands
                    .spawn((
                        ShadowView {
                            depth_texture_view,
                            pass_name: format!(
                                "shadow pass directional light {light_index} cascade {cascade_index}"),
                        },
                        ExtractedView {
                            viewport: UVec4::new(
                                0,
                                0,
                                directional_light_shadow_map.size as u32,
                                directional_light_shadow_map.size as u32,
                            ),
                            transform: GlobalTransform::from(cascade.view_transform),
                            projection: cascade.projection,
                            view_projection: Some(cascade.view_projection),
                            hdr: false,
                            color_grading: Default::default(),
                        },
                        RenderPhase::<Shadow>::default(),
                        LightEntity::Directional {
                            light_entity,
                            cascade_index,
                        },
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
                    #[cfg(any(not(feature = "webgl"), not(target_arch = "wasm32")))]
                    dimension: Some(TextureViewDimension::CubeArray),
                    #[cfg(all(feature = "webgl", target_arch = "wasm32"))]
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
                #[cfg(any(not(feature = "webgl"), not(target_arch = "wasm32")))]
                dimension: Some(TextureViewDimension::D2Array),
                #[cfg(all(feature = "webgl", target_arch = "wasm32"))]
                dimension: Some(TextureViewDimension::D2),
                aspect: TextureAspect::All,
                base_mip_level: 0,
                mip_level_count: None,
                base_array_layer: 0,
                array_layer_count: None,
            });

        commands.entity(entity).insert((
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
                offset: view_gpu_lights_writer.write(&gpu_lights),
            },
        ));
    }
}

// this must match CLUSTER_COUNT_SIZE in pbr.wgsl
// and must be large enough to contain MAX_UNIFORM_BUFFER_POINT_LIGHTS
const CLUSTER_COUNT_SIZE: u32 = 9;

const CLUSTER_OFFSET_MASK: u32 = (1 << (32 - (CLUSTER_COUNT_SIZE * 2))) - 1;
const CLUSTER_COUNT_MASK: u32 = (1 << CLUSTER_COUNT_SIZE) - 1;

// NOTE: With uniform buffer max binding size as 16384 bytes
// that means we can fit 256 point lights in one uniform
// buffer, which means the count can be at most 256 so it
// needs 9 bits.
// The array of indices can also use u8 and that means the
// offset in to the array of indices needs to be able to address
// 16384 values. log2(16384) = 14 bits.
// We use 32 bits to store the offset and counts so
// we pack the offset into the upper 14 bits of a u32,
// the point light count into bits 9-17, and the spot light count into bits 0-8.
//  [ 31     ..     18 | 17      ..      9 | 8       ..     0 ]
//  [      offset      | point light count | spot light count ]
// NOTE: This assumes CPU and GPU endianness are the same which is true
// for all common and tested x86/ARM CPUs and AMD/NVIDIA/Intel/Apple/etc GPUs
fn pack_offset_and_counts(offset: usize, point_count: usize, spot_count: usize) -> u32 {
    ((offset as u32 & CLUSTER_OFFSET_MASK) << (CLUSTER_COUNT_SIZE * 2))
        | (point_count as u32 & CLUSTER_COUNT_MASK) << CLUSTER_COUNT_SIZE
        | (spot_count as u32 & CLUSTER_COUNT_MASK)
}

#[derive(ShaderType)]
struct GpuClusterLightIndexListsUniform {
    data: Box<[UVec4; ViewClusterBindings::MAX_UNIFORM_ITEMS]>,
}

// NOTE: Assert at compile time that GpuClusterLightIndexListsUniform
// fits within the maximum uniform buffer binding size
const _: () = assert!(GpuClusterLightIndexListsUniform::SHADER_SIZE.get() <= 16384);

impl Default for GpuClusterLightIndexListsUniform {
    fn default() -> Self {
        Self {
            data: Box::new([UVec4::ZERO; ViewClusterBindings::MAX_UNIFORM_ITEMS]),
        }
    }
}

#[derive(ShaderType)]
struct GpuClusterOffsetsAndCountsUniform {
    data: Box<[UVec4; ViewClusterBindings::MAX_UNIFORM_ITEMS]>,
}

impl Default for GpuClusterOffsetsAndCountsUniform {
    fn default() -> Self {
        Self {
            data: Box::new([UVec4::ZERO; ViewClusterBindings::MAX_UNIFORM_ITEMS]),
        }
    }
}

#[derive(ShaderType, Default)]
struct GpuClusterLightIndexListsStorage {
    #[size(runtime)]
    data: Vec<u32>,
}

#[derive(ShaderType, Default)]
struct GpuClusterOffsetsAndCountsStorage {
    #[size(runtime)]
    data: Vec<UVec4>,
}

enum ViewClusterBuffers {
    Uniform {
        // NOTE: UVec4 is because all arrays in Std140 layout have 16-byte alignment
        cluster_light_index_lists: UniformBuffer<GpuClusterLightIndexListsUniform>,
        // NOTE: UVec4 is because all arrays in Std140 layout have 16-byte alignment
        cluster_offsets_and_counts: UniformBuffer<GpuClusterOffsetsAndCountsUniform>,
    },
    Storage {
        cluster_light_index_lists: StorageBuffer<GpuClusterLightIndexListsStorage>,
        cluster_offsets_and_counts: StorageBuffer<GpuClusterOffsetsAndCountsStorage>,
    },
}

impl ViewClusterBuffers {
    fn new(buffer_binding_type: BufferBindingType) -> Self {
        match buffer_binding_type {
            BufferBindingType::Storage { .. } => Self::storage(),
            BufferBindingType::Uniform => Self::uniform(),
        }
    }

    fn uniform() -> Self {
        ViewClusterBuffers::Uniform {
            cluster_light_index_lists: UniformBuffer::default(),
            cluster_offsets_and_counts: UniformBuffer::default(),
        }
    }

    fn storage() -> Self {
        ViewClusterBuffers::Storage {
            cluster_light_index_lists: StorageBuffer::default(),
            cluster_offsets_and_counts: StorageBuffer::default(),
        }
    }
}

#[derive(Component)]
pub struct ViewClusterBindings {
    n_indices: usize,
    n_offsets: usize,
    buffers: ViewClusterBuffers,
}

impl ViewClusterBindings {
    pub const MAX_OFFSETS: usize = 16384 / 4;
    const MAX_UNIFORM_ITEMS: usize = Self::MAX_OFFSETS / 4;
    pub const MAX_INDICES: usize = 16384;

    pub fn new(buffer_binding_type: BufferBindingType) -> Self {
        Self {
            n_indices: 0,
            n_offsets: 0,
            buffers: ViewClusterBuffers::new(buffer_binding_type),
        }
    }

    pub fn clear(&mut self) {
        match &mut self.buffers {
            ViewClusterBuffers::Uniform {
                cluster_light_index_lists,
                cluster_offsets_and_counts,
            } => {
                *cluster_light_index_lists.get_mut().data = [UVec4::ZERO; Self::MAX_UNIFORM_ITEMS];
                *cluster_offsets_and_counts.get_mut().data = [UVec4::ZERO; Self::MAX_UNIFORM_ITEMS];
            }
            ViewClusterBuffers::Storage {
                cluster_light_index_lists,
                cluster_offsets_and_counts,
                ..
            } => {
                cluster_light_index_lists.get_mut().data.clear();
                cluster_offsets_and_counts.get_mut().data.clear();
            }
        }
    }

    pub fn push_offset_and_counts(&mut self, offset: usize, point_count: usize, spot_count: usize) {
        match &mut self.buffers {
            ViewClusterBuffers::Uniform {
                cluster_offsets_and_counts,
                ..
            } => {
                let array_index = self.n_offsets >> 2; // >> 2 is equivalent to / 4
                if array_index >= Self::MAX_UNIFORM_ITEMS {
                    warn!("cluster offset and count out of bounds!");
                    return;
                }
                let component = self.n_offsets & ((1 << 2) - 1);
                let packed = pack_offset_and_counts(offset, point_count, spot_count);

                cluster_offsets_and_counts.get_mut().data[array_index][component] = packed;
            }
            ViewClusterBuffers::Storage {
                cluster_offsets_and_counts,
                ..
            } => {
                cluster_offsets_and_counts.get_mut().data.push(UVec4::new(
                    offset as u32,
                    point_count as u32,
                    spot_count as u32,
                    0,
                ));
            }
        }

        self.n_offsets += 1;
    }

    pub fn n_indices(&self) -> usize {
        self.n_indices
    }

    pub fn push_index(&mut self, index: usize) {
        match &mut self.buffers {
            ViewClusterBuffers::Uniform {
                cluster_light_index_lists,
                ..
            } => {
                let array_index = self.n_indices >> 4; // >> 4 is equivalent to / 16
                let component = (self.n_indices >> 2) & ((1 << 2) - 1);
                let sub_index = self.n_indices & ((1 << 2) - 1);
                let index = index as u32;

                cluster_light_index_lists.get_mut().data[array_index][component] |=
                    index << (8 * sub_index);
            }
            ViewClusterBuffers::Storage {
                cluster_light_index_lists,
                ..
            } => {
                cluster_light_index_lists.get_mut().data.push(index as u32);
            }
        }

        self.n_indices += 1;
    }

    pub fn write_buffers(&mut self, render_device: &RenderDevice, render_queue: &RenderQueue) {
        match &mut self.buffers {
            ViewClusterBuffers::Uniform {
                cluster_light_index_lists,
                cluster_offsets_and_counts,
            } => {
                cluster_light_index_lists.write_buffer(render_device, render_queue);
                cluster_offsets_and_counts.write_buffer(render_device, render_queue);
            }
            ViewClusterBuffers::Storage {
                cluster_light_index_lists,
                cluster_offsets_and_counts,
            } => {
                cluster_light_index_lists.write_buffer(render_device, render_queue);
                cluster_offsets_and_counts.write_buffer(render_device, render_queue);
            }
        }
    }

    pub fn light_index_lists_binding(&self) -> Option<BindingResource> {
        match &self.buffers {
            ViewClusterBuffers::Uniform {
                cluster_light_index_lists,
                ..
            } => cluster_light_index_lists.binding(),
            ViewClusterBuffers::Storage {
                cluster_light_index_lists,
                ..
            } => cluster_light_index_lists.binding(),
        }
    }

    pub fn offsets_and_counts_binding(&self) -> Option<BindingResource> {
        match &self.buffers {
            ViewClusterBuffers::Uniform {
                cluster_offsets_and_counts,
                ..
            } => cluster_offsets_and_counts.binding(),
            ViewClusterBuffers::Storage {
                cluster_offsets_and_counts,
                ..
            } => cluster_offsets_and_counts.binding(),
        }
    }

    pub fn min_size_cluster_light_index_lists(
        buffer_binding_type: BufferBindingType,
    ) -> NonZeroU64 {
        match buffer_binding_type {
            BufferBindingType::Storage { .. } => GpuClusterLightIndexListsStorage::min_size(),
            BufferBindingType::Uniform => GpuClusterLightIndexListsUniform::min_size(),
        }
    }

    pub fn min_size_cluster_offsets_and_counts(
        buffer_binding_type: BufferBindingType,
    ) -> NonZeroU64 {
        match buffer_binding_type {
            BufferBindingType::Storage { .. } => GpuClusterOffsetsAndCountsStorage::min_size(),
            BufferBindingType::Uniform => GpuClusterOffsetsAndCountsUniform::min_size(),
        }
    }
}

pub fn prepare_clusters(
    mut commands: Commands,
    render_device: Res<RenderDevice>,
    render_queue: Res<RenderQueue>,
    mesh_pipeline: Res<MeshPipeline>,
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
    let render_device = render_device.into_inner();
    let supports_storage_buffers = matches!(
        mesh_pipeline.clustered_forward_buffer_binding_type,
        BufferBindingType::Storage { .. }
    );
    for (entity, cluster_config, extracted_clusters) in &views {
        let mut view_clusters_bindings =
            ViewClusterBindings::new(mesh_pipeline.clustered_forward_buffer_binding_type);
        view_clusters_bindings.clear();

        let mut indices_full = false;

        let mut cluster_index = 0;
        for _y in 0..cluster_config.dimensions.y {
            for _x in 0..cluster_config.dimensions.x {
                for _z in 0..cluster_config.dimensions.z {
                    let offset = view_clusters_bindings.n_indices();
                    let cluster_lights = &extracted_clusters.data[cluster_index];
                    view_clusters_bindings.push_offset_and_counts(
                        offset,
                        cluster_lights.point_light_count,
                        cluster_lights.spot_light_count,
                    );

                    if !indices_full {
                        for entity in cluster_lights.iter() {
                            if let Some(light_index) = global_light_meta.entity_to_index.get(entity)
                            {
                                if view_clusters_bindings.n_indices()
                                    >= ViewClusterBindings::MAX_INDICES
                                    && !supports_storage_buffers
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

        view_clusters_bindings.write_buffers(render_device, &render_queue);

        commands.get_or_spawn(entity).insert(view_clusters_bindings);
    }
}

#[allow(clippy::too_many_arguments)]
pub fn queue_shadows<M: Material>(
    shadow_draw_functions: Res<DrawFunctions<Shadow>>,
    prepass_pipeline: Res<PrepassPipeline<M>>,
    render_meshes: Res<RenderAssets<Mesh>>,
    render_mesh_instances: Res<RenderMeshInstances>,
    render_materials: Res<RenderMaterials<M>>,
    render_material_instances: Res<RenderMaterialInstances<M>>,
    mut pipelines: ResMut<SpecializedMeshPipelines<PrepassPipeline<M>>>,
    pipeline_cache: Res<PipelineCache>,
    view_lights: Query<(Entity, &ViewLightEntities)>,
    mut view_light_shadow_phases: Query<(&LightEntity, &mut RenderPhase<Shadow>)>,
    point_light_entities: Query<&CubemapVisibleEntities, With<ExtractedPointLight>>,
    directional_light_entities: Query<&CascadesVisibleEntities, With<ExtractedDirectionalLight>>,
    spot_light_entities: Query<&VisibleEntities, With<ExtractedPointLight>>,
) where
    M::Data: PartialEq + Eq + Hash + Clone,
{
    for (entity, view_lights) in &view_lights {
        let draw_shadow_mesh = shadow_draw_functions.read().id::<DrawPrepass<M>>();
        for view_light_entity in view_lights.lights.iter().copied() {
            let (light_entity, mut shadow_phase) =
                view_light_shadow_phases.get_mut(view_light_entity).unwrap();
            let is_directional_light = matches!(light_entity, LightEntity::Directional { .. });
            let visible_entities = match light_entity {
                LightEntity::Directional {
                    light_entity,
                    cascade_index,
                } => directional_light_entities
                    .get(*light_entity)
                    .expect("Failed to get directional light visible entities")
                    .entities
                    .get(&entity)
                    .expect("Failed to get directional light visible entities for view")
                    .get(*cascade_index)
                    .expect("Failed to get directional light visible entities for cascade"),
                LightEntity::Point {
                    light_entity,
                    face_index,
                } => point_light_entities
                    .get(*light_entity)
                    .expect("Failed to get point light visible entities")
                    .get(*face_index),
                LightEntity::Spot { light_entity } => spot_light_entities
                    .get(*light_entity)
                    .expect("Failed to get spot light visible entities"),
            };
            // NOTE: Lights with shadow mapping disabled will have no visible entities
            // so no meshes will be queued
            for entity in visible_entities.iter().copied() {
                let Some(mesh_instance) = render_mesh_instances.get(&entity) else {
                    continue;
                };
                if !mesh_instance.shadow_caster {
                    continue;
                }
                let Some(material_asset_id) = render_material_instances.get(&entity) else {
                    continue;
                };
                let Some(material) = render_materials.get(material_asset_id) else {
                    continue;
                };
                let Some(mesh) = render_meshes.get(mesh_instance.mesh_asset_id) else {
                    continue;
                };

                let mut mesh_key =
                    MeshPipelineKey::from_primitive_topology(mesh.primitive_topology)
                        | MeshPipelineKey::DEPTH_PREPASS;
                if mesh.morph_targets.is_some() {
                    mesh_key |= MeshPipelineKey::MORPH_TARGETS;
                }
                if is_directional_light {
                    mesh_key |= MeshPipelineKey::DEPTH_CLAMP_ORTHO;
                }
                mesh_key |= match material.properties.alpha_mode {
                    AlphaMode::Mask(_)
                    | AlphaMode::Blend
                    | AlphaMode::Premultiplied
                    | AlphaMode::Add => MeshPipelineKey::MAY_DISCARD,
                    _ => MeshPipelineKey::NONE,
                };
                let pipeline_id = pipelines.specialize(
                    &pipeline_cache,
                    &prepass_pipeline,
                    MaterialPipelineKey {
                        mesh_key,
                        bind_group_data: material.key.clone(),
                    },
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
                    distance: 0.0, // TODO: sort front-to-back
                    batch_range: 0..1,
                    dynamic_offset: None,
                });
            }
        }
    }
}

pub struct Shadow {
    pub distance: f32,
    pub entity: Entity,
    pub pipeline: CachedRenderPipelineId,
    pub draw_function: DrawFunctionId,
    pub batch_range: Range<u32>,
    pub dynamic_offset: Option<NonMaxU32>,
}

impl PhaseItem for Shadow {
    type SortKey = usize;

    #[inline]
    fn entity(&self) -> Entity {
        self.entity
    }

    #[inline]
    fn sort_key(&self) -> Self::SortKey {
        self.pipeline.id()
    }

    #[inline]
    fn draw_function(&self) -> DrawFunctionId {
        self.draw_function
    }

    #[inline]
    fn sort(items: &mut [Self]) {
        // The shadow phase is sorted by pipeline id for performance reasons.
        // Grouping all draw commands using the same pipeline together performs
        // better than rebinding everything at a high rate.
        radsort::sort_by_key(items, |item| item.sort_key());
    }

    #[inline]
    fn batch_range(&self) -> &Range<u32> {
        &self.batch_range
    }

    #[inline]
    fn batch_range_mut(&mut self) -> &mut Range<u32> {
        &mut self.batch_range
    }

    #[inline]
    fn dynamic_offset(&self) -> Option<NonMaxU32> {
        self.dynamic_offset
    }

    #[inline]
    fn dynamic_offset_mut(&mut self) -> &mut Option<NonMaxU32> {
        &mut self.dynamic_offset
    }
}

impl CachedRenderPipelinePhaseItem for Shadow {
    #[inline]
    fn cached_pipeline(&self) -> CachedRenderPipelineId {
        self.pipeline
    }
}

pub struct ShadowPassNode {
    main_view_query: QueryState<&'static ViewLightEntities>,
    view_light_query: QueryState<(&'static ShadowView, &'static RenderPhase<Shadow>)>,
}

impl ShadowPassNode {
    pub fn new(world: &mut World) -> Self {
        Self {
            main_view_query: QueryState::new(world),
            view_light_query: QueryState::new(world),
        }
    }
}

impl Node for ShadowPassNode {
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
        let view_entity = graph.view_entity();
        if let Ok(view_lights) = self.main_view_query.get_manual(world, view_entity) {
            for view_light_entity in view_lights.lights.iter().copied() {
                let (view_light, shadow_phase) = self
                    .view_light_query
                    .get_manual(world, view_light_entity)
                    .unwrap();

                if shadow_phase.items.is_empty() {
                    continue;
                }

                let mut render_pass =
                    render_context.begin_tracked_render_pass(RenderPassDescriptor {
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
                    });

                shadow_phase.render(&mut render_pass, world, view_light_entity);
            }
        }

        Ok(())
    }
}
