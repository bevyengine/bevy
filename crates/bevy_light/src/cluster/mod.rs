//! Spatial clustering of objects to accelerate rendering performance.

use bevy_asset::Handle;
use bevy_camera::{
    prelude::ViewVisibility,
    primitives::Aabb,
    visibility::{self, Visibility, VisibilityClass},
};
use bevy_ecs::{
    component::Component,
    entity::Entity,
    query::{Or, With, Without},
    reflect::ReflectComponent,
    resource::Resource,
    system::{Commands, Query},
};
use bevy_image::Image;
use bevy_math::{AspectRatio, UVec2, UVec3, Vec3A, Vec3Swizzles as _};
use bevy_reflect::{std_traits::ReflectDefault, Reflect};
use bevy_transform::components::Transform;
use tracing::warn;

use crate::LightProbe;

pub mod assign;

#[cfg(test)]
mod test;

// Clustered-forward rendering notes
// The main initial reference material used was this rather accessible article:
// http://www.aortiz.me/2018/12/21/CG.html
// Some inspiration was taken from “Practical Clustered Shading” which is part 2 of:
// https://efficientshading.com/2015/01/01/real-time-many-light-management-and-shadows-with-clustered-shading/
// (Also note that Part 3 of the above shows how we could support the shadow mapping for many lights.)
// The z-slicing method mentioned in the aortiz article is originally from Tiago Sousa's Siggraph 2016 talk about Doom 2016:
// http://advances.realtimerendering.com/s2016/Siggraph2016_idTech6.pdf

/// Cluster configuration depends on rendering capabilities, these are exposed here.
/// They are automatically set by `bevy_pbr`, but a custom renderer may configure these too.
#[derive(Clone, Resource, Debug)]
#[expect(missing_docs, reason = "self explanatory")]
pub struct GlobalClusterSettings {
    pub supports_storage_buffers: bool,
    pub clustered_decals_are_usable: bool,
    /// Settings relating to GPU clustering, if GPU clustering is enabled.
    ///
    /// To disable GPU light clustering, set this to `None`.
    pub gpu_clustering: Option<GlobalClusterGpuSettings>,
    pub max_uniform_buffer_clusterable_objects: usize,
    pub view_cluster_bindings_max_indices: usize,
}

/// Settings relating to GPU clustering.
#[derive(Clone, Copy, Debug)]
pub struct GlobalClusterGpuSettings {
    /// The initial capacity of the list of Z slices.
    ///
    /// If there are too many clusterable objects on screen, this can overflow.
    /// Bevy will detect this situation and resize it, but you might see a few
    /// incorrect frames before Bevy resizes the list. To avoid this issue, you
    /// can set this to a higher value.
    ///
    /// The default value is
    /// `bevy::pbr::cluster::GPU_CLUSTERING_INITIAL_Z_SLICE_LIST_SIZE`.
    pub initial_z_slice_list_capacity: usize,

    /// The initial capacity of the list of clusterable objects.
    ///
    /// If there are too many clusterable objects on screen, this can overflow.
    /// Bevy will detect this situation and resize it, but you might see a few
    /// incorrect frames before Bevy resizes the list. To avoid this issue, you
    /// can set this to a higher value.
    ///
    /// The default value is
    /// `bevy::pbr::cluster::GPU_CLUSTERING_INITIAL_INDEX_LIST_CAPACITY`.
    pub initial_index_list_capacity: usize,
}

/// Configure the far z-plane mode used for the furthest depth slice for clustered forward
/// rendering
#[derive(Debug, Copy, Clone, Reflect)]
#[reflect(Clone)]
pub enum ClusterFarZMode {
    /// Calculate the required maximum z-depth based on the clusterable objects
    /// that were visible on the previous frame. Makes better use of available
    /// clusters, speeding up GPU lighting operations at the expense of using
    /// more indices in the clusterable object index lists.
    MaxClusterableObjectRange,
    /// Constant max z-depth
    Constant(f32),
}

