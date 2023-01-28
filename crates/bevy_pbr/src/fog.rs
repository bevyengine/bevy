use crate::ReflectComponent;
use bevy_ecs::{prelude::*, query::QueryItem};
use bevy_math::Vec3;
use bevy_reflect::Reflect;
use bevy_render::{color::Color, extract_component::ExtractComponent, prelude::Camera};

/// Configures the “classic” computer graphics [distance fog](https://en.wikipedia.org/wiki/Distance_fog) effect,
/// in which objects appear progressively more covered in atmospheric haze the further away they are from the camera.
/// Affects meshes rendered via the PBR [`StandardMaterial`](crate::StandardMaterial).
///
/// ## Falloff
///
/// The rate at which fog intensity increases with distance is controlled by the falloff mode.
/// Currently, the following fog falloff modes are supported:
///
/// - [`FogFalloff::Linear`]
/// - [`FogFalloff::Exponential`]
/// - [`FogFalloff::ExponentialSquared`]
/// - [`FogFalloff::Atmospheric`]
///
/// ## Example
///
/// ```
/// # use bevy_ecs::prelude::*;
/// # use bevy_render::prelude::*;
/// # use bevy_core_pipeline::prelude::*;
/// # use bevy_pbr::prelude::*;
/// # fn system(mut commands: Commands) {
/// commands.spawn((
///     // Setup your camera as usual
///     Camera3dBundle {
///         // ... camera options
/// #       ..Default::default()
///     },
///     // Add fog to the same entity
///     FogSettings {
///         color: Color::WHITE,
///         falloff: FogFalloff::Exponential { density: 1e-3 },
///         ..Default::default()
///     },
/// ));
/// # }
/// # bevy_ecs::system::assert_is_system(system);
/// ```
///
/// ## Material Override
///
/// Once enabled for a specific camera, the fog effect can also be disabled for individual
/// [`StandardMaterial`](crate::StandardMaterial) instances via the `fog_enabled` flag.
#[derive(Debug, Clone, Component, Reflect)]
#[reflect(Component)]
pub struct FogSettings {
    /// The color of the fog effect.
    ///
    /// **Tip:** The alpha channel of the color can be used to “modulate” the fog effect without
    /// changing the fog falloff mode or parameters.
    pub color: Color,

    /// Color used to modulate the influence of directional light colors on the
    /// fog, where the view direction aligns with each directional light direction,
    /// producing a “glow” or light dispersion effect. (e.g. around the sun)
    ///
    /// Use [`Color::NONE`] to disable the effect.
    pub directional_light_color: Color,

    /// The exponent applied to the directional light alignment calculation.
    /// A higher value means a more concentrated “glow”.
    pub directional_light_exponent: f32,

    /// Determines which falloff mode to use, and its parameters.
    pub falloff: FogFalloff,
}

