use crate::app::{AppBuilder, Plugin};
use std::any::TypeId;

pub trait AddDefaultPlugins {
    /// Add all default plugins
    fn add_default_plugins(&mut self) -> &mut Self;
    /// Add default plugins using `default_plugin_builder` to control which are added
    fn add_default_plugins_with_builder(
        &mut self,
        default_plugin_builder: DefaultPluginBuilder,
    ) -> &mut Self;
}

enum PluginState {
    Default,
    Custom(fn(&mut AppBuilder)),
    None,
}

impl Default for PluginState {
    fn default() -> Self {
        PluginState::Default
    }
}

/// Helper to enable / disable default plugins, or override a default plugin with a custom that will be loaded at
/// the same time as the default one.
#[derive(Default)]
pub struct DefaultPluginBuilder {
    plugins: std::collections::HashMap<TypeId, PluginState>,
}

impl DefaultPluginBuilder {
    /// Override the default plugin `T` with a custom builder
    pub fn with_custom<T: Plugin>(&mut self, builder: fn(&mut AppBuilder)) -> &mut Self {
        self.plugins
            .insert(TypeId::of::<T>(), PluginState::Custom(builder));
        self
    }

    /// Keep using the default plugin `T`. This is the default
    pub fn with_default<T: Plugin>(&mut self) -> &mut Self {
        self.plugins.insert(TypeId::of::<T>(), PluginState::Default);
        self
    }

    /// Disable the default plugin `T`
    pub fn disable<T: Plugin>(&mut self) -> &mut Self {
        self.plugins.insert(TypeId::of::<T>(), PluginState::None);
        self
    }

    fn build_plugin<T: Plugin + Default>(&self, app: &mut AppBuilder) {
        match self.plugins.get(&TypeId::of::<T>()) {
            None | Some(PluginState::Default) => {
                app.add_plugin(T::default());
            }
            Some(PluginState::Custom(custom_builder)) => {
                custom_builder(app);
            }
            Some(PluginState::None) => (),
        }
    }

    fn build(self, app: &mut AppBuilder) -> &mut AppBuilder {
        self.build_plugin::<bevy_type_registry::TypeRegistryPlugin>(app);
        self.build_plugin::<bevy_core::CorePlugin>(app);
        self.build_plugin::<bevy_transform::TransformPlugin>(app);
        self.build_plugin::<bevy_diagnostic::DiagnosticsPlugin>(app);
        self.build_plugin::<bevy_input::InputPlugin>(app);
        self.build_plugin::<bevy_window::WindowPlugin>(app);
        self.build_plugin::<bevy_asset::AssetPlugin>(app);
        self.build_plugin::<bevy_scene::ScenePlugin>(app);

        #[cfg(feature = "bevy_render")]
        self.build_plugin::<bevy_render::RenderPlugin>(app);

        #[cfg(feature = "bevy_sprite")]
        self.build_plugin::<bevy_sprite::SpritePlugin>(app);

        #[cfg(feature = "bevy_pbr")]
        self.build_plugin::<bevy_pbr::PbrPlugin>(app);

        #[cfg(feature = "bevy_ui")]
        self.build_plugin::<bevy_ui::UiPlugin>(app);

        #[cfg(feature = "bevy_text")]
        self.build_plugin::<bevy_text::TextPlugin>(app);

        #[cfg(feature = "bevy_audio")]
        self.build_plugin::<bevy_audio::AudioPlugin>(app);

        #[cfg(feature = "bevy_gilrs")]
        self.build_plugin::<bevy_gilrs::GilrsPlugin>(app);

        #[cfg(feature = "bevy_gltf")]
        self.build_plugin::<bevy_gltf::GltfPlugin>(app);

        #[cfg(feature = "bevy_winit")]
        self.build_plugin::<bevy_winit::WinitPlugin>(app);

        #[cfg(feature = "bevy_wgpu")]
        self.build_plugin::<bevy_wgpu::WgpuPlugin>(app);
        app
    }
}

impl AddDefaultPlugins for AppBuilder {
    fn add_default_plugins(&mut self) -> &mut Self {
        DefaultPluginBuilder::default().build(self);

        self
    }

    fn add_default_plugins_with_builder(
        &mut self,
        default_plugin_builder: DefaultPluginBuilder,
    ) -> &mut Self {
        default_plugin_builder.build(self);

        self
    }
}
