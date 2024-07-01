//! Irradiance volumes, also known as voxel global illumination.
//!
//! An *irradiance volume* is a cuboid voxel region consisting of
//! regularly-spaced precomputed samples of diffuse indirect light. They're
//! ideal if you have a dynamic object such as a character that can move about
//! static non-moving geometry such as a level in a game, and you want that
//! dynamic object to be affected by the light bouncing off that static
//! geometry.
//!
//! To use irradiance volumes, you need to precompute, or *bake*, the indirect
//! light in your scene. Bevy doesn't currently come with a way to do this.
//! Fortunately, [Blender] provides a [baking tool] as part of the Eevee
//! renderer, and its irradiance volumes are compatible with those used by Bevy.
//! The [`bevy-baked-gi`] project provides a tool, `export-blender-gi`, that can
//! extract the baked irradiance volumes from the Blender `.blend` file and
//! package them up into a `.ktx2` texture for use by the engine. See the
//! documentation in the `bevy-baked-gi` project for more details on this
//! workflow.
//!
//! Like all light probes in Bevy, irradiance volumes are 1×1×1 cubes that can
//! be arbitrarily scaled, rotated, and positioned in a scene with the
//! [`bevy_transform::components::Transform`] component. The 3D voxel grid will
//! be stretched to fill the interior of the cube, and the illumination from the
//! irradiance volume will apply to all fragments within that bounding region.
//!
//! Bevy's irradiance volumes are based on Valve's [*ambient cubes*] as used in
//! *Half-Life 2* ([Mitchell 2006, slide 27]). These encode a single color of
//! light from the six 3D cardinal directions and blend the sides together
//! according to the surface normal. For an explanation of why ambient cubes
//! were chosen over spherical harmonics, see [Why ambient cubes?] below.
//!
//! If you wish to use a tool other than `export-blender-gi` to produce the
//! irradiance volumes, you'll need to pack the irradiance volumes in the
//! following format. The irradiance volume of resolution *(Rx, Ry, Rz)* is
//! expected to be a 3D texture of dimensions *(Rx, 2Ry, 3Rz)*. The unnormalized
//! texture coordinate *(s, t, p)* of the voxel at coordinate *(x, y, z)* with
//! side *S* ∈ *{-X, +X, -Y, +Y, -Z, +Z}* is as follows:
//!
//! ```text
//! s = x
//!
//! t = y + ⎰  0 if S ∈ {-X, -Y, -Z}
//!         ⎱ Ry if S ∈ {+X, +Y, +Z}
//!
//!         ⎧   0 if S ∈ {-X, +X}
//! p = z + ⎨  Rz if S ∈ {-Y, +Y}
//!         ⎩ 2Rz if S ∈ {-Z, +Z}
//! ```
//!
//! Visually, in a left-handed coordinate system with Y up, viewed from the
//! right, the 3D texture looks like a stacked series of voxel grids, one for
//! each cube side, in this order:
//!
//! | **+X** | **+Y** | **+Z** |
//! | ------ | ------ | ------ |
//! | **-X** | **-Y** | **-Z** |
//!
//! A terminology note: Other engines may refer to irradiance volumes as *voxel
//! global illumination*, *VXGI*, or simply as *light probes*. Sometimes *light
//! probe* refers to what Bevy calls a reflection probe. In Bevy, *light probe*
//! is a generic term that encompasses all cuboid bounding regions that capture
//! indirect illumination, whether based on voxels or not.
//!
//! Note that, if binding arrays aren't supported (e.g. on WebGPU or WebGL 2),
//! then only the closest irradiance volume to the view will be taken into
//! account during rendering. The required `wgpu` features are
//! [`bevy_render::settings::WgpuFeatures::TEXTURE_BINDING_ARRAY`] and
//! [`bevy_render::settings::WgpuFeatures::SAMPLED_TEXTURE_AND_STORAGE_BUFFER_ARRAY_NON_UNIFORM_INDEXING`].
//!
//! ## Why ambient cubes?
//!
//! This section describes the motivation behind the decision to use ambient
//! cubes in Bevy. It's not needed to use the feature; feel free to skip it
//! unless you're interested in its internal design.
//!
//! Bevy uses *Half-Life 2*-style ambient cubes (usually abbreviated as *HL2*)
//! as the representation of irradiance for light probes instead of the
//! more-popular spherical harmonics (*SH*). This might seem to be a surprising
//! choice, but it turns out to work well for the specific case of voxel
//! sampling on the GPU. Spherical harmonics have two problems that make them
//! less ideal for this use case:
//!
//! 1. The level 1 spherical harmonic coefficients can be negative. That
//!     prevents the use of the efficient [RGB9E5 texture format], which only
//!     encodes unsigned floating point numbers, and forces the use of the
//!     less-efficient [RGBA16F format] if hardware interpolation is desired.
//!
//! 2. As an alternative to RGBA16F, level 1 spherical harmonics can be
//!     normalized and scaled to the SH0 base color, as [Frostbite] does. This
//!     allows them to be packed in standard LDR RGBA8 textures. However, this
//!     prevents the use of hardware trilinear filtering, as the nonuniform scale
//!     factor means that hardware interpolation no longer produces correct results.
//!     The 8 texture fetches needed to interpolate between voxels can be upwards of
//!     twice as slow as the hardware interpolation.
//!
//! The following chart summarizes the costs and benefits of ambient cubes,
//! level 1 spherical harmonics, and level 2 spherical harmonics:
//!
//! | Technique                | HW-interpolated samples | Texel fetches | Bytes per voxel | Quality |
//! | ------------------------ | ----------------------- | ------------- | --------------- | ------- |
//! | Ambient cubes            |                       3 |             0 |              24 | Medium  |
//! | Level 1 SH, compressed   |                       0 |            36 |              16 | Low     |
//! | Level 1 SH, uncompressed |                       4 |             0 |              24 | Low     |
//! | Level 2 SH, compressed   |                       0 |            72 |              28 | High    |
//! | Level 2 SH, uncompressed |                       9 |             0 |              54 | High    |
//!
//! (Note that the number of bytes per voxel can be reduced using various
//! texture compression methods, but the overall ratios remain similar.)
//!
//! From these data, we can see that ambient cubes balance fast lookups (from
//! leveraging hardware interpolation) with relatively-small storage
//! requirements and acceptable quality. Hence, they were chosen for irradiance
//! volumes in Bevy.
//!
//! [*ambient cubes*]: https://advances.realtimerendering.com/s2006/Mitchell-ShadingInValvesSourceEngine.pdf
//!
//! [spherical harmonics]: https://en.wikipedia.org/wiki/Spherical_harmonic_lighting
//!
//! [RGB9E5 texture format]: https://www.khronos.org/opengl/wiki/Small_Float_Formats#RGB9_E5
//!
//! [RGBA16F format]: https://www.khronos.org/opengl/wiki/Small_Float_Formats#Low-bitdepth_floats
//!
//! [Frostbite]: https://media.contentapi.ea.com/content/dam/eacom/frostbite/files/gdc2018-precomputedgiobalilluminationinfrostbite.pdf#page=53
//!
//! [Mitchell 2006, slide 27]: https://advances.realtimerendering.com/s2006/Mitchell-ShadingInValvesSourceEngine.pdf#page=27
//!
//! [Blender]: http://blender.org/
//!
//! [baking tool]: https://docs.blender.org/manual/en/latest/render/eevee/render_settings/indirect_lighting.html
//!
//! [`bevy-baked-gi`]: https://github.com/pcwalton/bevy-baked-gi
//!
//! [Why ambient cubes?]: #why-ambient-cubes