/// Allows switching between different fog falloff modes, and configuring their parameters.
///
/// ## Convenience Methods
///
/// When using non-linear fog modes it can be hard to determine the right parameter values
/// for a given scene.
///
/// For easier artistic control, instead of creating the enum variants directly, you can use the
/// visibility-based convenience methods:
///
/// - For `FogFalloff::Exponential`:
///     - [`FogFalloff::from_visibility()`]
///     - [`FogFalloff::from_visibility_contrast()`]
///
/// - For `FogFalloff::ExponentialSquared`:
///     - [`FogFalloff::from_visibility_squared()`]
///     - [`FogFalloff::from_visibility_contrast_squared()`]
///
/// - For `FogFalloff::Atmospheric`:
///     - [`FogFalloff::from_visibility_color()`]
///     - [`FogFalloff::from_visibility_colors()`]
///     - [`FogFalloff::from_visibility_contrast_color()`]
///     - [`FogFalloff::from_visibility_contrast_colors()`]
#[derive(Debug, Clone, Reflect)]
pub enum FogFalloff {
    /// A linear fog falloff that grows in intensity between `start` and `end` distances.
    ///
    /// This falloff mode is simpler to control than other modes, however it can produce results that look “artificial”, depending on the scene.
    ///
    /// ## Formula
    ///
    /// The fog intensity for a given point in the scene is determined by the following formula:
    ///
    /// ```text
    /// let fog_intensity = 1.0 - ((end - distance) / (end - start)).clamp(0.0, 1.0);
    /// ```
    ///
    /// <svg width="370" height="212" viewBox="0 0 370 212" fill="none">
    /// <title>Plot showing how linear fog falloff behaves for start and end values of 0.8 and 2.2, respectively.</title>
    /// <path d="M331 151H42V49" stroke="currentColor" stroke-width="2"/>
    /// <text font-family="sans-serif" fill="currentColor" style="white-space: pre" font-family="Inter" font-size="12" letter-spacing="0em"><tspan x="136" y="173.864">1</tspan></text>
    /// <text font-family="sans-serif" fill="currentColor" style="white-space: pre" font-family="Inter" font-size="12" letter-spacing="0em"><tspan x="30" y="53.8636">1</tspan></text>
    /// <text font-family="sans-serif" fill="currentColor" style="white-space: pre" font-family="Inter" font-size="12" letter-spacing="0em"><tspan x="42" y="173.864">0</tspan></text>
    /// <text font-family="sans-serif" fill="currentColor" style="white-space: pre" font-family="Inter" font-size="12" letter-spacing="0em"><tspan x="232" y="173.864">2</tspan></text>
    /// <text font-family="sans-serif" fill="currentColor" style="white-space: pre" font-family="Inter" font-size="12" letter-spacing="0em"><tspan x="332" y="173.864">3</tspan></text>
    /// <text font-family="sans-serif" fill="currentColor" style="white-space: pre" font-family="Inter" font-size="12" letter-spacing="0em"><tspan x="161" y="190.864">distance</tspan></text>
    /// <text font-family="sans-serif" transform="translate(10 132) rotate(-90)" fill="currentColor" style="white-space: pre" font-family="Inter" font-size="12" letter-spacing="0em"><tspan x="0" y="11.8636">fog intensity</tspan></text>
    /// <path d="M43 150H117.227L263 48H331" stroke="#FF00E5"/>
    /// <path d="M118 151V49" stroke="#FF00E5" stroke-dasharray="1 4"/>
    /// <path d="M263 151V49" stroke="#FF00E5" stroke-dasharray="1 4"/>
    /// <text font-family="sans-serif" fill="#FF00E5" style="white-space: pre" font-family="Inter" font-size="10" letter-spacing="0em"><tspan x="121" y="58.6364">start</tspan></text>
    /// <text font-family="sans-serif" fill="#FF00E5" style="white-space: pre" font-family="Inter" font-size="10" letter-spacing="0em"><tspan x="267" y="58.6364">end</tspan></text>
    /// </svg>
    Linear {
        /// Distance from the camera where fog is completely transparent, in world units.
        start: f32,

        /// Distance from the camera where fog is completely opaque, in world units.
        end: f32,
    },

