use std::collections::HashSet;

use bevy_ecs::prelude::*;
use bevy_math::{Mat4, UVec2, UVec3, Vec2, Vec3, Vec3Swizzles, Vec4, Vec4Swizzles};
use bevy_render::{
    camera::{Camera, CameraProjection, OrthographicProjection},
    color::Color,
    primitives::{Aabb, CubemapFrusta, Frustum, Sphere},
    view::{ComputedVisibility, RenderLayers, Visibility, VisibleEntities},
};
use bevy_transform::components::GlobalTransform;
use bevy_window::Windows;

use crate::{
    calculate_cluster_factors, CubeMapFace, CubemapVisibleEntities, ViewClusterBindings,
    CUBE_MAP_FACES, POINT_LIGHT_NEAR_Z,
};

/// A light that emits light in all directions from a central point.
///
/// Real-world values for `intensity` (luminous power in lumens) based on the electrical power
/// consumption of the type of real-world light are:
///
/// | Luminous Power (lumen) (i.e. the intensity member) | Incandescent non-halogen (Watts) | Incandescent halogen (Watts) | Compact fluorescent (Watts) | LED (Watts |
/// |------|-----|----|--------|-------|
/// | 200  | 25  |    | 3-5    | 3     |
/// | 450  | 40  | 29 | 9-11   | 5-8   |
/// | 800  | 60  |    | 13-15  | 8-12  |
/// | 1100 | 75  | 53 | 18-20  | 10-16 |
/// | 1600 | 100 | 72 | 24-28  | 14-17 |
/// | 2400 | 150 |    | 30-52  | 24-30 |
/// | 3100 | 200 |    | 49-75  | 32    |
/// | 4000 | 300 |    | 75-100 | 40.5  |
///
/// Source: [Wikipedia](https://en.wikipedia.org/wiki/Lumen_(unit)#Lighting)
#[derive(Component, Debug, Clone, Copy)]
pub struct PointLight {
    pub color: Color,
    pub intensity: f32,
    pub range: f32,
    pub radius: f32,
    pub shadows_enabled: bool,
    pub shadow_depth_bias: f32,
    /// A bias applied along the direction of the fragment's surface normal. It is scaled to the
    /// shadow map's texel size so that it can be small close to the camera and gets larger further
    /// away.
    pub shadow_normal_bias: f32,
}

impl Default for PointLight {
    fn default() -> Self {
        PointLight {
            color: Color::rgb(1.0, 1.0, 1.0),
            /// Luminous power in lumens
            intensity: 800.0, // Roughly a 60W non-halogen incandescent bulb
            range: 20.0,
            radius: 0.0,
            shadows_enabled: false,
            shadow_depth_bias: Self::DEFAULT_SHADOW_DEPTH_BIAS,
            shadow_normal_bias: Self::DEFAULT_SHADOW_NORMAL_BIAS,
        }
    }
}

impl PointLight {
    pub const DEFAULT_SHADOW_DEPTH_BIAS: f32 = 0.02;
    pub const DEFAULT_SHADOW_NORMAL_BIAS: f32 = 0.6;
}

#[derive(Clone, Debug)]
pub struct PointLightShadowMap {
    pub size: usize,
}

impl Default for PointLightShadowMap {
    fn default() -> Self {
        Self { size: 1024 }
    }
}

/// A Directional light.
///
/// Directional lights don't exist in reality but they are a good
/// approximation for light sources VERY far away, like the sun or
/// the moon.
///
/// Valid values for `illuminance` are:
///
/// | Illuminance (lux) | Surfaces illuminated by                        |
/// |-------------------|------------------------------------------------|
/// | 0.0001            | Moonless, overcast night sky (starlight)       |
/// | 0.002             | Moonless clear night sky with airglow          |
/// | 0.05–0.3          | Full moon on a clear night                     |
/// | 3.4               | Dark limit of civil twilight under a clear sky |
/// | 20–50             | Public areas with dark surroundings            |
/// | 50                | Family living room lights                      |
/// | 80                | Office building hallway/toilet lighting        |
/// | 100               | Very dark overcast day                         |
/// | 150               | Train station platforms                        |
/// | 320–500           | Office lighting                                |
/// | 400               | Sunrise or sunset on a clear day.              |
/// | 1000              | Overcast day; typical TV studio lighting       |
/// | 10,000–25,000     | Full daylight (not direct sun)                 |
/// | 32,000–100,000    | Direct sunlight                                |
///
/// Source: [Wikipedia](https://en.wikipedia.org/wiki/Lux)
#[derive(Component, Debug, Clone)]
pub struct DirectionalLight {
    pub color: Color,
    /// Illuminance in lux
    pub illuminance: f32,
    pub shadows_enabled: bool,
    pub shadow_projection: OrthographicProjection,
    pub shadow_depth_bias: f32,
    /// A bias applied along the direction of the fragment's surface normal. It is scaled to the
    /// shadow map's texel size so that it is automatically adjusted to the orthographic projection.
    pub shadow_normal_bias: f32,
}

