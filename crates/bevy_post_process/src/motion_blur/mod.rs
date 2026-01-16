//! Per-object, per-pixel motion blur.
//!
//! Add the [`MotionBlur`] component to a camera to enable motion blur.

use bevy_app::{App, Plugin};
use bevy_asset::embedded_asset;
use bevy_camera::Camera;
use bevy_core_pipeline::{
    core_3d::graph::{Core3d, Node3d},
    prepass::{DepthPrepass, MotionVectorPrepass},
};
use bevy_ecs::{
    component::Component,
    query::{QueryItem, With},
    reflect::ReflectComponent,
    schedule::IntoScheduleConfigs,
};
use bevy_reflect::{std_traits::ReflectDefault, Reflect};
use bevy_render::{
    extract_component::{ExtractComponent, ExtractComponentPlugin, UniformComponentPlugin},
    render_graph::{RenderGraphExt, ViewNodeRunner},
    render_resource::{ShaderType, SpecializedRenderPipelines},
    Render, RenderApp, RenderStartup, RenderSystems,
};

pub mod node;
pub mod pipeline;

/// A component that enables and configures motion blur when added to a camera.
///
/// Motion blur is an effect that simulates how moving objects blur as they change position during
/// the exposure of film, a sensor, or an eyeball.
///
/// Because rendering simulates discrete steps in time, we use per-pixel motion vectors to estimate
/// the path of objects between frames. This kind of implementation has some artifacts:
/// - Fast moving objects in front of a stationary object or when in front of empty space, will not
///   have their edges blurred.
/// - Transparent objects do not write to depth or motion vectors, so they cannot be blurred.
///
/// Other approaches, such as *A Reconstruction Filter for Plausible Motion Blur* produce more
/// correct results, but are more expensive and complex, and have other kinds of artifacts. This
/// implementation is relatively inexpensive and effective.
///
/// # Usage
///
/// Add the [`MotionBlur`] component to a camera to enable and configure motion blur for that
/// camera.
///
/// ```
/// # use bevy_post_process::motion_blur::MotionBlur;
/// # use bevy_camera::Camera3d;
/// # use bevy_ecs::prelude::*;
/// # fn test(mut commands: Commands) {
/// commands.spawn((
///     Camera3d::default(),
///     MotionBlur::default(),
/// ));
/// # }
/// ````
#[derive(Reflect, Component, Clone)]
#[reflect(Component, Default, Clone)]
#[require(DepthPrepass, MotionVectorPrepass)]
pub struct MotionBlur {
    /// The strength of motion blur from `0.0` to `1.0`.
    ///
    /// The shutter angle describes the fraction of a frame that a camera's shutter is open and
    /// exposing the film/sensor. For 24fps cinematic film, a shutter angle of 0.5 (180 degrees) is
    /// common. This means that the shutter was open for half of the frame, or 1/48th of a second.
    /// The lower the shutter angle, the less exposure time and thus less blur.
    ///
    /// A value greater than one is non-physical and results in an object's blur stretching further
    /// than it traveled in that frame. This might be a desirable effect for artistic reasons, but
    /// consider allowing users to opt out of this.
    ///
    /// This value is intentionally tied to framerate to avoid the aforementioned non-physical
    /// over-blurring. If you want to emulate a cinematic look, your options are:
    ///   - Framelimit your app to 24fps, and set the shutter angle to 0.5 (180 deg). Note that
    ///     depending on artistic intent or the action of a scene, it is common to set the shutter
    ///     angle between 0.125 (45 deg) and 0.5 (180 deg). This is the most faithful way to
    ///     reproduce the look of film.
    ///   - Set the shutter angle greater than one. For example, to emulate the blur strength of
    ///     film while rendering at 60fps, you would set the shutter angle to `60/24 * 0.5 = 1.25`.
    ///     Note that this will result in artifacts where the motion of objects will stretch further
    ///     than they moved between frames; users may find this distracting.
    pub shutter_angle: f32,
    /// The quality of motion blur, corresponding to the number of per-pixel samples taken in each
    /// direction during blur.
    ///
    /// Setting this to `1` results in each pixel being sampled once in the leading direction, once
    /// in the trailing direction, and once in the middle, for a total of 3 samples (`1 * 2 + 1`).
    /// Setting this to `3` will result in `3 * 2 + 1 = 7` samples. Setting this to `0` is
    /// equivalent to disabling motion blur.
    pub samples: u32,
}

impl Default for MotionBlur {
    fn default() -> Self {
        Self {
            shutter_angle: 0.5,
            samples: 1,
        }
    }
}

impl ExtractComponent for MotionBlur {
    type QueryData = &'static Self;
    type QueryFilter = With<Camera>;
    type Out = MotionBlurUniform;

    fn extract_component(item: QueryItem<Self::QueryData>) -> Option<Self::Out> {
        Some(MotionBlurUniform {
            shutter_angle: item.shutter_angle,
            samples: item.samples,
            #[cfg(all(feature = "webgl", target_arch = "wasm32", not(feature = "webgpu")))]
            _webgl2_padding: Default::default(),
        })
    }
}

#[doc(hidden)]
#[derive(Component, ShaderType, Clone)]
pub struct MotionBlurUniform {
    shutter_angle: f32,
    samples: u32,
    #[cfg(all(feature = "webgl", target_arch = "wasm32", not(feature = "webgpu")))]
    // WebGL2 structs must be 16 byte aligned.
    _webgl2_padding: bevy_math::Vec2,
}

/// Adds support for per-object motion blur to the app. See [`MotionBlur`] for details.
#[derive(Default)]
pub struct MotionBlurPlugin;
impl Plugin for MotionBlurPlugin {
    fn build(&self, app: &mut App) {
        embedded_asset!(app, "motion_blur.wgsl");

        app.add_plugins((
            ExtractComponentPlugin::<MotionBlur>::default(),
            UniformComponentPlugin::<MotionBlurUniform>::default(),
        ));

        let Some(render_app) = app.get_sub_app_mut(RenderApp) else {
            return;
        };

        render_app
            .init_resource::<SpecializedRenderPipelines<pipeline::MotionBlurPipeline>>()
            .add_systems(RenderStartup, pipeline::init_motion_blur_pipeline)
            .add_systems(
                Render,
                pipeline::prepare_motion_blur_pipelines.in_set(RenderSystems::Prepare),
            );

        render_app
            .add_render_graph_node::<ViewNodeRunner<node::MotionBlurNode>>(
                Core3d,
                Node3d::MotionBlur,
            )
            .add_render_graph_edges(
                Core3d,
                (
                    Node3d::StartMainPassPostProcessing,
                    Node3d::MotionBlur,
                    Node3d::Bloom, // we want blurred areas to bloom and tonemap properly.
                ),
            );
    }
}
