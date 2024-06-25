use bevy_asset::AssetId;
use bevy_color::ColorToComponents;
use bevy_core_pipeline::core_3d::CORE_3D_DEPTH_FORMAT;
use bevy_ecs::entity::EntityHashSet;
use bevy_ecs::prelude::*;
use bevy_ecs::{entity::EntityHashMap, system::lifetimeless::Read};
use bevy_math::{Mat4, UVec4, Vec2, Vec3, Vec3Swizzles, Vec4, Vec4Swizzles};
use bevy_render::mesh::Mesh;
use bevy_render::{
    diagnostic::RecordDiagnostics,
    mesh::GpuMesh,
    primitives::{CascadesFrusta, CubemapFrusta, Frustum, HalfSpace},
    render_asset::RenderAssets,
    render_graph::{Node, NodeRunError, RenderGraphContext},
    render_phase::*,
    render_resource::*,
    renderer::{RenderContext, RenderDevice, RenderQueue},
    texture::*,
    view::{ExtractedView, RenderLayers, ViewVisibility, VisibleEntities, WithMesh},
    Extract,
};
use bevy_transform::{components::GlobalTransform, prelude::Transform};
#[cfg(feature = "trace")]
use bevy_utils::tracing::info_span;
use bevy_utils::tracing::{error, warn};
use std::{hash::Hash, ops::Range};

use crate::*;

#[derive(Component)]
pub struct ExtractedPointLight {
    pub color: LinearRgba,
    /// luminous intensity in lumens per steradian
    pub intensity: f32,
    pub range: f32,
    pub radius: f32,
    pub transform: GlobalTransform,
    pub shadows_enabled: bool,
    pub shadow_depth_bias: f32,
    pub shadow_normal_bias: f32,
    pub spot_light_angles: Option<(f32, f32)>,
}

#[derive(Component, Debug)]
pub struct ExtractedDirectionalLight {
    pub color: LinearRgba,
    pub illuminance: f32,
    pub transform: GlobalTransform,
    pub shadows_enabled: bool,
    pub volumetric: bool,
    pub shadow_depth_bias: f32,
    pub shadow_normal_bias: f32,
    pub cascade_shadow_config: CascadeShadowConfig,
    pub cascades: EntityHashMap<Vec<Cascade>>,
    pub frusta: EntityHashMap<Vec<Frustum>>,
    pub render_layers: RenderLayers,
}

// NOTE: These must match the bit flags in bevy_pbr/src/render/mesh_view_types.wgsl!
bitflags::bitflags! {
    #[repr(transparent)]
    struct PointLightFlags: u32 {
        const SHADOWS_ENABLED            = 1 << 0;
        const SPOT_LIGHT_Y_NEGATIVE      = 1 << 1;
        const NONE                       = 0;
        const UNINITIALIZED              = 0xFFFF;
    }
}

#[derive(Copy, Clone, ShaderType, Default, Debug)]
pub struct GpuDirectionalCascade {
    clip_from_world: Mat4,
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
    skip: u32,
}

// NOTE: These must match the bit flags in bevy_pbr/src/render/mesh_view_types.wgsl!
bitflags::bitflags! {
    #[repr(transparent)]
    struct DirectionalLightFlags: u32 {
        const SHADOWS_ENABLED            = 1 << 0;
        const VOLUMETRIC                 = 1 << 1;
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
}

//NOTE: When running bevy on Adreno GPU chipsets in WebGL, any value above 1 will result in a crash
// when loading the wgsl "pbr_functions.wgsl" in the function apply_fog.
#[cfg(all(feature = "webgl", target_arch = "wasm32", not(feature = "webgpu")))]
pub const MAX_DIRECTIONAL_LIGHTS: usize = 1;
#[cfg(any(
    not(feature = "webgl"),
    not(target_arch = "wasm32"),
    feature = "webgpu"
))]
pub const MAX_DIRECTIONAL_LIGHTS: usize = 10;
#[cfg(any(
    not(feature = "webgl"),
    not(target_arch = "wasm32"),
    feature = "webgpu"
))]
pub const MAX_CASCADES_PER_LIGHT: usize = 4;
#[cfg(all(feature = "webgl", target_arch = "wasm32", not(feature = "webgpu")))]
pub const MAX_CASCADES_PER_LIGHT: usize = 1;

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