impl Default for DirectionalLight {
    fn default() -> Self {
        let size = 100.0;
        DirectionalLight {
            color: Color::rgb(1.0, 1.0, 1.0),
            illuminance: 100000.0,
            shadows_enabled: false,
            shadow_projection: OrthographicProjection {
                left: -size,
                right: size,
                bottom: -size,
                top: size,
                near: -size,
                far: size,
                ..Default::default()
            },
            shadow_depth_bias: Self::DEFAULT_SHADOW_DEPTH_BIAS,
            shadow_normal_bias: Self::DEFAULT_SHADOW_NORMAL_BIAS,
        }
    }
}

impl DirectionalLight {
    pub const DEFAULT_SHADOW_DEPTH_BIAS: f32 = 0.02;
    pub const DEFAULT_SHADOW_NORMAL_BIAS: f32 = 0.6;
}

#[derive(Clone, Debug)]
pub struct DirectionalLightShadowMap {
    pub size: usize,
}

impl Default for DirectionalLightShadowMap {
    fn default() -> Self {
        #[cfg(feature = "webgl2")]
        return Self { size: 4096 };
        #[cfg(not(feature = "webgl2"))]
        return Self { size: 2048 };
    }
}

/// An ambient light, which lights the entire scene equally.
#[derive(Debug)]
pub struct AmbientLight {
    pub color: Color,
    /// A direct scale factor multiplied with `color` before being passed to the shader.
    pub brightness: f32,
}

impl Default for AmbientLight {
    fn default() -> Self {
        Self {
            color: Color::rgb(1.0, 1.0, 1.0),
            brightness: 0.05,
        }
    }
}

/// Add this component to make a [`Mesh`](bevy_render::mesh::Mesh) not cast shadows.
#[derive(Component)]
pub struct NotShadowCaster;
/// Add this component to make a [`Mesh`](bevy_render::mesh::Mesh) not receive shadows.
#[derive(Component)]
pub struct NotShadowReceiver;

#[derive(Debug, Hash, PartialEq, Eq, Clone, SystemLabel)]
pub enum SimulationLightSystems {
    AddClusters,
    UpdateClusters,
    AssignLightsToClusters,
    UpdateDirectionalLightFrusta,
    UpdatePointLightFrusta,
    CheckLightVisibility,
}

// Clustered-forward rendering notes
// The main initial reference material used was this rather accessible article:
// http://www.aortiz.me/2018/12/21/CG.html
// Some inspiration was taken from “Practical Clustered Shading” which is part 2 of:
// https://efficientshading.com/2015/01/01/real-time-many-light-management-and-shadows-with-clustered-shading/
// (Also note that Part 3 of the above shows how we could support the shadow mapping for many lights.)
// The z-slicing method mentioned in the aortiz article is originally from Tiago Sousa’s Siggraph 2016 talk about Doom 2016:
// http://advances.realtimerendering.com/s2016/Siggraph2016_idTech6.pdf

#[derive(Component, Debug)]
pub struct Clusters {
    /// Tile size
    pub(crate) tile_size: UVec2,
    /// Number of clusters in x / y / z in the view frustum
    pub(crate) axis_slices: UVec3,
    aabbs: Vec<Aabb>,
    pub(crate) lights: Vec<VisiblePointLights>,
}

impl Clusters {
    fn new(tile_size: UVec2, screen_size: UVec2, z_slices: u32) -> Self {
        let mut clusters = Self {
            tile_size,
            axis_slices: Default::default(),
            aabbs: Default::default(),
            lights: Default::default(),
        };
        clusters.update(tile_size, screen_size, z_slices);
        clusters
    }