/// Configure the depth-slicing strategy for clustered forward rendering
#[derive(Debug, Copy, Clone, Reflect)]
#[reflect(Default, Clone)]
pub struct ClusterZConfig {
    /// Far `Z` plane of the first depth slice
    pub first_slice_depth: f32,
    /// Strategy for how to evaluate the far `Z` plane of the furthest depth slice
    pub far_z_mode: ClusterFarZMode,
}

/// Configuration of the clustering strategy for clustered forward rendering
#[derive(Debug, Copy, Clone, Component, Reflect)]
#[reflect(Component, Debug, Default, Clone)]
pub enum ClusterConfig {
    /// Disable cluster calculations for this view
    None,
    /// One single cluster. Optimal for low-light complexity scenes or scenes where
    /// most lights affect the entire scene.
    Single,
    /// Explicit `X`, `Y` and `Z` counts (may yield non-square `X/Y` clusters depending on the aspect ratio)
    XYZ {
        /// The dimensions of the cluster grid.
        dimensions: UVec3,
        /// How to distribute the `Z` slices spatially.
        z_config: ClusterZConfig,
        /// Specify if clusters should automatically resize in `X/Y` if there is a risk of exceeding
        /// the available cluster-object index limit
        dynamic_resizing: bool,
    },
    /// Fixed number of `Z` slices, `X` and `Y` calculated to give square clusters
    /// with at most total clusters. For top-down games where lights will generally always be within a
    /// short depth range, it may be useful to use this configuration with 1 or few `Z` slices. This
    /// would reduce the number of lights per cluster by distributing more clusters in screen space
    /// `X/Y` which matches how lights are distributed in the scene.
    FixedZ {
        /// The total number of clusters to distribute.
        total: u32,
        /// The number of `Z` slices to distribute the clusters over.
        z_slices: u32,
        /// How to distribute the `Z` slices spatially.
        z_config: ClusterZConfig,
        /// Specify if clusters should automatically resize in `X/Y` if there is a risk of exceeding
        /// the available clusterable object index limit
        dynamic_resizing: bool,
    },
}

/// The cluster geometry generated by [`ClusterConfig`].
#[derive(Component, Debug)]
pub struct Clusters {
    /// The dimensions of the rectangle the cluster occupies in screen-space, rounded up to the nearest pixel.
    pub tile_size: UVec2,
    /// Number of clusters in `X` / `Y` / `Z` in the view frustum
    pub dimensions: UVec3,
    /// Distance to the far plane of the first depth slice. The first depth slice is special
    /// and explicitly-configured to avoid having unnecessarily many slices close to the camera.
    pub near: f32,
    /// Distance to the far plane of the last depth slice. This may change depending on [`ClusterZConfig`] used.
    pub far: f32,
    /// The farthest Z value of any bounding sphere of any clusterable object on
    /// the previous frame.
    ///
    /// This is used for the [`ClusterFarZMode::MaxClusterableObjectRange`]
    /// feature.
    pub last_frame_farthest_z: Option<f32>,
    /// The sum of the number of objects that all clusters contained last frame.
    ///
    /// This is used for the `dynamic_resizing` feature, which automatically
    /// grows the number of clusters if clusters have likely become too large.
    pub last_frame_total_cluster_index_count: Option<usize>,
    /// All objects within the cluster.
    pub clusterable_objects: ClusterableObjects,
}

/// The list of objects within a cluster, if known to the CPU.
#[derive(Debug)]
pub enum ClusterableObjects {
    /// The list of objects in the cluster is known to the CPU.
    Cpu(Vec<ObjectsInClusterCpu>),
    /// The list of objects in the cluster is unknown to the CPU, because GPU
    /// clustering is being used.
    Gpu,
}

/// The [`VisibilityClass`] used for clusterables (decals, point lights, spot
/// lights, and light probes).
///
/// [`VisibilityClass`]: bevy_camera::visibility::VisibilityClass
pub struct ClusterVisibilityClass;

/// All objects that potentially intersect a single cluster.
#[derive(Clone, Default, Debug)]
pub struct ObjectsInClusterCpu {
    /// A list of all clusterable objects that are potentially visible from this
    /// view.
    clusterables: Vec<Entity>,

