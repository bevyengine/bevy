//! Environment maps and reflection probes.
//!
//! An *environment map* consists of a pair of diffuse and specular cubemaps
//! that together reflect the static surrounding area of a region in space. When
//! available, the PBR shader uses these to apply diffuse light and calculate
//! specular reflections.
//!
//! Environment maps come in two flavors, depending on what other components the
//! entities they're attached to have:
//!
//! 1. If attached to a view, they represent the objects located a very far
//!    distance from the view, in a similar manner to a skybox. Essentially, these
//!    *view environment maps* represent a higher-quality replacement for
//!    [`AmbientLight`](crate::AmbientLight) for outdoor scenes. The indirect light from such
//!    environment maps are added to every point of the scene, including
//!    interior enclosed areas.
//!
//! 2. If attached to a [`crate::LightProbe`], environment maps represent the immediate
//!    surroundings of a specific location in the scene. These types of
//!    environment maps are known as *reflection probes*.
//!
//! Typically, environment maps are static (i.e. "baked", calculated ahead of
//! time) and so only reflect fixed static geometry. The environment maps must
//! be pre-filtered into a pair of cubemaps, one for the diffuse component and
//! one for the specular component, according to the [split-sum approximation].
//! To pre-filter your environment map, you can use the [glTF IBL Sampler] or
//! its [artist-friendly UI]. The diffuse map uses the Lambertian distribution,
//! while the specular map uses the GGX distribution.
//!
//! The Khronos Group has [several pre-filtered environment maps] available for
//! you to use.
//!
//! Currently, reflection probes (i.e. environment maps attached to light
//! probes) use binding arrays (also known as bindless textures) and
//! consequently aren't supported on WebGL2 or WebGPU. Reflection probes are
//! also unsupported if GLSL is in use, due to `naga` limitations. Environment
//! maps attached to views are, however, supported on all platforms.
//!
//! [split-sum approximation]: https://cdn2.unrealengine.com/Resources/files/2013SiggraphPresentationsNotes-26915738.pdf
//!
//! [glTF IBL Sampler]: https://github.com/KhronosGroup/glTF-IBL-Sampler
//!
//! [artist-friendly UI]: https://github.com/pcwalton/gltf-ibl-sampler-egui
//!
//! [several pre-filtered environment maps]: https://github.com/KhronosGroup/glTF-Sample-Environments

use bevy_asset::AssetId;
use bevy_ecs::{query::QueryItem, system::lifetimeless::Read};
use bevy_image::Image;
use bevy_light::EnvironmentMapLight;
use bevy_render::{
    extract_instances::ExtractInstance,
    render_asset::RenderAssets,
    render_resource::{
        binding_types::{self, uniform_buffer},
        BindGroupLayoutEntryBuilder, Sampler, SamplerBindingType, ShaderStages, TextureSampleType,
        TextureView,
    },
    renderer::{RenderAdapter, RenderDevice},
    texture::{FallbackImage, GpuImage},
};

use core::{num::NonZero, ops::Deref};

use crate::{
    add_cubemap_texture_view, binding_arrays_are_usable, EnvironmentMapUniform,
    MAX_VIEW_LIGHT_PROBES,
};

use super::{LightProbeComponent, RenderViewLightProbes};

/// Like [`EnvironmentMapLight`], but contains asset IDs instead of handles.
///
/// This is for use in the render app.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct EnvironmentMapIds {
    /// The blurry image that represents diffuse radiance surrounding a region.
    pub(crate) diffuse: AssetId<Image>,
    /// The typically-sharper, mipmapped image that represents specular radiance
    /// surrounding a region.
    pub(crate) specular: AssetId<Image>,
}

