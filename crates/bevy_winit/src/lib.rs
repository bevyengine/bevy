#![cfg_attr(docsrs, feature(doc_auto_cfg))]
#![forbid(unsafe_code)]
#![doc(
    html_logo_url = "https://bevy.org/assets/icon.png",
    html_favicon_url = "https://bevy.org/assets/icon.png"
)]

//! `bevy_winit` provides utilities to handle window creation and the eventloop through [`winit`]
//!
//! Most commonly, the [`WinitPlugin`] is used as part of
//! [`DefaultPlugins`](https://docs.rs/bevy/latest/bevy/struct.DefaultPlugins.html).
//! The app's [runner](bevy_app::App::runner) is set by `WinitPlugin` and handles the `winit` [`EventLoop`].
//! See `winit_runner` for details.

extern crate alloc;

use bevy_derive::Deref;
use bevy_reflect::prelude::ReflectDefault;
use bevy_reflect::Reflect;
use bevy_window::{RawHandleWrapperHolder, WindowEvent};
use core::cell::RefCell;
use core::marker::PhantomData;
use winit::{event_loop::EventLoop, window::WindowId};

use bevy_a11y::AccessibilityRequested;
use bevy_app::{App, Last, Plugin};
use bevy_ecs::prelude::*;
use bevy_window::{exit_on_all_closed, CursorOptions, Window, WindowCreated};
use system::{changed_cursor_options, changed_windows, check_keyboard_focus_lost, despawn_windows};
pub use system::{create_monitors, create_windows};
#[cfg(all(target_family = "wasm", target_os = "unknown"))]
pub use winit::platform::web::CustomCursorExtWebSys;
pub use winit::{
    event_loop::EventLoopProxy,
    window::{CustomCursor as WinitCustomCursor, CustomCursorSource},
};
pub use winit_config::*;
pub use winit_windows::*;

use crate::{
    accessibility::{AccessKitPlugin, WinitActionRequestHandlers},
    state::winit_runner,
    winit_monitors::WinitMonitors,
};

pub mod accessibility;
mod converters;
mod cursor;
mod state;
mod system;
mod winit_config;
mod winit_monitors;
mod winit_windows;

thread_local! {
    /// Temporary storage of WinitWindows data to replace usage of `!Send` resources. This will be replaced with proper
    /// storage of `!Send` data after issue #17667 is complete.
    pub static WINIT_WINDOWS: RefCell<WinitWindows> = const { RefCell::new(WinitWindows::new()) };
}

/// A [`Plugin`] that uses `winit` to create and manage windows, and receive window and input
/// events.
///
/// This plugin will add systems and resources that sync with the `winit` backend and also
/// replace the existing [`App`] runner with one that constructs an [event loop](EventLoop) to
/// receive window and input events from the OS.
///
/// The `T` event type can be used to pass custom events to the `winit`'s loop, and handled as events
/// in systems.
///
/// When using eg. `MinimalPlugins` you can add this using `WinitPlugin::<WakeUp>::default()`, where
/// `WakeUp` is the default event that bevy uses.
#[derive(Default)]
pub struct WinitPlugin<T: BufferedEvent = WakeUp> {
    /// Allows the window (and the event loop) to be created on any thread
    /// instead of only the main thread.
    ///
    /// See [`EventLoopBuilder::build`](winit::event_loop::EventLoopBuilder::build) for more information on this.
    ///
    /// # Supported platforms
    ///
    /// Only works on Linux (X11/Wayland) and Windows.
    /// This field is ignored on other platforms.
    pub run_on_any_thread: bool,
    marker: PhantomData<T>,
}

impl<T: BufferedEvent> Plugin for WinitPlugin<T> {
    fn name(&self) -> &str {
        "bevy_winit::WinitPlugin"
    }

