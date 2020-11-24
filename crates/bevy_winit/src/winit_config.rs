/// A resource for configuring usage of the `rust_winit` library.
#[derive(Debug, Default)]
pub struct WinitConfig {
    /// Configures the winit library to return control to the main thread after
    /// the [run](bevy_app::App::run) loop is exited. Winit strongly recommends
    /// avoiding this when possible. Before using this please read and understand
    /// the [caveats](winit::platform::desktop::EventLoopExtDesktop::run_return)
    /// in the winit documentation.
    ///
    /// This feature is only available on desktop `target_os` configurations.
    /// Namely `windows`, `macos`, `linux`, `dragonfly`, `freebsd`, `netbsd`, and
    /// `openbsd`. If set to true on an unsupported platform
    /// [run](bevy_app::App::run) will panic.
    pub return_from_run: bool,
}