    /// The number of each clusterable object type.
    pub counts: ClusterableObjectCounts,
}

/// Stores the number of each type of clusterable object in a single cluster.
///
/// Note that `reflection_probes` and `irradiance_volumes` won't be clustered if
/// fewer than 3 SSBOs are available, which usually means on WebGL 2.
#[derive(Clone, Copy, Default, Debug)]
pub struct ClusterableObjectCounts {
    /// The number of point lights in the cluster.
    pub point_lights: u32,
    /// The number of spot lights in the cluster.
    pub spot_lights: u32,
    /// The number of reflection probes in the cluster.
    pub reflection_probes: u32,
    /// The number of irradiance volumes in the cluster.
    pub irradiance_volumes: u32,
    /// The number of decals in the cluster.
    pub decals: u32,
}

/// An object that projects a decal onto surfaces within its bounds.
///
/// Conceptually, a clustered decal is a 1×1×1 cube centered on its origin. It
/// projects its images onto surfaces in the -Z direction (thus you may find
/// [`Transform::looking_at`] useful).
///
/// Each decal may project any of a base color texture, a normal map, a
/// metallic/roughness map, and/or a texture that specifies emissive light. In
/// addition, you may associate an arbitrary integer [`Self::tag`] with each
/// clustered decal, which Bevy doesn't use, but that you can use in your
/// shaders in order to associate application-specific data with your decals.
///
/// Clustered decals are the highest-quality types of decals that Bevy supports,
/// but they require bindless textures. This means that they presently can't be
/// used on WebGL 2, WebGPU, macOS, or iOS. Bevy's clustered decals can be used
/// with forward or deferred rendering and don't require a prepass.
#[derive(Component, Debug, Clone, Default, Reflect)]
#[reflect(Component, Debug, Clone, Default)]
#[require(Transform, ViewVisibility, Visibility, VisibilityClass)]
#[component(on_add = visibility::add_visibility_class::<ClusterVisibilityClass>)]
pub struct ClusteredDecal {
    /// The image that the clustered decal projects onto the base color of the
    /// surface material.
    ///
    /// This must be a 2D image. If it has an alpha channel, it'll be alpha
    /// blended with the underlying surface and/or other decals. All decal
    /// images in the scene must use the same sampler.
    pub base_color_texture: Option<Handle<Image>>,

    /// The normal map that the clustered decal projects onto surfaces.
    ///
    /// Bevy uses the *Whiteout* method to combine normal maps from decals with
    /// any normal map that the surface has, as described in the
    /// [*Blending in Detail* article].
    ///
    /// Note that the normal map must be three-channel and must be in OpenGL
    /// format, not DirectX format. That is, the green channel must point up,
    /// not down.
    ///
    /// [*Blending in Detail* article]: https://blog.selfshadow.com/publications/blending-in-detail/
    pub normal_map_texture: Option<Handle<Image>>,

    /// The metallic-roughness map that the clustered decal projects onto
    /// surfaces.
    ///
    /// Metallic and roughness PBR parameters are blended onto the base surface
    /// using the alpha channel of the base color.
    ///
    /// Metallic is expected to be in the blue channel, while roughness is
    /// expected to be in the green channel, following glTF conventions.
    pub metallic_roughness_texture: Option<Handle<Image>>,

    /// The emissive map that the clustered decal projects onto surfaces.
    ///
    /// Including this texture effectively causes the decal to glow. The
    /// emissive component is blended onto the surface according to the alpha
    /// channel.
    pub emissive_texture: Option<Handle<Image>>,

    /// An application-specific tag you can use for any purpose you want, in
    /// conjunction with a custom shader.
    ///
    /// This value is exposed to the shader via the iterator API
    /// (`bevy_pbr::decal::clustered::clustered_decal_iterator_new` and
    /// `bevy_pbr::decal::clustered::clustered_decal_iterator_next`).
    ///
    /// For example, you might use the tag to restrict the set of surfaces to
    /// which a decal can be rendered.
    ///
    /// See the `clustered_decals` example for an example of use.
    pub tag: u32,
}

