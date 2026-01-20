use bevy_camera::{ClearColor, NormalizedRenderTarget};
use bevy_ecs::{
    prelude::*,
    schedule::{IntoScheduleConfigs, Schedule, ScheduleLabel, SystemSet},
};
use bevy_platform::collections::HashSet;
use bevy_render::{
    camera::{ExtractedCamera, SortedCameras},
    render_resource::{
        CommandEncoderDescriptor, LoadOp, Operations, RenderPassColorAttachment,
        RenderPassDescriptor, StoreOp,
    },
    renderer::{CurrentViewEntity, PendingCommandBuffers, RenderDevice, RenderQueue},
    view::ExtractedWindows,
};
use tracing::info_span;

/// Schedule label for the Core 3D rendering pipeline.
#[derive(ScheduleLabel, Debug, Clone, PartialEq, Eq, Hash, Default)]
pub struct Core3d;

#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub enum Core3dSystems {
    Prepass,
    MainPass,
    PostProcess,
}

impl Core3d {
    pub fn base_schedule() -> Schedule {
        use bevy_ecs::schedule::ScheduleBuildSettings;
        use Core3dSystems::*;

        let mut schedule = Schedule::new(Self);

        schedule.set_build_settings(ScheduleBuildSettings {
            auto_insert_apply_deferred: false,
            ..Default::default()
        });

        schedule.configure_sets((Prepass, MainPass, PostProcess).chain());

        schedule
    }
}

#[derive(ScheduleLabel, Debug, Clone, PartialEq, Eq, Hash, Default)]
pub struct Core2d;

#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub enum Core2dSystems {
    MainPass,
    PostProcess,
}

impl Core2d {
    pub fn base_schedule() -> Schedule {
        use bevy_ecs::schedule::ScheduleBuildSettings;
        use Core2dSystems::*;

        let mut schedule = Schedule::new(Self);

        schedule.set_build_settings(ScheduleBuildSettings {
            auto_insert_apply_deferred: false,
            ..Default::default()
        });

        schedule.configure_sets((MainPass, PostProcess).chain());

        schedule
    }
}

pub fn camera_driver(world: &mut World) {
    let sorted_cameras: Vec<_> = {
        let sorted = world.resource::<SortedCameras>();
        sorted.0.iter().map(|c| (c.entity, c.order)).collect()
    };

    let mut camera_windows = HashSet::default();

    for camera in sorted_cameras {
        #[cfg(feature = "trace")]
        let (camera_entity, order) = camera;
        #[cfg(not(feature = "trace"))]
        let (camera_entity, _) = camera;
        let Some(camera) = world.get::<ExtractedCamera>(camera_entity) else {
            continue;
        };

        let schedule = camera.schedule;
        let target = camera.target.clone();

        let mut run_schedule = true;
        if let Some(NormalizedRenderTarget::Window(window_ref)) = &target {
            let window_entity = window_ref.entity();
            let windows = world.resource::<ExtractedWindows>();
            if windows
                .windows
                .get(&window_entity)
                .is_some_and(|w| w.physical_width > 0 && w.physical_height > 0)
            {
                camera_windows.insert(window_entity);
            } else {
                run_schedule = false;
            }
        }

        if run_schedule {
            world.insert_resource(CurrentViewEntity::new(camera_entity));

            #[cfg(feature = "trace")]
            let _span = tracing::info_span!(
                "camera_schedule",
                camera = format!("Camera {} ({:?})", order, camera_entity)
            )
            .entered();

            world.run_schedule(schedule);
            submit_pending_command_buffers(world);
        }
    }

    world.remove_resource::<CurrentViewEntity>();
    handle_uncovered_swap_chains(world, &camera_windows);
}

fn submit_pending_command_buffers(world: &mut World) {
    let mut pending = world.resource_mut::<PendingCommandBuffers>();
    let buffer_count = pending.len();
    let buffers = pending.take();

    if !buffers.is_empty() {
        let _span = info_span!("queue_submit", count = buffer_count).entered();
        let queue = world.resource::<RenderQueue>();
        queue.submit(buffers);
    }
}

fn handle_uncovered_swap_chains(world: &mut World, camera_windows: &HashSet<Entity>) {
    let windows_to_clear: Vec<_> = {
        let clear_color = world.resource::<ClearColor>().0.to_linear();
        let windows = world.resource::<ExtractedWindows>();

        windows
            .iter()
            .filter_map(|(window_entity, window)| {
                if camera_windows.contains(window_entity) {
                    return None;
                }
                let swap_chain_texture = window.swap_chain_texture_view.as_ref()?;
                Some((swap_chain_texture.clone(), clear_color))
            })
            .collect()
    };

    if windows_to_clear.is_empty() {
        return;
    }

    let render_device = world.resource::<RenderDevice>();
    let render_queue = world.resource::<RenderQueue>();

    let mut encoder = render_device.create_command_encoder(&CommandEncoderDescriptor::default());

    for (swap_chain_texture, clear_color) in &windows_to_clear {
        #[cfg(feature = "trace")]
        let _span = tracing::info_span!("no_camera_clear_pass").entered();

        let pass_descriptor = RenderPassDescriptor {
            label: Some("no_camera_clear_pass"),
            color_attachments: &[Some(RenderPassColorAttachment {
                view: swap_chain_texture,
                depth_slice: None,
                resolve_target: None,
                ops: Operations {
                    load: LoadOp::Clear((*clear_color).into()),
                    store: StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
        };

        encoder.begin_render_pass(&pass_descriptor);
    }

    render_queue.submit([encoder.finish()]);
}