use bevy_ecs::component::Component;
use bevy_render::{
    render_asset::RenderAssets,
    render_resource::{
        binding_types, BindGroupLayoutEntryBuilder, Sampler, SamplerBindingType, Shader,
        TextureSampleType, TextureView,
    },
    renderer::RenderDevice,
    texture::{FallbackImage, GpuImage, Image},
};
use std::{num::NonZeroU32, ops::Deref};

use bevy_asset::{AssetId, Handle};
use bevy_reflect::Reflect;

use crate::{
    add_cubemap_texture_view, binding_arrays_are_usable, RenderViewLightProbes,
    MAX_VIEW_LIGHT_PROBES,
};

use super::LightProbeComponent;

pub const IRRADIANCE_VOLUME_SHADER_HANDLE: Handle<Shader> =
    Handle::weak_from_u128(160299515939076705258408299184317675488);

/// On WebGL and WebGPU, we must disable irradiance volumes, as otherwise we can
/// overflow the number of texture bindings when deferred rendering is in use
/// (see issue #11885).
pub(crate) const IRRADIANCE_VOLUMES_ARE_USABLE: bool = cfg!(not(target_arch = "wasm32"));

/// The component that defines an irradiance volume.
///
/// See [`crate::irradiance_volume`] for detailed information.
#[derive(Clone, Default, Reflect, Component, Debug)]
pub struct IrradianceVolume {
    /// The 3D texture that represents the ambient cubes, encoded in the format
    /// described in [`crate::irradiance_volume`].
    pub voxels: Handle<Image>,

