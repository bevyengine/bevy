#[cfg(target_pointer_width = "16")]
compile_error!("bevy_render cannot compile for a 16-bit platform.");

extern crate core;

pub mod as_bind_group;
pub mod camera;
pub mod color;
pub mod extract_component;
mod extract_param;
pub mod extract_resource;
pub mod globals;
pub mod mesh;
pub mod primitives;
pub mod rangefinder;
pub mod render_asset;
pub mod render_graph;
pub mod render_phase;
mod spatial_bundle;
pub mod texture;
pub mod view;

pub use as_bind_group::*;
pub use extract_param::Extract;
pub use once_cell;

pub mod prelude {
    #[doc(hidden)]
    pub use crate::{
        camera::{Camera, OrthographicProjection, PerspectiveProjection, Projection},
        color::Color,
        mesh::{shape, Mesh},
        spatial_bundle::SpatialBundle,
        texture::{Image, ImagePlugin},
        view::{ComputedVisibility, Msaa, Visibility, VisibilityBundle},
    };
}

use crate::{
    camera::CameraPlugin,
    mesh::MeshPlugin,
    render_graph::render_system,
    view::{ViewPlugin, WindowRenderPlugin},
};
use bevy_app::{App, AppLabel, Plugin};
use bevy_asset::{AssetEvent, AssetServer, Assets};
use bevy_ecs::prelude::*;
use bevy_gpu::{Adapter, AdapterInfo, Device, Instance, PipelineCache, Queue, Settings, Shader};
use bevy_hierarchy::ValidParentCheckPlugin;
use globals::GlobalsPlugin;
use std::{
    any::TypeId,
    ops::{Deref, DerefMut},
};

/// Contains the default Bevy rendering backend based on wgpu.
#[derive(Default)]
pub struct RenderPlugin {
    pub settings: Settings,
}

/// The labels of the default App rendering stages.
#[derive(Debug, Hash, PartialEq, Eq, Clone, StageLabel)]
pub enum RenderStage {
    /// Extract data from the "app world" and insert it into the "render world".
    /// This step should be kept as short as possible to increase the "pipelining potential" for
    /// running the next frame while rendering the current frame.
    Extract,

    /// Prepare render resources from the extracted data for the GPU.
    Prepare,

    /// Create [`BindGroups`](crate::render_resource::BindGroup) that depend on
    /// [`Prepare`](RenderStage::Prepare) data and queue up draw calls to run during the
    /// [`Render`](RenderStage::Render) stage.
    Queue,

    // TODO: This could probably be moved in favor of a system ordering abstraction in Render or Queue
    /// Sort the [`RenderPhases`](crate::render_phase::RenderPhase) here.
    PhaseSort,

    /// Actual rendering happens here.
    /// In most cases, only the render backend should insert resources here.
    Render,

    /// Cleanup render resources here.
    Cleanup,
}

/// The simulation [`World`] of the application, stored as a resource.
/// This resource is only available during [`RenderStage::Extract`] and not
/// during command application of that stage.
/// See [`Extract`] for more details.
#[derive(Resource, Default)]
pub struct MainWorld(World);

