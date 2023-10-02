#![allow(clippy::type_complexity)]

#[cfg(target_pointer_width = "16")]
compile_error!("bevy_render cannot compile for a 16-bit platform.");

extern crate core;

pub mod batching;
pub mod camera;
pub mod color;
pub mod extract_component;
mod extract_param;
pub mod extract_resource;
pub mod globals;
pub mod gpu_component_array_buffer;
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
pub mod prelude {
    #[doc(hidden)]
    pub use crate::{
        camera::{Camera, OrthographicProjection, PerspectiveProjection, Projection},
        color::Color,
        mesh::{morph::MorphWeights, shape, Mesh},
        render_resource::Shader,
        spatial_bundle::SpatialBundle,
        texture::{Image, ImagePlugin},
        view::{InheritedVisibility, Msaa, ViewVisibility, Visibility, VisibilityBundle},
        ExtractSchedule,
    };
}

pub use extract_param::Extract;

use bevy_hierarchy::ValidParentCheckPlugin;
use bevy_window::{PrimaryWindow, RawHandleWrapper};
use globals::GlobalsPlugin;
use renderer::{RenderAdapter, RenderAdapterInfo, RenderDevice, RenderQueue};

use crate::{
    camera::CameraPlugin,
    mesh::{morph::MorphPlugin, Mesh, MeshPlugin},
    render_asset::prepare_assets,
    render_resource::{PipelineCache, Shader, ShaderLoader},
    renderer::{render_system, RenderInstance},
    settings::RenderCreation,
    view::{ViewPlugin, WindowRenderPlugin},
};
use bevy_app::{App, AppLabel, Plugin, SubApp};
use bevy_asset::{load_internal_asset, AssetApp, AssetServer, Handle};
use bevy_ecs::{prelude::*, schedule::ScheduleLabel, system::SystemState};
use bevy_utils::tracing::debug;
use std::{
    ops::{Deref, DerefMut},
    sync::{Arc, Mutex},
};

/// Contains the default Bevy rendering backend based on wgpu.
#[derive(Default)]
pub struct RenderPlugin {
    pub render_creation: RenderCreation,
}

/// The labels of the default App rendering sets.
///
/// The sets run in the order listed, with [`apply_deferred`] inserted between each set.
///
/// The `*Flush` sets are assigned to the copy of [`apply_deferred`]
/// that runs immediately after the matching system set.
/// These can be useful for ordering, but you almost never want to add your systems to these sets.
#[derive(Debug, Hash, PartialEq, Eq, Clone, SystemSet)]
pub enum RenderSet {
    /// The copy of [`apply_deferred`] that runs at the beginning of this schedule.
    /// This is used for applying the commands from the [`ExtractSchedule`]
    ExtractCommands,
    /// Prepare assets that have been created/modified/removed this frame.
    PrepareAssets,
    /// Create any additional views such as those used for shadow mapping.
    ManageViews,
    /// The copy of [`apply_deferred`] that runs immediately after [`ManageViews`](RenderSet::ManageViews).
    ManageViewsFlush,
    /// Queue drawable entities as phase items in [`RenderPhase`](crate::render_phase::RenderPhase)s
    /// ready for sorting
    Queue,
    /// A sub-set within Queue where mesh entity queue systems are executed. Ensures `prepare_assets::<Mesh>` is completed.
    QueueMeshes,
    // TODO: This could probably be moved in favor of a system ordering abstraction in Render or Queue
    /// Sort the [`RenderPhases`](render_phase::RenderPhase) here.
    PhaseSort,
    /// Prepare render resources from extracted data for the GPU based on their sorted order.
    /// Create [`BindGroups`](crate::render_resource::BindGroup) that depend on those data.
    Prepare,
    /// A sub-set within Prepare for initializing buffers, textures and uniforms for use in bind groups.
    PrepareResources,
    /// The copy of [`apply_deferred`] that runs between [`PrepareResources`](RenderSet::PrepareResources) and ['PrepareBindGroups'](RenderSet::PrepareBindGroups).
    PrepareResourcesFlush,
    /// A sub-set within Prepare for constructing bind groups, or other data that relies on render resources prepared in [`PrepareResources`](RenderSet::PrepareResources).
    PrepareBindGroups,
    /// The copy of [`apply_deferred`] that runs immediately after [`Prepare`](RenderSet::Prepare).
    PrepareFlush,
    /// Actual rendering happens here.
    /// In most cases, only the render backend should insert resources here.
    Render,
    /// The copy of [`apply_deferred`] that runs immediately after [`Render`](RenderSet::Render).
    RenderFlush,
    /// Cleanup render resources here.
    Cleanup,
    /// The copy of [`apply_deferred`] that runs immediately after [`Cleanup`](RenderSet::Cleanup).
    CleanupFlush,
}

