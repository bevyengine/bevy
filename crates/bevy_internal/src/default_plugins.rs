use bevy_app::{PluginGroup, PluginGroupBuilder};

/// This plugin group will add all the default plugins:
/// * [`LogPlugin`](crate::log::LogPlugin)
/// * [`CorePlugin`](crate::core::CorePlugin)
/// * [`TimePlugin`](crate::time::TimePlugin)
/// * [`TransformPlugin`](crate::transform::TransformPlugin)
/// * [`HierarchyPlugin`](crate::hierarchy::HierarchyPlugin)
/// * [`DiagnosticsPlugin`](crate::diagnostic::DiagnosticsPlugin)
/// * [`InputPlugin`](crate::input::InputPlugin)
/// * [`WindowPlugin`](crate::window::WindowPlugin)
/// * [`AssetPlugin`](crate::asset::AssetPlugin)
/// * [`ScenePlugin`](crate::scene::ScenePlugin)
/// * [`RenderPlugin`](crate::render::RenderPlugin) - with feature `bevy_render`
/// * [`SpritePlugin`](crate::sprite::SpritePlugin) - with feature `bevy_sprite`
/// * [`PbrPlugin`](crate::pbr::PbrPlugin) - with feature `bevy_pbr`
/// * [`UiPlugin`](crate::ui::UiPlugin) - with feature `bevy_ui`
/// * [`TextPlugin`](crate::text::TextPlugin) - with feature `bevy_text`
/// * [`AudioPlugin`](crate::audio::AudioPlugin) - with feature `bevy_audio`
/// * [`GilrsPlugin`](crate::gilrs::GilrsPlugin) - with feature `bevy_gilrs`
/// * [`GltfPlugin`](crate::gltf::GltfPlugin) - with feature `bevy_gltf`
/// * [`WinitPlugin`](crate::winit::WinitPlugin) - with feature `bevy_winit`
///
/// See also [`MinimalPlugins`] for a slimmed down option
pub struct DefaultPlugins;

impl PluginGroup for DefaultPlugins {
    fn build(self) -> PluginGroupBuilder {
        let mut group = PluginGroupBuilder::start::<Self>();
        group = group
            .add(bevy_log::LogPlugin::default())
            .add(bevy_core::CorePlugin::default())
            .add(bevy_time::TimePlugin::default())
            .add(bevy_transform::TransformPlugin::default())
            .add(bevy_hierarchy::HierarchyPlugin::default())
            .add(bevy_diagnostic::DiagnosticsPlugin::default())
            .add(bevy_input::InputPlugin::default())
            .add(bevy_window::WindowPlugin::default());

        #[cfg(feature = "bevy_asset")]
        {
            group = group.add(bevy_asset::AssetPlugin::default());
        }

        #[cfg(feature = "debug_asset_server")]
        {
            group = group.add(bevy_asset::debug_asset_server::DebugAssetServerPlugin::default());
        }

        #[cfg(feature = "bevy_scene")]
        {
            group = group.add(bevy_scene::ScenePlugin::default());
        }

        #[cfg(feature = "bevy_winit")]
        {
            group = group.add(bevy_winit::WinitPlugin::default());
        }

        #[cfg(feature = "bevy_render")]
        {
            group = group
                .add(bevy_render::RenderPlugin::default())
                // NOTE: Load this after renderer initialization so that it knows about the supported
                // compressed texture formats
                .add(bevy_render::texture::ImagePlugin::default());
        }

        #[cfg(feature = "bevy_core_pipeline")]
        {
            group = group.add(bevy_core_pipeline::CorePipelinePlugin::default());
        }

        #[cfg(feature = "bevy_sprite")]
        {
            group = group.add(bevy_sprite::SpritePlugin::default());
        }

        #[cfg(feature = "bevy_text")]
        {
            group = group.add(bevy_text::TextPlugin::default());
        }

        #[cfg(feature = "bevy_ui")]
        {
            group = group.add(bevy_ui::UiPlugin::default());
        }

        #[cfg(feature = "bevy_pbr")]
        {
            group = group.add(bevy_pbr::PbrPlugin::default());
        }

        // NOTE: Load this after renderer initialization so that it knows about the supported
        // compressed texture formats
        #[cfg(feature = "bevy_gltf")]
        {
            group = group.add(bevy_gltf::GltfPlugin::default());
        }

        #[cfg(feature = "bevy_audio")]
        {
            group = group.add(bevy_audio::AudioPlugin::default());
        }

        #[cfg(feature = "bevy_gilrs")]
        {
            group = group.add(bevy_gilrs::GilrsPlugin::default());
        }

        #[cfg(feature = "bevy_animation")]
        {
            group = group.add(bevy_animation::AnimationPlugin::default());
        }

        group
    }
}

/// Minimal plugin group that will add the following plugins:
/// * [`CorePlugin`](crate::core::CorePlugin)
/// * [`TimePlugin`](crate::time::TimePlugin)
/// * [`ScheduleRunnerPlugin`](crate::app::ScheduleRunnerPlugin)
///
/// See also [`DefaultPlugins`] for a more complete set of plugins
pub struct MinimalPlugins;

impl PluginGroup for MinimalPlugins {
    fn build(self) -> PluginGroupBuilder {
        PluginGroupBuilder::start::<Self>()
            .add(bevy_core::CorePlugin::default())
            .add(bevy_time::TimePlugin::default())
            .add(bevy_app::ScheduleRunnerPlugin::default())
    }
}
