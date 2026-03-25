//! Provides types to specify atmosphere lighting, scattering terms, etc.

use alloc::{borrow::Cow, sync::Arc};
use bevy_asset::{Asset, AssetEvent, AssetId, Handle};
use bevy_camera::Hdr;
use bevy_color::{ColorToComponents, Gray, LinearRgba};
use bevy_ecs::{
    component::Component,
    message::MessageReader,
    system::{Res, ResMut},
};
use bevy_image::Image;
use bevy_math::curve::{FunctionCurve, Interval, SampleAutoCurve};
use bevy_math::{ops, Curve, FloatPow, Vec3};
use bevy_platform::collections::HashSet;
use bevy_reflect::TypePath;
use core::f32::{self, consts::PI};
use smallvec::SmallVec;
use wgpu_types::TextureFormat;

/// Enables atmospheric scattering for an HDR camera.
#[derive(Clone, Component)]
#[require(Hdr)]
pub struct Atmosphere {
    /// Radius of the planet
    ///
    /// units: m
    pub bottom_radius: f32,

    /// Radius at which we consider the atmosphere to 'end' for our
    /// calculations (from center of planet)
    ///
    /// units: m
    pub top_radius: f32,

    /// An approximation of the average albedo (or color, roughly) of the
    /// planet's surface. This is used when calculating multiscattering.
    ///
    /// units: N/A
    pub ground_albedo: Vec3,

    /// A handle to a [`ScatteringMedium`], which describes the substance
    /// of the atmosphere and how it scatters light.
    pub medium: Handle<ScatteringMedium>,
}

impl Atmosphere {
    /// An atmosphere like that of earth. Use this with a [`ScatteringMedium::earth`] handle.
    pub fn earth(medium: Handle<ScatteringMedium>) -> Self {
        const EARTH_BOTTOM_RADIUS: f32 = 6_360_000.0;
        const EARTH_TOP_RADIUS: f32 = 6_460_000.0;
        const EARTH_ALBEDO: Vec3 = Vec3::splat(0.3);
        Self {
            bottom_radius: EARTH_BOTTOM_RADIUS,
            top_radius: EARTH_TOP_RADIUS,
            ground_albedo: EARTH_ALBEDO,
            medium,
        }
    }

    /// Martian atmosphere; use this with a [`ScatteringMedium::mars`] handle.
    ///
    /// Mean radius 3389.50 ± 0.2 km [Seidelmann et al. 2007, Table 4].
    ///
    /// [Seidelmann et al. 2007, Table 4]: https://doi.org/10.1007/s10569-007-9072-y
    pub fn mars(medium: Handle<ScatteringMedium>) -> Self {
        const MARS_BOTTOM_RADIUS: f32 = 3_389_500.0;
        const MARS_TOP_RADIUS: f32 = 3_509_500.0;
        const MARS_ALBEDO: Vec3 = Vec3::splat(0.1);
        Self {
            bottom_radius: MARS_BOTTOM_RADIUS,
            top_radius: MARS_TOP_RADIUS,
            ground_albedo: MARS_ALBEDO,
            medium,
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
/// `bevy_pbr::GpuScatteringMedium` for more info.
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
        ScatteringMedium::earth(256, 256)
    }
}

impl ScatteringMedium {
    /// Returns a scattering medium with a default label and the
    /// specified scattering terms.
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

    /// Consumes and returns this scattering medium with a new label.
    pub fn with_label(self, label: impl Into<Cow<'static, str>>) -> Self {
        Self {
            label: Some(label.into()),
            ..self
        }
    }

    /// Consumes and returns this scattering medium with each scattering terms'
    /// densities multiplied by `multiplier`.
    pub fn with_density_multiplier(mut self, multiplier: f32) -> Self {
        self.terms.iter_mut().for_each(|term| {
            term.absorption *= multiplier;
            term.scattering *= multiplier;
        });

        self
    }