    /// Scale factor applied to the diffuse and specular light generated by this component.
    ///
    /// After applying this multiplier, the resulting values should
    /// be in units of [cd/m^2](https://en.wikipedia.org/wiki/Candela_per_square_metre).
    ///
    /// See also <https://google.github.io/filament/Filament.html#lighting/imagebasedlights/iblunit>.
    pub intensity: f32,
}

/// All the bind group entries necessary for PBR shaders to access the
/// irradiance volumes exposed to a view.
pub(crate) enum RenderViewIrradianceVolumeBindGroupEntries<'a> {
    /// The version used when binding arrays aren't available on the current platform.
    Single {
        /// The texture view of the closest light probe.
        texture_view: &'a TextureView,
        /// A sampler used to sample voxels of the irradiance volume.
        sampler: &'a Sampler,
    },

    /// The version used when binding arrays are available on the current
    /// platform.
    Multiple {
        /// A texture view of the voxels of each irradiance volume, in the same
        /// order that they are supplied to the view (i.e. in the same order as
        /// `binding_index_to_cubemap` in [`RenderViewLightProbes`]).
        ///
        /// This is a vector of `wgpu::TextureView`s. But we don't want to import
        /// `wgpu` in this crate, so we refer to it indirectly like this.
        texture_views: Vec<&'a <TextureView as Deref>::Target>,

        /// A sampler used to sample voxels of the irradiance volumes.
        sampler: &'a Sampler,
    },
}

