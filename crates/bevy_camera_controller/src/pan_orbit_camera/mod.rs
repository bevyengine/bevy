//! Upstream of awesome [crate](https://github.com/aevyrie/bevy_editor_cam) made by @aevyrie.
//!
//! A production-ready camera controller for 3D editors; intended for anyone who needs to rapidly
//! and intuitively navigate virtual spaces.
//!
//! Camera controllers are very subjective! As someone who has spent years using camera controllers
//! in mechanical engineering CAD software, I've developed my own opinions about what matters in a
//! camera controller. This is my attempt to make the controller I've always wanted, that fixes the
//! annoyances I've encountered.
//!
//! *Because* camera controllers are so subjective, I felt the need to write out the impetus for
//! making this thing, what matters to me, and how I decided between conflicting goals. Somehow,
//! this ended up as a manifesto of sorts. If you came here to learn how to use or extend this
//! plugin, I've boiled the manifesto down into two sentences:
//!
//! > A camera controller needs to be responsive, robust, and satisfying to use. When there is
//! > conflict between these needs, they should be prioritized in that order.
//!
//! Now that you've absorbed my wisdom, feel free to skip ahead to the [Usage](crate#usage) section.
//!
//! Or don't. It's up to you.
//!
//! # Philosophy
//!
//! These are the properties of a good editor camera controller, in order of importance. These are
//! the driving values for the choices I've made here. You might disagree and have different values
//! or priorities!
//!
//! ## Responsive
//!
//! A good camera controller should never feel floaty or disconnected. It should go exactly where
//! the user commands it to go. Responsiveness isn't simply "low latency", it's about respecting the
//! user's intent.
//!
//! #### First-order input
//!
//! The most precise inputs are first-order, that is, controlling the position of something
//! directly, instead of its velocity (second-order) or acceleration (third-order). An example of
//! this is using a mouse vs. a gamepad for controlling the rotation of a first person view. The
//! mouse is first order, the position of the mouse on the mousepad directly corresponds with the
//! direction the player is facing. Conversely, a joystick controls the velocity of the view
//! rotation. All that is to say, where possible, the camera controller should use pointer inputs
//! *directly*.
//!
//! #### Pixel-perfect panning
//!
//! When you click and drag to pan the scene, the thing you click on should stick to your pointer,
//! and never drift. This should hold true even if inputs are being smoothed.
//!
//! #### Intuitive zoom
//!
//! The camera should zoom in and out in the direction you are pointing. If the user is hovering
//! over something, the speed of the camera should automatically adjust to quickly zoom up to it
//! without clipping through it.
//!
//! #### Predictable rotation
//!
//! When you click and drag to orbit the scene in 3d, the center of rotation should be located where
//! your pointer was when the drag started.
//!
//! #### Intuitive perspective toggle
//!
//! Toggling between different fields of view, or between perspective and orthographic projections,
//! should not cause the camera view to jump or change suddenly. The view should smoothly warp,
//! keeping the last interacted point stationary on the screen.
//!
//! ## Robust
//!
//! A camera controller should work in any scenario, and handle failure gracefully and
//! unsurprisingly when inputs are ambiguous.
//!
//! #### Works in all conditions:
//!
//! All of features in the previous section should work regardless of framerate, distance, scale,
//! camera field of view, and camera projection - including orthographic.
//!
//! #### Graceful fallback
//!
//! if nothing is under the pointer when a camera motion starts, the last-known depth should be
//! used, to prevent erratic behavior when the hit test fails. If a user was orbiting around a point
//! on an object, then clicks to rotate about empty space, the camera should not shoot off into
//! space because nothing was under the cursor.
//!
//! ### Satisfying
//!
//! The controller should *feel* good to use.
//!
//! #### Momentum
//!
//! Panning and orbiting should support configurable momentum, to allow you to "flick" the camera
//! through the scene to cover distance and make the feel of the camera tunable. This is especially
//! useful for trackpad and touch users.
//!
//! #### Smoothness
//!
//! The smoothness of inputs should be configurable as a tradeoff between fluidity of motion and
//! responsiveness. This is particularly useful when showing the screen to other people, where fast
//! motions can be disorienting or even nauseating.
//!
//! # Usage
//!
//! This plugin only requires three things to work. The `bevy_picking` plugin for hit tests, the
//! [`DefaultPanOrbitCameraPlugins`] plugin group, and the [`PanOrbitCamera`](crate::pan_orbit_camera::prelude::PanOrbitCamera)
//! component. Controller settings are configured per-camera in the
//! [`PanOrbitCamera`](crate::pan_orbit_camera::prelude::PanOrbitCamera) component.
//!
//! ## Getting Started
//!
//! #### 1. Add `bevy_picking`
//!
//! The camera controller uses [`bevy_picking`] for pointer interactions. If you already it along
//! with a picking backend, then using this camera controller is essentially free because it can
//! reuse those same hit tests you are already running.
//!
//! #### 2. Add `DefaultPanOrbitCameraPlugins`
//!
//! This is a plugin group that adds the camera controller, as well as all the [extensions]. You can
//! instead add [`controller::MinimalPanOrbitCameraPlugin`], though you will need to add your own input
//! plugin if you do.
//!
//! ```
//! # let mut app = bevy_app::App::new();
//! app.add_plugins(bevy_camera_controller::pan_orbit_camera::DefaultPanOrbitCameraPlugins);
//! ```
//!
//! #### 3. Insert the `PanOrbitCamera` component
//!
//! Finally, insert [`controller::component::PanOrbitCamera`] onto any cameras that you want to control.
//! This marks the cameras as controllable and holds all camera controller settings.
//!
//! ```
//! # use bevy_ecs::system::Commands;
//! # use bevy_camera_controller::pan_orbit_camera::prelude::*;
//! # fn test(mut commands: Commands) {
//! commands.spawn((
//!     // Camera
//!     PanOrbitCamera::default(),
//! ));
//! # }
//! ```
//!
//! # Other notable features
//!
//! I've also implemented a few other features that are handy for a camera controller like this.
//!
//! ### Compatible with floating origins and other controllers
//!
//! This controller does all computations in view space. The result of this is that you can move the
//! camera wherever you want, update its transform, and it will continue to behave normally, as long
//! as the camera isn't being controlled by the user while you do this. This means you can control
//! this camera with another camera controller, or use it in a floating origin system.
//!
//! ### Independent skybox
//!
//! When working in a CAD context, it is common to use orthographic projections to remove
//! perspective distortion from the image. However, because an ortho projection has zero field of
//! view, the view of the skybox is infinitesimally small, i.e. only a single pixel of the skybox is
//! visible. To fix this, an [extension](extensions) is provided to attach a skybox to a camera that
//! is independent from that camera's field of view.
//!
//! ### Pointer and Hit Test Agnostic
//!
//! Users of this library shouldn't be forced into using any particular hit testing method, like CPU
//! raycasting. The controller uses [`bevy_picking`] to work with:
//!
//! - Arbitrary hit testing backends, including those written by users. See
//!   [`bevy_picking::backend`] for more information.
//! - Any number of pointing inputs, including touch.
//! - Viewports and multi-pass rendering.

