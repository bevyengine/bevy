#[cfg(target_pointer_width = "16")]
compile_error!("bevy_render cannot compile for a 16-bit platform.");

extern crate core;

pub mod camera;
pub mod color;
pub mod extract_component;
mod extract_param;
pub mod extract_resource;
pub mod globals;
pub mod mesh;
pub mod pipelined_rendering;
pub mod primitives;
pub mod render_asset;
pub mod render_graph;
pub mod render_phase;
pub mod render_resource;
pub mod renderer;
pub mod settings;
mod spatial_bundle;
pub mod texture;
pub mod view;

use bevy_hierarchy::ValidParentCheckPlugin;
pub use extract_param::Extract;

pub mod prelude {
    #[doc(hidden)]
    pub use crate::{
        camera::{Camera, OrthographicProjection, PerspectiveProjection, Projection},
        color::Color,
        mesh::{shape, Mesh},
        render_resource::Shader,
        spatial_bundle::SpatialBundle,
        texture::{Image, ImagePlugin},
        view::{ComputedVisibility, Msaa, Visibility, VisibilityBundle},
        ExtractSchedule,
    };
}

use bevy_window::{PrimaryWindow, RawHandleWrapper};
use globals::GlobalsPlugin;
pub use once_cell;

use crate::{
    camera::CameraPlugin,
    mesh::MeshPlugin,
    render_resource::{PipelineCache, Shader, ShaderLoader},
    renderer::{render_system, RenderInstance},
    settings::WgpuSettings,
    view::{ViewPlugin, WindowRenderPlugin},
};
use bevy_app::{App, AppLabel, CoreSchedule, Plugin, SubApp};
use bevy_asset::{AddAsset, AssetServer};
use bevy_ecs::{prelude::*, schedule::ScheduleLabel, system::SystemState};
use bevy_utils::tracing::debug;
use std::ops::{Deref, DerefMut};

/// Contains the default Bevy rendering backend based on wgpu.
#[derive(Default)]
pub struct RenderPlugin {
    pub wgpu_settings: WgpuSettings,
}

/// The labels of the default App rendering sets.
///
/// The sets run in the order listed, with [`apply_system_buffers`] inserted between each set.
///
/// The `*Flush` sets are assigned to the copy of [`apply_system_buffers`]
/// that runs immediately after the matching system set.
/// These can be useful for ordering, but you almost never want to add your systems to these sets.
#[derive(Debug, Hash, PartialEq, Eq, Clone, SystemSet)]
pub enum RenderSet {
    /// The copy of [`apply_system_buffers`] that runs at the begining of this schedule.
    /// This is used for applying the commands from the [`ExtractSchedule`]
    ExtractCommands,
    /// Prepare render resources from the extracted data for the GPU.
    Prepare,
    /// The copy of [`apply_system_buffers`] that runs immediately after `Prepare`.
    PrepareFlush,
    /// Create [`BindGroups`](crate::render_resource::BindGroup) that depend on
    /// [`Prepare`](RenderSet::Prepare) data and queue up draw calls to run during the
    /// [`Render`](RenderSet::Render) step.
    Queue,
    /// The copy of [`apply_system_buffers`] that runs immediately after `Queue`.
    QueueFlush,
    // TODO: This could probably be moved in favor of a system ordering abstraction in Render or Queue
    /// Sort the [`RenderPhases`](crate::render_phase::RenderPhase) here.
    PhaseSort,
    /// The copy of [`apply_system_buffers`] that runs immediately after `PhaseSort`.
    PhaseSortFlush,
    /// Actual rendering happens here.
    /// In most cases, only the render backend should insert resources here.
    Render,
    /// The copy of [`apply_system_buffers`] that runs immediately after `Render`.
    RenderFlush,
    /// Cleanup render resources here.
    Cleanup,
    /// The copy of [`apply_system_buffers`] that runs immediately after `Cleanup`.
    CleanupFlush,
}

