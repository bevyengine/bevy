use self::assign::ClusterableObjectType;
use crate::material_bind_groups::MaterialBindGroupAllocator;
use crate::*;
use bevy_asset::UntypedAssetId;
use bevy_color::ColorToComponents;
use bevy_core_pipeline::core_3d::{Camera3d, CORE_3D_DEPTH_FORMAT};
use bevy_derive::{Deref, DerefMut};
use bevy_ecs::component::Tick;
use bevy_ecs::system::SystemChangeTick;
use bevy_ecs::{
    entity::{EntityHashMap, EntityHashSet},
    prelude::*,
    system::lifetimeless::Read,
};
use bevy_math::{ops, Mat4, UVec4, Vec2, Vec3, Vec3Swizzles, Vec4, Vec4Swizzles};
use bevy_platform::collections::{HashMap, HashSet};
use bevy_platform::hash::FixedHasher;
use bevy_render::experimental::occlusion_culling::{
    OcclusionCulling, OcclusionCullingSubview, OcclusionCullingSubviewEntities,
};
use bevy_render::sync_world::MainEntityHashMap;
use bevy_render::{
    batching::gpu_preprocessing::{GpuPreprocessingMode, GpuPreprocessingSupport},
    camera::SortedCameras,
    mesh::allocator::MeshAllocator,
    view::{NoIndirectDrawing, RetainedViewEntity},
};
use bevy_render::{
    diagnostic::RecordDiagnostics,
    mesh::RenderMesh,
    primitives::{CascadesFrusta, CubemapFrusta, Frustum, HalfSpace},
    render_asset::RenderAssets,
    render_graph::{Node, NodeRunError, RenderGraphContext},
    render_phase::*,
    render_resource::*,
    renderer::{RenderContext, RenderDevice, RenderQueue},
    texture::*,
    view::{ExtractedView, RenderLayers, ViewVisibility},
    Extract,
};
use bevy_render::{
    mesh::allocator::SlabId,
    sync_world::{MainEntity, RenderEntity},
};
use bevy_transform::{components::GlobalTransform, prelude::Transform};
use bevy_utils::default;
use core::{hash::Hash, marker::PhantomData, ops::Range};
#[cfg(feature = "trace")]
use tracing::info_span;
use tracing::{error, warn};

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
    pub shadow_map_near_z: f32,
    pub spot_light_angles: Option<(f32, f32)>,
    pub volumetric: bool,
    pub soft_shadows_enabled: bool,
    /// whether this point light contributes diffuse light to lightmapped meshes
    pub affects_lightmapped_mesh_diffuse: bool,
}

#[derive(Component, Debug)]
pub struct ExtractedDirectionalLight {
    pub color: LinearRgba,
    pub illuminance: f32,
    pub transform: GlobalTransform,
    pub shadows_enabled: bool,
    pub volumetric: bool,
    /// whether this directional light contributes diffuse light to lightmapped
    /// meshes
    pub affects_lightmapped_mesh_diffuse: bool,
    pub shadow_depth_bias: f32,
    pub shadow_normal_bias: f32,
    pub cascade_shadow_config: CascadeShadowConfig,
    pub cascades: EntityHashMap<Vec<Cascade>>,
    pub frusta: EntityHashMap<Vec<Frustum>>,
    pub render_layers: RenderLayers,
    pub soft_shadow_size: Option<f32>,
    /// True if this light is using two-phase occlusion culling.
    pub occlusion_culling: bool,
}

// NOTE: These must match the bit flags in bevy_pbr/src/render/mesh_view_types.wgsl!
bitflags::bitflags! {
    #[repr(transparent)]
    struct PointLightFlags: u32 {
        const SHADOWS_ENABLED                   = 1 << 0;
        const SPOT_LIGHT_Y_NEGATIVE             = 1 << 1;
        const VOLUMETRIC                        = 1 << 2;
        const AFFECTS_LIGHTMAPPED_MESH_DIFFUSE  = 1 << 3;
        const NONE                              = 0;
        const UNINITIALIZED                     = 0xFFFF;
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
    soft_shadow_size: f32,
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
        const SHADOWS_ENABLED                   = 1 << 0;
        const VOLUMETRIC                        = 1 << 1;
        const AFFECTS_LIGHTMAPPED_MESH_DIFFUSE  = 1 << 2;
        const NONE                              = 0;
        const UNINITIALIZED                     = 0xFFFF;
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
    ambient_light_affects_lightmapped_meshes: u32,
}

// NOTE: When running bevy on Adreno GPU chipsets in WebGL, any value above 1 will result in a crash
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
    pub point_light_comparison_sampler: Sampler,
    #[cfg(feature = "experimental_pbr_pcss")]
    pub point_light_linear_sampler: Sampler,
    pub directional_light_comparison_sampler: Sampler,
    #[cfg(feature = "experimental_pbr_pcss")]
    pub directional_light_linear_sampler: Sampler,
}

// TODO: this pattern for initializing the shaders / pipeline isn't ideal. this should be handled by the asset system
impl FromWorld for ShadowSamplers {
    fn from_world(world: &mut World) -> Self {
        let render_device = world.resource::<RenderDevice>();

        let base_sampler_descriptor = SamplerDescriptor {
            address_mode_u: AddressMode::ClampToEdge,
            address_mode_v: AddressMode::ClampToEdge,
            address_mode_w: AddressMode::ClampToEdge,
            mag_filter: FilterMode::Linear,
            min_filter: FilterMode::Linear,
            mipmap_filter: FilterMode::Nearest,
            ..default()
        };

        ShadowSamplers {
            point_light_comparison_sampler: render_device.create_sampler(&SamplerDescriptor {
                compare: Some(CompareFunction::GreaterEqual),
                ..base_sampler_descriptor
            }),
            #[cfg(feature = "experimental_pbr_pcss")]
            point_light_linear_sampler: render_device.create_sampler(&base_sampler_descriptor),
            directional_light_comparison_sampler: render_device.create_sampler(
                &SamplerDescriptor {
                    compare: Some(CompareFunction::GreaterEqual),
                    ..base_sampler_descriptor
                },
            ),
            #[cfg(feature = "experimental_pbr_pcss")]
            directional_light_linear_sampler: render_device
                .create_sampler(&base_sampler_descriptor),
        }
    }
}