#![warn(missing_docs)]

pub mod controller;
pub mod extensions;
pub mod input;

/// Common imports.
pub mod prelude {
    pub use crate::{
        pan_orbit_camera::controller::{component::*, transform_adapter::*, *},
        pan_orbit_camera::DefaultPanOrbitCameraPlugins,
    };
}

use bevy_app::{prelude::*, PluginGroupBuilder};
use bevy_ecs::prelude::SystemSet;

/// Adds [`bevy_editor_cam`](crate) functionality with all extensions and the default input plugin.
///
/// This is intended for a quick and easy setup. You can add the individual plugins yourself if you
/// want more control over the setup.
///
/// To be more precise, this plugin group adds the following plugins:
///
/// - [`controller::MinimalPanOrbitCameraPlugin`]
/// - [`input::DefaultInputPlugin`]
/// - [`extensions::dolly_zoom::DollyZoomPlugin`]
/// - [`extensions::look_to::LookToPlugin`]
/// - [`extensions::anchor_indicator::AnchorIndicatorPlugin`] (if the `extension_anchor_indicator` feature is enabled)
/// - [`extensions::independent_skybox::IndependentSkyboxPlugin`] (if the `extension_independent_skybox` feature is enabled)
pub struct DefaultPanOrbitCameraPlugins;

/// This system set may alter the camera position in the `PreUpdate` schedule.
#[derive(SystemSet, Default, Debug, Clone, PartialEq, Eq, Hash)]
pub struct SyncCameraPosition;

impl PluginGroup for DefaultPanOrbitCameraPlugins {
    fn build(self) -> PluginGroupBuilder {
        let group = PluginGroupBuilder::start::<Self>()
            .add(input::DefaultInputPlugin)
            .add(controller::MinimalPanOrbitCameraPlugin)
            .add(extensions::dolly_zoom::DollyZoomPlugin)
            .add(extensions::look_to::LookToPlugin);

        #[cfg(feature = "extension_anchor_indicator")]
        let group = group.add(extensions::anchor_indicator::AnchorIndicatorPlugin);

        #[cfg(feature = "extension_independent_skybox")]
        let group = group.add(extensions::independent_skybox::IndependentSkyboxPlugin);

        group
    }
}