impl RenderSet {
    /// Sets up the base structure of the rendering [`Schedule`].
    ///
    /// The sets defined in this enum are configured to run in order,
    /// and a copy of [`apply_system_buffers`] is inserted at each `*Flush` label.
    pub fn base_schedule() -> Schedule {
        use RenderSet::*;

        let mut schedule = Schedule::new();

        // Create "stage-like" structure using buffer flushes + ordering
        schedule.add_system(apply_system_buffers.in_set(PrepareFlush));
        schedule.add_system(apply_system_buffers.in_set(QueueFlush));
        schedule.add_system(apply_system_buffers.in_set(PhaseSortFlush));
        schedule.add_system(apply_system_buffers.in_set(RenderFlush));
        schedule.add_system(apply_system_buffers.in_set(CleanupFlush));

        schedule.configure_set(ExtractCommands.before(Prepare));
        schedule.configure_set(Prepare.after(ExtractCommands).before(PrepareFlush));
        schedule.configure_set(Queue.after(PrepareFlush).before(QueueFlush));
        schedule.configure_set(PhaseSort.after(QueueFlush).before(PhaseSortFlush));
        schedule.configure_set(Render.after(PhaseSortFlush).before(RenderFlush));
        schedule.configure_set(Cleanup.after(RenderFlush).before(CleanupFlush));

        schedule
    }
}

/// Schedule which extract data from the main world and inserts it into the render world.
///
/// This step should be kept as short as possible to increase the "pipelining potential" for
/// running the next frame while rendering the current frame.
///
/// This schedule is run on the main world, but its buffers are not applied
/// via [`Schedule::apply_system_buffers`](bevy_ecs::schedule::Schedule) until it is returned to the render world.
#[derive(ScheduleLabel, PartialEq, Eq, Debug, Clone, Hash)]
pub struct ExtractSchedule;

/// The simulation [`World`] of the application, stored as a resource.
/// This resource is only available during [`ExtractSchedule`] and not
/// during command application of that schedule.
/// See [`Extract`] for more details.
#[derive(Resource, Default)]
pub struct MainWorld(World);

