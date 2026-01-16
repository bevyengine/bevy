use alloc::{borrow::Cow, sync::Arc};
use core::f32::{self, consts::PI};

use bevy_app::{App, Plugin};
use bevy_asset::{Asset, AssetApp, AssetId};
use bevy_ecs::{
    resource::Resource,
    system::{Commands, Res, SystemParamItem},
};
use bevy_math::{ops, Curve, FloatPow, Vec3, Vec4};
use bevy_reflect::TypePath;
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
        app.init_asset::<ScatteringMedium>()
            .add_plugins(RenderAssetPlugin::<GpuScatteringMedium>::default());

        if let Some(render_app) = app.get_sub_app_mut(RenderApp) {
            render_app.add_systems(RenderStartup, init_scattering_medium_sampler);
        }
    }
}

/// An asset that defines how a material scatters light.
///
/// In order to calculate how light passes through a medium,
/// you need three pieces of information:
/// - how much light the medium *absorbs* per unit length
/// - how much light the medium *scatters* per unit length
/// - what *directions* the medium is likely to scatter light in.
///
/// The first two are fairly simple, and are sometimes referred to together
/// (accurately enough) as the medium's [optical density].
///
/// The last, defined by a [phase function], is the most important in creating
/// the look of a medium. Our brains are very good at noticing (if unconsciously)
/// that a dust storm scatters light differently than a rain cloud, for example.
/// See the docs on [`PhaseFunction`] for more info.
///
/// In reality, media are often composed of multiple elements that scatter light
/// independently, for Earth's atmosphere is composed of the gas itself, but also
/// suspended dust and particulate. These each scatter light differently, and are
/// distributed in different amounts at different altitudes. In a [`ScatteringMedium`],
/// these are each represented by a [`ScatteringTerm`]
///
/// ## Technical Details
///
/// A [`ScatteringMedium`] is represented on the GPU by a set of two LUTs, which
/// are re-created every time the asset is modified. See the docs on
/// [`GpuScatteringMedium`] for more info.
///
/// [optical density]: https://en.wikipedia.org/wiki/Optical_Density
/// [phase function]: https://www.pbr-book.org/4ed/Volume_Scattering/Phase_Functions
#[derive(TypePath, Asset, Clone)]
pub struct ScatteringMedium {
    /// An optional label for the medium, used when creating the LUTs on the GPU.
    pub label: Option<Cow<'static, str>>,
    /// The resolution at which to sample the falloff distribution of each
    /// scattering term. Custom or more detailed distributions may benefit
    /// from a higher value, at the cost of more memory use.
    pub falloff_resolution: u32,
    /// The resolution at which to sample the phase function of each scattering
    /// term. Custom or more detailed phase functions may benefit from a higher
    /// value, at the cost of more memory use.
    pub phase_resolution: u32,
    /// The list of [`ScatteringTerm`]s that compose this [`ScatteringMedium`]
    pub terms: SmallVec<[ScatteringTerm; 1]>,
}

impl Default for ScatteringMedium {
    fn default() -> Self {
        ScatteringMedium::earthlike(256, 256)
    }
}

impl ScatteringMedium {
    // Returns a scattering medium with a default label and the
    // specified scattering terms.
    pub fn new(
        falloff_resolution: u32,
        phase_resolution: u32,
        terms: impl IntoIterator<Item = ScatteringTerm>,
    ) -> Self {
        Self {
            label: None,
            falloff_resolution,
            phase_resolution,
            terms: terms.into_iter().collect(),
        }
    }

    // Consumes and returns this scattering medium with a new label.
    pub fn with_label(self, label: impl Into<Cow<'static, str>>) -> Self {
        Self {
            label: Some(label.into()),
            ..self
        }
    }

    // Consumes and returns this scattering medium with each scattering terms'
    // densities multiplied by `multiplier`.
    pub fn with_density_multiplier(mut self, multiplier: f32) -> Self {
        self.terms.iter_mut().for_each(|term| {
            term.absorption *= multiplier;
            term.scattering *= multiplier;
        });

        self
    }

    /// Returns a scattering medium representing an earthlike atmosphere.
    ///
    /// Uses physically-based scale heights from Earth's atmosphere, assuming
    /// a 60 km atmosphere height:
    /// - Rayleigh (molecular) scattering: 8 km scale height
    /// - Mie (aerosol) scattering: 1.2 km scale height
    pub fn earthlike(falloff_resolution: u32, phase_resolution: u32) -> Self {
        Self::new(
            falloff_resolution,
            phase_resolution,
            [
                // Rayleigh scattering Term
                ScatteringTerm {
                    absorption: Vec3::ZERO,
                    scattering: Vec3::new(5.802e-6, 13.558e-6, 33.100e-6),
                    falloff: Falloff::Exponential { scale: 8.0 / 60.0 },
                    phase: PhaseFunction::Rayleigh,
                },
                // Mie scattering Term
                ScatteringTerm {
                    absorption: Vec3::splat(3.996e-6),
                    scattering: Vec3::splat(0.444e-6),
                    falloff: Falloff::Exponential { scale: 1.2 / 60.0 },
                    phase: PhaseFunction::Mie { asymmetry: 0.8 },
                },
                // Ozone scattering Term
                ScatteringTerm {
                    absorption: Vec3::new(0.650e-6, 1.881e-6, 0.085e-6),
                    scattering: Vec3::ZERO,
                    falloff: Falloff::Tent {
                        center: 0.75,
                        width: 0.3,
                    },
                    phase: PhaseFunction::Isotropic,
                },
            ],
        )
        .with_label("earthlike_atmosphere")
    }
}