    /// An exponential fog falloff with a given `density`.
    ///
    /// Initially gains intensity quickly with distance, then more slowly. Typically produces more natural results than [`FogFalloff::Linear`],
    /// but is a bit harder to control.
    ///
    /// To move the fog “further away”, use lower density values. To move it “closer” use higher density values.
    ///
    /// ## Tips
    ///
    /// - Use the [`FogFalloff::from_visibility()`] convenience method to create an exponential falloff with the proper
    /// density for a desired visibility distance in world units;
    /// - It's not _unusual_ to have very large or very small values for the density, depending on the scene
    /// scale. Typically, for scenes with objects in the scale of thousands of units, you might want density values
    /// in the ballpark of `0.001`. Conversely, for really small scale scenes you might want really high values of
    /// density;
    /// - Combine the `density` parameter with the [`FogSettings`] `color`'s alpha channel for easier artistic control.
    ///
    /// ## Formula
    ///
    /// The fog intensity for a given point in the scene is determined by the following formula:
    ///
    /// ```text
    /// let fog_intensity = 1.0 - 1.0 / (distance * density).exp();
    /// ```
    ///
    /// <svg width="370" height="212" viewBox="0 0 370 212" fill="none">
    /// <title>Plot showing how exponential fog falloff behaves for different density values</title>
    /// <mask id="mask0_3_31" style="mask-type:alpha" maskUnits="userSpaceOnUse" x="42" y="42" width="286" height="108">
    /// <rect x="42" y="42" width="286" height="108" fill="#D9D9D9"/>
    /// </mask>
    /// <g mask="url(#mask0_3_31)">
    /// <path d="M42 150C42 150 98.3894 53 254.825 53L662 53" stroke="#FF003D" stroke-width="1"/>
    /// <path d="M42 150C42 150 139.499 53 409.981 53L1114 53" stroke="#001AFF" stroke-width="1"/>
    /// <path d="M42 150C42 150 206.348 53 662.281 53L1849 53" stroke="#14FF00" stroke-width="1"/>
    /// </g>
    /// <path d="M331 151H42V49" stroke="currentColor" stroke-width="2"/>
    /// <text font-family="sans-serif" fill="currentColor" style="white-space: pre" font-size="12" letter-spacing="0em"><tspan x="136" y="173.864">1</tspan></text>
    /// <text font-family="sans-serif" fill="currentColor" style="white-space: pre" font-size="12" letter-spacing="0em"><tspan x="30" y="53.8636">1</tspan></text>
    /// <text font-family="sans-serif" fill="currentColor" style="white-space: pre" font-size="12" letter-spacing="0em"><tspan x="42" y="173.864">0</tspan></text>
    /// <text font-family="sans-serif" fill="currentColor" style="white-space: pre" font-size="12" letter-spacing="0em"><tspan x="232" y="173.864">2</tspan></text>
    /// <text font-family="sans-serif" fill="currentColor" style="white-space: pre" font-size="12" letter-spacing="0em"><tspan x="332" y="173.864">3</tspan></text>
    /// <text font-family="sans-serif" fill="#FF003D" style="white-space: pre" font-size="10" letter-spacing="0em"><tspan x="77" y="64.6364">density = 2</tspan></text>
    /// <text font-family="sans-serif" fill="#001AFF" style="white-space: pre" font-size="10" letter-spacing="0em"><tspan x="236" y="76.6364">density = 1</tspan></text>
    /// <text font-family="sans-serif" fill="#14FF00" style="white-space: pre" font-size="10" letter-spacing="0em"><tspan x="205" y="115.636">density = 0.5</tspan></text>
    /// <text font-family="sans-serif" fill="currentColor" style="white-space: pre" font-size="12" letter-spacing="0em"><tspan x="161" y="190.864">distance</tspan></text>
    /// <text font-family="sans-serif" transform="translate(10 132) rotate(-90)" fill="currentColor" style="white-space: pre" font-size="12" letter-spacing="0em"><tspan x="0" y="11.8636">fog intensity</tspan></text>
    /// </svg>
    Exponential {
        /// Multiplier applied to the world distance (within the exponential fog falloff calculation).
        density: f32,
    },