    fn from_screen_size_and_z_slices(screen_size: UVec2, z_slices: u32) -> Self {
        let aspect_ratio = screen_size.x as f32 / screen_size.y as f32;
        let n_tiles_y =
            ((ViewClusterBindings::MAX_OFFSETS as u32 / z_slices) as f32 / aspect_ratio).sqrt();
        // NOTE: Round down the number of tiles in order to avoid overflowing the maximum number of
        // clusters.
        let n_tiles = UVec2::new(
            (aspect_ratio * n_tiles_y).floor() as u32,
            n_tiles_y.floor() as u32,
        );
        Clusters::new((screen_size + UVec2::ONE) / n_tiles, screen_size, Z_SLICES)
    }

    fn update(&mut self, tile_size: UVec2, screen_size: UVec2, z_slices: u32) {
        self.tile_size = tile_size;
        self.axis_slices = UVec3::new(
            (screen_size.x + 1) / tile_size.x,
            (screen_size.y + 1) / tile_size.y,
            z_slices,
        );
    }
}

fn clip_to_view(inverse_projection: Mat4, clip: Vec4) -> Vec4 {
    let view = inverse_projection * clip;
    view / view.w
}

fn screen_to_view(screen_size: Vec2, inverse_projection: Mat4, screen: Vec2, ndc_z: f32) -> Vec4 {
    let tex_coord = screen / screen_size;
    let clip = Vec4::new(
        tex_coord.x * 2.0 - 1.0,
        (1.0 - tex_coord.y) * 2.0 - 1.0,
        ndc_z,
        1.0,
    );
    clip_to_view(inverse_projection, clip)
}

// Calculate the intersection of a ray from the eye through the view space position to a z plane
fn line_intersection_to_z_plane(origin: Vec3, p: Vec3, z: f32) -> Vec3 {
    let v = p - origin;
    let t = (z - Vec3::Z.dot(origin)) / Vec3::Z.dot(v);
    origin + t * v
}

#[allow(clippy::too_many_arguments)]
fn compute_aabb_for_cluster(
    z_near: f32,
    z_far: f32,
    tile_size: Vec2,
    screen_size: Vec2,
    inverse_projection: Mat4,
    is_orthographic: bool,
    cluster_dimensions: UVec3,
    ijk: UVec3,
) -> Aabb {
    let ijk = ijk.as_vec3();

    // Calculate the minimum and maximum points in screen space
    let p_min = ijk.xy() * tile_size;
    let p_max = p_min + tile_size;

    let cluster_min;
    let cluster_max;
    if is_orthographic {
        // Use linear depth slicing for orthographic

        // Convert to view space at the cluster near and far planes
        // NOTE: 1.0 is the near plane due to using reverse z projections
        let p_min = screen_to_view(
            screen_size,
            inverse_projection,
            p_min,
            1.0 - (ijk.z / cluster_dimensions.z as f32),
        )
        .xyz();
        let p_max = screen_to_view(
            screen_size,
            inverse_projection,
            p_max,
            1.0 - ((ijk.z + 1.0) / cluster_dimensions.z as f32),
        )
        .xyz();

        cluster_min = p_min.min(p_max);
        cluster_max = p_min.max(p_max);
    } else {
        // Convert to view space at the near plane
        // NOTE: 1.0 is the near plane due to using reverse z projections
        let p_min = screen_to_view(screen_size, inverse_projection, p_min, 1.0);
        let p_max = screen_to_view(screen_size, inverse_projection, p_max, 1.0);

        let z_far_over_z_near = -z_far / -z_near;
        let cluster_near = -z_near * z_far_over_z_near.powf(ijk.z / cluster_dimensions.z as f32);
        // NOTE: This could be simplified to:
        // cluster_far = cluster_near * z_far_over_z_near;
        let cluster_far =
            -z_near * z_far_over_z_near.powf((ijk.z + 1.0) / cluster_dimensions.z as f32);

        // Calculate the four intersection points of the min and max points with the cluster near and far planes
        let p_min_near = line_intersection_to_z_plane(Vec3::ZERO, p_min.xyz(), cluster_near);
        let p_min_far = line_intersection_to_z_plane(Vec3::ZERO, p_min.xyz(), cluster_far);
        let p_max_near = line_intersection_to_z_plane(Vec3::ZERO, p_max.xyz(), cluster_near);
        let p_max_far = line_intersection_to_z_plane(Vec3::ZERO, p_max.xyz(), cluster_far);

        cluster_min = p_min_near.min(p_min_far).min(p_max_near.min(p_max_far));
        cluster_max = p_min_near.max(p_min_far).max(p_max_near.max(p_max_far));
    }

    Aabb::from_min_max(cluster_min, cluster_max)
}