#[allow(clippy::too_many_arguments)]
pub fn extract_lights(
    mut commands: Commands,
    point_light_shadow_map: Extract<Res<PointLightShadowMap>>,
    directional_light_shadow_map: Extract<Res<DirectionalLightShadowMap>>,
    global_point_lights: Extract<Res<GlobalVisibleClusterableObjects>>,
    point_lights: Extract<
        Query<(
            &PointLight,
            &CubemapVisibleEntities,
            &GlobalTransform,
            &ViewVisibility,
            &CubemapFrusta,
        )>,
    >,
    spot_lights: Extract<
        Query<(
            &SpotLight,
            &VisibleEntities,
            &GlobalTransform,
            &ViewVisibility,
            &Frustum,
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
                &CascadesFrusta,
                &GlobalTransform,
                &ViewVisibility,
                Option<&RenderLayers>,
                Option<&VolumetricLight>,
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
        let Ok((point_light, cubemap_visible_entities, transform, view_visibility, frusta)) =
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
            color: point_light.color.into(),
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
            (
                extracted_point_light,
                render_cubemap_visible_entities,
                (*frusta).clone(),
            ),
        ));
    }
    *previous_point_lights_len = point_lights_values.len();
    commands.insert_or_spawn_batch(point_lights_values);

    let mut spot_lights_values = Vec::with_capacity(*previous_spot_lights_len);
    for entity in global_point_lights.iter().copied() {
        if let Ok((spot_light, visible_entities, transform, view_visibility, frustum)) =
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
                        color: spot_light.color.into(),
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
                    *frustum,
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
        frusta,
        transform,
        view_visibility,
        maybe_layers,
        volumetric_light,
    ) in &directional_lights
    {
        if !view_visibility.get() {
            continue;
        }

        // TODO: As above
        let render_visible_entities = visible_entities.clone();
        commands.get_or_spawn(entity).insert((
            ExtractedDirectionalLight {
                color: directional_light.color.into(),
                illuminance: directional_light.illuminance,
                transform: *transform,
                volumetric: volumetric_light.is_some(),
                shadows_enabled: directional_light.shadows_enabled,
                shadow_depth_bias: directional_light.shadow_depth_bias,
                // The factor of SQRT_2 is for the worst-case diagonal offset
                shadow_normal_bias: directional_light.shadow_normal_bias * std::f32::consts::SQRT_2,
                cascade_shadow_config: cascade_config.clone(),
                cascades: cascades.cascades.clone(),
                frusta: frusta.frusta.clone(),
                render_layers: maybe_layers.unwrap_or_default().clone(),
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
    pub depth_attachment: DepthAttachment,
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
pub(crate) fn spot_light_world_from_view(transform: &GlobalTransform) -> Mat4 {
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

pub(crate) fn spot_light_clip_from_view(angle: f32) -> Mat4 {
    // spot light projection FOV is 2x the angle from spot light center to outer edge
    Mat4::perspective_infinite_reverse_rh(angle * 2.0, 1.0, POINT_LIGHT_NEAR_Z)
}

#[allow(clippy::too_many_arguments)]
pub fn prepare_lights(
    mut commands: Commands,
    mut texture_cache: ResMut<TextureCache>,
    render_device: Res<RenderDevice>,
    render_queue: Res<RenderQueue>,
    mut global_light_meta: ResMut<GlobalClusterableObjectMeta>,
    mut light_meta: ResMut<LightMeta>,
    views: Query<(
        Entity,
        &ExtractedView,
        &ExtractedClusterConfig,
        Option<&RenderLayers>,
    )>,
    ambient_light: Res<AmbientLight>,
    point_light_shadow_map: Res<PointLightShadowMap>,
    directional_light_shadow_map: Res<DirectionalLightShadowMap>,
    mut shadow_render_phases: ResMut<ViewBinnedRenderPhases<Shadow>>,
    mut max_directional_lights_warning_emitted: Local<bool>,
    mut max_cascades_per_light_warning_emitted: Local<bool>,
    point_lights: Query<(
        Entity,
        &ExtractedPointLight,
        AnyOf<(&CubemapFrusta, &Frustum)>,
    )>,
    directional_lights: Query<(Entity, &ExtractedDirectionalLight)>,
    mut live_shadow_mapping_lights: Local<EntityHashSet>,
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

    #[cfg(any(
        not(feature = "webgl"),
        not(target_arch = "wasm32"),
        feature = "webgpu"
    ))]
    let max_texture_array_layers = render_device.limits().max_texture_array_layers as usize;
    #[cfg(any(
        not(feature = "webgl"),
        not(target_arch = "wasm32"),
        feature = "webgpu"
    ))]
    let max_texture_cubes = max_texture_array_layers / 6;
    #[cfg(all(feature = "webgl", target_arch = "wasm32", not(feature = "webgpu")))]
    let max_texture_array_layers = 1;
    #[cfg(all(feature = "webgl", target_arch = "wasm32", not(feature = "webgpu")))]
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

    let directional_volumetric_enabled_count = directional_lights
        .iter()
        .take(MAX_DIRECTIONAL_LIGHTS)
        .filter(|(_, light)| light.volumetric)
        .count()
        .min(max_texture_array_layers / MAX_CASCADES_PER_LIGHT);

    let directional_shadow_enabled_count = directional_lights
        .iter()
        .take(MAX_DIRECTIONAL_LIGHTS)
        .filter(|(_, light)| light.shadows_enabled)
        .count()
        .min(max_texture_array_layers / MAX_CASCADES_PER_LIGHT);

    let spot_light_shadow_maps_count = point_lights
        .iter()
        .filter(|(_, light, _)| light.shadows_enabled && light.spot_light_angles.is_some())
        .count()
        .min(max_texture_array_layers - directional_shadow_enabled_count * MAX_CASCADES_PER_LIGHT);

    // Sort lights by
    // - point-light vs spot-light, so that we can iterate point lights and spot lights in contiguous blocks in the fragment shader,
    // - then those with shadows enabled first, so that the index can be used to render at most `point_light_shadow_maps_count`
    //   point light shadows and `spot_light_shadow_maps_count` spot light shadow maps,
    // - then by entity as a stable key to ensure that a consistent set of lights are chosen if the light count limit is exceeded.
    point_lights.sort_by(|(entity_1, light_1, _), (entity_2, light_2, _)| {
        crate::cluster::clusterable_object_order(
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
    // - those with volumetric (and shadows) enabled first, so that the
    //   volumetric lighting pass can quickly find the volumetric lights;
    // - then those with shadows enabled second, so that the index can be used
    //   to render at most `directional_light_shadow_maps_count` directional light
    //   shadows
    // - then by entity as a stable key to ensure that a consistent set of
    //   lights are chosen if the light count limit is exceeded.
    directional_lights.sort_by(|(entity_1, light_1), (entity_2, light_2)| {
        directional_light_order(
            (entity_1, &light_1.volumetric, &light_1.shadows_enabled),
            (entity_2, &light_2.volumetric, &light_2.shadows_enabled),
        )
    });

    if global_light_meta.entity_to_index.capacity() < point_lights.len() {
        global_light_meta
            .entity_to_index
            .reserve(point_lights.len());
    }

    let mut gpu_point_lights = Vec::new();
    for (index, &(entity, light, _)) in point_lights.iter().enumerate() {
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

        gpu_point_lights.push(GpuClusterableObject {
            light_custom_data,
            // premultiply color by intensity
            // we don't use the alpha at all, so no reason to multiply only [0..3]
            color_inverse_square_range: (Vec4::from_slice(&light.color.to_f32_array())
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

        // Lights are sorted, volumetric and shadow enabled lights are first
        if light.volumetric
            && light.shadows_enabled
            && (index < directional_volumetric_enabled_count)
        {
            flags |= DirectionalLightFlags::VOLUMETRIC;
        }
        // Shadow enabled lights are second
        if light.shadows_enabled && (index < directional_shadow_enabled_count) {
            flags |= DirectionalLightFlags::SHADOWS_ENABLED;
        }

        let num_cascades = light
            .cascade_shadow_config
            .bounds
            .len()
            .min(MAX_CASCADES_PER_LIGHT);
        gpu_directional_lights[index] = GpuDirectionalLight {
            // Set to true later when necessary.
            skip: 0u32,
            // Filled in later.
            cascades: [GpuDirectionalCascade::default(); MAX_CASCADES_PER_LIGHT],
            // premultiply color by illuminance
            // we don't use the alpha at all, so no reason to multiply only [0..3]
            color: Vec4::from_slice(&light.color.to_f32_array()) * light.illuminance,
            // direction is negated to be ready for N.L
            dir_to_light: light.transform.back().into(),
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

    global_light_meta
        .gpu_clusterable_objects
        .set(gpu_point_lights);
    global_light_meta
        .gpu_clusterable_objects
        .write_buffer(&render_device, &render_queue);

    live_shadow_mapping_lights.clear();

    // set up light data for each view
    for (entity, extracted_view, clusters, maybe_layers) in &views {
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
                format: CORE_3D_DEPTH_FORMAT,
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
                format: CORE_3D_DEPTH_FORMAT,
                label: Some("directional_light_shadow_map_texture"),
                usage: TextureUsages::RENDER_ATTACHMENT | TextureUsages::TEXTURE_BINDING,
                view_formats: &[],
            },
        );
        let mut view_lights = Vec::new();

        let is_orthographic = extracted_view.clip_from_view.w_axis.w == 1.0;
        let cluster_factors_zw = calculate_cluster_factors(
            clusters.near,
            clusters.far,
            clusters.dimensions.z as f32,
            is_orthographic,
        );

        let n_clusters = clusters.dimensions.x * clusters.dimensions.y * clusters.dimensions.z;
        let mut gpu_lights = GpuLights {
            directional_lights: gpu_directional_lights,
            ambient_color: Vec4::from_slice(&LinearRgba::from(ambient_light.color).to_f32_array())
                * ambient_light.brightness,
            cluster_factors: Vec4::new(
                clusters.dimensions.x as f32 / extracted_view.viewport.z as f32,
                clusters.dimensions.y as f32 / extracted_view.viewport.w as f32,
                cluster_factors_zw.x,
                cluster_factors_zw.y,
            ),
            cluster_dimensions: clusters.dimensions.extend(n_clusters),
            n_directional_lights: directional_lights.iter().len().min(MAX_DIRECTIONAL_LIGHTS)
                as u32,
            // spotlight shadow maps are stored in the directional light array, starting at num_directional_cascades_enabled.
            // the spot lights themselves start in the light array at point_light_count. so to go from light
            // index to shadow map index, we need to subtract point light count and add directional shadowmap count.
            spot_light_shadowmap_offset: num_directional_cascades_enabled as i32
                - point_light_count as i32,
        };

        // TODO: this should select lights based on relevance to the view instead of the first ones that show up in a query
        for &(light_entity, light, (point_light_frusta, _)) in point_lights
            .iter()
            // Lights are sorted, shadow enabled lights are first
            .take(point_light_shadow_maps_count)
            .filter(|(_, light, _)| light.shadows_enabled)
        {
            let light_index = *global_light_meta
                .entity_to_index
                .get(&light_entity)
                .unwrap();
            // ignore scale because we don't want to effectively scale light radius and range
            // by applying those as a view transform to shadow map rendering of objects
            // and ignore rotation because we want the shadow map projections to align with the axes
            let view_translation = GlobalTransform::from_translation(light.transform.translation());

            for (face_index, (view_rotation, frustum)) in cube_face_rotations
                .iter()
                .zip(&point_light_frusta.unwrap().frusta)
                .enumerate()
            {
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
                            depth_attachment: DepthAttachment::new(depth_texture_view, Some(0.0)),
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
                            world_from_view: view_translation * *view_rotation,
                            clip_from_world: None,
                            clip_from_view: cube_face_projection,
                            hdr: false,
                            color_grading: Default::default(),
                        },
                        *frustum,
                        LightEntity::Point {
                            light_entity,
                            face_index,
                        },
                    ))
                    .id();
                view_lights.push(view_light_entity);

                shadow_render_phases.insert_or_clear(view_light_entity);
                live_shadow_mapping_lights.insert(view_light_entity);
            }
        }

        // spot lights
        for (light_index, &(light_entity, light, (_, spot_light_frustum))) in point_lights
            .iter()
            .skip(point_light_count)
            .take(spot_light_shadow_maps_count)
            .enumerate()
        {
            let spot_world_from_view = spot_light_world_from_view(&light.transform);
            let spot_world_from_view = spot_world_from_view.into();

            let angle = light.spot_light_angles.expect("lights should be sorted so that \
                [point_light_count..point_light_count + spot_light_shadow_maps_count] are spot lights").1;
            let spot_projection = spot_light_clip_from_view(angle);

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
                        depth_attachment: DepthAttachment::new(depth_texture_view, Some(0.0)),
                        pass_name: format!("shadow pass spot light {light_index}"),
                    },
                    ExtractedView {
                        viewport: UVec4::new(
                            0,
                            0,
                            directional_light_shadow_map.size as u32,
                            directional_light_shadow_map.size as u32,
                        ),
                        world_from_view: spot_world_from_view,
                        clip_from_view: spot_projection,
                        clip_from_world: None,
                        hdr: false,
                        color_grading: Default::default(),
                    },
                    *spot_light_frustum.unwrap(),
                    LightEntity::Spot { light_entity },
                ))
                .id();

            view_lights.push(view_light_entity);

            shadow_render_phases.insert_or_clear(view_light_entity);
            live_shadow_mapping_lights.insert(view_light_entity);
        }

        // directional lights
        let mut directional_depth_texture_array_index = 0u32;
        let view_layers = maybe_layers.unwrap_or_default();
        for (light_index, &(light_entity, light)) in directional_lights
            .iter()
            .enumerate()
            .take(MAX_DIRECTIONAL_LIGHTS)
        {
            let gpu_light = &mut gpu_lights.directional_lights[light_index];

            // Check if the light intersects with the view.
            if !view_layers.intersects(&light.render_layers) {
                gpu_light.skip = 1u32;
                continue;
            }

            // Only deal with cascades when shadows are enabled.
            if (gpu_light.flags & DirectionalLightFlags::SHADOWS_ENABLED.bits()) == 0u32 {
                continue;
            }

            let cascades = light
                .cascades
                .get(&entity)
                .unwrap()
                .iter()
                .take(MAX_CASCADES_PER_LIGHT);
            let frusta = light
                .frusta
                .get(&entity)
                .unwrap()
                .iter()
                .take(MAX_CASCADES_PER_LIGHT);
            for (cascade_index, ((cascade, frustum), bound)) in cascades
                .zip(frusta)
                .zip(&light.cascade_shadow_config.bounds)
                .enumerate()
            {
                gpu_lights.directional_lights[light_index].cascades[cascade_index] =
                    GpuDirectionalCascade {
                        clip_from_world: cascade.clip_from_world,
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

                let mut frustum = *frustum;
                // Push the near clip plane out to infinity for directional lights
                frustum.half_spaces[4] =
                    HalfSpace::new(frustum.half_spaces[4].normal().extend(f32::INFINITY));

                let view_light_entity = commands
                    .spawn((
                        ShadowView {
                            depth_attachment: DepthAttachment::new(depth_texture_view, Some(0.0)),
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
                            world_from_view: GlobalTransform::from(cascade.world_from_cascade),
                            clip_from_view: cascade.clip_from_cascade,
                            clip_from_world: Some(cascade.clip_from_world),
                            hdr: false,
                            color_grading: Default::default(),
                        },
                        frustum,
                        LightEntity::Directional {
                            light_entity,
                            cascade_index,
                        },
                    ))
                    .id();
                view_lights.push(view_light_entity);

                shadow_render_phases.insert_or_clear(view_light_entity);
                live_shadow_mapping_lights.insert(view_light_entity);
            }
        }

        let point_light_depth_texture_view =
            point_light_depth_texture
                .texture
                .create_view(&TextureViewDescriptor {
                    label: Some("point_light_shadow_map_array_texture_view"),
                    format: None,
                    // NOTE: iOS Simulator is missing CubeArray support so we use Cube instead.
                    // See https://github.com/bevyengine/bevy/pull/12052 - remove if support is added.
                    #[cfg(all(
                        not(feature = "ios_simulator"),
                        any(
                            not(feature = "webgl"),
                            not(target_arch = "wasm32"),
                            feature = "webgpu"
                        )
                    ))]
                    dimension: Some(TextureViewDimension::CubeArray),
                    #[cfg(any(
                        feature = "ios_simulator",
                        all(feature = "webgl", target_arch = "wasm32", not(feature = "webgpu"))
                    ))]
                    dimension: Some(TextureViewDimension::Cube),
                    aspect: TextureAspect::DepthOnly,
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
                #[cfg(any(
                    not(feature = "webgl"),
                    not(target_arch = "wasm32"),
                    feature = "webgpu"
                ))]
                dimension: Some(TextureViewDimension::D2Array),
                #[cfg(all(feature = "webgl", target_arch = "wasm32", not(feature = "webgpu")))]
                dimension: Some(TextureViewDimension::D2),
                aspect: TextureAspect::DepthOnly,
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

    shadow_render_phases.retain(|entity, _| live_shadow_mapping_lights.contains(entity));
}