/// An individual element of a [`ScatteringMedium`].
///
/// A [`ScatteringMedium`] can be built out of a number of simpler [`ScatteringTerm`]s,
/// which correspond to an individual element of the medium. For example, Earth's
/// atmosphere would be (roughly) composed of two [`ScatteringTerm`]s: the atmospheric
/// gases themselves, which extend to the edge of space, and suspended dust particles,
/// which are denser but lie closer to the ground.
#[derive(Default, Clone)]
pub struct ScatteringTerm {
    /// This term's optical absorption density, or how much light of each wavelength
    /// it absorbs per meter.
    ///
    /// units: m^-1
    pub absorption: Vec3,
    /// This term's optical scattering density, or how much light of each wavelength
    /// it scatters per meter.
    ///
    /// units: m^-1
    pub scattering: Vec3,
    /// This term's falloff distribution. See the docs on [`Falloff`] for more info.
    pub falloff: Falloff,
    /// This term's [phase function], which determines the character of how it
    /// scatters light. See the docs on [`PhaseFunction`] for more info.
    ///
    /// [phase function]: https://www.pbr-book.org/4ed/Volume_Scattering/Phase_Functions
    pub phase: PhaseFunction,
}

/// Describes how the media in a [`ScatteringTerm`] is distributed.
///
/// This is closely related to the optical density values [`ScatteringTerm::absorption`] and
/// [`ScatteringTerm::scattering`]. Most media aren't the same density everywhere;
/// near the edge of space Earth's atmosphere is much less dense, and it absorbs
/// and scatters less light.
///
/// [`Falloff`] determines how the density of a medium changes as a function of
/// an abstract "falloff parameter" `p`. `p = 1` denotes where the medium is the
/// densest, i.e. at the surface of the Earth, `p = 0` denotes where the medium
/// fades away completely, i.e. at the edge of space, and values between scale
/// linearly with distance, so `p = 0.5` would be halfway between the surface
/// and the edge of space.
///
/// When processing a [`ScatteringMedium`], the `absorption` and `scattering` values
/// for each [`ScatteringTerm`] are multiplied by the value of the falloff function, `f(p)`.
#[derive(Default, Clone)]
pub enum Falloff {
    /// A simple linear falloff function, which essentially
    /// passes the falloff parameter through unchanged.
    ///
    /// f(1) = 1
    /// f(0) = 0
    /// f(p) = p
    #[default]
    Linear,
    /// An exponential falloff function parametrized by a proportional scale.
    /// When paired with an absolute "falloff distance" like the distance from
    /// Earth's surface to the edge of space, this is analogous to the "height
    /// scale" value common in atmospheric scattering literature, though it will
    /// diverge from this for large or negative `scale` values.
    ///
    /// f(1) = 1
    /// f(0) = 0
    /// f(p) = (e^((1-p)/s) - e^(1/s))/(e - e^(1/s))
    Exponential {
        /// The "scale" of the exponential falloff. Values closer to zero will
        /// produce steeper falloff, and values farther from zero will produce
        /// gentler falloff, approaching linear falloff as scale goes to `+-∞`.
        ///
        /// Negative values change the *concavity* of the falloff function:
        /// rather than an initial narrow region of steep falloff followed by a
        /// wide region of gentle falloff, there will be an initial wide region
        /// of gentle falloff followed by a narrow region of steep falloff.
        ///
        /// domain: (-∞, ∞)
        ///
        /// NOTE, this function is not defined when `scale == 0`.
        /// In that case, it will fall back to linear falloff.
        scale: f32,
    },
    /// A tent-shaped falloff function, which produces a triangular
    /// peak at the center and linearly falls off to either side.
    ///
    /// f(`center`) = 1
    /// f(`center` +- `width` / 2) = 0
    Tent {
        /// The center of the tent function peak
        ///
        /// domain: [0, 1]
        center: f32,
        /// The total width of the tent function peak
        ///
        /// domain: [0, 1]
        width: f32,
    },
    /// A falloff function defined by a custom curve.
    ///
    /// domain: [0, 1],
    /// range: [0, 1],
    Curve(Arc<dyn Curve<f32> + Send + Sync>),
}

impl Falloff {
    /// Returns a falloff function corresponding to a custom curve.
    pub fn from_curve(curve: impl Curve<f32> + Send + Sync + 'static) -> Self {
        Self::Curve(Arc::new(curve))
    }