const Z_SLICES: u32 = 24;

pub fn add_clusters(
    mut commands: Commands,
    windows: Res<Windows>,
    cameras: Query<(Entity, &Camera), Without<Clusters>>,
) {
    for (entity, camera) in cameras.iter() {
        let window = match windows.get(camera.window) {
            Some(window) => window,
            None => continue,
        };
        let clusters = Clusters::from_screen_size_and_z_slices(
            UVec2::new(window.physical_width(), window.physical_height()),
            Z_SLICES,
        );
        commands.entity(entity).insert(clusters);
    }
}

pub fn update_clusters(windows: Res<Windows>, mut views: Query<(&Camera, &mut Clusters)>) {
    for (camera, mut clusters) in views.iter_mut() {
        let is_orthographic = camera.projection_matrix.w_axis.w == 1.0;
        let inverse_projection = camera.projection_matrix.inverse();
        let window = windows.get(camera.window).unwrap();
        let screen_size_u32 = UVec2::new(window.physical_width(), window.physical_height());
        // Don't update clusters if screen size is 0.
        if screen_size_u32.x == 0 || screen_size_u32.y == 0 {
            continue;
        }
        *clusters =
            Clusters::from_screen_size_and_z_slices(screen_size_u32, clusters.axis_slices.z);
        let screen_size = screen_size_u32.as_vec2();
        let tile_size_u32 = clusters.tile_size;
        let tile_size = tile_size_u32.as_vec2();

        // Calculate view space AABBs
        // NOTE: It is important that these are iterated in a specific order
        // so that we can calculate the cluster index in the fragment shader!
        // I (Rob Swain) choose to scan along rows of tiles in x,y, and for each tile then scan
        // along z
        let mut aabbs = Vec::with_capacity(
            (clusters.axis_slices.y * clusters.axis_slices.x * clusters.axis_slices.z) as usize,
        );
        for y in 0..clusters.axis_slices.y {
            for x in 0..clusters.axis_slices.x {
                for z in 0..clusters.axis_slices.z {
                    aabbs.push(compute_aabb_for_cluster(
                        camera.near,
                        camera.far,
                        tile_size,
                        screen_size,
                        inverse_projection,
                        is_orthographic,
                        clusters.axis_slices,
                        UVec3::new(x, y, z),
                    ));
                }
            }
        }
        clusters.aabbs = aabbs;
    }
}

#[derive(Clone, Component, Debug, Default)]
pub struct VisiblePointLights {
    pub entities: Vec<Entity>,
}

impl VisiblePointLights {
    pub fn from_light_count(count: usize) -> Self {
        Self {
            entities: Vec::with_capacity(count),
        }
    }

    pub fn iter(&self) -> impl DoubleEndedIterator<Item = &Entity> {
        self.entities.iter()
    }

    pub fn len(&self) -> usize {
        self.entities.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entities.is_empty()
    }
}

fn view_z_to_z_slice(cluster_factors: Vec2, view_z: f32, is_orthographic: bool) -> u32 {
    if is_orthographic {
        // NOTE: view_z is correct in the orthographic case
        ((view_z - cluster_factors.x) * cluster_factors.y).floor() as u32
    } else {
        // NOTE: had to use -view_z to make it positive else log(negative) is nan
        ((-view_z).ln() * cluster_factors.x - cluster_factors.y).floor() as u32
    }
}

fn ndc_position_to_cluster(
    cluster_dimensions: UVec3,
    cluster_factors: Vec2,
    is_orthographic: bool,
    ndc_p: Vec3,
    view_z: f32,
) -> UVec3 {
    let cluster_dimensions_f32 = cluster_dimensions.as_vec3();
    let frag_coord =
        (ndc_p.xy() * Vec2::new(0.5, -0.5) + Vec2::splat(0.5)).clamp(Vec2::ZERO, Vec2::ONE);
    let xy = (frag_coord * cluster_dimensions_f32.xy()).floor();
    let z_slice = view_z_to_z_slice(cluster_factors, view_z, is_orthographic);
    xy.as_uvec2()
        .extend(z_slice)
        .clamp(UVec3::ZERO, cluster_dimensions - UVec3::ONE)
}

