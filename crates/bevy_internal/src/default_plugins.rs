#![allow(dead_code)]
use bevy_app::{plugin_group, Plugin, PluginGroup, PluginGroupBuilder};

plugin_group!(
    /// This plugin group will add all the default plugins for a *Bevy* application:
    DefaultPlugins {
        bevy_log:::LogPlugin,
        bevy_core:::TaskPoolPlugin,
        bevy_core:::TypeRegistrationPlugin,
        bevy_core:::FrameCountPlugin,
        bevy_time:::TimePlugin,
        bevy_transform:::TransformPlugin,
        bevy_hierarchy:::HierarchyPlugin,
        bevy_diagnostic:::DiagnosticsPlugin,
        bevy_input:::InputPlugin,
        bevy_window:::WindowPlugin,
        bevy_a11y:::AccessibilityPlugin,
        #[cfg(feature = "bevy_asset")]
        bevy_asset:::AssetPlugin,
        #[cfg(feature = "bevy_scene")]
        bevy_scene:::ScenePlugin,
        #[cfg(feature = "bevy_winit")]
        bevy_winit:::WinitPlugin,
        #[cfg(feature = "bevy_render")]
        bevy_render:::RenderPlugin,
        // NOTE: Load this after renderer initialization so that it knows about the supported
        // compressed texture formats
        #[cfg(feature = "bevy_render")]
        bevy_render::texture:::ImagePlugin,
        #[cfg(feature = "bevy_render")]
        #[custom(cfg(all(not(target_arch = "wasm32"), feature = "multi-threaded")))]
        bevy_render::pipelined_rendering:::PipelinedRenderingPlugin,
        #[cfg(feature = "bevy_sprite")]
        bevy_sprite:::SpritePlugin,
        #[cfg(feature = "bevy_text")]
        bevy_text:::TextPlugin,
        #[cfg(feature = "bevy_ui")]
        bevy_ui:::UiPlugin,
        #[cfg(feature = "bevy_pbr")]
        bevy_pbr:::PbrPlugin,
        #[cfg(feature = "bevy_gltf")]
        bevy_gltf:::GltfPlugin,
        #[cfg(feature = "bevy_audio")]
        bevy_audio:::AudioPlugin,
        #[cfg(feature = "bevy_gilrs")]
        bevy_gilrs:::GilrsPlugin,
        #[cfg(feature = "bevy_animation")]
        bevy_animation:::AnimationPlugin,
        #[cfg(feature = "bevy_gizmos")]
        bevy_gizmos:::GizmoPlugin,
        :IgnoreAmbiguitiesPlugin,
    }
    /// [`DefaultPlugins`] obeys *Cargo feature* flags. Users may exert control over this plugin group
    /// by disabling `default-features` in their `Cargo.toml` and enabling only those features
    /// that they wish to use.
    ///
    /// [`DefaultPlugins`] contains all the plugins typically required to build
    /// a *Bevy* application which includes a *window* and presentation components.
    /// For *headless* cases – without a *window* or presentation, see [`MinimalPlugins`].
);

#[derive(Default)]
struct IgnoreAmbiguitiesPlugin;

impl Plugin for IgnoreAmbiguitiesPlugin {
    #[allow(unused_variables)] // Variables are used depending on enabled features
    fn build(&self, app: &mut bevy_app::App) {
        // bevy_ui owns the Transform and cannot be animated
        #[cfg(all(feature = "bevy_animation", feature = "bevy_ui"))]
        app.ignore_ambiguity(
            bevy_app::PostUpdate,
            bevy_animation::animation_player,
            bevy_ui::ui_layout_system,
        );

        #[cfg(feature = "bevy_render")]
        if let Ok(render_app) = app.get_sub_app_mut(bevy_render::RenderApp) {
            #[cfg(all(feature = "bevy_gizmos", feature = "bevy_sprite"))]
            {
                render_app.ignore_ambiguity(
                    bevy_render::Render,
                    bevy_gizmos::GizmoRenderSystem::QueueLineGizmos2d,
                    bevy_sprite::queue_sprites,
                );
                render_app.ignore_ambiguity(
                    bevy_render::Render,
                    bevy_gizmos::GizmoRenderSystem::QueueLineGizmos2d,
                    bevy_sprite::queue_material2d_meshes::<bevy_sprite::ColorMaterial>,
                );
            }
            #[cfg(all(feature = "bevy_gizmos", feature = "bevy_pbr"))]
            {
                render_app.ignore_ambiguity(
                    bevy_render::Render,
                    bevy_gizmos::GizmoRenderSystem::QueueLineGizmos3d,
                    bevy_pbr::queue_material_meshes::<bevy_pbr::StandardMaterial>,
                );
            }
        }
    }
}

plugin_group!(
    /// This plugin group will add the minimal plugins for a *Bevy* application:
    MinimalPlugins {
        bevy_core:::TaskPoolPlugin,
        bevy_core:::TypeRegistrationPlugin,
        bevy_core:::FrameCountPlugin,
        bevy_time:::TimePlugin,
        bevy_app:::ScheduleRunnerPlugin,
    }
    /// This group of plugins is intended for use for minimal, *headless* programs –
    /// see the [*Bevy* *headless* example](https://github.com/bevyengine/bevy/blob/main/examples/app/headless.rs)
    /// – and includes a [schedule runner (`ScheduleRunnerPlugin`)](crate::app::ScheduleRunnerPlugin)
    /// to provide functionality that would otherwise be driven by a windowed application's
    /// *event loop* or *message loop*.
    ///
    /// Windowed applications that wish to use a reduced set of plugins should consider the
    /// [`DefaultPlugins`] plugin group which can be controlled with *Cargo* *feature* flags.
);