    fn build(&self, app: &mut App) {
        let mut event_loop_builder = EventLoop::<T>::with_user_event();

        // linux check is needed because x11 might be enabled on other platforms.
        #[cfg(all(target_os = "linux", feature = "x11"))]
        {
            use winit::platform::x11::EventLoopBuilderExtX11;

            // This allows a Bevy app to be started and ran outside the main thread.
            // A use case for this is to allow external applications to spawn a thread
            // which runs a Bevy app without requiring the Bevy app to need to reside on
            // the main thread, which can be problematic.
            event_loop_builder.with_any_thread(self.run_on_any_thread);
        }

        // linux check is needed because wayland might be enabled on other platforms.
        #[cfg(all(target_os = "linux", feature = "wayland"))]
        {
            use winit::platform::wayland::EventLoopBuilderExtWayland;
            event_loop_builder.with_any_thread(self.run_on_any_thread);
        }

        #[cfg(target_os = "windows")]
        {
            use winit::platform::windows::EventLoopBuilderExtWindows;
            event_loop_builder.with_any_thread(self.run_on_any_thread);
        }

        #[cfg(target_os = "android")]
        {
            use winit::platform::android::EventLoopBuilderExtAndroid;
            let msg = "Bevy must be setup with the #[bevy_main] macro on Android";
            event_loop_builder
                .with_android_app(bevy_android::ANDROID_APP.get().expect(msg).clone());
        }

        let event_loop = event_loop_builder
            .build()
            .expect("Failed to build event loop");

        app.init_resource::<WinitMonitors>()
            .init_resource::<WinitSettings>()
            .insert_resource(DisplayHandleWrapper(event_loop.owned_display_handle()))
            .add_event::<RawWinitWindowEvent>()
            .set_runner(|app| winit_runner(app, event_loop))
            .add_systems(
                Last,
                (
                    // `exit_on_all_closed` only checks if windows exist but doesn't access data,
                    // so we don't need to care about its ordering relative to `changed_windows`
                    changed_windows.ambiguous_with(exit_on_all_closed),
                    changed_cursor_options,
                    despawn_windows,
                    check_keyboard_focus_lost,
                )
                    .chain(),
            );

        app.add_plugins(AccessKitPlugin);
        app.add_plugins(cursor::WinitCursorPlugin);
    }
}

/// The default event that can be used to wake the window loop
/// Wakes up the loop if in wait state
#[derive(Debug, Default, Clone, Copy, BufferedEvent, Reflect)]
#[reflect(Debug, Default, Clone)]
pub struct WakeUp;

/// The original window event as produced by Winit. This is meant as an escape
/// hatch for power users that wish to add custom Winit integrations.
/// If you want to process events for your app or game, you should instead use
/// `bevy::window::WindowEvent`, or one of its sub-events.
///
/// When you receive this event it has already been handled by Bevy's main loop.
/// Sending these events will NOT cause them to be processed by Bevy.
#[derive(Debug, Clone, BufferedEvent)]
pub struct RawWinitWindowEvent {
    /// The window for which the event was fired.
    pub window_id: WindowId,
    /// The raw winit window event.
    pub event: winit::event::WindowEvent,
}

/// A wrapper type around [`winit::event_loop::EventLoopProxy`] with the specific
/// [`winit::event::Event::UserEvent`] used in the [`WinitPlugin`].
///
/// The `EventLoopProxy` can be used to request a redraw from outside bevy.
///
/// Use `Res<EventLoopProxy>` to receive this resource.
#[derive(Resource, Deref)]
pub struct EventLoopProxyWrapper<T: 'static>(EventLoopProxy<T>);

/// A wrapper around [`winit::event_loop::OwnedDisplayHandle`]
///
/// The `DisplayHandleWrapper` can be used to build integrations that rely on direct
/// access to the display handle
///
/// Use `Res<DisplayHandleWrapper>` to receive this resource.
#[derive(Resource, Deref)]
pub struct DisplayHandleWrapper(pub winit::event_loop::OwnedDisplayHandle);

trait AppSendEvent {
    fn send(&mut self, event: impl Into<WindowEvent>);
}

impl AppSendEvent for Vec<WindowEvent> {
    fn send(&mut self, event: impl Into<WindowEvent>) {
        self.push(Into::<WindowEvent>::into(event));
    }
}

/// The parameters of the [`create_windows`] system.
pub type CreateWindowParams<'w, 's, F = ()> = (
    Commands<'w, 's>,
    Query<
        'w,
        's,
        (
            Entity,
            &'static mut Window,
            &'static CursorOptions,
            Option<&'static RawHandleWrapperHolder>,
        ),
        F,
    >,
    EventWriter<'w, WindowCreated>,
    ResMut<'w, WinitActionRequestHandlers>,
    Res<'w, AccessibilityRequested>,
    Res<'w, WinitMonitors>,
);

/// The parameters of the [`create_monitors`] system.
pub type CreateMonitorParams<'w, 's> = (Commands<'w, 's>, ResMut<'w, WinitMonitors>);