pub fn extract_lights(
    mut commands: Commands,
    point_light_shadow_map: Extract<Res<PointLightShadowMap>>,
    directional_light_shadow_map: Extract<Res<DirectionalLightShadowMap>>,
    global_visible_clusterable: Extract<Res<GlobalVisibleClusterableObjects>>,
    previous_point_lights: Query<
        Entity,
        (
            With<RenderCubemapVisibleEntities>,
            With<ExtractedPointLight>,
        ),
    >,
    previous_spot_lights: Query<
        Entity,
        (With<RenderVisibleMeshEntities>, With<ExtractedPointLight>),
    >,
    point_lights: Extract<
        Query<(
            Entity,
            RenderEntity,
            &PointLight,
            &CubemapVisibleEntities,
            &GlobalTransform,
            &ViewVisibility,
            &CubemapFrusta,
            Option<&VolumetricLight>,
        )>,
    >,
    spot_lights: Extract<
        Query<(
            Entity,
            RenderEntity,
            &SpotLight,
            &VisibleMeshEntities,
            &GlobalTransform,
            &ViewVisibility,
            &Frustum,
            Option<&VolumetricLight>,
        )>,
    >,
    directional_lights: Extract<
        Query<
            (
                Entity,
                RenderEntity,
                &DirectionalLight,
                &CascadesVisibleEntities,
                &Cascades,
                &CascadeShadowConfig,
                &CascadesFrusta,
                &GlobalTransform,
                &ViewVisibility,
                Option<&RenderLayers>,
                Option<&VolumetricLight>,
                Has<OcclusionCulling>,
            ),
            Without<SpotLight>,
        >,
    >,
    mapper: Extract<Query<RenderEntity>>,
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

    // Clear previous visible entities for all point/spot lights as they might not be in the
    // `global_visible_clusterable` list anymore.
    commands.try_insert_batch(
        previous_point_lights
            .iter()
            .map(|render_entity| (render_entity, RenderCubemapVisibleEntities::default()))
            .collect::<Vec<_>>(),
    );
    commands.try_insert_batch(
        previous_spot_lights
            .iter()
            .map(|render_entity| (render_entity, RenderVisibleMeshEntities::default()))
            .collect::<Vec<_>>(),
    );

    // This is the point light shadow map texel size for one face of the cube as a distance of 1.0
    // world unit from the light.
    // point_light_texel_size = 2.0 * 1.0 * tan(PI / 4.0) / cube face width in texels
    // PI / 4.0 is half the cube face fov, tan(PI / 4.0) = 1.0, so this simplifies to:
    // point_light_texel_size = 2.0 / cube face width in texels
    // NOTE: When using various PCF kernel sizes, this will need to be adjusted, according to:
    // https://catlikecoding.com/unity/tutorials/custom-srp/point-and-spot-shadows/
    let point_light_texel_size = 2.0 / point_light_shadow_map.size as f32;

    let mut point_lights_values = Vec::with_capacity(*previous_point_lights_len);
    for entity in global_visible_clusterable.iter().copied() {
        let Ok((
            main_entity,
            render_entity,
            point_light,
            cubemap_visible_entities,
            transform,
            view_visibility,
            frusta,
            volumetric_light,
        )) = point_lights.get(entity)
        else {
            continue;
        };
        if !view_visibility.get() {
            continue;
        }
        let render_cubemap_visible_entities = RenderCubemapVisibleEntities {
            data: cubemap_visible_entities
                .iter()
                .map(|v| create_render_visible_mesh_entities(&mapper, v))
                .collect::<Vec<_>>()
                .try_into()
                .unwrap(),
        };

        let extracted_point_light = ExtractedPointLight {
            color: point_light.color.into(),
            // NOTE: Map from luminous power in lumens to luminous intensity in lumens per steradian
            // for a point light. See https://google.github.io/filament/Filament.html#mjx-eqn-pointLightLuminousPower
            // for details.
            intensity: point_light.intensity / (4.0 * core::f32::consts::PI),
            range: point_light.range,
            radius: point_light.radius,
            transform: *transform,
            shadows_enabled: point_light.shadows_enabled,
            shadow_depth_bias: point_light.shadow_depth_bias,
            // The factor of SQRT_2 is for the worst-case diagonal offset
            shadow_normal_bias: point_light.shadow_normal_bias
                * point_light_texel_size
                * core::f32::consts::SQRT_2,
            shadow_map_near_z: point_light.shadow_map_near_z,
            spot_light_angles: None,
            volumetric: volumetric_light.is_some(),
            affects_lightmapped_mesh_diffuse: point_light.affects_lightmapped_mesh_diffuse,
            #[cfg(feature = "experimental_pbr_pcss")]
            soft_shadows_enabled: point_light.soft_shadows_enabled,
            #[cfg(not(feature = "experimental_pbr_pcss"))]
            soft_shadows_enabled: false,
        };
        point_lights_values.push((
            render_entity,
            (
                extracted_point_light,
                render_cubemap_visible_entities,
                (*frusta).clone(),
                MainEntity::from(main_entity),
            ),
        ));
    }
    *previous_point_lights_len = point_lights_values.len();
    commands.try_insert_batch(point_lights_values);

    let mut spot_lights_values = Vec::with_capacity(*previous_spot_lights_len);
    for entity in global_visible_clusterable.iter().copied() {
        if let Ok((
            main_entity,
            render_entity,
            spot_light,
            visible_entities,
            transform,
            view_visibility,
            frustum,
            volumetric_light,
        )) = spot_lights.get(entity)
        {
            if !view_visibility.get() {
                continue;
            }
            let render_visible_entities =
                create_render_visible_mesh_entities(&mapper, visible_entities);

            let texel_size =
                2.0 * ops::tan(spot_light.outer_angle) / directional_light_shadow_map.size as f32;

            spot_lights_values.push((
                render_entity,
                (
                    ExtractedPointLight {
                        color: spot_light.color.into(),
                        // NOTE: Map from luminous power in lumens to luminous intensity in lumens per steradian
                        // for a point light. See https://google.github.io/filament/Filament.html#mjx-eqn-pointLightLuminousPower
                        // for details.
                        // Note: Filament uses a divisor of PI for spot lights. We choose to use the same 4*PI divisor
                        // in both cases so that toggling between point light and spot light keeps lit areas lit equally,
                        // which seems least surprising for users
                        intensity: spot_light.intensity / (4.0 * core::f32::consts::PI),
                        range: spot_light.range,
                        radius: spot_light.radius,
                        transform: *transform,
                        shadows_enabled: spot_light.shadows_enabled,
                        shadow_depth_bias: spot_light.shadow_depth_bias,
                        // The factor of SQRT_2 is for the worst-case diagonal offset
                        shadow_normal_bias: spot_light.shadow_normal_bias
                            * texel_size
                            * core::f32::consts::SQRT_2,
                        shadow_map_near_z: spot_light.shadow_map_near_z,
                        spot_light_angles: Some((spot_light.inner_angle, spot_light.outer_angle)),
                        volumetric: volumetric_light.is_some(),
                        affects_lightmapped_mesh_diffuse: spot_light
                            .affects_lightmapped_mesh_diffuse,
                        #[cfg(feature = "experimental_pbr_pcss")]
                        soft_shadows_enabled: spot_light.soft_shadows_enabled,
                        #[cfg(not(feature = "experimental_pbr_pcss"))]
                        soft_shadows_enabled: false,
                    },
                    render_visible_entities,
                    *frustum,
                    MainEntity::from(main_entity),
                ),
            ));
        }
    }
    *previous_spot_lights_len = spot_lights_values.len();
    commands.try_insert_batch(spot_lights_values);

    for (
        main_entity,
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
        occlusion_culling,
    ) in &directional_lights
    {
        if !view_visibility.get() {
            commands
                .get_entity(entity)
                .expect("Light entity wasn't synced.")
                .remove::<(ExtractedDirectionalLight, RenderCascadesVisibleEntities)>();
            continue;
        }

        // TODO: update in place instead of reinserting.
        let mut extracted_cascades = EntityHashMap::default();
        let mut extracted_frusta = EntityHashMap::default();
        let mut cascade_visible_entities = EntityHashMap::default();
        for (e, v) in cascades.cascades.iter() {
            if let Ok(entity) = mapper.get(*e) {
                extracted_cascades.insert(entity, v.clone());
            } else {
                break;
            }
        }
        for (e, v) in frusta.frusta.iter() {
            if let Ok(entity) = mapper.get(*e) {
                extracted_frusta.insert(entity, v.clone());
            } else {
                break;
            }
        }
        for (e, v) in visible_entities.entities.iter() {
            if let Ok(entity) = mapper.get(*e) {
                cascade_visible_entities.insert(
                    entity,
                    v.iter()
                        .map(|v| create_render_visible_mesh_entities(&mapper, v))
                        .collect(),
                );
            } else {
                break;
            }
        }

        commands
            .get_entity(entity)
            .expect("Light entity wasn't synced.")
            .insert((
                ExtractedDirectionalLight {
                    color: directional_light.color.into(),
                    illuminance: directional_light.illuminance,
                    transform: *transform,
                    volumetric: volumetric_light.is_some(),
                    affects_lightmapped_mesh_diffuse: directional_light
                        .affects_lightmapped_mesh_diffuse,
                    #[cfg(feature = "experimental_pbr_pcss")]
                    soft_shadow_size: directional_light.soft_shadow_size,
                    #[cfg(not(feature = "experimental_pbr_pcss"))]
                    soft_shadow_size: None,
                    shadows_enabled: directional_light.shadows_enabled,
                    shadow_depth_bias: directional_light.shadow_depth_bias,
                    // The factor of SQRT_2 is for the worst-case diagonal offset
                    shadow_normal_bias: directional_light.shadow_normal_bias
                        * core::f32::consts::SQRT_2,
                    cascade_shadow_config: cascade_config.clone(),
                    cascades: extracted_cascades,
                    frusta: extracted_frusta,
                    render_layers: maybe_layers.unwrap_or_default().clone(),
                    occlusion_culling,
                },
                RenderCascadesVisibleEntities {
                    entities: cascade_visible_entities,
                },
                MainEntity::from(main_entity),
            ));
    }
}

