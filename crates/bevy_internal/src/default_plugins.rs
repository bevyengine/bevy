use bevy_app::{PluginGroup, PluginGroupBuilder};

use bevy_app::ScheduleRunnerPlugin;
use bevy_asset::AssetPlugin;
#[cfg(feature = "bevy_audio")]
use bevy_audio::AudioPlugin;
use bevy_core::CorePlugin;
use bevy_diagnostic::DiagnosticsPlugin;
#[cfg(feature = "bevy_gilrs")]
use bevy_gilrs::GilrsPlugin;
#[cfg(feature = "bevy_gltf")]
use bevy_gltf::GltfPlugin;
use bevy_input::InputPlugin;
use bevy_log::LogPlugin;
#[cfg(feature = "bevy_pbr")]
use bevy_pbr::PbrPlugin;
#[cfg(feature = "bevy_render")]
use bevy_render::RenderPlugin;
use bevy_scene::ScenePlugin;
#[cfg(feature = "bevy_sprite")]
use bevy_sprite::SpritePlugin;
#[cfg(feature = "bevy_text")]
use bevy_text::TextPlugin;
use bevy_transform::TransformPlugin;
#[cfg(feature = "bevy_ui")]
use bevy_ui::UiPlugin;
#[cfg(feature = "bevy_wgpu")]
use bevy_wgpu::WgpuPlugin;
use bevy_window::WindowPlugin;
#[cfg(feature = "bevy_winit")]
use bevy_winit::WinitPlugin;

/// This plugin group will add all the default plugins:
/// * [`LogPlugin`]
/// * [`CorePlugin`]
/// * [`TransformPlugin`]
/// * [`DiagnosticsPlugin`]
/// * [`InputPlugin`]
/// * [`WindowPlugin`]
/// * [`AssetPlugin`]
/// * [`ScenePlugin`]
/// * [`RenderPlugin`] - with feature `bevy_render`
/// * [`SpritePlugin`] - with feature `bevy_sprite`
/// * [`PbrPlugin`] - with feature `bevy_pbr`
/// * [`UiPlugin`] - with feature `bevy_ui`
/// * [`TextPlugin`] - with feature `bevy_text`
/// * [`AudioPlugin`] - with feature `bevy_audio`
/// * [`GilrsPlugin`] - with feature `bevy_gilrs`
/// * [`GltfPlugin`] - with feature `bevy_gltf`
/// * [`WinitPlugin`] - with feature `bevy_winit`
/// * [`WgpuPlugin`] - with feature `bevy_wgpu`
///
/// See also [`MinimalPlugins`] for a slimmed down option
pub struct DefaultPlugins;

impl PluginGroup for DefaultPlugins {
    fn build(&mut self, group: &mut PluginGroupBuilder) {
        group.add(LogPlugin::default());
        group.add(CorePlugin::default());
        group.add(TransformPlugin::default());
        group.add(DiagnosticsPlugin::default());
        group.add(InputPlugin::default());
        group.add(WindowPlugin::default());
        group.add(AssetPlugin::default());
        group.add(ScenePlugin::default());

        #[cfg(feature = "bevy_render")]
        group.add(RenderPlugin::default());

        #[cfg(feature = "bevy_sprite")]
        group.add(SpritePlugin::default());

        #[cfg(feature = "bevy_pbr")]
        group.add(PbrPlugin::default());

        #[cfg(feature = "bevy_ui")]
        group.add(UiPlugin::default());

        #[cfg(feature = "bevy_text")]
        group.add(TextPlugin::default());

        #[cfg(feature = "bevy_audio")]
        group.add(AudioPlugin::default());

        #[cfg(feature = "bevy_gilrs")]
        group.add(GilrsPlugin::default());

        #[cfg(feature = "bevy_gltf")]
        group.add(GltfPlugin::default());

        #[cfg(feature = "bevy_winit")]
        group.add(WinitPlugin::default());

        #[cfg(feature = "bevy_wgpu")]
        group.add(WgpuPlugin::default());
    }
}

/// Minimal plugin group that will add the following plugins:
/// * [`CorePlugin`]
/// * [`ScheduleRunnerPlugin`]
///
/// See also [`DefaultPlugins`] for a more complete set of plugins
pub struct MinimalPlugins;

impl PluginGroup for MinimalPlugins {
    fn build(&mut self, group: &mut PluginGroupBuilder) {
        group.add(CorePlugin::default());
        group.add(ScheduleRunnerPlugin::default());
    }
}

pub struct PipelinedDefaultPlugins;

impl PluginGroup for PipelinedDefaultPlugins {
    fn build(&mut self, group: &mut PluginGroupBuilder) {
        group.add(bevy_log::LogPlugin::default());
        group.add(bevy_core::CorePlugin::default());
        group.add(bevy_transform::TransformPlugin::default());
        group.add(bevy_diagnostic::DiagnosticsPlugin::default());
        group.add(bevy_input::InputPlugin::default());
        group.add(bevy_window::WindowPlugin::default());
        group.add(bevy_asset::AssetPlugin::default());
        group.add(bevy_scene::ScenePlugin::default());

        #[cfg(feature = "bevy_winit")]
        group.add(bevy_winit::WinitPlugin::default());

        #[cfg(feature = "bevy_render2")]
        {
            group.add(bevy_render2::RenderPlugin::default());
        }

        #[cfg(feature = "bevy_core_pipeline")]
        {
            group.add(bevy_core_pipeline::CorePipelinePlugin::default());

            #[cfg(feature = "bevy_sprite2")]
            group.add(bevy_sprite2::SpritePlugin::default());

            #[cfg(feature = "bevy_text2")]
            group.add(bevy_text2::TextPlugin::default());

            #[cfg(feature = "bevy_ui2")]
            group.add(bevy_ui2::UiPlugin::default());

            #[cfg(feature = "bevy_pbr2")]
            group.add(bevy_pbr2::PbrPlugin::default());

            #[cfg(feature = "bevy_gltf2")]
            group.add(bevy_gltf2::GltfPlugin::default());
        }
    }
}