fn cluster_to_index(cluster_dimensions: UVec3, cluster: UVec3) -> usize {
    ((cluster.y * cluster_dimensions.x + cluster.x) * cluster_dimensions.z + cluster.z) as usize
}

// NOTE: Run this before update_point_light_frusta!
pub fn assign_lights_to_clusters(
    mut commands: Commands,
    mut global_lights: ResMut<VisiblePointLights>,
    mut views: Query<(Entity, &GlobalTransform, &Camera, &Frustum, &mut Clusters)>,
    lights: Query<(Entity, &GlobalTransform, &PointLight)>,
) {
    let light_count = lights.iter().count();
    let mut global_lights_set = HashSet::with_capacity(light_count);
    for (view_entity, view_transform, camera, frustum, mut clusters) in views.iter_mut() {
        let view_transform = view_transform.compute_matrix();
        let inverse_view_transform = view_transform.inverse();
        let cluster_count = clusters.aabbs.len();
        let is_orthographic = camera.projection_matrix.w_axis.w == 1.0;
        let cluster_factors = calculate_cluster_factors(
            camera.near,
            camera.far,
            clusters.axis_slices.z as f32,
            is_orthographic,
        );

        let mut clusters_lights =
            vec![VisiblePointLights::from_light_count(light_count); cluster_count];
        let mut visible_lights_set = HashSet::with_capacity(light_count);

        for (light_entity, light_transform, light) in lights.iter() {
            let light_sphere = Sphere {
                center: light_transform.translation,
                radius: light.range,
            };

            // Check if the light is within the view frustum
            if !frustum.intersects_sphere(&light_sphere) {
                continue;
            }

            // Calculate an AABB for the light in view space, find the corresponding clusters for the min and max
            // points of the AABB, then iterate over just those clusters for this light
            let light_aabb_view = Aabb {
                center: (inverse_view_transform * light_sphere.center.extend(1.0)).xyz(),
                half_extents: Vec3::splat(light_sphere.radius),
            };
            let (light_aabb_view_min, light_aabb_view_max) =
                (light_aabb_view.min(), light_aabb_view.max());
            // Is there a cheaper way to do this? The problem is that because of perspective
            // the point at max z but min xy may be less xy in screenspace, and similar. As
            // such, projecting the min and max xy at both the closer and further z and taking
            // the min and max of those projected points addresses this.
            let (
                light_aabb_view_xymin_near,
                light_aabb_view_xymin_far,
                light_aabb_view_xymax_near,
                light_aabb_view_xymax_far,
            ) = (
                light_aabb_view_min,
                light_aabb_view_min.xy().extend(light_aabb_view_max.z),
                light_aabb_view_max.xy().extend(light_aabb_view_min.z),
                light_aabb_view_max,
            );
            let (
                light_aabb_clip_xymin_near,
                light_aabb_clip_xymin_far,
                light_aabb_clip_xymax_near,
                light_aabb_clip_xymax_far,
            ) = (
                camera.projection_matrix * light_aabb_view_xymin_near.extend(1.0),
                camera.projection_matrix * light_aabb_view_xymin_far.extend(1.0),
                camera.projection_matrix * light_aabb_view_xymax_near.extend(1.0),
                camera.projection_matrix * light_aabb_view_xymax_far.extend(1.0),
            );
            let (
                light_aabb_ndc_xymin_near,
                light_aabb_ndc_xymin_far,
                light_aabb_ndc_xymax_near,
                light_aabb_ndc_xymax_far,
            ) = (
                light_aabb_clip_xymin_near.xyz() / light_aabb_clip_xymin_near.w,
                light_aabb_clip_xymin_far.xyz() / light_aabb_clip_xymin_far.w,
                light_aabb_clip_xymax_near.xyz() / light_aabb_clip_xymax_near.w,
                light_aabb_clip_xymax_far.xyz() / light_aabb_clip_xymax_far.w,
            );
            let (light_aabb_ndc_min, light_aabb_ndc_max) = (
                light_aabb_ndc_xymin_near
                    .min(light_aabb_ndc_xymin_far)
                    .min(light_aabb_ndc_xymax_near)
                    .min(light_aabb_ndc_xymax_far),
                light_aabb_ndc_xymin_near
                    .max(light_aabb_ndc_xymin_far)
                    .max(light_aabb_ndc_xymax_near)
                    .max(light_aabb_ndc_xymax_far),
            );
            let min_cluster = ndc_position_to_cluster(
                clusters.axis_slices,
                cluster_factors,
                is_orthographic,
                light_aabb_ndc_min,
                light_aabb_view_min.z,
            );
            let max_cluster = ndc_position_to_cluster(
                clusters.axis_slices,
                cluster_factors,
                is_orthographic,
                light_aabb_ndc_max,
                light_aabb_view_max.z,
            );
            let (min_cluster, max_cluster) =
                (min_cluster.min(max_cluster), min_cluster.max(max_cluster));
            for y in min_cluster.y..=max_cluster.y {
                for x in min_cluster.x..=max_cluster.x {
                    for z in min_cluster.z..=max_cluster.z {
                        let cluster_index =
                            cluster_to_index(clusters.axis_slices, UVec3::new(x, y, z));
                        let cluster_aabb = &clusters.aabbs[cluster_index];
                        if light_sphere.intersects_obb(cluster_aabb, &view_transform) {
                            global_lights_set.insert(light_entity);
                            visible_lights_set.insert(light_entity);
                            clusters_lights[cluster_index].entities.push(light_entity);
                        }
                    }
                }
            }
        }

        for cluster_lights in clusters_lights.iter_mut() {
            cluster_lights.entities.shrink_to_fit();
        }

        clusters.lights = clusters_lights;
        commands.entity(view_entity).insert(VisiblePointLights {
            entities: visible_lights_set.into_iter().collect(),
        });
    }
    global_lights.entities = global_lights_set.into_iter().collect();
}