    /// A squared exponential fog falloff with a given `density`.
    ///
    /// Similar to [`FogFalloff::Exponential`], but grows more slowly in intensity for closer distances
    /// before “catching up”.
    ///
    /// To move the fog “further away”, use lower density values. To move it “closer” use higher density values.
    ///
    /// ## Tips
    ///
    /// - Use the [`FogFalloff::from_visibility_squared()`] convenience method to create an exponential squared falloff
    /// with the proper density for a desired visibility distance in world units;
    /// - Combine the `density` parameter with the [`FogSettings`] `color`'s alpha channel for easier artistic control.
    ///
    /// ## Formula
    ///
    /// The fog intensity for a given point in the scene is determined by the following formula:
    ///
    /// ```text
    /// let fog_intensity = 1.0 - 1.0 / (distance * density).powi(2).exp();
    /// ```
    ///
    /// <svg width="370" height="212" viewBox="0 0 370 212" fill="none">
    /// <title>Plot showing how exponential squared fog falloff behaves for different density values</title>
    /// <mask id="mask0_1_3" style="mask-type:alpha" maskUnits="userSpaceOnUse" x="42" y="42" width="286" height="108">
    /// <rect x="42" y="42" width="286" height="108" fill="#D9D9D9"/>
    /// </mask>
    /// <g mask="url(#mask0_1_3)">
    /// <path d="M42 150C75.4552 150 74.9241 53.1724 166.262 53.1724L404 53.1724" stroke="#FF003D" stroke-width="1"/>
    /// <path d="M42 150C107.986 150 106.939 53.1724 287.091 53.1724L756 53.1724" stroke="#001AFF" stroke-width="1"/>
    /// <path d="M42 150C166.394 150 164.42 53.1724 504.035 53.1724L1388 53.1724" stroke="#14FF00" stroke-width="1"/>
    /// </g>
    /// <path d="M331 151H42V49" stroke="currentColor" stroke-width="2"/>
    /// <text font-family="sans-serif" fill="currentColor" style="white-space: pre" font-size="12" letter-spacing="0em"><tspan x="136" y="173.864">1</tspan></text>
    /// <text font-family="sans-serif" fill="currentColor" style="white-space: pre" font-size="12" letter-spacing="0em"><tspan x="30" y="53.8636">1</tspan></text>
    /// <text font-family="sans-serif" fill="currentColor" style="white-space: pre" font-size="12" letter-spacing="0em"><tspan x="42" y="173.864">0</tspan></text>
    /// <text font-family="sans-serif" fill="currentColor" style="white-space: pre" font-size="12" letter-spacing="0em"><tspan x="232" y="173.864">2</tspan></text>
    /// <text font-family="sans-serif" fill="currentColor" style="white-space: pre" font-size="12" letter-spacing="0em"><tspan x="332" y="173.864">3</tspan></text>
    /// <text font-family="sans-serif" fill="#FF003D" style="white-space: pre" font-size="10" letter-spacing="0em"><tspan x="61" y="54.6364">density = 2</tspan></text>
    /// <text font-family="sans-serif" fill="#001AFF" style="white-space: pre" font-size="10" letter-spacing="0em"><tspan x="168" y="84.6364">density = 1</tspan></text>
    /// <text font-family="sans-serif" fill="#14FF00" style="white-space: pre" font-size="10" letter-spacing="0em"><tspan x="174" y="121.636">density = 0.5</tspan></text>
    /// <text font-family="sans-serif" fill="currentColor" style="white-space: pre" font-size="12" letter-spacing="0em"><tspan x="161" y="190.864">distance</tspan></text>
    /// <text font-family="sans-serif" transform="translate(10 132) rotate(-90)" fill="currentColor" style="white-space: pre" font-size="12" letter-spacing="0em"><tspan x="0" y="11.8636">fog intensity</tspan></text>
    /// </svg>
    ExponentialSquared {
        /// Multiplier applied to the world distance (within the exponential squared fog falloff calculation).
        density: f32,
    },

    /// A more general form of the [`FogFalloff::Exponential`] mode. The falloff formula is separated into
    /// two terms, `extinction` and `inscattering`, for a somewhat simplified atmospheric scattering model.
    /// Additionally, individual color channels can have their own density values, resulting in a total of
    /// six different configuration parameters.
    ///
    /// ## Tips
    ///
    /// - Use the [`FogFalloff::from_visibility_colors()`] or [`FogFalloff::from_visibility_color()`] convenience methods
    /// to create an atmospheric falloff with the proper densities for a desired visibility distance in world units and
    /// extinction and inscattering colors;
    /// - Combine the atmospheric fog parameters with the [`FogSettings`] `color`'s alpha channel for easier artistic control.
    ///
    /// ## Formula
    ///
    /// Unlike other modes, atmospheric falloff doesn't use a simple intensity-based blend of fog color with
    /// object color. Instead, it calculates per-channel extinction and inscattering factors, which are
    /// then used to calculate the final color.
    ///
    /// ```text
    /// let extinction_factor = 1.0 - 1.0 / (distance * extinction).exp();
    /// let inscattering_factor = 1.0 - 1.0 / (distance * inscattering).exp();
    /// let result = input_color * (1.0 - extinction_factor) + fog_color * inscattering_factor;
    /// ```
    ///
    /// ## Equivalence to [`FogFalloff::Exponential`]
    ///
    /// For a density value of `D`, the following two falloff modes will produce identical visual results:
    ///
    /// ```
    /// # use bevy_pbr::prelude::*;
    /// # use bevy_math::prelude::*;
    /// # const D: f32 = 0.5;
    /// #
    /// let exponential = FogFalloff::Exponential {
    ///     density: D,
    /// };
    ///
    /// let atmospheric = FogFalloff::Atmospheric {
    ///     extinction: Vec3::new(D, D, D),
    ///     inscattering: Vec3::new(D, D, D),
    /// };
    /// ```
    ///
    /// **Note:** While the results are identical, [`FogFalloff::Atmospheric`] is computationally more expensive.
    Atmospheric {
        /// Controls how much light is removed due to atmospheric “extinction”, i.e. loss of light due to
        /// photons being absorbed by atmospheric particles.
        ///
        /// Each component can be thought of as an independent per `R`/`G`/`B` channel `density` factor from
        /// [`FogFalloff::Exponential`]: Multiplier applied to the world distance (within the fog
        /// falloff calculation) for that specific channel.
        ///
        /// **Note:**
        /// This value is not a `Color`, since it affects the channels exponentially in a non-intuitive way.
        /// For artistic control, use the [`FogFalloff::from_visibility_colors()`] convenience method.
        extinction: Vec3,

        /// Controls how much light is added due to light scattering from the sun through the atmosphere.
        ///
        /// Each component can be thought of as an independent per `R`/`G`/`B` channel `density` factor from
        /// [`FogFalloff::Exponential`]: A multiplier applied to the world distance (within the fog
        /// falloff calculation) for that specific channel.
        ///
        /// **Note:**
        /// This value is not a `Color`, since it affects the channels exponentially in a non-intuitive way.
        /// For artistic control, use the [`FogFalloff::from_visibility_colors()`] convenience method.
        inscattering: Vec3,
    },
}

