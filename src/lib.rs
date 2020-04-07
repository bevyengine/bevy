//! Bevy is a modular game engine built in Rust, with a focus on developer productivity and performance.
//! 
//! It has the following design goals:
//! * Provide a first class user-experience for both 2D and 3D games
//! * Easy for newbies to pick up, but infinitely flexible for power users
//! * Fast iterative compile times. Ideally less than 1 second for small to medium sized projects
//! * Data-first game development using ECS (Entity Component Systems)
//! * High performance and parallel architecture
//! * Use the latest and greatest rendering technologies and techniques
//! 
//! The "bevy" crate is just a container crate that makes it easier to consume Bevy components.
//! The defaults provide a "full" engine experience, but you can easily enable / disable features
//! in your project's Cargo.toml to meet your specific needs. See Bevy's Cargo.toml for a full list of features available.

#![feature(min_specialization)]

pub mod prelude;

pub use bevy_app as app;
pub use glam as math;
pub use legion;

#[cfg(feature = "asset")]
pub use bevy_asset as asset;
#[cfg(feature = "core")]
pub use bevy_core as core;
#[cfg(feature = "derive")]
pub use bevy_derive as derive;
#[cfg(feature = "diagnostic")]
pub use bevy_diagnostic as diagnostic;
#[cfg(feature = "input")]
pub use bevy_input as input;
#[cfg(feature = "render")]
pub use bevy_render as render;
#[cfg(feature = "serialization")]
pub use bevy_serialization as serialization;
#[cfg(feature = "transform")]
pub use bevy_transform as transform;
#[cfg(feature = "ui")]
pub use bevy_ui as ui;
#[cfg(feature = "window")]
pub use bevy_window as window;

use app::AppBuilder;

pub trait AddDefaultPlugins {
    fn add_default_plugins(&mut self) -> &mut Self;
}

impl AddDefaultPlugins for AppBuilder {
    fn add_default_plugins(&mut self) -> &mut Self {
        #[cfg(feature = "core")]
        self.add_plugin(bevy_core::CorePlugin::default());

        #[cfg(feature = "input")]
        self.add_plugin(bevy_input::InputPlugin::default());

        #[cfg(feature = "window")]
        self.add_plugin(bevy_window::WindowPlugin::default());

        #[cfg(feature = "render")]
        self.add_plugin(bevy_render::RenderPlugin::default());

        #[cfg(feature = "ui")]
        self.add_plugin(ui::UiPlugin::default());

        #[cfg(feature = "winit")]
        self.add_plugin(bevy_winit::WinitPlugin::default());
        #[cfg(not(feature = "winit"))]
        self.add_plugin(bevy_app::schedule_runner::ScheduleRunnerPlugin::default());

        #[cfg(feature = "wgpu")]
        self.add_plugin(bevy_wgpu::WgpuRendererPlugin::default());

        self
    }
}