fn create_render_visible_mesh_entities(
    mapper: &Extract<Query<RenderEntity>>,
    visible_entities: &VisibleMeshEntities,
) -> RenderVisibleMeshEntities {
    RenderVisibleMeshEntities {
        entities: visible_entities
            .iter()
            .map(|e| {
                let render_entity = mapper.get(*e).unwrap_or(Entity::PLACEHOLDER);
                (render_entity, MainEntity::from(*e))
            })
            .collect(),
    }
}

#[derive(Component, Default, Deref, DerefMut)]
/// Component automatically attached to a light entity to track light-view entities
/// for each view.
pub struct LightViewEntities(EntityHashMap<Vec<Entity>>);

// TODO: using required component
pub(crate) fn add_light_view_entities(
    trigger: Trigger<OnAdd, (ExtractedDirectionalLight, ExtractedPointLight)>,
    mut commands: Commands,
) {
    if let Ok(mut v) = commands.get_entity(trigger.target()) {
        v.insert(LightViewEntities::default());
    }
}

/// Removes [`LightViewEntities`] when light is removed. See [`add_light_view_entities`].
pub(crate) fn extracted_light_removed(
    trigger: Trigger<OnRemove, (ExtractedDirectionalLight, ExtractedPointLight)>,
    mut commands: Commands,
) {
    if let Ok(mut v) = commands.get_entity(trigger.target()) {
        v.try_remove::<LightViewEntities>();
    }
}

pub(crate) fn remove_light_view_entities(
    trigger: Trigger<OnRemove, LightViewEntities>,
    query: Query<&LightViewEntities>,
    mut commands: Commands,
) {
    if let Ok(entities) = query.get(trigger.target()) {
        for v in entities.0.values() {
            for e in v.iter().copied() {
                if let Ok(mut v) = commands.get_entity(e) {
                    v.despawn();
                }
            }
        }
    }
}

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