/// The Render App World. This is only available as a resource during the Extract step.
#[derive(Resource, Default)]
pub struct RenderWorld(World);

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
    /// Initializes the renderer, sets up the [`RenderStage`](RenderStage) and creates the rendering sub-app.
    fn build(&self, app: &mut App) {
        app.init_resource::<ScratchMainWorld>();

        let instance = app.world.get_resource::<Instance>();
        let device = app.world.get_resource::<Device>();
        let queue = app.world.get_resource::<Queue>();
        let adapter = app.world.get_resource::<Adapter>();
        let adapter_info = app.world.get_resource::<AdapterInfo>();

        // Todo: replace with if let chain
        if let (Some(instance), Some(device), Some(queue), Some(adapter), Some(adapter_info)) =
            (instance, device, queue, adapter, adapter_info)
        {
            let pipeline_cache = PipelineCache::new(device.clone());
            let asset_server = app.world.resource::<AssetServer>().clone();

            let mut render_app = App::empty();
            let mut extract_stage = SystemStage::parallel().with_system(extract_shaders);
            // Get the ComponentId for MainWorld. This does technically 'waste' a `WorldId`, but that's probably fine
            render_app.init_resource::<MainWorld>();
            render_app.world.remove_resource::<MainWorld>();
            let main_world_in_render = render_app
                .world
                .components()
                .get_resource_id(TypeId::of::<MainWorld>());
            // `Extract` systems must read from the main world. We want to emit an error when that doesn't occur
            // Safe to unwrap: Ensured it existed just above
            extract_stage.set_must_read_resource(main_world_in_render.unwrap());
            // don't apply buffers when the stage finishes running
            // extract stage runs on the render world, but buffers are applied
            // after access to the main world is removed
            // See also https://github.com/bevyengine/bevy/issues/5082
            extract_stage.set_apply_buffers(false);
            render_app
                .add_stage(RenderStage::Extract, extract_stage)
                .add_stage(RenderStage::Prepare, SystemStage::parallel())
                .add_stage(RenderStage::Queue, SystemStage::parallel())
                .add_stage(RenderStage::PhaseSort, SystemStage::parallel())
                .add_stage(
                    RenderStage::Render,
                    SystemStage::parallel()
                        .with_system(process_pipeline_queue_system)
                        .with_system(render_system.at_end()),
                )
                .add_stage(RenderStage::Cleanup, SystemStage::parallel())
                .init_resource::<render_graph::RenderGraph>()
                .insert_resource(instance.clone())
                .insert_resource(device.clone())
                .insert_resource(queue.clone())
                .insert_resource(adapter.clone())
                .insert_resource(adapter_info.clone())
                .insert_resource(pipeline_cache)
                .insert_resource(asset_server);

            let (sender, receiver) = bevy_time::create_time_channels();
            app.insert_resource(receiver);
            render_app.insert_resource(sender);

            app.add_sub_app(RenderApp, render_app, move |app_world, render_app| {
                #[cfg(feature = "trace")]
                let _render_span = bevy_utils::tracing::info_span!("renderer subapp").entered();
                {
                    #[cfg(feature = "trace")]
                    let _stage_span =
                        bevy_utils::tracing::info_span!("stage", name = "reserve_and_flush")
                            .entered();

                    // reserve all existing app entities for use in render_app
                    // they can only be spawned using `get_or_spawn()`
                    let total_count = app_world.entities().total_count();

                    assert_eq!(
                        render_app.world.entities().len(),
                        0,
                        "An entity was spawned after the entity list was cleared last frame and before the extract stage began. This is not supported",
                    );

                    // This is safe given the clear_entities call in the past frame and the assert above
                    unsafe {
                        render_app
                            .world
                            .entities_mut()
                            .flush_and_reserve_invalid_assuming_no_entities(total_count);
                    }
                }

                {
                    #[cfg(feature = "trace")]
                    let _stage_span =
                        bevy_utils::tracing::info_span!("stage", name = "extract").entered();

                    // extract
                    extract(app_world, render_app);
                }

                {
                    #[cfg(feature = "trace")]
                    let _stage_span =
                        bevy_utils::tracing::info_span!("stage", name = "prepare").entered();

                    // prepare
                    let prepare = render_app
                        .schedule
                        .get_stage_mut::<SystemStage>(RenderStage::Prepare)
                        .unwrap();
                    prepare.run(&mut render_app.world);
                }

                {
                    #[cfg(feature = "trace")]
                    let _stage_span =
                        bevy_utils::tracing::info_span!("stage", name = "queue").entered();

                    // queue
                    let queue = render_app
                        .schedule
                        .get_stage_mut::<SystemStage>(RenderStage::Queue)
                        .unwrap();
                    queue.run(&mut render_app.world);
                }

                {
                    #[cfg(feature = "trace")] 
                    let _stage_span =
                        bevy_utils::tracing::info_span!("stage", name = "sort").entered();

                    // phase sort
                    let phase_sort = render_app
                        .schedule
                        .get_stage_mut::<SystemStage>(RenderStage::PhaseSort)
                        .unwrap();
                    phase_sort.run(&mut render_app.world);
                }

                {
                    #[cfg(feature = "trace")]
                    let _stage_span =
                        bevy_utils::tracing::info_span!("stage", name = "render").entered();

                    // render
                    let render = render_app
                        .schedule
                        .get_stage_mut::<SystemStage>(RenderStage::Render)
                        .unwrap();
                    render.run(&mut render_app.world);
                }

                {
                    #[cfg(feature = "trace")]
                    let _stage_span =
                        bevy_utils::tracing::info_span!("stage", name = "cleanup").entered();

                    // cleanup
                    let cleanup = render_app
                        .schedule
                        .get_stage_mut::<SystemStage>(RenderStage::Cleanup)
                        .unwrap();
                    cleanup.run(&mut render_app.world);
                }
                {
                    #[cfg(feature = "trace")]
                    let _stage_span =
                        bevy_utils::tracing::info_span!("stage", name = "clear_entities").entered();

                    render_app.world.clear_entities();
                }
            });
        }

        app.add_plugin(ValidParentCheckPlugin::<view::ComputedVisibility>::default())
            .add_plugin(WindowRenderPlugin)
            .add_plugin(CameraPlugin)
            .add_plugin(ViewPlugin)
            .add_plugin(MeshPlugin)
            .add_plugin(GlobalsPlugin);

        app.register_type::<color::Color>()
            .register_type::<primitives::Aabb>()
            .register_type::<primitives::CubemapFrusta>()
            .register_type::<primitives::Frustum>();
    }
}