/// For each shadow cascade, iterates over all the meshes "visible" from it and
/// adds them to [`BinnedRenderPhase`]s or [`SortedRenderPhase`]s as
/// appropriate.
#[allow(clippy::too_many_arguments)]
pub fn queue_shadows<M: Material>(
    shadow_draw_functions: Res<DrawFunctions<Shadow>>,
    prepass_pipeline: Res<PrepassPipeline<M>>,
    render_meshes: Res<RenderAssets<GpuMesh>>,
    render_mesh_instances: Res<RenderMeshInstances>,
    render_materials: Res<RenderAssets<PreparedMaterial<M>>>,
    render_material_instances: Res<RenderMaterialInstances<M>>,
    mut shadow_render_phases: ResMut<ViewBinnedRenderPhases<Shadow>>,
    mut pipelines: ResMut<SpecializedMeshPipelines<PrepassPipeline<M>>>,
    pipeline_cache: Res<PipelineCache>,
    render_lightmaps: Res<RenderLightmaps>,
    view_lights: Query<(Entity, &ViewLightEntities)>,
    mut view_light_entities: Query<&LightEntity>,
    point_light_entities: Query<&CubemapVisibleEntities, With<ExtractedPointLight>>,
    directional_light_entities: Query<&CascadesVisibleEntities, With<ExtractedDirectionalLight>>,
    spot_light_entities: Query<&VisibleEntities, With<ExtractedPointLight>>,
) where
    M::Data: PartialEq + Eq + Hash + Clone,
{
    for (entity, view_lights) in &view_lights {
        let draw_shadow_mesh = shadow_draw_functions.read().id::<DrawPrepass<M>>();
        for view_light_entity in view_lights.lights.iter().copied() {
            let Ok(light_entity) = view_light_entities.get_mut(view_light_entity) else {
                continue;
            };
            let Some(shadow_phase) = shadow_render_phases.get_mut(&view_light_entity) else {
                continue;
            };

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
            let mut light_key = MeshPipelineKey::DEPTH_PREPASS;
            light_key.set(MeshPipelineKey::DEPTH_CLAMP_ORTHO, is_directional_light);

            // NOTE: Lights with shadow mapping disabled will have no visible entities
            // so no meshes will be queued

            for entity in visible_entities.iter::<WithMesh>().copied() {
                let Some(mesh_instance) = render_mesh_instances.render_mesh_queue_data(entity)
                else {
                    continue;
                };
                if !mesh_instance
                    .flags
                    .contains(RenderMeshInstanceFlags::SHADOW_CASTER)
                {
                    continue;
                }
                let Some(material_asset_id) = render_material_instances.get(&entity) else {
                    continue;
                };
                let Some(material) = render_materials.get(*material_asset_id) else {
                    continue;
                };
                let Some(mesh) = render_meshes.get(mesh_instance.mesh_asset_id) else {
                    continue;
                };

                let mut mesh_key =
                    light_key | MeshPipelineKey::from_bits_retain(mesh.key_bits.bits());

                // Even though we don't use the lightmap in the shadow map, the
                // `SetMeshBindGroup` render command will bind the data for it. So
                // we need to include the appropriate flag in the mesh pipeline key
                // to ensure that the necessary bind group layout entries are
                // present.
                if render_lightmaps.render_lightmaps.contains_key(&entity) {
                    mesh_key |= MeshPipelineKey::LIGHTMAPPED;
                }

                mesh_key |= match material.properties.alpha_mode {
                    AlphaMode::Mask(_)
                    | AlphaMode::Blend
                    | AlphaMode::Premultiplied
                    | AlphaMode::Add
                    | AlphaMode::AlphaToCoverage => MeshPipelineKey::MAY_DISCARD,
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

                mesh_instance
                    .material_bind_group_id
                    .set(material.get_bind_group_id());

                shadow_phase.add(
                    ShadowBinKey {
                        draw_function: draw_shadow_mesh,
                        pipeline: pipeline_id,
                        asset_id: mesh_instance.mesh_asset_id,
                    },
                    entity,
                    mesh_instance.should_batch(),
                );
            }
        }
    }
}

pub struct Shadow {
    pub key: ShadowBinKey,
    pub representative_entity: Entity,
    pub batch_range: Range<u32>,
    pub extra_index: PhaseItemExtraIndex,
}

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ShadowBinKey {
    /// The identifier of the render pipeline.
    pub pipeline: CachedRenderPipelineId,

    /// The function used to draw.
    pub draw_function: DrawFunctionId,

    /// The mesh.
    pub asset_id: AssetId<Mesh>,
}

impl PhaseItem for Shadow {
    #[inline]
    fn entity(&self) -> Entity {
        self.representative_entity
    }

    #[inline]
    fn draw_function(&self) -> DrawFunctionId {
        self.key.draw_function
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
    fn extra_index(&self) -> PhaseItemExtraIndex {
        self.extra_index
    }

    #[inline]
    fn batch_range_and_extra_index_mut(&mut self) -> (&mut Range<u32>, &mut PhaseItemExtraIndex) {
        (&mut self.batch_range, &mut self.extra_index)
    }
}

impl BinnedPhaseItem for Shadow {
    type BinKey = ShadowBinKey;

    #[inline]
    fn new(
        key: Self::BinKey,
        representative_entity: Entity,
        batch_range: Range<u32>,
        extra_index: PhaseItemExtraIndex,
    ) -> Self {
        Shadow {
            key,
            representative_entity,
            batch_range,
            extra_index,
        }
    }
}

impl CachedRenderPipelinePhaseItem for Shadow {
    #[inline]
    fn cached_pipeline(&self) -> CachedRenderPipelineId {
        self.key.pipeline
    }
}

pub struct ShadowPassNode {
    main_view_query: QueryState<Read<ViewLightEntities>>,
    view_light_query: QueryState<Read<ShadowView>>,
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

    fn run<'w>(
        &self,
        graph: &mut RenderGraphContext,
        render_context: &mut RenderContext<'w>,
        world: &'w World,
    ) -> Result<(), NodeRunError> {
        let diagnostics = render_context.diagnostic_recorder();

        let view_entity = graph.view_entity();

        let Some(shadow_render_phases) = world.get_resource::<ViewBinnedRenderPhases<Shadow>>()
        else {
            return Ok(());
        };

        let time_span = diagnostics.time_span(render_context.command_encoder(), "shadows");

        if let Ok(view_lights) = self.main_view_query.get_manual(world, view_entity) {
            for view_light_entity in view_lights.lights.iter().copied() {
                let Some(shadow_phase) = shadow_render_phases.get(&view_light_entity) else {
                    continue;
                };

                let view_light = self
                    .view_light_query
                    .get_manual(world, view_light_entity)
                    .unwrap();

                let depth_stencil_attachment =
                    Some(view_light.depth_attachment.get_attachment(StoreOp::Store));

                let diagnostics = render_context.diagnostic_recorder();
                render_context.add_command_buffer_generation_task(move |render_device| {
                    #[cfg(feature = "trace")]
                    let _shadow_pass_span = info_span!("", "{}", view_light.pass_name).entered();
                    let mut command_encoder =
                        render_device.create_command_encoder(&CommandEncoderDescriptor {
                            label: Some("shadow_pass_command_encoder"),
                        });

                    let render_pass = command_encoder.begin_render_pass(&RenderPassDescriptor {
                        label: Some(&view_light.pass_name),
                        color_attachments: &[],
                        depth_stencil_attachment,
                        timestamp_writes: None,
                        occlusion_query_set: None,
                    });

                    let mut render_pass = TrackedRenderPass::new(&render_device, render_pass);
                    let pass_span =
                        diagnostics.pass_span(&mut render_pass, view_light.pass_name.clone());

                    shadow_phase.render(&mut render_pass, world, view_light_entity);

                    pass_span.end(&mut render_pass);
                    drop(render_pass);
                    command_encoder.finish()
                });
            }
        }

        time_span.end(render_context.command_encoder());

        Ok(())
    }
}