/// A component that holds the shadow cascade views for all shadow cascades
/// associated with a camera.
///
/// Note: Despite the name, this component actually holds the shadow cascade
/// views, not the lights themselves.
#[derive(Component)]
pub struct ViewLightEntities {
    /// The shadow cascade views for all shadow cascades associated with a
    /// camera.
    ///
    /// Note: Despite the name, this component actually holds the shadow cascade
    /// views, not the lights themselves.
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
        let z_slices_of_ln_zfar_over_znear = (z_slices - 1.0) / ops::ln(far / near);
        Vec2::new(
            z_slices_of_ln_zfar_over_znear,
            ops::ln(near) * z_slices_of_ln_zfar_over_znear,
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

pub(crate) fn spot_light_clip_from_view(angle: f32, near_z: f32) -> Mat4 {
    // spot light projection FOV is 2x the angle from spot light center to outer edge
    Mat4::perspective_infinite_reverse_rh(angle * 2.0, 1.0, near_z)
}

pub fn prepare_lights(
    mut commands: Commands,
    mut texture_cache: ResMut<TextureCache>,
    (render_device, render_queue): (Res<RenderDevice>, Res<RenderQueue>),
    mut global_light_meta: ResMut<GlobalClusterableObjectMeta>,
    mut light_meta: ResMut<LightMeta>,
    views: Query<
        (
            Entity,
            MainEntity,
            &ExtractedView,
            &ExtractedClusterConfig,
            Option<&RenderLayers>,
            Has<NoIndirectDrawing>,
            Option<&AmbientLight>,
        ),
        With<Camera3d>,
    >,
    ambient_light: Res<AmbientLight>,
    point_light_shadow_map: Res<PointLightShadowMap>,
    directional_light_shadow_map: Res<DirectionalLightShadowMap>,
    mut shadow_render_phases: ResMut<ViewBinnedRenderPhases<Shadow>>,
    (
        mut max_directional_lights_warning_emitted,
        mut max_cascades_per_light_warning_emitted,
        mut live_shadow_mapping_lights,
    ): (Local<bool>, Local<bool>, Local<HashSet<RetainedViewEntity>>),
    point_lights: Query<(
        Entity,
        &MainEntity,
        &ExtractedPointLight,
        AnyOf<(&CubemapFrusta, &Frustum)>,
    )>,
    directional_lights: Query<(Entity, &MainEntity, &ExtractedDirectionalLight)>,
    mut light_view_entities: Query<&mut LightViewEntities>,
    sorted_cameras: Res<SortedCameras>,
    gpu_preprocessing_support: Res<GpuPreprocessingSupport>,
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
            .any(|(_, _, light)| light.cascade_shadow_config.bounds.len() > MAX_CASCADES_PER_LIGHT)
    {
        warn!(
            "The number of cascades configured for a directional light exceeds the supported limit of {}.",
            MAX_CASCADES_PER_LIGHT
        );
        *max_cascades_per_light_warning_emitted = true;
    }

    let point_light_count = point_lights
        .iter()
        .filter(|light| light.2.spot_light_angles.is_none())
        .count();

    let point_light_volumetric_enabled_count = point_lights
        .iter()
        .filter(|(_, _, light, _)| light.volumetric && light.spot_light_angles.is_none())
        .count()
        .min(max_texture_cubes);

    let point_light_shadow_maps_count = point_lights
        .iter()
        .filter(|light| light.2.shadows_enabled && light.2.spot_light_angles.is_none())
        .count()
        .min(max_texture_cubes);

    let directional_volumetric_enabled_count = directional_lights
        .iter()
        .take(MAX_DIRECTIONAL_LIGHTS)
        .filter(|(_, _, light)| light.volumetric)
        .count()
        .min(max_texture_array_layers / MAX_CASCADES_PER_LIGHT);

    let directional_shadow_enabled_count = directional_lights
        .iter()
        .take(MAX_DIRECTIONAL_LIGHTS)
        .filter(|(_, _, light)| light.shadows_enabled)
        .count()
        .min(max_texture_array_layers / MAX_CASCADES_PER_LIGHT);

    let spot_light_count = point_lights
        .iter()
        .filter(|(_, _, light, _)| light.spot_light_angles.is_some())
        .count()
        .min(max_texture_array_layers - directional_shadow_enabled_count * MAX_CASCADES_PER_LIGHT);

    let spot_light_volumetric_enabled_count = point_lights
        .iter()
        .filter(|(_, _, light, _)| light.volumetric && light.spot_light_angles.is_some())
        .count()
        .min(max_texture_array_layers - directional_shadow_enabled_count * MAX_CASCADES_PER_LIGHT);

    let spot_light_shadow_maps_count = point_lights
        .iter()
        .filter(|(_, _, light, _)| light.shadows_enabled && light.spot_light_angles.is_some())
        .count()
        .min(max_texture_array_layers - directional_shadow_enabled_count * MAX_CASCADES_PER_LIGHT);

    // Sort lights by
    // - point-light vs spot-light, so that we can iterate point lights and spot lights in contiguous blocks in the fragment shader,
    // - then those with shadows enabled first, so that the index can be used to render at most `point_light_shadow_maps_count`
    //   point light shadows and `spot_light_shadow_maps_count` spot light shadow maps,
    // - then by entity as a stable key to ensure that a consistent set of lights are chosen if the light count limit is exceeded.
    point_lights.sort_by_cached_key(|(entity, _, light, _)| {
        (
            ClusterableObjectType::from_point_or_spot_light(light).ordering(),
            *entity,
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
    // - because entities are unique, we can use `sort_unstable_by_key`
    //   and still end up with a stable order.
    directional_lights.sort_unstable_by_key(|(entity, _, light)| {
        (light.volumetric, light.shadows_enabled, *entity)
    });

    if global_light_meta.entity_to_index.capacity() < point_lights.len() {
        global_light_meta
            .entity_to_index
            .reserve(point_lights.len());
    }

    let mut gpu_point_lights = Vec::new();
    for (index, &(entity, _, light, _)) in point_lights.iter().enumerate() {
        let mut flags = PointLightFlags::NONE;

        // Lights are sorted, shadow enabled lights are first
        if light.shadows_enabled
            && (index < point_light_shadow_maps_count
                || (light.spot_light_angles.is_some()
                    && index - point_light_count < spot_light_shadow_maps_count))
        {
            flags |= PointLightFlags::SHADOWS_ENABLED;
        }

        let cube_face_projection = Mat4::perspective_infinite_reverse_rh(
            core::f32::consts::FRAC_PI_2,
            1.0,
            light.shadow_map_near_z,
        );
        if light.shadows_enabled
            && light.volumetric
            && (index < point_light_volumetric_enabled_count
                || (light.spot_light_angles.is_some()
                    && index - point_light_count < spot_light_volumetric_enabled_count))
        {
            flags |= PointLightFlags::VOLUMETRIC;
        }

        if light.affects_lightmapped_mesh_diffuse {
            flags |= PointLightFlags::AFFECTS_LIGHTMAPPED_MESH_DIFFUSE;
        }

        let (light_custom_data, spot_light_tan_angle) = match light.spot_light_angles {
            Some((inner, outer)) => {
                let light_direction = light.transform.forward();
                if light_direction.y.is_sign_negative() {
                    flags |= PointLightFlags::SPOT_LIGHT_Y_NEGATIVE;
                }

                let cos_outer = ops::cos(outer);
                let spot_scale = 1.0 / f32::max(ops::cos(inner) - cos_outer, 1e-4);
                let spot_offset = -cos_outer * spot_scale;

                (
                    // For spot lights: the direction (x,z), spot_scale and spot_offset
                    light_direction.xz().extend(spot_scale).extend(spot_offset),
                    ops::tan(outer),
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
            shadow_map_near_z: light.shadow_map_near_z,
            spot_light_tan_angle,
            pad_a: 0.0,
            pad_b: 0.0,
            soft_shadow_size: if light.soft_shadows_enabled {
                light.radius
            } else {
                0.0
            },
        });
        global_light_meta.entity_to_index.insert(entity, index);
    }

    let mut gpu_directional_lights = [GpuDirectionalLight::default(); MAX_DIRECTIONAL_LIGHTS];
    let mut num_directional_cascades_enabled = 0usize;
    for (index, (_light_entity, _, light)) in directional_lights
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

        if light.affects_lightmapped_mesh_diffuse {
            flags |= DirectionalLightFlags::AFFECTS_LIGHTMAPPED_MESH_DIFFUSE;
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
            soft_shadow_size: light.soft_shadow_size.unwrap_or_default(),
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

    let mut point_light_depth_attachments = HashMap::<u32, DepthAttachment>::default();
    let mut directional_light_depth_attachments = HashMap::<u32, DepthAttachment>::default();

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

    let point_light_depth_texture_view =
        point_light_depth_texture
            .texture
            .create_view(&TextureViewDescriptor {
                label: Some("point_light_shadow_map_array_texture_view"),
                format: None,
                // NOTE: iOS Simulator is missing CubeArray support so we use Cube instead.
                // See https://github.com/bevyengine/bevy/pull/12052 - remove if support is added.
                #[cfg(all(
                    not(target_abi = "sim"),
                    any(
                        not(feature = "webgl"),
                        not(target_arch = "wasm32"),
                        feature = "webgpu"
                    )
                ))]
                dimension: Some(TextureViewDimension::CubeArray),
                #[cfg(any(
                    target_abi = "sim",
                    all(feature = "webgl", target_arch = "wasm32", not(feature = "webgpu"))
                ))]
                dimension: Some(TextureViewDimension::Cube),
                usage: None,
                aspect: TextureAspect::DepthOnly,
                base_mip_level: 0,
                mip_level_count: None,
                base_array_layer: 0,
                array_layer_count: None,
            });

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

    let directional_light_depth_texture_view =
        directional_light_depth_texture
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
                usage: None,
                aspect: TextureAspect::DepthOnly,
                base_mip_level: 0,
                mip_level_count: None,
                base_array_layer: 0,
                array_layer_count: None,
            });

    let mut live_views = EntityHashSet::with_capacity(views_count);

    // set up light data for each view
    for (
        entity,
        camera_main_entity,
        extracted_view,
        clusters,
        maybe_layers,
        no_indirect_drawing,
        maybe_ambient_override,
    ) in sorted_cameras
        .0
        .iter()
        .filter_map(|sorted_camera| views.get(sorted_camera.entity).ok())
    {
        live_views.insert(entity);

        let mut view_lights = Vec::new();
        let mut view_occlusion_culling_lights = Vec::new();

        let gpu_preprocessing_mode = gpu_preprocessing_support.min(if !no_indirect_drawing {
            GpuPreprocessingMode::Culling
        } else {
            GpuPreprocessingMode::PreprocessingOnly
        });

        let is_orthographic = extracted_view.clip_from_view.w_axis.w == 1.0;
        let cluster_factors_zw = calculate_cluster_factors(
            clusters.near,
            clusters.far,
            clusters.dimensions.z as f32,
            is_orthographic,
        );

        let n_clusters = clusters.dimensions.x * clusters.dimensions.y * clusters.dimensions.z;
        let ambient_light = maybe_ambient_override.unwrap_or(&ambient_light);
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
            ambient_light_affects_lightmapped_meshes: ambient_light.affects_lightmapped_meshes
                as u32,
        };

        // TODO: this should select lights based on relevance to the view instead of the first ones that show up in a query
        for &(light_entity, light_main_entity, light, (point_light_frusta, _)) in point_lights
            .iter()
            // Lights are sorted, shadow enabled lights are first
            .take(point_light_count.min(max_texture_cubes))
        {
            let Ok(mut light_view_entities) = light_view_entities.get_mut(light_entity) else {
                continue;
            };

            if !light.shadows_enabled {
                if let Some(entities) = light_view_entities.remove(&entity) {
                    despawn_entities(&mut commands, entities);
                }
                continue;
            }

            let light_index = *global_light_meta
                .entity_to_index
                .get(&light_entity)
                .unwrap();
            // ignore scale because we don't want to effectively scale light radius and range
            // by applying those as a view transform to shadow map rendering of objects
            // and ignore rotation because we want the shadow map projections to align with the axes
            let view_translation = GlobalTransform::from_translation(light.transform.translation());

            // for each face of a cube and each view we spawn a light entity
            let light_view_entities = light_view_entities
                .entry(entity)
                .or_insert_with(|| (0..6).map(|_| commands.spawn_empty().id()).collect());

            let cube_face_projection = Mat4::perspective_infinite_reverse_rh(
                core::f32::consts::FRAC_PI_2,
                1.0,
                light.shadow_map_near_z,
            );

            for (face_index, ((view_rotation, frustum), view_light_entity)) in cube_face_rotations
                .iter()
                .zip(&point_light_frusta.unwrap().frusta)
                .zip(light_view_entities.iter().copied())
                .enumerate()
            {
                let mut first = false;
                let base_array_layer = (light_index * 6 + face_index) as u32;

                let depth_attachment = point_light_depth_attachments
                    .entry(base_array_layer)
                    .or_insert_with(|| {
                        first = true;

                        let depth_texture_view =
                            point_light_depth_texture
                                .texture
                                .create_view(&TextureViewDescriptor {
                                    label: Some("point_light_shadow_map_texture_view"),
                                    format: None,
                                    dimension: Some(TextureViewDimension::D2),
                                    usage: None,
                                    aspect: TextureAspect::All,
                                    base_mip_level: 0,
                                    mip_level_count: None,
                                    base_array_layer,
                                    array_layer_count: Some(1u32),
                                });

                        DepthAttachment::new(depth_texture_view, Some(0.0))
                    })
                    .clone();

                let retained_view_entity = RetainedViewEntity::new(
                    *light_main_entity,
                    Some(camera_main_entity.into()),
                    face_index as u32,
                );

                commands.entity(view_light_entity).insert((
                    ShadowView {
                        depth_attachment,
                        pass_name: format!(
                            "shadow pass point light {} {}",
                            light_index,
                            face_index_to_name(face_index)
                        ),
                    },
                    ExtractedView {
                        retained_view_entity,
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
                ));

                if !matches!(gpu_preprocessing_mode, GpuPreprocessingMode::Culling) {
                    commands.entity(view_light_entity).insert(NoIndirectDrawing);
                }

                view_lights.push(view_light_entity);

                if first {
                    // Subsequent views with the same light entity will reuse the same shadow map
                    shadow_render_phases
                        .prepare_for_new_frame(retained_view_entity, gpu_preprocessing_mode);
                    live_shadow_mapping_lights.insert(retained_view_entity);
                }
            }
        }

        // spot lights
        for (light_index, &(light_entity, light_main_entity, light, (_, spot_light_frustum))) in
            point_lights
                .iter()
                .skip(point_light_count)
                .take(spot_light_count)
                .enumerate()
        {
            let Ok(mut light_view_entities) = light_view_entities.get_mut(light_entity) else {
                continue;
            };

            if !light.shadows_enabled {
                if let Some(entities) = light_view_entities.remove(&entity) {
                    despawn_entities(&mut commands, entities);
                }
                continue;
            }

            let spot_world_from_view = spot_light_world_from_view(&light.transform);
            let spot_world_from_view = spot_world_from_view.into();

            let angle = light.spot_light_angles.expect("lights should be sorted so that \
                [point_light_count..point_light_count + spot_light_shadow_maps_count] are spot lights").1;
            let spot_projection = spot_light_clip_from_view(angle, light.shadow_map_near_z);

            let mut first = false;
            let base_array_layer = (num_directional_cascades_enabled + light_index) as u32;

            let depth_attachment = directional_light_depth_attachments
                .entry(base_array_layer)
                .or_insert_with(|| {
                    first = true;

                    let depth_texture_view = directional_light_depth_texture.texture.create_view(
                        &TextureViewDescriptor {
                            label: Some("spot_light_shadow_map_texture_view"),
                            format: None,
                            dimension: Some(TextureViewDimension::D2),
                            usage: None,
                            aspect: TextureAspect::All,
                            base_mip_level: 0,
                            mip_level_count: None,
                            base_array_layer,
                            array_layer_count: Some(1u32),
                        },
                    );

                    DepthAttachment::new(depth_texture_view, Some(0.0))
                })
                .clone();

            let light_view_entities = light_view_entities
                .entry(entity)
                .or_insert_with(|| vec![commands.spawn_empty().id()]);

            let view_light_entity = light_view_entities[0];

            let retained_view_entity =
                RetainedViewEntity::new(*light_main_entity, Some(camera_main_entity.into()), 0);

            commands.entity(view_light_entity).insert((
                ShadowView {
                    depth_attachment,
                    pass_name: format!("shadow pass spot light {light_index}"),
                },
                ExtractedView {
                    retained_view_entity,
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
            ));

            if !matches!(gpu_preprocessing_mode, GpuPreprocessingMode::Culling) {
                commands.entity(view_light_entity).insert(NoIndirectDrawing);
            }

            view_lights.push(view_light_entity);

            if first {
                // Subsequent views with the same light entity will reuse the same shadow map
                shadow_render_phases
                    .prepare_for_new_frame(retained_view_entity, gpu_preprocessing_mode);
                live_shadow_mapping_lights.insert(retained_view_entity);
            }
        }

        // directional lights
        let mut directional_depth_texture_array_index = 0u32;
        let view_layers = maybe_layers.unwrap_or_default();
        for (light_index, &(light_entity, light_main_entity, light)) in directional_lights
            .iter()
            .enumerate()
            .take(MAX_DIRECTIONAL_LIGHTS)
        {
            let gpu_light = &mut gpu_lights.directional_lights[light_index];

            let Ok(mut light_view_entities) = light_view_entities.get_mut(light_entity) else {
                continue;
            };

            // Check if the light intersects with the view.
            if !view_layers.intersects(&light.render_layers) {
                gpu_light.skip = 1u32;
                if let Some(entities) = light_view_entities.remove(&entity) {
                    despawn_entities(&mut commands, entities);
                }
                continue;
            }

            // Only deal with cascades when shadows are enabled.
            if (gpu_light.flags & DirectionalLightFlags::SHADOWS_ENABLED.bits()) == 0u32 {
                if let Some(entities) = light_view_entities.remove(&entity) {
                    despawn_entities(&mut commands, entities);
                }
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

            let iter = cascades
                .zip(frusta)
                .zip(&light.cascade_shadow_config.bounds);

            let light_view_entities = light_view_entities.entry(entity).or_insert_with(|| {
                (0..iter.len())
                    .map(|_| commands.spawn_empty().id())
                    .collect()
            });
            if light_view_entities.len() != iter.len() {
                let entities = core::mem::take(light_view_entities);
                despawn_entities(&mut commands, entities);
                light_view_entities.extend((0..iter.len()).map(|_| commands.spawn_empty().id()));
            }

            for (cascade_index, (((cascade, frustum), bound), view_light_entity)) in
                iter.zip(light_view_entities.iter().copied()).enumerate()
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
                            usage: None,
                            aspect: TextureAspect::All,
                            base_mip_level: 0,
                            mip_level_count: None,
                            base_array_layer: directional_depth_texture_array_index,
                            array_layer_count: Some(1u32),
                        });

                // NOTE: For point and spotlights, we reuse the same depth attachment for all views.
                // However, for directional lights, we want a new depth attachment for each view,
                // so that the view is cleared for each view.
                let depth_attachment = DepthAttachment::new(depth_texture_view.clone(), Some(0.0));

                directional_depth_texture_array_index += 1;

                let mut frustum = *frustum;
                // Push the near clip plane out to infinity for directional lights
                frustum.half_spaces[4] =
                    HalfSpace::new(frustum.half_spaces[4].normal().extend(f32::INFINITY));

                let retained_view_entity = RetainedViewEntity::new(
                    *light_main_entity,
                    Some(camera_main_entity.into()),
                    cascade_index as u32,
                );

                commands.entity(view_light_entity).insert((
                    ShadowView {
                        depth_attachment,
                        pass_name: format!(
                            "shadow pass directional light {light_index} cascade {cascade_index}"
                        ),
                    },
                    ExtractedView {
                        retained_view_entity,
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
                ));

                if !matches!(gpu_preprocessing_mode, GpuPreprocessingMode::Culling) {
                    commands.entity(view_light_entity).insert(NoIndirectDrawing);
                }

                view_lights.push(view_light_entity);

                // If this light is using occlusion culling, add the appropriate components.
                if light.occlusion_culling {
                    commands.entity(view_light_entity).insert((
                        OcclusionCulling,
                        OcclusionCullingSubview {
                            depth_texture_view,
                            depth_texture_size: directional_light_shadow_map.size as u32,
                        },
                    ));
                    view_occlusion_culling_lights.push(view_light_entity);
                }

                // Subsequent views with the same light entity will **NOT** reuse the same shadow map
                // (Because the cascades are unique to each view)
                // TODO: Implement GPU culling for shadow passes.
                shadow_render_phases
                    .prepare_for_new_frame(retained_view_entity, gpu_preprocessing_mode);
                live_shadow_mapping_lights.insert(retained_view_entity);
            }
        }

        commands.entity(entity).insert((
            ViewShadowBindings {
                point_light_depth_texture: point_light_depth_texture.texture.clone(),
                point_light_depth_texture_view: point_light_depth_texture_view.clone(),
                directional_light_depth_texture: directional_light_depth_texture.texture.clone(),
                directional_light_depth_texture_view: directional_light_depth_texture_view.clone(),
            },
            ViewLightEntities {
                lights: view_lights,
            },
            ViewLightsUniformOffset {
                offset: view_gpu_lights_writer.write(&gpu_lights),
            },
        ));

        // Make a link from the camera to all shadow cascades with occlusion
        // culling enabled.
        if !view_occlusion_culling_lights.is_empty() {
            commands
                .entity(entity)
                .insert(OcclusionCullingSubviewEntities(
                    view_occlusion_culling_lights,
                ));
        }
    }

    // Despawn light-view entities for views that no longer exist
    for mut entities in &mut light_view_entities {
        for (_, light_view_entities) in
            entities.extract_if(|entity, _| !live_views.contains(entity))
        {
            despawn_entities(&mut commands, light_view_entities);
        }
    }

    shadow_render_phases.retain(|entity, _| live_shadow_mapping_lights.contains(entity));
}