impl Default for ClusterZConfig {
    fn default() -> Self {
        Self {
            first_slice_depth: 5.0,
            far_z_mode: ClusterFarZMode::MaxClusterableObjectRange,
        }
    }
}

impl Default for ClusterConfig {
    fn default() -> Self {
        // 24 depth slices, square clusters with at most 4096 total clusters
        // use max light distance as clusters max `Z`-depth, first slice extends to 5.0
        Self::FixedZ {
            total: 4096,
            z_slices: 24,
            z_config: ClusterZConfig::default(),
            dynamic_resizing: true,
        }
    }
}

impl ClusterConfig {
    fn dimensions_for_screen_size(&self, screen_size: UVec2) -> UVec3 {
        match &self {
            ClusterConfig::None => UVec3::ZERO,
            ClusterConfig::Single => UVec3::ONE,
            ClusterConfig::XYZ { dimensions, .. } => *dimensions,
            ClusterConfig::FixedZ {
                total, z_slices, ..
            } => {
                let aspect_ratio: f32 = AspectRatio::try_from_pixels(screen_size.x, screen_size.y)
                    .expect("Failed to calculate aspect ratio for Cluster: screen dimensions must be positive, non-zero values")
                    .ratio();
                let mut z_slices = *z_slices;
                if *total < z_slices {
                    warn!("ClusterConfig has more z-slices than total clusters!");
                    z_slices = *total;
                }
                let per_layer = *total as f32 / z_slices as f32;

                let y = f32::sqrt(per_layer / aspect_ratio);

                let mut x = (y * aspect_ratio) as u32;
                let mut y = y as u32;

                // check extremes
                if x == 0 {
                    x = 1;
                    y = per_layer as u32;
                }
                if y == 0 {
                    x = per_layer as u32;
                    y = 1;
                }

                UVec3::new(x, y, z_slices)
            }
        }
    }

    fn first_slice_depth(&self) -> f32 {
        match self {
            ClusterConfig::None | ClusterConfig::Single => 0.0,
            ClusterConfig::XYZ { z_config, .. } | ClusterConfig::FixedZ { z_config, .. } => {
                z_config.first_slice_depth
            }
        }
    }

    fn far_z_mode(&self) -> ClusterFarZMode {
        match self {
            ClusterConfig::None => ClusterFarZMode::Constant(0.0),
            ClusterConfig::Single => ClusterFarZMode::MaxClusterableObjectRange,
            ClusterConfig::XYZ { z_config, .. } | ClusterConfig::FixedZ { z_config, .. } => {
                z_config.far_z_mode
            }
        }
    }

    fn dynamic_resizing(&self) -> bool {
        match self {
            ClusterConfig::None | ClusterConfig::Single => false,
            ClusterConfig::XYZ {
                dynamic_resizing, ..
            }
            | ClusterConfig::FixedZ {
                dynamic_resizing, ..
            } => *dynamic_resizing,
        }
    }
}

impl Default for Clusters {
    fn default() -> Clusters {
        Clusters {
            tile_size: UVec2::ZERO,
            dimensions: UVec3::ZERO,
            near: 0.0,
            far: 0.0,
            last_frame_farthest_z: None,
            last_frame_total_cluster_index_count: None,
            // Although we start with CPU clustering, this will be switched to
            // GPU clustering if that's enabled.
            clusterable_objects: ClusterableObjects::Cpu(vec![]),
        }
    }
}

impl Clusters {
    fn update(&mut self, screen_size: UVec2, requested_dimensions: UVec3) {
        debug_assert!(
            requested_dimensions.x > 0 && requested_dimensions.y > 0 && requested_dimensions.z > 0
        );

        let tile_size = (screen_size.as_vec2() / requested_dimensions.xy().as_vec2())
            .ceil()
            .as_uvec2()
            .max(UVec2::ONE);
        self.tile_size = tile_size;
        self.dimensions = (screen_size.as_vec2() / tile_size.as_vec2())
            .ceil()
            .as_uvec2()
            .extend(requested_dimensions.z)
            .max(UVec3::ONE);

        // NOTE: Maximum 4096 clusters due to uniform buffer size constraints
        debug_assert!(self.dimensions.x * self.dimensions.y * self.dimensions.z <= 4096);
    }