impl FogFalloff {
    /// Creates a [`FogFalloff::Exponential`] value from the given visibility distance in world units,
    /// using the revised Koschmieder contrast threshold, [`FogFalloff::REVISED_KOSCHMIEDER_CONTRAST_THRESHOLD`].
    pub fn from_visibility(visibility: f32) -> FogFalloff {
        FogFalloff::from_visibility_contrast(
            visibility,
            FogFalloff::REVISED_KOSCHMIEDER_CONTRAST_THRESHOLD,
        )
    }

    /// Creates a [`FogFalloff::Exponential`] value from the given visibility distance in world units,
    /// and a given contrast threshold in the range of `0.0` to `1.0`.
    pub fn from_visibility_contrast(visibility: f32, contrast_threshold: f32) -> FogFalloff {
        FogFalloff::Exponential {
            density: FogFalloff::koschmieder(visibility, contrast_threshold),
        }
    }

    /// Creates a [`FogFalloff::ExponentialSquared`] value from the given visibility distance in world units,
    /// using the revised Koschmieder contrast threshold, [`FogFalloff::REVISED_KOSCHMIEDER_CONTRAST_THRESHOLD`].
    pub fn from_visibility_squared(visibility: f32) -> FogFalloff {
        FogFalloff::from_visibility_contrast_squared(
            visibility,
            FogFalloff::REVISED_KOSCHMIEDER_CONTRAST_THRESHOLD,
        )
    }

    /// Creates a [`FogFalloff::ExponentialSquared`] value from the given visibility distance in world units,
    /// and a given contrast threshold in the range of `0.0` to `1.0`.
    pub fn from_visibility_contrast_squared(
        visibility: f32,
        contrast_threshold: f32,
    ) -> FogFalloff {
        FogFalloff::ExponentialSquared {
            density: (FogFalloff::koschmieder(visibility, contrast_threshold) / visibility).sqrt(),
        }
    }

    /// Creates a [`FogFalloff::Atmospheric`] value from the given visibility distance in world units,
    /// and a shared color for both extinction and inscattering, using the revised Koschmieder contrast threshold,
    /// [`FogFalloff::REVISED_KOSCHMIEDER_CONTRAST_THRESHOLD`].
    pub fn from_visibility_color(
        visibility: f32,
        extinction_inscattering_color: Color,
    ) -> FogFalloff {
        FogFalloff::from_visibility_contrast_colors(
            visibility,
            FogFalloff::REVISED_KOSCHMIEDER_CONTRAST_THRESHOLD,
            extinction_inscattering_color,
            extinction_inscattering_color,
        )
    }

    /// Creates a [`FogFalloff::Atmospheric`] value from the given visibility distance in world units,
    /// extinction and inscattering colors, using the revised Koschmieder contrast threshold,
    /// [`FogFalloff::REVISED_KOSCHMIEDER_CONTRAST_THRESHOLD`].
    ///
    /// ## Tips
    /// - Alpha values of the provided colors can modulate the `extinction` and `inscattering` effects;
    /// - Using an `extinction_color` of [`Color::WHITE`] or [`Color::NONE`] disables the extinction effect;
    /// - Using an `inscattering_color` of [`Color::BLACK`] or [`Color::NONE`] disables the inscattering effect.
    pub fn from_visibility_colors(
        visibility: f32,
        extinction_color: Color,
        inscattering_color: Color,
    ) -> FogFalloff {
        FogFalloff::from_visibility_contrast_colors(
            visibility,
            FogFalloff::REVISED_KOSCHMIEDER_CONTRAST_THRESHOLD,
            extinction_color,
            inscattering_color,
        )
    }