fn despawn_entities(commands: &mut Commands, entities: Vec<Entity>) {
    if entities.is_empty() {
        return;
    }
    commands.queue(move |world: &mut World| {
        for entity in entities {
            world.despawn(entity);
        }
    });
}

// These will be extracted in the material extraction, which will also clear the needs_specialization
// collection.
pub fn check_light_entities_needing_specialization<M: Material>(
    needs_specialization: Query<Entity, (With<MeshMaterial3d<M>>, Changed<NotShadowCaster>)>,
    mut entities_needing_specialization: ResMut<EntitiesNeedingSpecialization<M>>,
    mut removed_components: RemovedComponents<NotShadowCaster>,
) {
    for entity in &needs_specialization {
        entities_needing_specialization.push(entity);
    }

    for removed in removed_components.read() {
        entities_needing_specialization.entities.push(removed);
    }
}

#[derive(Resource, Deref, DerefMut, Default, Debug, Clone)]
pub struct LightKeyCache(HashMap<RetainedViewEntity, MeshPipelineKey>);

#[derive(Resource, Deref, DerefMut, Default, Debug, Clone)]
pub struct LightSpecializationTicks(HashMap<RetainedViewEntity, Tick>);

#[derive(Resource, Deref, DerefMut)]
pub struct SpecializedShadowMaterialPipelineCache<M> {
    // view light entity -> view pipeline cache
    #[deref]
    map: HashMap<RetainedViewEntity, SpecializedShadowMaterialViewPipelineCache<M>>,
    marker: PhantomData<M>,
}