/// All the bind group entries necessary for PBR shaders to access the
/// environment maps exposed to a view.
pub(crate) enum RenderViewEnvironmentMapBindGroupEntries<'a> {
    /// The version used when binding arrays aren't available on the current
    /// platform.
    Single {
        /// The texture view of the view's diffuse cubemap.
        diffuse_texture_view: &'a TextureView,

        /// The texture view of the view's specular cubemap.
        specular_texture_view: &'a TextureView,

        /// The sampler used to sample elements of both `diffuse_texture_views` and
        /// `specular_texture_views`.
        sampler: &'a Sampler,
    },

    /// The version used when binding arrays are available on the current
    /// platform.
    Multiple {
        /// A texture view of each diffuse cubemap, in the same order that they are
        /// supplied to the view (i.e. in the same order as
        /// `binding_index_to_cubemap` in [`RenderViewLightProbes`]).
        ///
        /// This is a vector of `wgpu::TextureView`s. But we don't want to import
        /// `wgpu` in this crate, so we refer to it indirectly like this.
        diffuse_texture_views: Vec<&'a <TextureView as Deref>::Target>,

        /// As above, but for specular cubemaps.
        specular_texture_views: Vec<&'a <TextureView as Deref>::Target>,

        /// The sampler used to sample elements of both `diffuse_texture_views` and
        /// `specular_texture_views`.
        sampler: &'a Sampler,
    },
}

/// Information about the environment map attached to the view, if any. This is
/// a global environment map that lights everything visible in the view, as
/// opposed to a light probe which affects only a specific area.
pub struct EnvironmentMapViewLightProbeInfo {
    /// The index of the diffuse and specular cubemaps in the binding arrays.
    pub(crate) cubemap_index: i32,
    /// The smallest mip level of the specular cubemap.
    pub(crate) smallest_specular_mip_level: u32,
    /// The scale factor applied to the diffuse and specular light in the
    /// cubemap. This is in units of cd/m² (candela per square meter).
    pub(crate) intensity: f32,
    /// Whether this lightmap affects the diffuse lighting of lightmapped
    /// meshes.
    pub(crate) affects_lightmapped_mesh_diffuse: bool,
}

impl ExtractInstance for EnvironmentMapIds {
    type QueryData = Read<EnvironmentMapLight>;

    type QueryFilter = ();

    fn extract(item: QueryItem<'_, '_, Self::QueryData>) -> Option<Self> {
        Some(EnvironmentMapIds {
            diffuse: item.diffuse_map.id(),
            specular: item.specular_map.id(),
        })
    }
}

/// Returns the bind group layout entries for the environment map diffuse and
/// specular binding arrays respectively, in addition to the sampler.
pub(crate) fn get_bind_group_layout_entries(
    render_device: &RenderDevice,
    render_adapter: &RenderAdapter,
) -> [BindGroupLayoutEntryBuilder; 4] {
    let mut texture_cube_binding =
        binding_types::texture_cube(TextureSampleType::Float { filterable: true });
    if binding_arrays_are_usable(render_device, render_adapter) {
        texture_cube_binding =
            texture_cube_binding.count(NonZero::<u32>::new(MAX_VIEW_LIGHT_PROBES as _).unwrap());
    }

    [
        texture_cube_binding,
        texture_cube_binding,
        binding_types::sampler(SamplerBindingType::Filtering),
        uniform_buffer::<EnvironmentMapUniform>(true).visibility(ShaderStages::FRAGMENT),
    ]
}

impl<'a> RenderViewEnvironmentMapBindGroupEntries<'a> {
    /// Looks up and returns the bindings for the environment map diffuse and
    /// specular binding arrays respectively, as well as the sampler.
    pub(crate) fn get(
        render_view_environment_maps: Option<&RenderViewLightProbes<EnvironmentMapLight>>,
        images: &'a RenderAssets<GpuImage>,
        fallback_image: &'a FallbackImage,
        render_device: &RenderDevice,
        render_adapter: &RenderAdapter,
    ) -> RenderViewEnvironmentMapBindGroupEntries<'a> {
        if binding_arrays_are_usable(render_device, render_adapter) {
            let mut diffuse_texture_views = vec![];
            let mut specular_texture_views = vec![];
            let mut sampler = None;

            if let Some(environment_maps) = render_view_environment_maps {
                for &cubemap_id in &environment_maps.binding_index_to_textures {
                    add_cubemap_texture_view(
                        &mut diffuse_texture_views,
                        &mut sampler,
                        cubemap_id.diffuse,
                        images,
                        fallback_image,
                    );
                    add_cubemap_texture_view(
                        &mut specular_texture_views,
                        &mut sampler,
                        cubemap_id.specular,
                        images,
                        fallback_image,
                    );
                }
            }

            // Pad out the bindings to the size of the binding array using fallback
            // textures. This is necessary on D3D12 and Metal.
            diffuse_texture_views.resize(MAX_VIEW_LIGHT_PROBES, &*fallback_image.cube.texture_view);
            specular_texture_views
                .resize(MAX_VIEW_LIGHT_PROBES, &*fallback_image.cube.texture_view);

            return RenderViewEnvironmentMapBindGroupEntries::Multiple {
                diffuse_texture_views,
                specular_texture_views,
                sampler: sampler.unwrap_or(&fallback_image.cube.sampler),
            };
        }