    fn sample(&self, p: f32) -> f32 {
        match self {
            Falloff::Linear => p,
            Falloff::Exponential { scale } => {
                // fill discontinuity at scale == 0,
                // arbitrarily choose linear falloff
                if *scale == 0.0 {
                    p
                } else {
                    let s = -1.0 / scale;
                    let exp_p_s = ops::exp((1.0 - p) * s);
                    let exp_s = ops::exp(s);
                    (exp_p_s - exp_s) / (1.0 - exp_s)
                }
            }
            Falloff::Tent { center, width } => (1.0 - (p - center).abs() / (0.5 * width)).max(0.0),
            Falloff::Curve(curve) => curve.sample(p).unwrap_or(0.0),
        }
    }
}

/// Describes how a [`ScatteringTerm`] scatters light in different directions.
///
/// A [phase function] is a function `f: [-1, 1] -> [0, ∞)`, symmetric about `x=0`
/// whose input is the cosine of the angle between an incoming light direction and
/// and outgoing light direction, and whose output is the proportion of the incoming
/// light that is actually scattered in that direction.
///
/// The phase function has an important effect on the "look" of a medium in a scene.
/// Media consisting of particles of a different size or shape scatter light differently,
/// and our brains are very good at telling the difference. A dust cloud, which might
/// correspond roughly to `PhaseFunction::Mie { asymmetry: 0.8 }`, looks quite different
/// from the rest of the sky (atmospheric gases), which correspond to `PhaseFunction::Rayleigh`
///
/// [phase function]: https://www.pbr-book.org/4ed/Volume_Scattering/Phase_Functions
#[derive(Clone)]
pub enum PhaseFunction {
    /// A phase function that scatters light evenly in all directions.
    Isotropic,

    /// A phase function representing [Rayleigh scattering].
    ///
    /// Rayleigh scattering occurs naturally for particles much smaller than
    /// the wavelengths of visible light, such as gas molecules in the atmosphere.
    /// It's generally wavelength-dependent, where shorter wavelengths are scattered
    /// more strongly, so [scattering](ScatteringTerm::scattering) should have
    /// higher values for blue than green and green than red. Particles that
    /// participate in Rayleigh scattering don't absorb any light, either.
    ///
    /// [Rayleigh scattering]: https://en.wikipedia.org/wiki/Rayleigh_scattering
    Rayleigh,

    /// The [Henyey-Greenstein phase function], which approximates [Mie scattering].
    ///
    /// Mie scattering occurs naturally for spherical particles of dust
    /// and aerosols roughly the same size as the wavelengths of visible light,
    /// so it's useful for representing dust or sea spray. It's generally
    /// wavelength-independent, so [absorption](ScatteringTerm::absorption)
    /// and [scattering](ScatteringTerm::scattering) should be set to a greyscale value.
    ///
    /// [Mie scattering]: https://en.wikipedia.org/wiki/Mie_scattering
    /// [Henyey-Greenstein phase function]: https://www.oceanopticsbook.info/view/scattering/level-2/the-henyey-greenstein-phase-function
    Mie {
        /// Whether the Mie scattering function is biased towards scattering
        /// light forwards (asymmetry > 0) or backwards (asymmetry < 0).
        ///
        /// domain: [-1, 1]
        asymmetry: f32,
    },

    /// A phase function defined by a custom curve, where the input
    /// is the cosine of the angle between the incoming light ray
    /// and the scattered light ray, and the output is the fraction
    /// of the incoming light scattered in that direction.
    ///
    /// Note: it's important for photorealism that the phase function
    /// be *energy conserving*, meaning that in total no more light can
    /// be scattered than actually entered the medium. For this to be
    /// the case, the integral of the phase function over its domain must
    /// be equal to 1/2π.
    ///
    ///   1
    /// ∫   p(x) dx = 1/2π
    ///  -1
    ///
    /// domain: [-1, 1]
    /// range: [0, 1]
    Curve(Arc<dyn Curve<f32> + Send + Sync>),
}

impl PhaseFunction {
    /// A phase function defined by a custom curve.
    pub fn from_curve(curve: impl Curve<f32> + Send + Sync + 'static) -> Self {
        Self::Curve(Arc::new(curve))
    }

    fn sample(&self, neg_l_dot_v: f32) -> f32 {
        const FRAC_4_PI: f32 = 0.25 / PI;
        const FRAC_3_16_PI: f32 = 0.1875 / PI;
        match self {
            PhaseFunction::Isotropic => FRAC_4_PI,
            PhaseFunction::Rayleigh => FRAC_3_16_PI * (1.0 + neg_l_dot_v * neg_l_dot_v),
            PhaseFunction::Mie { asymmetry } => {
                let denom = 1.0 + asymmetry.squared() - 2.0 * asymmetry * neg_l_dot_v;
                FRAC_4_PI * (1.0 - asymmetry.squared()) / (denom * denom.sqrt())
            }
            PhaseFunction::Curve(curve) => curve.sample(neg_l_dot_v).unwrap_or(0.0),
        }
    }
}

impl Default for PhaseFunction {
    fn default() -> Self {
        Self::Mie { asymmetry: 0.8 }
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