#[derive(Deref, DerefMut)]
pub struct SpecializedShadowMaterialViewPipelineCache<M> {
    #[deref]
    map: MainEntityHashMap<(Tick, CachedRenderPipelineId)>,
    marker: PhantomData<M>,
}

impl<M> Default for SpecializedShadowMaterialPipelineCache<M> {
    fn default() -> Self {
        Self {
            map: HashMap::default(),
            marker: PhantomData,
        }
    }
}

impl<M> Default for SpecializedShadowMaterialViewPipelineCache<M> {
    fn default() -> Self {
        Self {
            map: MainEntityHashMap::default(),
            marker: PhantomData,
        }
    }
}

pub fn check_views_lights_need_specialization(
    view_lights: Query<&ViewLightEntities, With<ExtractedView>>,
    view_light_entities: Query<(&LightEntity, &ExtractedView)>,
    shadow_render_phases: Res<ViewBinnedRenderPhases<Shadow>>,
    mut light_key_cache: ResMut<LightKeyCache>,
    mut light_specialization_ticks: ResMut<LightSpecializationTicks>,
    ticks: SystemChangeTick,
) {
    for view_lights in &view_lights {
        for view_light_entity in view_lights.lights.iter().copied() {
            let Ok((light_entity, extracted_view_light)) =
                view_light_entities.get(view_light_entity)
            else {
                continue;
            };
            if !shadow_render_phases.contains_key(&extracted_view_light.retained_view_entity) {
                continue;
            }

            let is_directional_light = matches!(light_entity, LightEntity::Directional { .. });
            let mut light_key = MeshPipelineKey::DEPTH_PREPASS;
            light_key.set(MeshPipelineKey::UNCLIPPED_DEPTH_ORTHO, is_directional_light);
            if let Some(current_key) =
                light_key_cache.get_mut(&extracted_view_light.retained_view_entity)
            {
                if *current_key != light_key {
                    light_key_cache.insert(extracted_view_light.retained_view_entity, light_key);
                    light_specialization_ticks
                        .insert(extracted_view_light.retained_view_entity, ticks.this_run());
                }
            } else {
                light_key_cache.insert(extracted_view_light.retained_view_entity, light_key);
                light_specialization_ticks
                    .insert(extracted_view_light.retained_view_entity, ticks.this_run());
            }
        }
    }
}

