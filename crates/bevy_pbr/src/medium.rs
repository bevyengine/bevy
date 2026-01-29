use bevy_light::atmosphere::{ScatteringMedium, ScatteringTerm};

use bevy_app::{App, Plugin};
use bevy_asset::AssetId;
use bevy_ecs::{
    resource::Resource,
    system::{Commands, Res, SystemParamItem},
};
use bevy_math::Vec4;
use bevy_render::{
    render_asset::{PrepareAssetError, RenderAsset, RenderAssetPlugin},
    render_resource::{
        Extent3d, FilterMode, Sampler, SamplerDescriptor, Texture, TextureDataOrder,
        TextureDescriptor, TextureDimension, TextureFormat, TextureUsages, TextureView,
        TextureViewDescriptor,
    },
    renderer::{RenderDevice, RenderQueue},
    RenderApp, RenderStartup,
};
use smallvec::SmallVec;

#[doc(hidden)]
pub struct ScatteringMediumPlugin;

impl Plugin for ScatteringMediumPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(RenderAssetPlugin::<GpuScatteringMedium>::default());

        if let Some(render_app) = app.get_sub_app_mut(RenderApp) {
            render_app.add_systems(RenderStartup, init_scattering_medium_sampler);
        }
    }
}

/// The GPU representation of a [`ScatteringMedium`].
pub struct GpuScatteringMedium {
    /// The terms of the scattering medium.
    pub terms: SmallVec<[ScatteringTerm; 1]>,
    /// The resolution at which to sample the falloff distribution of each
    /// scattering term.
    pub falloff_resolution: u32,
    /// The resolution at which to sample the phase function of each
    /// scattering term.
    pub phase_resolution: u32,
    /// The `density_lut`, a 2D `falloff_resolution x 2` LUT which contains the
    /// medium's optical density with respect to the atmosphere's "falloff parameter",
    /// a linear value which is 1.0 at the planet's surface and 0.0 at the edge of
    /// space. The first and second rows correspond to absorption density and
    /// scattering density respectively.
    pub density_lut: Texture,
    /// The default [`TextureView`] of the `density_lut`
    pub density_lut_view: TextureView,
    /// The `scattering_lut`, a 2D `falloff_resolution x phase_resolution` LUT which
    /// contains the medium's scattering density multiplied by the phase function, with
    /// the U axis corresponding to the falloff parameter and the V axis corresponding
    /// to `neg_LdotV * 0.5 + 0.5`, where `neg_LdotV` is the dot product of the light
    /// direction and the incoming view vector.
    pub scattering_lut: Texture,
    /// The default [`TextureView`] of the `scattering_lut`
    pub scattering_lut_view: TextureView,
}

impl RenderAsset for GpuScatteringMedium {
    type SourceAsset = ScatteringMedium;

    type Param = (Res<'static, RenderDevice>, Res<'static, RenderQueue>);

