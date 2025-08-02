//! NVIDIA Deep Learning Super Sampling (DLSS).
//!
//! DLSS uses machine learning models to upscale and anti-alias images.
//!
//! Requires a NVIDIA RTX GPU, and the Windows/Linux Vulkan rendering backend. Does not work on other platforms.
//!
//! See https://github.com/bevyengine/dlss_wgpu for licensing requirements and setup instructions.
//!
//! # Usage
//! 1. Enable Bevy's `dlss` feature
//! 2. During app setup, insert the `DlssProjectId` resource before `DefaultPlugins`
//! 3. Check for the presence of `Option<Res<DlssSupported>>` at runtime to see if DLSS is supported on the current machine
//! 4. Add the `Dlss` component to your camera entity, optionally setting a specific `DlssPerfQualityMode` (defaults to `Auto`)
//! 5. Optionally add sharpening via `ContrastAdaptiveSharpening`
//! 6. Custom rendering code, including third party crates, should account for the optional `MainPassResolutionOverride` to work with DLSS (see the `custom_render_phase` example)

mod extract;
mod node;
mod prepare;

use bevy_app::{App, Plugin};
use bevy_core_pipeline::{
    core_3d::graph::{Core3d, Node3d},
    prepass::{DepthPrepass, MotionVectorPrepass},
};
use bevy_ecs::{
    component::Component, prelude::ReflectComponent, resource::Resource,
    schedule::IntoScheduleConfigs,
};
use bevy_reflect::{prelude::ReflectDefault, reflect_remote, Reflect};
use bevy_render::{
    camera::{MipBias, TemporalJitter},
    render_graph::{RenderGraphExt, ViewNodeRunner},
    renderer::RenderDevice,
    view::{prepare_view_targets, Hdr},
    ExtractSchedule, Render, RenderApp, RenderSystems,
};
use std::sync::{Arc, Mutex};
use tracing::info;

pub use bevy_render::{
    DlssProjectId, DlssRayReconstructionSupported, DlssSuperResolutionSupported,
};
pub use dlss_wgpu::DlssPerfQualityMode;

pub struct DlssPlugin;

impl Plugin for DlssPlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<Dlss>();
    }

    fn finish(&self, app: &mut App) {
        if app
            .world()
            .get_resource::<DlssSuperResolutionSupported>()
            .is_none()
        {
            info!("DLSS is not supported on this system");
            return;
        }

        let dlss_project_id = app.world().resource::<DlssProjectId>().0;

        let render_app = app.get_sub_app_mut(RenderApp).unwrap();
        let render_device = render_app.world().resource::<RenderDevice>().clone();

        let dlss_sdk =
            dlss_wgpu::DlssSdk::new(dlss_project_id, render_device.wgpu_device().clone());
        if dlss_sdk.is_err() {
            app.world_mut()
                .remove_resource::<DlssSuperResolutionSupported>();
            info!("DLSS is not supported on this system");
            return;
        }

        render_app
            .insert_resource(DlssSdk(dlss_sdk.unwrap()))
            .add_systems(ExtractSchedule, extract::extract_dlss)
            .add_systems(
                Render,
                prepare::prepare_dlss
                    .in_set(RenderSystems::ManageViews)
                    .before(prepare_view_targets),
            )
            .add_render_graph_node::<ViewNodeRunner<node::DlssNode>>(Core3d, Node3d::Dlss)
            .add_render_graph_edges(
                Core3d,
                (
                    Node3d::EndMainPass,
                    Node3d::MotionBlur, // Running before DLSS reduces edge artifacts and noise
                    Node3d::Dlss,
                    Node3d::Bloom,
                    Node3d::Tonemapping,
                ),
            );
    }
}

/// Camera component to enable DLSS.
#[derive(Component, Reflect, Clone, Default)]
#[reflect(Component, Default)]
#[require(TemporalJitter, MipBias, DepthPrepass, MotionVectorPrepass, Hdr)]
pub struct Dlss {
    #[reflect(remote = DlssPerfQualityModeRemoteReflect)]
    pub perf_quality_mode: DlssPerfQualityMode,
    pub reset: bool,
}

#[reflect_remote(DlssPerfQualityMode)]
#[derive(Default)]
enum DlssPerfQualityModeRemoteReflect {
    #[default]
    Auto,
    Dlaa,
    Quality,
    Balanced,
    Performance,
    UltraPerformance,
}

#[derive(Resource)]
struct DlssSdk(Arc<Mutex<dlss_wgpu::DlssSdk>>);
