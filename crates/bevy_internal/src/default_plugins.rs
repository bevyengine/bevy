use bevy_app::{PluginGroup, PluginGroupBuilder};

pub struct DefaultPlugins;

impl PluginGroup for DefaultPlugins {
    fn build(&mut self, group: &mut PluginGroupBuilder) {
        group.add(bevy_log::LogPlugin::default());
        group.add(bevy_reflect::ReflectPlugin::default());
        group.add(bevy_core::CorePlugin::default());
        group.add(bevy_transform::TransformPlugin::default());
        group.add(bevy_diagnostic::DiagnosticsPlugin::default());
        group.add(bevy_input::InputPlugin::default());
        group.add(bevy_window::WindowPlugin::default());
        group.add(bevy_asset::AssetPlugin::default());
        group.add(bevy_scene::ScenePlugin::default());

        #[cfg(feature = "bevy_render")]
        group.add(bevy_render::RenderPlugin::default());

        #[cfg(feature = "bevy_sprite")]
        group.add(bevy_sprite::SpritePlugin::default());

        #[cfg(feature = "bevy_pbr")]
        group.add(bevy_pbr::PbrPlugin::default());

        #[cfg(feature = "bevy_ui")]
        group.add(bevy_ui::UiPlugin::default());

        #[cfg(feature = "bevy_text")]
        group.add(bevy_text::TextPlugin::default());

        #[cfg(feature = "bevy_audio")]
        group.add(bevy_audio::AudioPlugin::default());

        #[cfg(feature = "bevy_gilrs")]
        group.add(bevy_gilrs::GilrsPlugin::default());

        #[cfg(feature = "bevy_gltf")]
        group.add(bevy_gltf::GltfPlugin::default());

        #[cfg(feature = "bevy_winit")]
        group.add(bevy_winit::WinitPlugin::default());

        #[cfg(feature = "bevy_wgpu")]
        group.add(bevy_wgpu::WgpuPlugin::default());
    }
}

pub struct MinimalPlugins;

impl PluginGroup for MinimalPlugins {
    fn build(&mut self, group: &mut PluginGroupBuilder) {
        group.add(bevy_reflect::ReflectPlugin::default());
        group.add(bevy_core::CorePlugin::default());
        group.add(bevy_app::ScheduleRunnerPlugin::default());
    }
}