    /// Creates a [`FogFalloff::Atmospheric`] value from the given visibility distance in world units,
    /// a contrast threshold in the range of `0.0` to `1.0`, and a shared color for both extinction and inscattering.
    pub fn from_visibility_contrast_color(
        visibility: f32,
        contrast_threshold: f32,
        extinction_inscattering_color: Color,
    ) -> FogFalloff {
        FogFalloff::from_visibility_contrast_colors(
            visibility,
            contrast_threshold,
            extinction_inscattering_color,
            extinction_inscattering_color,
        )
    }

    /// Creates a [`FogFalloff::Atmospheric`] value from the given visibility distance in world units,
    /// a contrast threshold in the range of `0.0` to `1.0`, extinction and inscattering colors.
    ///
    /// ## Tips
    /// - Alpha values of the provided colors can modulate the `extinction` and `inscattering` effects;
    /// - Using an `extinction_color` of [`Color::WHITE`] or [`Color::NONE`] disables the extinction effect;
    /// - Using an `inscattering_color` of [`Color::BLACK`] or [`Color::NONE`] disables the inscattering effect.
    pub fn from_visibility_contrast_colors(
        visibility: f32,
        contrast_threshold: f32,
        extinction_color: Color,
        inscattering_color: Color,
    ) -> FogFalloff {
        use std::f32::consts::E;

        let [r_e, g_e, b_e, a_e] = extinction_color.as_linear_rgba_f32();
        let [r_i, g_i, b_i, a_i] = inscattering_color.as_linear_rgba_f32();

        FogFalloff::Atmospheric {
            extinction: Vec3::new(
                // Values are subtracted from 1.0 here to preserve the intuitive/artistic meaning of
                // colors, since they're later subtracted. (e.g. by giving a blue extinction color, you
                // get blue and _not_ yellow results)
                (1.0 - r_e).powf(E),
                (1.0 - g_e).powf(E),
                (1.0 - b_e).powf(E),
            ) * FogFalloff::koschmieder(visibility, contrast_threshold)
                * a_e.powf(E),

            inscattering: Vec3::new(r_i.powf(E), g_i.powf(E), b_i.powf(E))
                * FogFalloff::koschmieder(visibility, contrast_threshold)
                * a_i.powf(E),
        }
    }

    /// A 2% contrast threshold was originally proposed by Koschmieder, being the
    /// minimum visual contrast at which a human observer could detect an object.
    /// We use a revised 5% contrast threshold, deemed more realistic for typical human observers.
    pub const REVISED_KOSCHMIEDER_CONTRAST_THRESHOLD: f32 = 0.05;

    /// Calculates the extinction coefficient β, from V and Cₜ, where:
    ///
    /// - Cₜ is the contrast threshold, in the range of `0.0` to `1.0`
    /// - V is the visibility distance in which a perfectly black object is still identifiable
    ///   against the horizon sky within the contrast threshold
    ///
    /// We start with Koschmieder's equation:
    ///
    /// ```text
    ///       -ln(Cₜ)
    ///  V = ─────────
    ///          β
    /// ```
    ///
    /// Multiplying both sides by β/V, that gives us:
    ///
    /// ```text
    ///       -ln(Cₜ)
    ///  β = ─────────
    ///          V
    /// ```
    ///
    /// See:
    /// - <https://en.wikipedia.org/wiki/Visibility>
    /// - <https://www.biral.com/wp-content/uploads/2015/02/Introduction_to_visibility-v2-2.pdf>
    pub fn koschmieder(v: f32, c_t: f32) -> f32 {
        -c_t.ln() / v
    }
}

impl Default for FogSettings {
    fn default() -> Self {
        FogSettings {
            color: Color::rgba(1.0, 1.0, 1.0, 1.0),
            falloff: FogFalloff::Linear {
                start: 0.0,
                end: 100.0,
            },
            directional_light_color: Color::NONE,
            directional_light_exponent: 8.0,
        }
    }
}

impl ExtractComponent for FogSettings {
    type Query = &'static Self;
    type Filter = With<Camera>;
    type Out = Self;

    fn extract_component(item: QueryItem<Self::Query>) -> Option<Self::Out> {
        Some(item.clone())
    }
}
