use bevy_app::{PluginGroup, PluginGroupBuilder};

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
///
/// See also [`MinimalPlugins`] for a slimmed down option
pub struct DefaultPlugins;

impl PluginGroup for DefaultPlugins {
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

        #[cfg(feature = "bevy_render")]
        group.add(bevy_render::RenderPlugin::default());

        #[cfg(feature = "bevy_core_pipeline")]
        group.add(bevy_core_pipeline::CorePipelinePlugin::default());

        #[cfg(feature = "bevy_sprite")]
        group.add(bevy_sprite::SpritePlugin::default());

        #[cfg(feature = "bevy_text")]
        group.add(bevy_text::TextPlugin::default());

        #[cfg(feature = "bevy_ui")]
        group.add(bevy_ui::UiPlugin::default());

        #[cfg(feature = "bevy_pbr")]
        group.add(bevy_pbr::PbrPlugin::default());

        #[cfg(feature = "bevy_gltf")]
        group.add(bevy_gltf::GltfPlugin::default());

        #[cfg(feature = "bevy_audio")]
        group.add(bevy_audio::AudioPlugin::default());
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
        group.add(bevy_core::CorePlugin::default());
        group.add(bevy_app::ScheduleRunnerPlugin::default());
    }
}