impl<'a> RenderViewIrradianceVolumeBindGroupEntries<'a> {
    /// Looks up and returns the bindings for any irradiance volumes visible in
    /// the view, as well as the sampler.
    pub(crate) fn get(
        render_view_irradiance_volumes: Option<&RenderViewLightProbes<IrradianceVolume>>,
        images: &'a RenderAssets<GpuImage>,
        fallback_image: &'a FallbackImage,
        render_device: &RenderDevice,
    ) -> RenderViewIrradianceVolumeBindGroupEntries<'a> {
        if binding_arrays_are_usable(render_device) {
            RenderViewIrradianceVolumeBindGroupEntries::get_multiple(
                render_view_irradiance_volumes,
                images,
                fallback_image,
            )
        } else {
            RenderViewIrradianceVolumeBindGroupEntries::get_single(
                render_view_irradiance_volumes,
                images,
                fallback_image,
            )
        }
    }

    /// Looks up and returns the bindings for any irradiance volumes visible in
    /// the view, as well as the sampler. This is the version used when binding
    /// arrays are available on the current platform.
    fn get_multiple(
        render_view_irradiance_volumes: Option<&RenderViewLightProbes<IrradianceVolume>>,
        images: &'a RenderAssets<GpuImage>,
        fallback_image: &'a FallbackImage,
    ) -> RenderViewIrradianceVolumeBindGroupEntries<'a> {
        let mut texture_views = vec![];
        let mut sampler = None;

        if let Some(irradiance_volumes) = render_view_irradiance_volumes {
            for &cubemap_id in &irradiance_volumes.binding_index_to_textures {
                add_cubemap_texture_view(
                    &mut texture_views,
                    &mut sampler,
                    cubemap_id,
                    images,
                    fallback_image,
                );
            }
        }

        // Pad out the bindings to the size of the binding array using fallback
        // textures. This is necessary on D3D12 and Metal.
        texture_views.resize(MAX_VIEW_LIGHT_PROBES, &*fallback_image.d3.texture_view);

        RenderViewIrradianceVolumeBindGroupEntries::Multiple {
            texture_views,
            sampler: sampler.unwrap_or(&fallback_image.d3.sampler),
        }
    }

    /// Looks up and returns the bindings for any irradiance volumes visible in
    /// the view, as well as the sampler. This is the version used when binding
    /// arrays aren't available on the current platform.
    fn get_single(
        render_view_irradiance_volumes: Option<&RenderViewLightProbes<IrradianceVolume>>,
        images: &'a RenderAssets<GpuImage>,
        fallback_image: &'a FallbackImage,
    ) -> RenderViewIrradianceVolumeBindGroupEntries<'a> {
        if let Some(irradiance_volumes) = render_view_irradiance_volumes {
            if let Some(irradiance_volume) = irradiance_volumes.render_light_probes.first() {
                if irradiance_volume.texture_index >= 0 {
                    if let Some(image_id) = irradiance_volumes
                        .binding_index_to_textures
                        .get(irradiance_volume.texture_index as usize)
                    {
                        if let Some(image) = images.get(*image_id) {
                            return RenderViewIrradianceVolumeBindGroupEntries::Single {
                                texture_view: &image.texture_view,
                                sampler: &image.sampler,
                            };
                        }
                    }
                }
            }
        }

        RenderViewIrradianceVolumeBindGroupEntries::Single {
            texture_view: &fallback_image.d3.texture_view,
            sampler: &fallback_image.d3.sampler,
        }
    }
}

/// Returns the bind group layout entries for the voxel texture and sampler
/// respectively.
pub(crate) fn get_bind_group_layout_entries(
    render_device: &RenderDevice,
) -> [BindGroupLayoutEntryBuilder; 2] {
    let mut texture_3d_binding =
        binding_types::texture_3d(TextureSampleType::Float { filterable: true });
    if binding_arrays_are_usable(render_device) {
        texture_3d_binding =
            texture_3d_binding.count(NonZeroU32::new(MAX_VIEW_LIGHT_PROBES as _).unwrap());
    }

    [
        texture_3d_binding,
        binding_types::sampler(SamplerBindingType::Filtering),
    ]
}

impl LightProbeComponent for IrradianceVolume {
    type AssetId = AssetId<Image>;

    // Irradiance volumes can't be attached to the view, so we store nothing
    // here.
    type ViewLightProbeInfo = ();

    fn id(&self, image_assets: &RenderAssets<GpuImage>) -> Option<Self::AssetId> {
        if image_assets.get(&self.voxels).is_none() {
            None
        } else {
            Some(self.voxels.id())
        }
    }

    fn intensity(&self) -> f32 {
        self.intensity
    }

    fn create_render_view_light_probes(
        _: Option<&Self>,
        _: &RenderAssets<GpuImage>,
    ) -> RenderViewLightProbes<Self> {
        RenderViewLightProbes::new()
    }
}