pub fn update_directional_light_frusta(
    mut views: Query<(&GlobalTransform, &DirectionalLight, &mut Frustum)>,
) {
    for (transform, directional_light, mut frustum) in views.iter_mut() {
        // The frustum is used for culling meshes to the light for shadow mapping
        // so if shadow mapping is disabled for this light, then the frustum is
        // not needed.
        if !directional_light.shadows_enabled {
            continue;
        }

        let view_projection = directional_light.shadow_projection.get_projection_matrix()
            * transform.compute_matrix().inverse();
        *frustum = Frustum::from_view_projection(
            &view_projection,
            &transform.translation,
            &transform.back(),
            directional_light.shadow_projection.far(),
        );
    }
}

// NOTE: Run this after assign_lights_to_clusters!
pub fn update_point_light_frusta(
    global_lights: Res<VisiblePointLights>,
    mut views: Query<(Entity, &GlobalTransform, &PointLight, &mut CubemapFrusta)>,
) {
    let projection =
        Mat4::perspective_infinite_reverse_rh(std::f32::consts::FRAC_PI_2, 1.0, POINT_LIGHT_NEAR_Z);
    let view_rotations = CUBE_MAP_FACES
        .iter()
        .map(|CubeMapFace { target, up }| GlobalTransform::identity().looking_at(*target, *up))
        .collect::<Vec<_>>();

    let global_lights_set = global_lights
        .entities
        .iter()
        .copied()
        .collect::<HashSet<_>>();
    for (entity, transform, point_light, mut cubemap_frusta) in views.iter_mut() {
        // The frusta are used for culling meshes to the light for shadow mapping
        // so if shadow mapping is disabled for this light, then the frusta are
        // not needed.
        // Also, if the light is not relevant for any cluster, it will not be in the
        // global lights set and so there is no need to update its frusta.
        if !point_light.shadows_enabled || !global_lights_set.contains(&entity) {
            continue;
        }

        // ignore scale because we don't want to effectively scale light radius and range
        // by applying those as a view transform to shadow map rendering of objects
        // and ignore rotation because we want the shadow map projections to align with the axes
        let view_translation = GlobalTransform::from_translation(transform.translation);
        let view_backward = transform.back();

        for (view_rotation, frustum) in view_rotations.iter().zip(cubemap_frusta.iter_mut()) {
            let view = view_translation * *view_rotation;
            let view_projection = projection * view.compute_matrix().inverse();

            *frustum = Frustum::from_view_projection(
                &view_projection,
                &transform.translation,
                &view_backward,
                point_light.range,
            );
        }
    }
}