    /// Returns a scattering medium representing an earth atmosphere.
    ///
    /// Uses physically-based scale heights from Earth's atmosphere, assuming
    /// a 60 km atmosphere height:
    /// - Rayleigh (molecular) scattering: 8 km scale height
    /// - Mie (aerosol) scattering: 1.2 km scale height
    pub fn earth(falloff_resolution: u32, phase_resolution: u32) -> Self {
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
        .with_label("earth_atmosphere")
    }

    /// Returns a scattering medium representing a Martian atmosphere [Schneegans et al. 2024].
    ///
    /// Constituents:
    /// - Rayleigh: carbon dioxide
    /// - Dust: wavelength-dependent Mie phase, double-exponential density
    ///
    /// Requires an Nx1 chromatic phase texture for the dust term.
    ///
    /// [Schneegans et al. 2024]: https://doi.org/10.1111/cgf.15010
    pub fn mars(falloff_resolution: u32, phase_resolution: u32, dust_phase: Handle<Image>) -> Self {
        const MARS_ATMOSPHERE_HEIGHT: f32 = 120_000.0;
        const RAYLEIGH_SCALE_HEIGHT: f32 = 8_000.0;

        // Dust density, from Fig. 8.
        let dust_falloff = Falloff::from_curve(FunctionCurve::new(Interval::UNIT, |p| {
            let h = (1.0 - p) * MARS_ATMOSPHERE_HEIGHT;
            0.75 * ops::exp(1.0 - ops::exp(h / 4_000.0))
                + 0.25 * ops::exp(1.0 - ops::exp(h / 20_000.0))
        }));

        Self::new(
            falloff_resolution,
            phase_resolution,
            [
                ScatteringTerm {
                    // Table 1: Eq. 3 with delta=0.09, refractive index=1.00000337
                    absorption: Vec3::ZERO,
                    scattering: Vec3::new(9.91e-8, 2.32e-7, 5.65e-7),
                    falloff: Falloff::Exponential {
                        scale: RAYLEIGH_SCALE_HEIGHT / MARS_ATMOSPHERE_HEIGHT,
                    },
                    phase: PhaseFunction::Rayleigh,
                },
                ScatteringTerm {
                    // Table 1: number density=5×10^9 m^-3, Mie Theory
                    absorption: Vec3::new(1.26e-6, 5.25e-6, 9.33e-6), // beta_abs per channel
                    scattering: Vec3::new(30.67e-6, 25.39e-6, 20.93e-6), // beta_sca per channel
                    falloff: dust_falloff,
                    phase: PhaseFunction::from_chromatic_texture(dust_phase),
                },
            ],
        )
        .with_label("mars_atmosphere")
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

    /// Evaluates the falloff function at the given coordinate.
    pub fn sample(&self, p: f32) -> f32 {
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

    /// A wavelength-dependent (chromatic) phase function returning linear RGB
    /// phase values per channel. Used when the phase varies with wavelength,
    /// for instance Mie scattering on Martian dust.
    ///
    /// Energy conservation applies per channel. Each of the channels must independently
    /// satisfy the equation above.
    ///
    /// domain: [-1, 1]
    /// range: [0, 1] per channel (R, G, B)
    ChromaticCurve(Arc<dyn Curve<LinearRgba> + Send + Sync>),

    /// A chromatic phase function sampled from an N×1 texture (R,G,B per column).
    ///
    /// Use `Rgba32Float` format. Columns map linearly to cos θ. The LUT spans the
    /// scattering hemisphere: first column is back-scattering (θ = π), last is
    /// forward-scattering (θ = 0).
    /// Resolved to [`PhaseFunction::ChromaticCurve`] when the image loads.
    ///
    /// To generate your own, compute the phase function using Mie theory (for example,
    /// using the `miepython` package) and write it as a 32-bit float texture (`OpenEXR` or `KTX2`).
    ChromaticTexture(Handle<Image>),
}

impl PhaseFunction {
    /// A phase function defined by a custom curve.
    pub fn from_curve(curve: impl Curve<f32> + Send + Sync + 'static) -> Self {
        Self::Curve(Arc::new(curve))
    }

    /// A wavelength-dependent phase function from a curve that returns linear RGBA.
    pub fn from_chromatic_curve(curve: impl Curve<LinearRgba> + Send + Sync + 'static) -> Self {
        Self::ChromaticCurve(Arc::new(curve))
    }

    /// A chromatic phase function from an N×1 texture. Resolved to a curve when loaded.
    pub fn from_chromatic_texture(image: Handle<Image>) -> Self {
        Self::ChromaticTexture(image)
    }

    /// Samples the phase function at the given value in [-1, 1].
    ///
    /// Returns `Some(LinearRgba)` with per-channel phase values (scalar phases use R=G=B).
    /// Returns `None` when the phase is not yet available (e.g. [`PhaseFunction::ChromaticTexture`] before load).
    pub fn sample(&self, neg_l_dot_v: f32) -> Option<LinearRgba> {
        const FRAC_4_PI: f32 = 0.25 / PI;
        const FRAC_3_16_PI: f32 = 0.1875 / PI;
        match self {
            PhaseFunction::Isotropic => Some(LinearRgba::gray(FRAC_4_PI)),
            PhaseFunction::Rayleigh => Some(LinearRgba::gray(
                FRAC_3_16_PI * (1.0 + neg_l_dot_v * neg_l_dot_v),
            )),
            PhaseFunction::Mie { asymmetry } => {
                let denom = 1.0 + asymmetry.squared() - 2.0 * asymmetry * neg_l_dot_v;
                Some(LinearRgba::from_vec3(Vec3::splat(
                    FRAC_4_PI * (1.0 - asymmetry.squared()) / (denom * denom.sqrt()),
                )))
            }
            PhaseFunction::Curve(curve) => curve
                .sample(neg_l_dot_v)
                .map(LinearRgba::gray)
                .or(Some(LinearRgba::gray(0.0))),
            PhaseFunction::ChromaticCurve(curve) => {
                curve.sample(neg_l_dot_v).or(Some(LinearRgba::gray(0.0)))
            }
            PhaseFunction::ChromaticTexture(_) => None,
        }
    }
}

impl Default for PhaseFunction {
    fn default() -> Self {
        Self::Mie { asymmetry: 0.8 }
    }
}

/// Resolves [`PhaseFunction::ChromaticTexture`] to [`PhaseFunction::ChromaticCurve`] when the image loads.
pub fn extract_chromatic_phase_textures(
    mut reader: MessageReader<AssetEvent<Image>>,
    images: Res<bevy_asset::Assets<Image>>,
    mut scattering_media: ResMut<bevy_asset::Assets<ScatteringMedium>>,
) {
    let extract_ids: HashSet<AssetId<Image>> = scattering_media
        .iter()
        .flat_map(|(_, m)| m.terms.iter())
        .filter_map(|t| {
            let PhaseFunction::ChromaticTexture(h) = &t.phase else {
                return None;
            };
            Some(h.id())
        })
        .collect();

    for event in reader.read() {
        let AssetEvent::LoadedWithDependencies { id } = event else {
            continue;
        };
        if !extract_ids.contains(id) {
            continue;
        }

        let Some(image) = images.get(*id) else {
            continue;
        };
        if image.texture_descriptor.format != TextureFormat::Rgba32Float {
            continue;
        }

        let width = image.texture_descriptor.size.width;
        if width == 0 {
            continue;
        }

        let Some(samples): Option<Vec<LinearRgba>> = (0..width)
            .map(|x| image.get_color_at_1d(x).ok().map(|c| c.to_linear()))
            .collect()
        else {
            continue;
        };

        let Ok(curve) = SampleAutoCurve::new(
            Interval::new(-1.0, 1.0).expect("[-1, 1] valid for cos θ"),
            samples,
        ) else {
            continue;
        };

        let new_phase = PhaseFunction::from_chromatic_curve(curve);

        for (_id, medium) in scattering_media.iter_mut() {
            for term in medium.terms.iter_mut() {
                if let PhaseFunction::ChromaticTexture(handle) = &term.phase
                    && handle.id() == *id
                {
                    term.phase = new_phase.clone();
                }
            }
        }
    }
}