    fn prepare_asset(
        source_asset: Self::SourceAsset,
        _asset_id: AssetId<Self::SourceAsset>,
        (render_device, render_queue): &mut SystemParamItem<Self::Param>,
        _previous_asset: Option<&Self>,
    ) -> Result<Self, PrepareAssetError<Self::SourceAsset>> {
        let mut density: Vec<Vec4> =
            Vec::with_capacity(2 * source_asset.falloff_resolution as usize);

        density.extend((0..source_asset.falloff_resolution).map(|i| {
            let falloff = (i as f32 + 0.5) / source_asset.falloff_resolution as f32;

            source_asset
                .terms
                .iter()
                .map(|term| term.absorption.extend(0.0) * term.falloff.sample(falloff))
                .sum::<Vec4>()
        }));

        density.extend((0..source_asset.falloff_resolution).map(|i| {
            let falloff = (i as f32 + 0.5) / source_asset.falloff_resolution as f32;

            source_asset
                .terms
                .iter()
                .map(|term| term.scattering.extend(0.0) * term.falloff.sample(falloff))
                .sum::<Vec4>()
        }));

        let mut scattering: Vec<Vec4> = Vec::with_capacity(
            source_asset.falloff_resolution as usize * source_asset.phase_resolution as usize,
        );

        scattering.extend(
            (0..source_asset.falloff_resolution * source_asset.phase_resolution).map(|raw_i| {
                let i = raw_i % source_asset.phase_resolution;
                let j = raw_i / source_asset.phase_resolution;
                let falloff = (i as f32 + 0.5) / source_asset.falloff_resolution as f32;
                let phase = (j as f32 + 0.5) / source_asset.phase_resolution as f32;
                let neg_l_dot_v = phase * 2.0 - 1.0;

                source_asset
                    .terms
                    .iter()
                    .map(|term| {
                        term.scattering.extend(0.0)
                            * term.falloff.sample(falloff)
                            * term.phase.sample(neg_l_dot_v)
                    })
                    .sum::<Vec4>()
            }),
        );

        let density_lut = render_device.create_texture_with_data(
            render_queue,
            &TextureDescriptor {
                label: source_asset
                    .label
                    .as_deref()
                    .map(|label| format!("{}_density_lut", label))
                    .as_deref()
                    .or(Some("scattering_medium_density_lut")),
                size: Extent3d {
                    width: source_asset.falloff_resolution,
                    height: 2,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: TextureDimension::D2,
                format: TextureFormat::Rgba32Float,
                usage: TextureUsages::TEXTURE_BINDING,
                view_formats: &[],
            },
            TextureDataOrder::LayerMajor,
            bytemuck::cast_slice(density.as_slice()),
        );

        let density_lut_view = density_lut.create_view(&TextureViewDescriptor {
            label: source_asset
                .label
                .as_deref()
                .map(|label| format!("{}_density_lut_view", label))
                .as_deref()
                .or(Some("scattering_medium_density_lut_view")),
            ..Default::default()
        });

        let scattering_lut = render_device.create_texture_with_data(
            render_queue,
            &TextureDescriptor {
                label: source_asset
                    .label
                    .as_deref()
                    .map(|label| format!("{}_scattering_lut", label))
                    .as_deref()
                    .or(Some("scattering_medium_scattering_lut")),
                size: Extent3d {
                    width: source_asset.falloff_resolution,
                    height: source_asset.phase_resolution,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: TextureDimension::D2,
                format: TextureFormat::Rgba32Float,
                usage: TextureUsages::TEXTURE_BINDING,
                view_formats: &[],
            },
            TextureDataOrder::LayerMajor,
            bytemuck::cast_slice(scattering.as_slice()),
        );

        let scattering_lut_view = scattering_lut.create_view(&TextureViewDescriptor {
            label: source_asset
                .label
                .as_deref()
                .map(|label| format!("{}_scattering_lut", label))
                .as_deref()
                .or(Some("scattering_medium_scattering_lut_view")),
            ..Default::default()
        });

        Ok(Self {
            terms: source_asset.terms,
            falloff_resolution: source_asset.falloff_resolution,
            phase_resolution: source_asset.phase_resolution,
            density_lut,
            density_lut_view,
            scattering_lut,
            scattering_lut_view,
        })
    }
}

/// The default sampler for all scattering media LUTs.
///
/// Just a bilinear clamp-to-edge sampler, nothing fancy.
#[derive(Resource)]
pub struct ScatteringMediumSampler(Sampler);

impl ScatteringMediumSampler {
    pub fn sampler(&self) -> &Sampler {
        &self.0
    }
}

fn init_scattering_medium_sampler(mut commands: Commands, render_device: Res<RenderDevice>) {
    let sampler = render_device.create_sampler(&SamplerDescriptor {
        label: Some("scattering_medium_sampler"),
        mag_filter: FilterMode::Linear,
        min_filter: FilterMode::Linear,
        ..Default::default()
    });

    commands.insert_resource(ScatteringMediumSampler(sampler));
}