pub fn check_light_mesh_visibility(
    // NOTE: VisiblePointLights is an alias for VisibleEntities so the Without<DirectionalLight>
    // is needed to avoid an unnecessary QuerySet
    visible_point_lights: Query<&VisiblePointLights, Without<DirectionalLight>>,
    mut point_lights: Query<(
        &PointLight,
        &GlobalTransform,
        &CubemapFrusta,
        &mut CubemapVisibleEntities,
        Option<&RenderLayers>,
    )>,
    mut directional_lights: Query<(
        &DirectionalLight,
        &Frustum,
        &mut VisibleEntities,
        Option<&RenderLayers>,
    )>,
    mut visible_entity_query: Query<
        (
            Entity,
            &Visibility,
            &mut ComputedVisibility,
            Option<&RenderLayers>,
            Option<&Aabb>,
            Option<&GlobalTransform>,
        ),
        Without<NotShadowCaster>,
    >,
) {
    // Directonal lights
    for (directional_light, frustum, mut visible_entities, maybe_view_mask) in
        directional_lights.iter_mut()
    {
        visible_entities.entities.clear();

        // NOTE: If shadow mapping is disabled for the light then it must have no visible entities
        if !directional_light.shadows_enabled {
            continue;
        }

        let view_mask = maybe_view_mask.copied().unwrap_or_default();

        for (
            entity,
            visibility,
            mut computed_visibility,
            maybe_entity_mask,
            maybe_aabb,
            maybe_transform,
        ) in visible_entity_query.iter_mut()
        {
            if !visibility.is_visible {
                continue;
            }

            let entity_mask = maybe_entity_mask.copied().unwrap_or_default();
            if !view_mask.intersects(&entity_mask) {
                continue;
            }

            // If we have an aabb and transform, do frustum culling
            if let (Some(aabb), Some(transform)) = (maybe_aabb, maybe_transform) {
                if !frustum.intersects_obb(aabb, &transform.compute_matrix()) {
                    continue;
                }
            }

            computed_visibility.is_visible = true;
            visible_entities.entities.push(entity);
        }

        // TODO: check for big changes in visible entities len() vs capacity() (ex: 2x) and resize
        // to prevent holding unneeded memory
    }

    // Point lights
    for visible_lights in visible_point_lights.iter() {
        for light_entity in visible_lights.entities.iter().copied() {
            if let Ok((
                point_light,
                transform,
                cubemap_frusta,
                mut cubemap_visible_entities,
                maybe_view_mask,
            )) = point_lights.get_mut(light_entity)
            {
                for visible_entities in cubemap_visible_entities.iter_mut() {
                    visible_entities.entities.clear();
                }

                // NOTE: If shadow mapping is disabled for the light then it must have no visible entities
                if !point_light.shadows_enabled {
                    continue;
                }

                let view_mask = maybe_view_mask.copied().unwrap_or_default();
                let light_sphere = Sphere {
                    center: transform.translation,
                    radius: point_light.range,
                };

                for (
                    entity,
                    visibility,
                    mut computed_visibility,
                    maybe_entity_mask,
                    maybe_aabb,
                    maybe_transform,
                ) in visible_entity_query.iter_mut()
                {
                    if !visibility.is_visible {
                        continue;
                    }

                    let entity_mask = maybe_entity_mask.copied().unwrap_or_default();
                    if !view_mask.intersects(&entity_mask) {
                        continue;
                    }

                    // If we have an aabb and transform, do frustum culling
                    if let (Some(aabb), Some(transform)) = (maybe_aabb, maybe_transform) {
                        let model_to_world = transform.compute_matrix();
                        // Do a cheap sphere vs obb test to prune out most meshes outside the sphere of the light
                        if !light_sphere.intersects_obb(aabb, &model_to_world) {
                            continue;
                        }
                        for (frustum, visible_entities) in cubemap_frusta
                            .iter()
                            .zip(cubemap_visible_entities.iter_mut())
                        {
                            if frustum.intersects_obb(aabb, &model_to_world) {
                                computed_visibility.is_visible = true;
                                visible_entities.entities.push(entity);
                            }
                        }
                    } else {
                        computed_visibility.is_visible = true;
                        for visible_entities in cubemap_visible_entities.iter_mut() {
                            visible_entities.entities.push(entity)
                        }
                    }
                }

                // TODO: check for big changes in visible entities len() vs capacity() (ex: 2x) and resize
                // to prevent holding unneeded memory
            }
        }
    }
}
