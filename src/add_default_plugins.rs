use crate::app::AppBuilder;

pub trait AddDefaultPlugins {
    fn add_default_plugins(&mut self) -> &mut Self;
}

impl AddDefaultPlugins for AppBuilder {
    fn add_default_plugins(&mut self) -> &mut Self {
        #[cfg(feature = "type_registry")]
        self.add_plugin(bevy_type_registry::TypeRegistryPlugin::default());

        #[cfg(feature = "core")]
        self.add_plugin(bevy_core::CorePlugin::default());

        #[cfg(feature = "diagnostic")]
        self.add_plugin(bevy_diagnostic::DiagnosticsPlugin::default());

        #[cfg(feature = "input")]
        self.add_plugin(bevy_input::InputPlugin::default());

        #[cfg(feature = "window")]
        self.add_plugin(bevy_window::WindowPlugin::default());

        #[cfg(feature = "asset")]
        self.add_plugin(bevy_asset::AssetPlugin::default());

        #[cfg(feature = "scene")]
        self.add_plugin(bevy_scene::ScenePlugin::default());

        #[cfg(feature = "render")]
        self.add_plugin(bevy_render::RenderPlugin::default());

        #[cfg(feature = "sprite")]
        self.add_plugin(bevy_sprite::SpritePlugin::default());

        #[cfg(feature = "pbr")]
        self.add_plugin(bevy_pbr::PbrPlugin::default());

        #[cfg(feature = "ui")]
        self.add_plugin(bevy_ui::UiPlugin::default());

        #[cfg(feature = "gltf")]
        self.add_plugin(bevy_gltf::GltfPlugin::default());

        #[cfg(feature = "text")]
        self.add_plugin(bevy_text::TextPlugin::default());

        #[cfg(feature = "winit")]
        self.add_plugin(bevy_winit::WinitPlugin::default());
        #[cfg(not(feature = "winit"))]
        self.add_plugin(bevy_app::schedule_runner::ScheduleRunnerPlugin::default());

        #[cfg(feature = "wgpu")]
        self.add_plugin(bevy_wgpu::WgpuPlugin::default());

        self
    }
}