pub fn specialize_shadows<M: Material>(
    prepass_pipeline: Res<PrepassPipeline<M>>,
    (
        render_meshes,
        render_mesh_instances,
        render_materials,
        render_material_instances,
        material_bind_group_allocator,
    ): (
        Res<RenderAssets<RenderMesh>>,
        Res<RenderMeshInstances>,
        Res<RenderAssets<PreparedMaterial<M>>>,
        Res<RenderMaterialInstances>,
        Res<MaterialBindGroupAllocator<M>>,
    ),
    shadow_render_phases: Res<ViewBinnedRenderPhases<Shadow>>,
    mut pipelines: ResMut<SpecializedMeshPipelines<PrepassPipeline<M>>>,
    pipeline_cache: Res<PipelineCache>,
    render_lightmaps: Res<RenderLightmaps>,
    view_lights: Query<(Entity, &ViewLightEntities), With<ExtractedView>>,
    view_light_entities: Query<(&LightEntity, &ExtractedView)>,
    point_light_entities: Query<&RenderCubemapVisibleEntities, With<ExtractedPointLight>>,
    directional_light_entities: Query<
        &RenderCascadesVisibleEntities,
        With<ExtractedDirectionalLight>,
    >,
    spot_light_entities: Query<&RenderVisibleMeshEntities, With<ExtractedPointLight>>,
    light_key_cache: Res<LightKeyCache>,
    mut specialized_material_pipeline_cache: ResMut<SpecializedShadowMaterialPipelineCache<M>>,
    light_specialization_ticks: Res<LightSpecializationTicks>,
    entity_specialization_ticks: Res<EntitySpecializationTicks<M>>,
    ticks: SystemChangeTick,
) where
    M::Data: PartialEq + Eq + Hash + Clone,
{
    // Record the retained IDs of all shadow views so that we can expire old
    // pipeline IDs.
    let mut all_shadow_views: HashSet<RetainedViewEntity, FixedHasher> = HashSet::default();

    for (entity, view_lights) in &view_lights {
        for view_light_entity in view_lights.lights.iter().copied() {
            let Ok((light_entity, extracted_view_light)) =
                view_light_entities.get(view_light_entity)
            else {
                continue;
            };

            all_shadow_views.insert(extracted_view_light.retained_view_entity);

            if !shadow_render_phases.contains_key(&extracted_view_light.retained_view_entity) {
                continue;
            }
            let Some(light_key) = light_key_cache.get(&extracted_view_light.retained_view_entity)
            else {
                continue;
            };

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

            let view_tick = light_specialization_ticks
                .get(&extracted_view_light.retained_view_entity)
                .unwrap();
            let view_specialized_material_pipeline_cache = specialized_material_pipeline_cache
                .entry(extracted_view_light.retained_view_entity)
                .or_default();

            for (_, visible_entity) in visible_entities.iter().copied() {
                let Some(material_instances) =
                    render_material_instances.instances.get(&visible_entity)
                else {
                    continue;
                };
                let Ok(material_asset_id) = material_instances.asset_id.try_typed::<M>() else {
                    continue;
                };
                let Some(mesh_instance) =
                    render_mesh_instances.render_mesh_queue_data(visible_entity)
                else {
                    continue;
                };
                let entity_tick = entity_specialization_ticks.get(&visible_entity).unwrap();
                let last_specialized_tick = view_specialized_material_pipeline_cache
                    .get(&visible_entity)
                    .map(|(tick, _)| *tick);
                let needs_specialization = last_specialized_tick.is_none_or(|tick| {
                    view_tick.is_newer_than(tick, ticks.this_run())
                        || entity_tick.is_newer_than(tick, ticks.this_run())
                });
                if !needs_specialization {
                    continue;
                }
                let Some(material) = render_materials.get(material_asset_id) else {
                    continue;
                };
                if !mesh_instance
                    .flags
                    .contains(RenderMeshInstanceFlags::SHADOW_CASTER)
                {
                    continue;
                }
                let Some(material_bind_group) =
                    material_bind_group_allocator.get(material.binding.group)
                else {
                    continue;
                };
                let Some(mesh) = render_meshes.get(mesh_instance.mesh_asset_id) else {
                    continue;
                };

                let mut mesh_key =
                    *light_key | MeshPipelineKey::from_bits_retain(mesh.key_bits.bits());

                // Even though we don't use the lightmap in the shadow map, the
                // `SetMeshBindGroup` render command will bind the data for it. So
                // we need to include the appropriate flag in the mesh pipeline key
                // to ensure that the necessary bind group layout entries are
                // present.
                if render_lightmaps
                    .render_lightmaps
                    .contains_key(&visible_entity)
                {
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
                        bind_group_data: material_bind_group
                            .get_extra_data(material.binding.slot)
                            .clone(),
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

                view_specialized_material_pipeline_cache
                    .insert(visible_entity, (ticks.this_run(), pipeline_id));
            }
        }
    }

    // Delete specialized pipelines belonging to views that have expired.
    specialized_material_pipeline_cache.retain(|view, _| all_shadow_views.contains(view));
}

/// For each shadow cascade, iterates over all the meshes "visible" from it and
/// adds them to [`BinnedRenderPhase`]s or [`SortedRenderPhase`]s as
/// appropriate.
pub fn queue_shadows<M: Material>(
    shadow_draw_functions: Res<DrawFunctions<Shadow>>,
    render_mesh_instances: Res<RenderMeshInstances>,
    render_materials: Res<RenderAssets<PreparedMaterial<M>>>,
    render_material_instances: Res<RenderMaterialInstances>,
    mut shadow_render_phases: ResMut<ViewBinnedRenderPhases<Shadow>>,
    gpu_preprocessing_support: Res<GpuPreprocessingSupport>,
    mesh_allocator: Res<MeshAllocator>,
    view_lights: Query<(Entity, &ViewLightEntities), With<ExtractedView>>,
    view_light_entities: Query<(&LightEntity, &ExtractedView)>,
    point_light_entities: Query<&RenderCubemapVisibleEntities, With<ExtractedPointLight>>,
    directional_light_entities: Query<
        &RenderCascadesVisibleEntities,
        With<ExtractedDirectionalLight>,
    >,
    spot_light_entities: Query<&RenderVisibleMeshEntities, With<ExtractedPointLight>>,
    specialized_material_pipeline_cache: Res<SpecializedShadowMaterialPipelineCache<M>>,
) where
    M::Data: PartialEq + Eq + Hash + Clone,
{
    let draw_shadow_mesh = shadow_draw_functions.read().id::<DrawPrepass<M>>();
    for (entity, view_lights) in &view_lights {
        for view_light_entity in view_lights.lights.iter().copied() {
            let Ok((light_entity, extracted_view_light)) =
                view_light_entities.get(view_light_entity)
            else {
                continue;
            };
            let Some(shadow_phase) =
                shadow_render_phases.get_mut(&extracted_view_light.retained_view_entity)
            else {
                continue;
            };

            let Some(view_specialized_material_pipeline_cache) =
                specialized_material_pipeline_cache.get(&extracted_view_light.retained_view_entity)
            else {
                continue;
            };

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

            for (entity, main_entity) in visible_entities.iter().copied() {
                let Some((current_change_tick, pipeline_id)) =
                    view_specialized_material_pipeline_cache.get(&main_entity)
                else {
                    continue;
                };

                // Skip the entity if it's cached in a bin and up to date.
                if shadow_phase.validate_cached_entity(main_entity, *current_change_tick) {
                    continue;
                }

                let Some(mesh_instance) = render_mesh_instances.render_mesh_queue_data(main_entity)
                else {
                    continue;
                };
                if !mesh_instance
                    .flags
                    .contains(RenderMeshInstanceFlags::SHADOW_CASTER)
                {
                    continue;
                }

                let Some(material_instance) = render_material_instances.instances.get(&main_entity)
                else {
                    continue;
                };
                let Ok(material_asset_id) = material_instance.asset_id.try_typed::<M>() else {
                    continue;
                };
                let Some(material) = render_materials.get(material_asset_id) else {
                    continue;
                };

                let (vertex_slab, index_slab) =
                    mesh_allocator.mesh_slabs(&mesh_instance.mesh_asset_id);

                let batch_set_key = ShadowBatchSetKey {
                    pipeline: *pipeline_id,
                    draw_function: draw_shadow_mesh,
                    material_bind_group_index: Some(material.binding.group.0),
                    vertex_slab: vertex_slab.unwrap_or_default(),
                    index_slab,
                };

                shadow_phase.add(
                    batch_set_key,
                    ShadowBinKey {
                        asset_id: mesh_instance.mesh_asset_id.into(),
                    },
                    (entity, main_entity),
                    mesh_instance.current_uniform_index,
                    BinnedRenderPhaseType::mesh(
                        mesh_instance.should_batch(),
                        &gpu_preprocessing_support,
                    ),
                    *current_change_tick,
                );
            }
        }
    }
}

pub struct Shadow {
    /// Determines which objects can be placed into a *batch set*.
    ///
    /// Objects in a single batch set can potentially be multi-drawn together,
    /// if it's enabled and the current platform supports it.
    pub batch_set_key: ShadowBatchSetKey,
    /// Information that separates items into bins.
    pub bin_key: ShadowBinKey,
    pub representative_entity: (Entity, MainEntity),
    pub batch_range: Range<u32>,
    pub extra_index: PhaseItemExtraIndex,
}

/// Information that must be identical in order to place opaque meshes in the
/// same *batch set*.
///
/// A batch set is a set of batches that can be multi-drawn together, if
/// multi-draw is in use.
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ShadowBatchSetKey {
    /// The identifier of the render pipeline.
    pub pipeline: CachedRenderPipelineId,

    /// The function used to draw.
    pub draw_function: DrawFunctionId,

    /// The ID of a bind group specific to the material.
    ///
    /// In the case of PBR, this is the `MaterialBindGroupIndex`.
    pub material_bind_group_index: Option<u32>,

    /// The ID of the slab of GPU memory that contains vertex data.
    ///
    /// For non-mesh items, you can fill this with 0 if your items can be
    /// multi-drawn, or with a unique value if they can't.
    pub vertex_slab: SlabId,

    /// The ID of the slab of GPU memory that contains index data, if present.
    ///
    /// For non-mesh items, you can safely fill this with `None`.
    pub index_slab: Option<SlabId>,
}

impl PhaseItemBatchSetKey for ShadowBatchSetKey {
    fn indexed(&self) -> bool {
        self.index_slab.is_some()
    }
}

/// Data used to bin each object in the shadow map phase.
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ShadowBinKey {
    /// The object.
    pub asset_id: UntypedAssetId,
}

impl PhaseItem for Shadow {
    #[inline]
    fn entity(&self) -> Entity {
        self.representative_entity.0
    }

    fn main_entity(&self) -> MainEntity {
        self.representative_entity.1
    }

    #[inline]
    fn draw_function(&self) -> DrawFunctionId {
        self.batch_set_key.draw_function
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
        self.extra_index.clone()
    }

    #[inline]
    fn batch_range_and_extra_index_mut(&mut self) -> (&mut Range<u32>, &mut PhaseItemExtraIndex) {
        (&mut self.batch_range, &mut self.extra_index)
    }
}

impl BinnedPhaseItem for Shadow {
    type BatchSetKey = ShadowBatchSetKey;
    type BinKey = ShadowBinKey;

    #[inline]
    fn new(
        batch_set_key: Self::BatchSetKey,
        bin_key: Self::BinKey,
        representative_entity: (Entity, MainEntity),
        batch_range: Range<u32>,
        extra_index: PhaseItemExtraIndex,
    ) -> Self {
        Shadow {
            batch_set_key,
            bin_key,
            representative_entity,
            batch_range,
            extra_index,
        }
    }
}

impl CachedRenderPipelinePhaseItem for Shadow {
    #[inline]
    fn cached_pipeline(&self) -> CachedRenderPipelineId {
        self.batch_set_key.pipeline
    }
}

/// The rendering node that renders meshes that were "visible" (so to speak)
/// from a light last frame.
///
/// If occlusion culling for a light is disabled, then this node simply renders
/// all meshes in range of the light.
#[derive(Deref, DerefMut)]
pub struct EarlyShadowPassNode(ShadowPassNode);

/// The rendering node that renders meshes that became newly "visible" (so to
/// speak) from a light this frame.
///
/// If occlusion culling for a light is disabled, then this node does nothing.
#[derive(Deref, DerefMut)]
pub struct LateShadowPassNode(ShadowPassNode);

/// Encapsulates rendering logic shared between the early and late shadow pass
/// nodes.
pub struct ShadowPassNode {
    /// The query that finds cameras in which shadows are visible.
    main_view_query: QueryState<Read<ViewLightEntities>>,
    /// The query that finds shadow cascades.
    view_light_query: QueryState<(Read<ShadowView>, Read<ExtractedView>, Has<OcclusionCulling>)>,
}

impl FromWorld for EarlyShadowPassNode {
    fn from_world(world: &mut World) -> Self {
        Self(ShadowPassNode::from_world(world))
    }
}

impl FromWorld for LateShadowPassNode {
    fn from_world(world: &mut World) -> Self {
        Self(ShadowPassNode::from_world(world))
    }
}

impl FromWorld for ShadowPassNode {
    fn from_world(world: &mut World) -> Self {
        Self {
            main_view_query: QueryState::new(world),
            view_light_query: QueryState::new(world),
        }
    }
}

impl Node for EarlyShadowPassNode {
    fn update(&mut self, world: &mut World) {
        self.0.update(world);
    }

    fn run<'w>(
        &self,
        graph: &mut RenderGraphContext,
        render_context: &mut RenderContext<'w>,
        world: &'w World,
    ) -> Result<(), NodeRunError> {
        self.0.run(graph, render_context, world, false)
    }
}

