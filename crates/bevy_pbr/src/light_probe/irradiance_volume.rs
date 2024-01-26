//! Irradiance volumes.

use bevy_ecs::component::Component;
use bevy_render::{
    render_asset::RenderAssets,
    render_resource::{
        binding_types, BindGroupLayoutEntryBuilder, Sampler, SamplerBindingType, Shader,
        TextureSampleType, TextureView,
    },
    renderer::RenderDevice,
    texture::{FallbackImage, Image},
};
use std::{num::NonZeroU32, ops::Deref};

use bevy_asset::{AssetId, Handle};
use bevy_reflect::Reflect;

use crate::{
    add_cubemap_texture_view, environment_map::binding_arrays_are_usable, RenderViewLightProbes,
    MAX_VIEW_LIGHT_PROBES,
};

use super::LightProbeComponent;

pub const IRRADIANCE_VOLUME_SHADER_HANDLE: Handle<Shader> =
    Handle::weak_from_u128(160299515939076705258408299184317675488);

/// The component that defines an irradiance volume.
#[derive(Clone, Default, Reflect, Component, Debug)]
pub struct IrradianceVolume {
    pub voxels: Handle<Image>,
    pub intensity: f32,
}

pub(crate) enum RenderViewIrradianceVolumeBindGroupEntries<'a> {
    Single {
        texture_view: &'a TextureView,
        sampler: &'a Sampler,
    },
    Multiple {
        texture_views: Vec<&'a <TextureView as Deref>::Target>,
        sampler: &'a Sampler,
    },
}

impl<'a> RenderViewIrradianceVolumeBindGroupEntries<'a> {
    pub(crate) fn get(
        render_view_irradiance_volumes: Option<&RenderViewLightProbes<IrradianceVolume>>,
        images: &'a RenderAssets<Image>,
        fallback_image: &'a FallbackImage,
        render_device: &RenderDevice,
    ) -> RenderViewIrradianceVolumeBindGroupEntries<'a> {
        if binding_arrays_are_usable(render_device) {
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
                    )
                }
            }

            // Pad out the bindings to the size of the binding array using fallback
            // textures. This is necessary on D3D12 and Metal.
            texture_views.resize(MAX_VIEW_LIGHT_PROBES, &*fallback_image.d3.texture_view);

            return RenderViewIrradianceVolumeBindGroupEntries::Multiple {
                texture_views,
                sampler: sampler.unwrap_or(&fallback_image.d3.sampler),
            };
        }

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

    type ViewLightProbeInfo = ();

    fn id(&self, image_assets: &RenderAssets<Image>) -> Option<Self::AssetId> {
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
        _: &RenderAssets<Image>,
    ) -> RenderViewLightProbes<Self> {
        RenderViewLightProbes::new()
    }
}