/// The main render schedule.
#[derive(ScheduleLabel, Debug, Hash, PartialEq, Eq, Clone)]
pub struct Render;

impl Render {
    /// Sets up the base structure of the rendering [`Schedule`].
    ///
    /// The sets defined in this enum are configured to run in order,
    /// and a copy of [`apply_deferred`] is inserted into each `*Flush` set.
    pub fn base_schedule() -> Schedule {
        use RenderSet::*;

        let mut schedule = Schedule::new(Self);

        // Create "stage-like" structure using buffer flushes + ordering
        schedule.add_systems((
            apply_deferred.in_set(ManageViewsFlush),
            apply_deferred.in_set(PrepareResourcesFlush),
            apply_deferred.in_set(RenderFlush),
            apply_deferred.in_set(PrepareFlush),
            apply_deferred.in_set(CleanupFlush),
        ));

        schedule.configure_sets(
            (
                ExtractCommands,
                ManageViews,
                ManageViewsFlush,
                Queue,
                PhaseSort,
                Prepare,
                PrepareFlush,
                Render,
                RenderFlush,
                Cleanup,
                CleanupFlush,
            )
                .chain(),
        );

        schedule.configure_sets((ExtractCommands, PrepareAssets, Prepare).chain());
        schedule.configure_sets(
            QueueMeshes
                .in_set(RenderSet::Queue)
                .after(prepare_assets::<Mesh>),
        );
        schedule.configure_sets(
            (PrepareResources, PrepareResourcesFlush, PrepareBindGroups)
                .chain()
                .in_set(Prepare),
        );

        schedule
    }
}

/// Schedule which extract data from the main world and inserts it into the render world.
///
/// This step should be kept as short as possible to increase the "pipelining potential" for
/// running the next frame while rendering the current frame.
///
/// This schedule is run on the main world, but its buffers are not applied
/// via [`Schedule::apply_deferred`](bevy_ecs::schedule::Schedule) until it is returned to the render world.
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

#[derive(Resource)]
struct FutureRendererResources(
    Arc<
        Mutex<
            Option<(
                RenderDevice,
                RenderQueue,
                RenderAdapterInfo,
                RenderAdapter,
                RenderInstance,
            )>,
        >,
    >,
);

/// A Label for the rendering sub-app.
#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq, AppLabel)]
pub struct RenderApp;

pub const INSTANCE_INDEX_SHADER_HANDLE: Handle<Shader> =
    Handle::weak_from_u128(10313207077636615845);
pub const MATHS_SHADER_HANDLE: Handle<Shader> = Handle::weak_from_u128(10665356303104593376);

