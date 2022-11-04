use crate::ReflectComponent;
use bevy_ecs::{prelude::*, query::QueryItem};
use bevy_reflect::Reflect;
use bevy_render::{color::Color, extract_component::ExtractComponent, prelude::Camera};

/// Configures the “classic” computer graphics [distance fog](https://en.wikipedia.org/wiki/Distance_fog) effect,
/// in which objects appear progressively more covered in atmospheric haze as they move further away from the camera.
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
/// [`StandardMaterial`](crate::StandardMaterial) instances via the `no_fog` flag.
#[derive(Debug, Clone, Component, Reflect)]
#[reflect(Component)]
pub struct FogSettings {
    /// The color of the fog effect.
    ///
    /// **Tip:** The alpha channel of the color can be used to “modulate” the fog effect without
    /// changing the fog falloff mode or parameters.
    pub color: Color,

    /// The color used for the fog the view direction aligns with directional lights
    /// Produces a “halo” or light dispersion effect (e.g. around the sun)
    pub scattering_color: Color,

    /// The exponent applied to the directional light alignment calculation
    /// A higher value means a more concentrated “halo”
    pub scattering_exponent: f32,

    /// Determines which falloff mode to use, and its parameters.
    pub falloff: FogFalloff,
}

/// Allows switching between the different fog falloff modes, and configuring their parameters.
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
        // Distance from the camera where fog is completely transparent
        start: f32,

        // Distance from the camera where fog is completely opaque
        end: f32,
    },

    /// An exponential fog falloff with a given `density`.
    ///
    /// Initially gains intensity quickly with distance, then more slowly. Typically produces more natural results than [`FogFalloff::Linear`],
    /// but is a bit harder to control.
    ///
    /// To move the fog “further away”, use lower density values. To move it “closer” use higher density values.
    ///
    /// **Note:** It's not _unusual_ to have very large or very small values for the density, depending on the scene
    /// scale. Typically, for scenes with objects in the scale of thousands of units, you might want density values
    /// in the ballpark of `1e-3`. Conversely, for really small scale scenes you might want really high values of
    /// density.
    ///
    /// **Tip:** You can combine the `density` parameter with the [`FogSettings`] `color`'s alpha channel for easier control.
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
    Exponential { density: f32 },

    /// A squared exponential fog falloff with a given `density`.
    ///
    /// Similar to [`FogFalloff::Exponential`], but grows more slowly in intensity for closer distances
    /// before “catching up”.
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
    ExponentialSquared { density: f32 },

    /// Behaves somewhat like [`FogFalloff::Exponential`] mode, however individual color channels can have
    /// their own density value. Additionally, the falloff formula is separated into two terms, for a
    /// somewhat simplified atmospheric scattering model, with `extinction` and `inscattering`, resulting
    /// in a total of six different configurable coefficients.
    Atmospheric {
        extinction: Color,
        inscattering: Color,
    },
}

impl Default for FogSettings {
    fn default() -> Self {
        FogSettings {
            color: Color::rgba(1.0, 1.0, 1.0, 1.0),
            falloff: FogFalloff::Linear {
                start: 0.0,
                end: 100.0,
            },
            scattering_color: Color::NONE,
            scattering_exponent: 8.0,
        }
    }
}

impl ExtractComponent for FogSettings {
    type Query = &'static Self;
    type Filter = With<Camera>;

    fn extract_component(item: QueryItem<Self::Query>) -> Self {
        item.clone()
    }
}
