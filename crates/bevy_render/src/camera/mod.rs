mod camera;
mod clear_color;
mod compositor;
mod manual_texture_view;
mod projection;
mod render;
mod render_target;
mod view;

use bevy_derive::{Deref, DerefMut};
use bevy_reflect::Reflect;
pub use camera::*;
pub use clear_color::*;
pub use compositor::*;
pub use manual_texture_view::*;
pub use projection::*;
pub use render::*;
pub use render_target::*;
use tracing::warn;
pub use view::*;

use crate::{
    extract_component::ExtractComponentPlugin,
    extract_resource::ExtractResourcePlugin,
    render_graph::{InternedRenderSubGraph, RenderGraphApp, RenderSubGraph},
    RenderApp,
};
use bevy_app::{App, Plugin};
use bevy_ecs::{
    component::{Component, HookContext},
    reflect::ReflectComponent,
    world::DeferredWorld,
};

#[derive(Default)]
pub struct CameraPlugin;

impl Plugin for CameraPlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<Camera>()
            .register_type::<ClearColor>()
            .register_type::<RenderGraphDriver>()
            .register_type::<CameraMainTextureUsages>()
            .register_type::<Exposure>()
            .register_type::<TemporalJitter>()
            .register_type::<MipBias>()
            .init_resource::<ManualTextureViews>()
            .init_resource::<ClearColor>()
            .add_plugins((
                CameraProjectionPlugin,
                ExtractResourcePlugin::<ManualTextureViews>::default(),
                ExtractResourcePlugin::<ClearColor>::default(),
                ExtractComponentPlugin::<CameraMainTextureUsages>::default(),
            ));

        // if let Some(render_app) = app.get_sub_app_mut(RenderApp) {
        //     render_app.add_systems(ExtractSchedule, extract_cameras);
        //     let camera_driver_node = CameraDriverNode::new(render_app.world_mut());
        //     let mut render_graph = render_app.world_mut().resource_mut::<RenderGraph>();
        //     render_graph.add_node(crate::graph::CameraDriverLabel, camera_driver_node);
        // }
    }

    fn finish(&self, app: &mut App) {
        let render_app = app.sub_app_mut(RenderApp);
        render_app.add_render_sub_graph(NoopRenderGraph);
    }
}

/// Configures the [`RenderGraph`](crate::render_graph::RenderGraph) name assigned to be run for a given entity.
/// This component does nothing on its own, and should be used alongside a [`View`], [`Camera`], or [`Compositor`].
#[derive(Component, Debug, Deref, DerefMut, Reflect, Clone)]
#[component(on_add = warn_on_noop_view_render_graph)]
#[reflect(opaque)]
#[reflect(Component, Debug, Clone)]
pub struct RenderGraphDriver(InternedRenderSubGraph);

impl RenderGraphDriver {
    /// Creates a new [`CameraRenderGraph`] from any string-like type.
    #[inline]
    pub fn new<T: RenderSubGraph>(name: T) -> Self {
        Self(name.intern())
    }

    /// Sets the graph name.
    #[inline]
    pub fn set<T: RenderSubGraph>(&mut self, name: T) {
        self.0 = name.intern();
    }
}

impl Default for RenderGraphDriver {
    fn default() -> Self {
        Self(NoopRenderGraph.intern())
    }
}

#[derive(Copy, Clone, PartialEq, Eq, Hash, Debug, RenderSubGraph)]
struct NoopRenderGraph;

fn warn_on_noop_view_render_graph(world: DeferredWorld, ctx: HookContext) {
    if world
        .entity(ctx.entity)
        .get::<RenderGraphDriver>()
        .is_some_and(|render_graph| render_graph.0 == NoopRenderGraph.intern())
    {
        warn!(
            concat!(
                "{}Entity {} spawned with a no-op render graph. If this entity is a camera, consider ",
                "adding a `Camera2d` or `Camera3d` component or manually adding a RenderGraphDriver ",
                "component if you need a custom render graph."
            ),
            ctx.caller
                .map(|location| format!("{location}: "))
                .unwrap_or_default(),
            ctx.entity,
        );
    }
}