impl Plugin for RenderPlugin {
    /// Initializes the renderer, sets up the [`RenderSet`](RenderSet) and creates the rendering sub-app.
    fn build(&self, app: &mut App) {
        app.init_asset::<Shader>()
            .init_asset_loader::<ShaderLoader>();

        match &self.render_creation {
            RenderCreation::Manual(device, queue, adapter_info, adapter, instance) => {
                let future_renderer_resources_wrapper = Arc::new(Mutex::new(Some((
                    device.clone(),
                    queue.clone(),
                    adapter_info.clone(),
                    adapter.clone(),
                    instance.clone(),
                ))));
                app.insert_resource(FutureRendererResources(
                    future_renderer_resources_wrapper.clone(),
                ));
                unsafe { initialize_render_app(app) };
            }
            RenderCreation::Automatic(render_creation) => {
                if let Some(backends) = render_creation.backends {
                    let future_renderer_resources_wrapper = Arc::new(Mutex::new(None));
                    app.insert_resource(FutureRendererResources(
                        future_renderer_resources_wrapper.clone(),
                    ));

                    let mut system_state: SystemState<
                        Query<&RawHandleWrapper, With<PrimaryWindow>>,
                    > = SystemState::new(&mut app.world);
                    let primary_window = system_state.get(&app.world).get_single().ok().cloned();

                    let settings = render_creation.clone();
                    let async_renderer = async move {
                        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
                            backends,
                            dx12_shader_compiler: settings.dx12_shader_compiler.clone(),
                        });
                        let surface = primary_window.map(|wrapper| unsafe {
                            // SAFETY: Plugins should be set up on the main thread.
                            let handle = wrapper.get_handle();
                            instance
                                .create_surface(&handle)
                                .expect("Failed to create wgpu surface")
                        });

                        let request_adapter_options = wgpu::RequestAdapterOptions {
                            power_preference: settings.power_preference,
                            compatible_surface: surface.as_ref(),
                            ..Default::default()
                        };

                        let (device, queue, adapter_info, render_adapter) =
                            renderer::initialize_renderer(
                                &instance,
                                &settings,
                                &request_adapter_options,
                            )
                            .await;
                        debug!("Configured wgpu adapter Limits: {:#?}", device.limits());
                        debug!("Configured wgpu adapter Features: {:#?}", device.features());
                        let mut future_renderer_resources_inner =
                            future_renderer_resources_wrapper.lock().unwrap();
                        *future_renderer_resources_inner = Some((
                            device,
                            queue,
                            adapter_info,
                            render_adapter,
                            RenderInstance(Arc::new(instance)),
                        ));
                    };
                    // In wasm, spawn a task and detach it for execution
                    #[cfg(target_arch = "wasm32")]
                    bevy_tasks::IoTaskPool::get()
                        .spawn_local(async_renderer)
                        .detach();
                    // Otherwise, just block for it to complete
                    #[cfg(not(target_arch = "wasm32"))]
                    futures_lite::future::block_on(async_renderer);

                    unsafe { initialize_render_app(app) };
                }
            }
        };

        app.add_plugins((
            ValidParentCheckPlugin::<view::InheritedVisibility>::default(),
            WindowRenderPlugin,
            CameraPlugin,
            ViewPlugin,
            MeshPlugin,
            GlobalsPlugin,
            MorphPlugin,
        ));

        app.register_type::<color::Color>()
            .register_type::<primitives::Aabb>()
            .register_type::<primitives::CascadesFrusta>()
            .register_type::<primitives::CubemapFrusta>()
            .register_type::<primitives::Frustum>();
    }

    fn ready(&self, app: &App) -> bool {
        app.world
            .get_resource::<FutureRendererResources>()
            .and_then(|frr| frr.0.try_lock().map(|locked| locked.is_some()).ok())
            .unwrap_or(true)
    }

    fn finish(&self, app: &mut App) {
        load_internal_asset!(
            app,
            INSTANCE_INDEX_SHADER_HANDLE,
            "instance_index.wgsl",
            Shader::from_wgsl_with_defs,
            vec![
                #[cfg(all(feature = "webgl", target_arch = "wasm32"))]
                "BASE_INSTANCE_WORKAROUND".into()
            ]
        );
        load_internal_asset!(app, MATHS_SHADER_HANDLE, "maths.wgsl", Shader::from_wgsl);
        if let Some(future_renderer_resources) =
            app.world.remove_resource::<FutureRendererResources>()
        {
            let (device, queue, adapter_info, render_adapter, instance) =
                future_renderer_resources.0.lock().unwrap().take().unwrap();

            app.insert_resource(device.clone())
                .insert_resource(queue.clone())
                .insert_resource(adapter_info.clone())
                .insert_resource(render_adapter.clone());

            let render_app = app.sub_app_mut(RenderApp);

            render_app
                .insert_resource(instance)
                .insert_resource(PipelineCache::new(device.clone()))
                .insert_resource(device)
                .insert_resource(queue)
                .insert_resource(render_adapter)
                .insert_resource(adapter_info);
        }
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

/// SAFETY: this function must be called from the main thread.
unsafe fn initialize_render_app(app: &mut App) {
    app.init_resource::<ScratchMainWorld>();

    let mut render_app = App::empty();
    render_app.main_schedule_label = Box::new(Render);

    let mut extract_schedule = Schedule::new(ExtractSchedule);
    extract_schedule.set_apply_final_deferred(false);

    render_app
        .add_schedule(extract_schedule)
        .add_schedule(Render::base_schedule())
        .init_resource::<render_graph::RenderGraph>()
        .insert_resource(app.world.resource::<AssetServer>().clone())
        .add_systems(ExtractSchedule, PipelineCache::extract_shaders)
        .add_systems(
            Render,
            (
                // This set applies the commands from the extract schedule while the render schedule
                // is running in parallel with the main app.
                apply_extract_commands.in_set(RenderSet::ExtractCommands),
                (
                    PipelineCache::process_pipeline_queue_system.before(render_system),
                    render_system,
                )
                    .in_set(RenderSet::Render),
                World::clear_entities.in_set(RenderSet::Cleanup),
            ),
        );

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

/// Applies the commands from the extract schedule. This happens during
/// the render schedule rather than during extraction to allow the commands to run in parallel with the
/// main app when pipelined rendering is enabled.
fn apply_extract_commands(render_world: &mut World) {
    render_world.resource_scope(|render_world, mut schedules: Mut<Schedules>| {
        schedules
            .get_mut(&ExtractSchedule)
            .unwrap()
            .apply_deferred(render_world);
    });
}