/// A "scratch" world used to avoid allocating new worlds every frame when
/// swapping out the [`MainWorld`] for [`RenderStage::Extract`].
#[derive(Resource, Default)]
struct ScratchMainWorld(World);

/// Executes the [`Extract`](RenderStage::Extract) stage of the renderer.
/// This updates the render world with the extracted ECS data of the current frame.
fn extract(app_world: &mut World, render_app: &mut App) {
    let extract = render_app
        .schedule
        .get_stage_mut::<SystemStage>(RenderStage::Extract)
        .unwrap();

    // temporarily add the app world to the render world as a resource
    let scratch_world = app_world.remove_resource::<ScratchMainWorld>().unwrap();
    let inserted_world = std::mem::replace(app_world, scratch_world.0);
    let running_world = &mut render_app.world;
    running_world.insert_resource(MainWorld(inserted_world));

    extract.run(running_world);
    // move the app world back, as if nothing happened.
    let inserted_world = running_world.remove_resource::<MainWorld>().unwrap();
    let scratch_world = std::mem::replace(app_world, inserted_world.0);
    app_world.insert_resource(ScratchMainWorld(scratch_world));

    // Note: We apply buffers (read, Commands) after the `MainWorld` has been removed from the render app's world
    // so that in future, pipelining will be able to do this too without any code relying on it.
    // see <https://github.com/bevyengine/bevy/issues/5082>
    extract.apply_buffers(running_world);
}

// TODO: move this
pub(crate) fn extract_shaders(
    mut cache: ResMut<PipelineCache>,
    shaders: Extract<Res<Assets<Shader>>>,
    mut events: Extract<EventReader<AssetEvent<Shader>>>,
) {
    for event in events.iter() {
        match event {
            AssetEvent::Created { handle } | AssetEvent::Modified { handle } => {
                if let Some(shader) = shaders.get(handle) {
                    cache.set_shader(handle, shader);
                }
            }
            AssetEvent::Removed { handle } => cache.remove_shader(handle),
        }
    }
}

// TODO: move this
pub(crate) fn process_pipeline_queue_system(mut cache: ResMut<PipelineCache>) {
    cache.process_queue();
}