    fn clear(&mut self, global_cluster_settings: &GlobalClusterSettings) {
        self.tile_size = UVec2::ONE;
        self.dimensions = UVec3::ZERO;
        self.near = 0.0;
        self.far = 0.0;

        match (
            &mut self.clusterable_objects,
            &global_cluster_settings.gpu_clustering,
        ) {
            (ClusterableObjects::Cpu(_), Some(_)) => {
                self.clusterable_objects = ClusterableObjects::Gpu;
            }
            (ClusterableObjects::Cpu(objects_in_cluster_cpu), None) => {
                objects_in_cluster_cpu.clear();
            }
            (ClusterableObjects::Gpu, Some(_)) => {
                self.clusterable_objects = ClusterableObjects::Cpu(vec![]);
            }
            (ClusterableObjects::Gpu, None) => {}
        }
    }

    fn reset_for_new_frame(
        &mut self,
        cluster_count: usize,
        global_cluster_settings: &GlobalClusterSettings,
    ) {
        match (
            &mut self.clusterable_objects,
            &global_cluster_settings.gpu_clustering,
        ) {
            (ClusterableObjects::Cpu(_), Some(_)) => {
                self.clusterable_objects = ClusterableObjects::Gpu;
            }

            (ClusterableObjects::Cpu(objects_in_cluster_cpu), None) => {
                for clusterable_objects in objects_in_cluster_cpu.iter_mut() {
                    clusterable_objects.clear();
                }
                objects_in_cluster_cpu.resize_with(cluster_count, ObjectsInClusterCpu::default);
            }

            (ClusterableObjects::Gpu, Some(_)) => {}

            (ClusterableObjects::Gpu, None) => {
                self.clusterable_objects =
                    ClusterableObjects::Cpu(vec![ObjectsInClusterCpu::default(); cluster_count]);
            }
        }
    }
}

impl ObjectsInClusterCpu {
    /// Clears out all objects in this cluster in preparation for a new frame.
    pub fn clear(&mut self) {
        self.clusterables.clear();
        self.counts = ClusterableObjectCounts::default();
    }

    /// Adds a spot light to the list.
    pub fn add_spot_light(&mut self, entity: Entity) {
        self.clusterables.push(entity);
        self.counts.spot_lights += 1;
    }

    /// Adds a point light to the list.
    pub fn add_point_light(&mut self, entity: Entity) {
        self.clusterables.push(entity);
        self.counts.point_lights += 1;
    }

    /// Adds a reflection probe to the list.
    pub fn add_reflection_probe(&mut self, entity: Entity) {
        self.clusterables.push(entity);
        self.counts.reflection_probes += 1;
    }

    /// Adds an irradiance volume to the list.
    pub fn add_irradiance_volume(&mut self, entity: Entity) {
        self.clusterables.push(entity);
        self.counts.irradiance_volumes += 1;
    }

    /// Adds a decal to the list.
    pub fn add_decal(&mut self, entity: Entity) {
        self.clusterables.push(entity);
        self.counts.decals += 1;
    }

    /// Iterates through all objects in this cluster.
    pub fn iter(&self) -> impl DoubleEndedIterator<Item = &Entity> {
        self.clusterables.iter()
    }
}

/// A system that adds AABBs to light probes and decals so that the visibility
/// determination works for them.
pub fn add_light_probe_and_decal_aabbs(
    mut commands: Commands,
    light_probes_and_decals_query: Query<
        Entity,
        (Or<(With<ClusteredDecal>, With<LightProbe>)>, Without<Aabb>),
    >,
) {
    for entity in &light_probes_and_decals_query {
        commands.entity(entity).insert(Aabb {
            center: Vec3A::ZERO,
            // Light probes are always unit-cube sized, the transform scale is what gives them their size.
            // Scale should not be included in the Aabb because it gets transformed by the GlobalTransform.
            half_extents: Vec3A::splat(0.5),
        });
    }
}
