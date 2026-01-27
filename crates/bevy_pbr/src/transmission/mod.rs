mod node;
mod phase;
mod texture;

use bevy_app::{App, Plugin};
use bevy_camera::Camera3d;
use bevy_core_pipeline::core_3d::graph::{Core3d, Node3d};
use bevy_ecs::prelude::*;
use bevy_reflect::prelude::*;
use bevy_render::{
    extract_component::{ExtractComponent, ExtractComponentPlugin},
    render_graph::{RenderGraphExt, ViewNodeRunner},
    render_phase::{sort_phase_system, AddRenderCommand, DrawFunctions, ViewSortedRenderPhases},
    ExtractSchedule, Render, RenderApp, RenderSystems,
};
use bevy_shader::load_shader_library;
pub use node::MainTransmissivePass3dNode;
pub use phase::Transmissive3d;
pub use texture::ViewTransmissionTexture;

use texture::prepare_core_3d_transmission_textures;

use crate::DrawMaterial;

/// Enables screen-space transmission for cameras.
pub struct ScreenSpaceTransmissionPlugin;

impl Plugin for ScreenSpaceTransmissionPlugin {
    fn build(&self, app: &mut App) {
        load_shader_library!(app, "transmission.wgsl");

        app.add_plugins(ExtractComponentPlugin::<ScreenSpaceTransmission>::default())
            .register_required_components::<Camera3d, ScreenSpaceTransmission>();

        let Some(render_app) = app.get_sub_app_mut(RenderApp) else {
            return;
        };

        render_app
            .init_resource::<DrawFunctions<Transmissive3d>>()
            .init_resource::<ViewSortedRenderPhases<Transmissive3d>>()
            .add_render_command::<Transmissive3d, DrawMaterial>()
            .add_systems(
                Render,
                sort_phase_system::<Transmissive3d>.in_set(RenderSystems::PhaseSort),
            )
            .add_systems(ExtractSchedule, phase::extract_transmissive_camera_phases)
            .add_systems(
                Render,
                prepare_core_3d_transmission_textures.in_set(RenderSystems::PrepareResources),
            )
            .add_render_graph_node::<ViewNodeRunner<MainTransmissivePass3dNode>>(
                Core3d,
                Node3d::MainTransmissivePass,
            )
            .add_render_graph_edges(
                Core3d,
                (
                    Node3d::MainOpaquePass,
                    Node3d::MainTransmissivePass,
                    Node3d::MainTransparentPass,
                ),
            );
    }
}

/// Configures transmission behavior, offering a trade-off between performance and visual fidelity.
#[derive(Component, Reflect, Clone, ExtractComponent)]
#[reflect(Component, Default, Clone)]
pub struct ScreenSpaceTransmission {
    /// How many individual steps should be performed in the `Transmissive3d` pass.
    ///
    /// Roughly corresponds to how many layers of transparency are rendered for screen space
    /// specular transmissive objects. Each step requires making one additional
    /// texture copy, so it's recommended to keep this number to a reasonably low value. Defaults to `1`.
    ///
    /// ### Notes
    ///
    /// - No copies will be performed if there are no transmissive materials currently being rendered,
    ///   regardless of this setting.
    /// - Setting this to `0` disables the screen-space refraction effect entirely, and falls
    ///   back to refracting only the environment map light's texture.
    /// - If set to more than `0`, any opaque [`clear_color`](bevy_camera::Camera::clear_color) will obscure the environment
    ///   map light's texture, preventing it from being visible through transmissive materials. If you'd like
    ///   to still have the environment map show up in your refractions, you can set the clear color's alpha to `0.0`.
    ///   Keep in mind that depending on the platform and your window settings, this may cause the window to become
    ///   transparent.
    pub screen_space_specular_transmission_steps: usize,
    /// The quality of the screen space specular transmission blur effect, applied to whatever's behind transmissive
    /// objects when their `roughness` is greater than `0.0`.
    ///
    /// Higher qualities are more GPU-intensive.
    ///
    /// **Note:** You can get better-looking results at any quality level by enabling TAA. See: `TemporalAntiAliasPlugin`
    pub screen_space_specular_transmission_quality: ScreenSpaceTransmissionQuality,
}

impl Default for ScreenSpaceTransmission {
    fn default() -> Self {
        Self {
            screen_space_specular_transmission_steps: 1,
            screen_space_specular_transmission_quality: Default::default(),
        }
    }
}

/// The quality of the screen space transmission blur effect, applied to whatever's behind transmissive
/// objects when their `roughness` is greater than `0.0`.
///
/// Higher qualities are more GPU-intensive.
///
/// **Note:** You can get better-looking results at any quality level by enabling TAA. See: `TemporalAntiAliasPlugin`
#[derive(Default, Clone, Copy, Reflect, PartialEq, PartialOrd, Debug)]
#[reflect(Default, Clone, Debug, PartialEq)]
pub enum ScreenSpaceTransmissionQuality {
    /// Best performance at the cost of quality. Suitable for lower end GPUs. (e.g. Mobile)
    ///
    /// `num_taps` = 4
    Low,

    /// A balanced option between quality and performance.
    ///
    /// `num_taps` = 8
    #[default]
    Medium,

    /// Better quality. Suitable for high end GPUs. (e.g. Desktop)
    ///
    /// `num_taps` = 16
    High,

    /// The highest quality, suitable for non-realtime rendering. (e.g. Pre-rendered cinematics and photo mode)
    ///
    /// `num_taps` = 32
    Ultra,
}