        if let Some(environment_maps) = render_view_environment_maps {
            if let Some(cubemap) = environment_maps.binding_index_to_textures.first() {
                if let (Some(diffuse_image), Some(specular_image)) =
                    (images.get(cubemap.diffuse), images.get(cubemap.specular))
                {
                    return RenderViewEnvironmentMapBindGroupEntries::Single {
                        diffuse_texture_view: &diffuse_image.texture_view,
                        specular_texture_view: &specular_image.texture_view,
                        sampler: &diffuse_image.sampler,
                    };
                }
            }
        }

        RenderViewEnvironmentMapBindGroupEntries::Single {
            diffuse_texture_view: &fallback_image.cube.texture_view,
            specular_texture_view: &fallback_image.cube.texture_view,
            sampler: &fallback_image.cube.sampler,
        }
    }
}

impl LightProbeComponent for EnvironmentMapLight {
    type AssetId = EnvironmentMapIds;

    // Information needed to render with the environment map attached to the
    // view.
    type ViewLightProbeInfo = EnvironmentMapViewLightProbeInfo;

    fn id(&self, image_assets: &RenderAssets<GpuImage>) -> Option<Self::AssetId> {
        if image_assets.get(&self.diffuse_map).is_none()
            || image_assets.get(&self.specular_map).is_none()
        {
            None
        } else {
            Some(EnvironmentMapIds {
                diffuse: self.diffuse_map.id(),
                specular: self.specular_map.id(),
            })
        }
    }

    fn intensity(&self) -> f32 {
        self.intensity
    }

    fn affects_lightmapped_mesh_diffuse(&self) -> bool {
        self.affects_lightmapped_mesh_diffuse
    }

    fn create_render_view_light_probes(
        view_component: Option<&EnvironmentMapLight>,
        image_assets: &RenderAssets<GpuImage>,
    ) -> RenderViewLightProbes<Self> {
        let mut render_view_light_probes = RenderViewLightProbes::new();

        // Find the index of the cubemap associated with the view, and determine
        // its smallest mip level.
        if let Some(EnvironmentMapLight {
            diffuse_map: diffuse_map_handle,
            specular_map: specular_map_handle,
            intensity,
            affects_lightmapped_mesh_diffuse,
            ..
        }) = view_component
        {
            if let (Some(_), Some(specular_map)) = (
                image_assets.get(diffuse_map_handle),
                image_assets.get(specular_map_handle),
            ) {
                render_view_light_probes.view_light_probe_info = EnvironmentMapViewLightProbeInfo {
                    cubemap_index: render_view_light_probes.get_or_insert_cubemap(
                        &EnvironmentMapIds {
                            diffuse: diffuse_map_handle.id(),
                            specular: specular_map_handle.id(),
                        },
                    ) as i32,
                    smallest_specular_mip_level: specular_map.mip_level_count - 1,
                    intensity: *intensity,
                    affects_lightmapped_mesh_diffuse: *affects_lightmapped_mesh_diffuse,
                };
            }
        };

        render_view_light_probes
    }
}

impl Default for EnvironmentMapViewLightProbeInfo {
    fn default() -> Self {
        Self {
            cubemap_index: -1,
            smallest_specular_mip_level: 0,
            intensity: 1.0,
            affects_lightmapped_mesh_diffuse: true,
        }
    }
}