impl Node for LateShadowPassNode {
    fn update(&mut self, world: &mut World) {
        self.0.update(world);
    }

    fn run<'w>(
        &self,
        graph: &mut RenderGraphContext,
        render_context: &mut RenderContext<'w>,
        world: &'w World,
    ) -> Result<(), NodeRunError> {
        self.0.run(graph, render_context, world, true)
    }
}

impl ShadowPassNode {
    fn update(&mut self, world: &mut World) {
        self.main_view_query.update_archetypes(world);
        self.view_light_query.update_archetypes(world);
    }

    /// Runs the node logic.
    ///
    /// `is_late` is true if this is the late shadow pass or false if this is
    /// the early shadow pass.
    fn run<'w>(
        &self,
        graph: &mut RenderGraphContext,
        render_context: &mut RenderContext<'w>,
        world: &'w World,
        is_late: bool,
    ) -> Result<(), NodeRunError> {
        let Some(shadow_render_phases) = world.get_resource::<ViewBinnedRenderPhases<Shadow>>()
        else {
            return Ok(());
        };

        if let Ok(view_lights) = self.main_view_query.get_manual(world, graph.view_entity()) {
            for view_light_entity in view_lights.lights.iter().copied() {
                let Ok((view_light, extracted_light_view, occlusion_culling)) =
                    self.view_light_query.get_manual(world, view_light_entity)
                else {
                    continue;
                };

                // There's no need for a late shadow pass if the light isn't
                // using occlusion culling.
                if is_late && !occlusion_culling {
                    continue;
                }

                let Some(shadow_phase) =
                    shadow_render_phases.get(&extracted_light_view.retained_view_entity)
                else {
                    continue;
                };

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

                    if let Err(err) =
                        shadow_phase.render(&mut render_pass, world, view_light_entity)
                    {
                        error!("Error encountered while rendering the shadow phase {err:?}");
                    }

                    pass_span.end(&mut render_pass);
                    drop(render_pass);
                    command_encoder.finish()
                });
            }
        }

        Ok(())
    }
}
