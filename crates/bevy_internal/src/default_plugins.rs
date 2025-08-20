use bevy_app::{plugin_group, Plugin};

plugin_group! {
    /// This plugin group will add all the default plugins for a *Bevy* application:
    pub struct DefaultPlugins {
        bevy_app:::PanicHandlerPlugin,
        #[cfg(feature = "bevy_log")]
        bevy_log:::LogPlugin,
        bevy_app:::TaskPoolPlugin,
        bevy_diagnostic:::FrameCountPlugin,
        bevy_time:::TimePlugin,
        bevy_transform:::TransformPlugin,
        bevy_diagnostic:::DiagnosticsPlugin,
        bevy_input:::InputPlugin,
        #[custom(cfg(not(feature = "bevy_window")))]
        bevy_app:::ScheduleRunnerPlugin,
        #[cfg(feature = "bevy_window")]
        bevy_window:::WindowPlugin,
        #[cfg(feature = "bevy_window")]
        bevy_a11y:::AccessibilityPlugin,
        #[cfg(feature = "std")]
        #[custom(cfg(any(all(unix, not(target_os = "horizon")), windows)))]
        bevy_app:::TerminalCtrlCHandlerPlugin,
        #[cfg(feature = "bevy_asset")]
        bevy_asset:::AssetPlugin,
        #[cfg(feature = "bevy_scene")]
        bevy_scene:::ScenePlugin,
        #[cfg(feature = "bevy_winit")]
        bevy_winit:::WinitPlugin,
        #[custom(cfg(all(feature = "dlss", not(feature = "force_disable_dlss"))))]
        bevy_anti_aliasing::dlss:::DlssInitPlugin,
        #[cfg(feature = "bevy_render")]
        bevy_render:::RenderPlugin,
        // NOTE: Load this after renderer initialization so that it knows about the supported
        // compressed texture formats.
        #[cfg(feature = "bevy_image")]
        bevy_image:::ImagePlugin,
        #[cfg(feature = "bevy_render")]
        #[custom(cfg(all(not(target_arch = "wasm32"), feature = "multi_threaded")))]
        bevy_render::pipelined_rendering:::PipelinedRenderingPlugin,
        #[cfg(feature = "bevy_core_pipeline")]
        bevy_core_pipeline:::CorePipelinePlugin,
        #[cfg(feature = "bevy_anti_aliasing")]
        bevy_anti_aliasing:::AntiAliasingPlugin,
        #[cfg(feature = "bevy_sprite")]
        bevy_sprite:::SpritePlugin,
        #[cfg(feature = "bevy_sprite_render")]
        bevy_sprite_render:::SpriteRenderingPlugin,
        #[cfg(feature = "bevy_text")]
        bevy_text:::TextPlugin,
        #[cfg(feature = "bevy_ui")]
        bevy_ui:::UiPlugin,
        #[cfg(feature = "bevy_ui_render")]
        bevy_ui_render:::UiRenderPlugin,
        #[cfg(feature = "bevy_pbr")]
        bevy_pbr:::PbrPlugin,
        // NOTE: Load this after renderer initialization so that it knows about the supported
        // compressed texture formats.
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
        #[cfg(feature = "bevy_state")]
        bevy_state::app:::StatesPlugin,
        #[cfg(feature = "bevy_dev_tools")]
        bevy_dev_tools:::DevToolsPlugin,
        #[cfg(feature = "bevy_ci_testing")]
        bevy_dev_tools::ci_testing:::CiTestingPlugin,
        #[cfg(feature = "hotpatching")]
        bevy_app::hotpatch:::HotPatchPlugin,
        #[plugin_group]
        #[cfg(feature = "bevy_picking")]
        bevy_picking:::DefaultPickingPlugins,
        #[doc(hidden)]
        :IgnoreAmbiguitiesPlugin,
    }
    /// [`DefaultPlugins`] obeys *Cargo* *feature* flags. Users may exert control over this plugin group
    /// by disabling `default-features` in their `Cargo.toml` and enabling only those features
    /// that they wish to use.
    ///
    /// [`DefaultPlugins`] contains all the plugins typically required to build
    /// a *Bevy* application which includes a *window* and presentation components.
    /// For the absolute minimum number of plugins needed to run a Bevy application, see [`MinimalPlugins`].
}

#[derive(Default)]
struct IgnoreAmbiguitiesPlugin;

impl Plugin for IgnoreAmbiguitiesPlugin {
    #[expect(
        clippy::allow_attributes,
        reason = "`unused_variables` is not always linted"
    )]
    #[allow(
        unused_variables,
        reason = "The `app` parameter is used only if a combination of crates that contain ambiguities with each other are enabled."
    )]
    fn build(&self, app: &mut bevy_app::App) {
        // bevy_ui owns the Transform and cannot be animated
        #[cfg(all(feature = "bevy_animation", feature = "bevy_ui"))]
        if app.is_plugin_added::<bevy_animation::AnimationPlugin>()
            && app.is_plugin_added::<bevy_ui::UiPlugin>()
        {
            app.ignore_ambiguity(
                bevy_app::PostUpdate,
                bevy_animation::advance_animations,
                bevy_ui::ui_layout_system,
            );
            app.ignore_ambiguity(
                bevy_app::PostUpdate,
                bevy_animation::animate_targets,
                bevy_ui::ui_layout_system,
            );
        }
    }
}

plugin_group! {
    /// This plugin group will add the minimal plugins for a *Bevy* application:
    pub struct MinimalPlugins {
        bevy_app:::TaskPoolPlugin,
        bevy_diagnostic:::FrameCountPlugin,
        bevy_time:::TimePlugin,
        bevy_app:::ScheduleRunnerPlugin,
        #[cfg(feature = "bevy_ci_testing")]
        bevy_dev_tools::ci_testing:::CiTestingPlugin,
    }
    /// This plugin group represents the absolute minimum, bare-bones, bevy application.
    /// Use this if you want to have absolute control over the plugins used.
    ///
    /// It includes a [schedule runner (`ScheduleRunnerPlugin`)](crate::app::ScheduleRunnerPlugin)
    /// to provide functionality that would otherwise be driven by a windowed application's
    /// *event loop* or *message loop*.
    ///
    /// By default, this loop will run as fast as possible, which can result in high CPU usage.
    /// You can add a delay using [`run_loop`](crate::app::ScheduleRunnerPlugin::run_loop),
    /// or remove the loop using [`run_once`](crate::app::ScheduleRunnerPlugin::run_once).
    /// # Example:
    /// ```rust, no_run
    /// # use std::time::Duration;
    /// # use bevy_app::{App, PluginGroup, ScheduleRunnerPlugin};
    /// # use bevy_internal::MinimalPlugins;
    /// App::new().add_plugins(MinimalPlugins.set(ScheduleRunnerPlugin::run_loop(
    ///     // Run 60 times per second.
    ///     Duration::from_secs_f64(1.0 / 60.0),
    /// ))).run();
}
