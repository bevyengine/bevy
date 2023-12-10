//! Per-object motion blur.
//!
//! Add the [`MotionBlurBundle`] to a camera to enable motion blur.

use crate::core_3d;
use bevy_app::{App, Plugin};
use bevy_asset::{load_internal_asset, Handle};
use bevy_ecs::{bundle::Bundle, component::Component, schedule::IntoSystemConfigs};
use bevy_render::{
    extract_component::{ExtractComponent, ExtractComponentPlugin, UniformComponentPlugin},
    render_graph::{RenderGraphApp, ViewNodeRunner},
    render_resource::{Shader, ShaderType, SpecializedRenderPipelines},
    renderer::RenderDevice,
    Render, RenderApp, RenderSet,
};

pub mod node;
pub mod pipeline;

#[derive(Bundle, Default)]
pub struct MotionBlurBundle {
    pub motion_blur: MotionBlur,
    pub depth_prepass: crate::prepass::DepthPrepass,
    pub motion_vector_prepass: crate::prepass::MotionVectorPrepass,
}

/// A component that enables and configures motion blur when added to a camera.
#[derive(Component, Clone, Copy, Debug, ExtractComponent, ShaderType)]
pub struct MotionBlur {
    /// Camera shutter angle from 0 to 1 (0-100%), which determines the strength of the blur.
    ///
    /// The shutter angle describes the fraction of a frame that a camera's shutter is open and
    /// exposing the film/sensor. For 24fps cinematic film, a shutter angle of 0.5 (180 degrees) is
    /// common. This means that the shutter was open for half of the frame, or 1/48th of a second.
    /// The lower the shutter angle, the less exposure time and thus less blur. A value greater than
    /// one is unrealistic and results in an object's blur stretching further than it traveled in
    /// that frame.
    pub shutter_angle: f32,
    /// The upper limit for how many samples will be taken per-pixel. The number of samples taken
    /// depends on the speed of an object.
    pub max_samples: u32,
    #[cfg(all(feature = "webgl", target_arch = "wasm32"))]
    // WebGL2 structs must be 16 byte aligned.
    pub _webgl2_padding: bevy_math::Vec3,
}

impl Default for MotionBlur {
    fn default() -> Self {
        Self {
            shutter_angle: 0.5,
            max_samples: 4,
            #[cfg(all(feature = "webgl", target_arch = "wasm32"))]
            _webgl2_padding: bevy_math::Vec3::default(),
        }
    }
}

pub const MOTION_BLUR_SHADER_HANDLE: Handle<Shader> =
    Handle::weak_from_u128(987457899187986082347921);

/// Adds support for per-object motion blur to the app.
pub struct MotionBlurPlugin;
impl Plugin for MotionBlurPlugin {
    fn build(&self, app: &mut App) {
        load_internal_asset!(
            app,
            MOTION_BLUR_SHADER_HANDLE,
            "motion_blur.wgsl",
            Shader::from_wgsl
        );

        app.add_plugins((
            ExtractComponentPlugin::<MotionBlur>::default(),
            UniformComponentPlugin::<MotionBlur>::default(),
        ));

        let Ok(render_app) = app.get_sub_app_mut(RenderApp) else {
            return;
        };

        render_app
            .init_resource::<SpecializedRenderPipelines<pipeline::MotionBlurPipeline>>()
            .add_systems(
                Render,
                (pipeline::prepare_motion_blur_pipelines.in_set(RenderSet::Prepare),),
            );

        render_app
            .add_render_graph_node::<ViewNodeRunner<node::MotionBlurNode>>(
                core_3d::graph::NAME,
                core_3d::graph::node::MOTION_BLUR,
            )
            .add_render_graph_edges(
                core_3d::graph::NAME,
                &[
                    core_3d::graph::node::END_MAIN_PASS,
                    core_3d::graph::node::MOTION_BLUR,
                    core_3d::graph::node::BLOOM, // we want blurred areas to bloom and tonemap properly.
                ],
            );
    }

    fn finish(&self, app: &mut App) {
        let Ok(render_app) = app.get_sub_app_mut(RenderApp) else {
            return;
        };

        let render_device = render_app.world.resource::<RenderDevice>().clone();

        render_app.insert_resource(pipeline::MotionBlurPipeline::new(&render_device));
    }
}