impl Deref for MainWorld {
    type Target = World;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for MainWorld {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

pub mod main_graph {
    pub mod node {
        pub const CAMERA_DRIVER: &str = "camera_driver";
    }
}

/// A Label for the rendering sub-app.
#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq, AppLabel)]
pub struct RenderApp;

impl Plugin for RenderPlugin {
    /// Initializes the renderer, sets up the [`RenderSet`](RenderSet) and creates the rendering sub-app.
    fn build(&self, app: &mut App) {
        app.add_asset::<Shader>()
            .add_debug_asset::<Shader>()
            .init_asset_loader::<ShaderLoader>()
            .init_debug_asset_loader::<ShaderLoader>();

        let mut system_state: SystemState<Query<&RawHandleWrapper, With<PrimaryWindow>>> =
            SystemState::new(&mut app.world);
        let primary_window = system_state.get(&app.world);

        if let Some(backends) = self.wgpu_settings.backends {
            let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
                backends,
                dx12_shader_compiler: self.wgpu_settings.dx12_shader_compiler.clone(),
            });
            let surface = primary_window.get_single().ok().map(|wrapper| unsafe {
                // SAFETY: Plugins should be set up on the main thread.
                let handle = wrapper.get_handle();
                instance
                    .create_surface(&handle)
                    .expect("Failed to create wgpu surface")
            });

            let request_adapter_options = wgpu::RequestAdapterOptions {
                power_preference: self.wgpu_settings.power_preference,
                compatible_surface: surface.as_ref(),
                ..Default::default()
            };
            let (device, queue, adapter_info, render_adapter) =
                futures_lite::future::block_on(renderer::initialize_renderer(
                    &instance,
                    &self.wgpu_settings,
                    &request_adapter_options,
                ));
            debug!("Configured wgpu adapter Limits: {:#?}", device.limits());
            debug!("Configured wgpu adapter Features: {:#?}", device.features());
            app.insert_resource(device.clone())
                .insert_resource(queue.clone())
                .insert_resource(adapter_info.clone())
                .insert_resource(render_adapter.clone())
                .init_resource::<ScratchMainWorld>();

            let pipeline_cache = PipelineCache::new(device.clone());
            let asset_server = app.world.resource::<AssetServer>().clone();

            let mut render_app = App::empty();
            render_app.add_simple_outer_schedule();
            let mut render_schedule = RenderSet::base_schedule();

            // Prepare the schedule which extracts data from the main world to the render world
            render_app.edit_schedule(ExtractSchedule, |schedule| {
                schedule
                    .set_apply_final_buffers(false)
                    .add_system(PipelineCache::extract_shaders);
            });

            // This set applies the commands from the extract stage while the render schedule
            // is running in parallel with the main app.
            render_schedule.add_system(apply_extract_commands.in_set(RenderSet::ExtractCommands));

            render_schedule.add_system(
                PipelineCache::process_pipeline_queue_system
                    .before(render_system)
                    .in_set(RenderSet::Render),
            );
            render_schedule.add_system(render_system.in_set(RenderSet::Render));

            render_schedule.add_system(World::clear_entities.in_set(RenderSet::Cleanup));

            render_app
                .add_schedule(CoreSchedule::Main, render_schedule)
                .init_resource::<render_graph::RenderGraph>()
                .insert_resource(RenderInstance(instance))
                .insert_resource(device)
                .insert_resource(queue)
                .insert_resource(render_adapter)
                .insert_resource(adapter_info)
                .insert_resource(pipeline_cache)
                .insert_resource(asset_server);

            let (sender, receiver) = bevy_time::create_time_channels();
            app.insert_resource(receiver);
            render_app.insert_resource(sender);

            app.insert_sub_app(RenderApp, SubApp::new(render_app, move |main_world, render_app| {
                #[cfg(feature = "trace")]
                let _render_span = bevy_utils::tracing::info_span!("extract main app to render subapp").entered();
                {
                    #[cfg(feature = "trace")]
                    let _stage_span =
                        bevy_utils::tracing::info_span!("reserve_and_flush")
                            .entered();

                    // reserve all existing main world entities for use in render_app
                    // they can only be spawned using `get_or_spawn()`
                    let total_count = main_world.entities().total_count();

                    assert_eq!(
                        render_app.world.entities().len(),
                        0,
                        "An entity was spawned after the entity list was cleared last frame and before the extract schedule began. This is not supported",
                    );

                    // This is safe given the clear_entities call in the past frame and the assert above
                    unsafe {
                        render_app
                            .world
                            .entities_mut()
                            .flush_and_reserve_invalid_assuming_no_entities(total_count);
                    }
                }

                // run extract schedule
                extract(main_world, render_app);
            }));
        }

        app.add_plugin(ValidParentCheckPlugin::<view::ComputedVisibility>::default())
            .add_plugin(WindowRenderPlugin)
            .add_plugin(CameraPlugin)
            .add_plugin(ViewPlugin)
            .add_plugin(MeshPlugin)
            .add_plugin(GlobalsPlugin);

        app.register_type::<color::Color>()
            .register_type::<primitives::Aabb>()
            .register_type::<primitives::CascadesFrusta>()
            .register_type::<primitives::CubemapFrusta>()
            .register_type::<primitives::Frustum>();
    }
}

/// A "scratch" world used to avoid allocating new worlds every frame when
/// swapping out the [`MainWorld`] for [`ExtractSchedule`].
#[derive(Resource, Default)]
struct ScratchMainWorld(World);

/// Executes the [`ExtractSchedule`] step of the renderer.
/// This updates the render world with the extracted ECS data of the current frame.
fn extract(main_world: &mut World, render_app: &mut App) {
    // temporarily add the app world to the render world as a resource
    let scratch_world = main_world.remove_resource::<ScratchMainWorld>().unwrap();
    let inserted_world = std::mem::replace(main_world, scratch_world.0);
    render_app.world.insert_resource(MainWorld(inserted_world));

    render_app.world.run_schedule(ExtractSchedule);

    // move the app world back, as if nothing happened.
    let inserted_world = render_app.world.remove_resource::<MainWorld>().unwrap();
    let scratch_world = std::mem::replace(main_world, inserted_world.0);
    main_world.insert_resource(ScratchMainWorld(scratch_world));
}

/// Applies the commands from the extract schedule. This happens during
/// the render schedule rather than during extraction to allow the commands to run in parallel with the
/// main app when pipelined rendering is enabled.
fn apply_extract_commands(render_world: &mut World) {
    render_world.resource_scope(|render_world, mut schedules: Mut<Schedules>| {
        schedules
            .get_mut(&ExtractSchedule)
            .unwrap()
            .apply_system_buffers(render_world);
    });
}
