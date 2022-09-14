use bevy_app::{PluginGroup, PluginGroupBuilder};

/// This plugin group will add all the default plugins:
/// * [`LogPlugin`](bevy_log::LogPlugin)
/// * [`CorePlugin`](bevy_core::CorePlugin)
/// * [`TimePlugin`](bevy_time::TimePlugin)
/// * [`TransformPlugin`](bevy_transform::TransformPlugin)
/// * [`HierarchyPlugin`](bevy_hierarchy::HierarchyPlugin)
/// * [`DiagnosticsPlugin`](bevy_diagnostic::DiagnosticsPlugin)
/// * [`InputPlugin`](bevy_input::InputPlugin)
/// * [`WindowPlugin`](bevy_window::WindowPlugin)
/// * [`AssetPlugin`](bevy_asset::AssetPlugin)
/// * [`ScenePlugin`](bevy_scene::ScenePlugin)
/// * [`RenderPlugin`](bevy_render::RenderPlugin) - with feature `bevy_render`
/// * [`SpritePlugin`](bevy_sprite::SpritePlugin) - with feature `bevy_sprite`
/// * [`PbrPlugin`](bevy_pbr::PbrPlugin) - with feature `bevy_pbr`
/// * [`UiPlugin`](bevy_ui::UiPlugin) - with feature `bevy_ui`
/// * [`TextPlugin`](bevy_text::TextPlugin) - with feature `bevy_text`
/// * [`AudioPlugin`](bevy_audio::AudioPlugin) - with feature `bevy_audio`
/// * [`GilrsPlugin`](bevy_gilrs::GilrsPlugin) - with feature `bevy_gilrs`
/// * [`GltfPlugin`](bevy_gltf::GltfPlugin) - with feature `bevy_gltf`
/// * [`WinitPlugin`](bevy_winit::WinitPlugin) - with feature `bevy_winit`
///
/// See also [`MinimalPlugins`] for a slimmed down option
pub struct DefaultPlugins;

impl PluginGroup for DefaultPlugins {
    fn build(&mut self, group: &mut PluginGroupBuilder) {
        group.add(bevy_log::LogPlugin::default());
        group.add(bevy_core::CorePlugin::default());
        group.add(bevy_time::TimePlugin::default());
        group.add(bevy_transform::TransformPlugin::default());
        group.add(bevy_hierarchy::HierarchyPlugin::default());
        group.add(bevy_diagnostic::DiagnosticsPlugin::default());
        group.add(bevy_input::InputPlugin::default());
        group.add(bevy_window::WindowPlugin::default());

        #[cfg(feature = "bevy_asset")]
        group.add(bevy_asset::AssetPlugin::default());

        #[cfg(feature = "debug_asset_server")]
        group.add(bevy_asset::debug_asset_server::DebugAssetServerPlugin::default());

        #[cfg(feature = "bevy_scene")]
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

        // NOTE: Load this after renderer initialization so that it knows about the supported
        // compressed texture formats
        #[cfg(feature = "bevy_gltf")]
        group.add(bevy_gltf::GltfPlugin::default());

        #[cfg(feature = "bevy_audio")]
        group.add(bevy_audio::AudioPlugin::default());

        #[cfg(feature = "bevy_gilrs")]
        group.add(bevy_gilrs::GilrsPlugin::default());

        #[cfg(feature = "bevy_animation")]
        group.add(bevy_animation::AnimationPlugin::default());
    }
}

/// Minimal plugin group that will add the following plugins:
/// * [`CorePlugin`](bevy_core::CorePlugin)
/// * [`TimePlugin`](bevy_time::TimePlugin)
/// * [`ScheduleRunnerPlugin`](bevy_app::ScheduleRunnerPlugin)
///
/// See also [`DefaultPlugins`] for a more complete set of plugins
pub struct MinimalPlugins;

impl PluginGroup for MinimalPlugins {
    fn build(&mut self, group: &mut PluginGroupBuilder) {
        group.add(bevy_core::CorePlugin::default());
        group.add(bevy_time::TimePlugin::default());
        group.add(bevy_app::ScheduleRunnerPlugin::default());
    }
}
