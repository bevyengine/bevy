mod compositor;
pub mod manual_texture_view;
mod render;
mod render_target;
mod view;
pub mod window;

use bevy_app::{App, Plugin};
use bevy_derive::{Deref, DerefMut};
use bevy_ecs::{
    component::{Component, HookContext},
    reflect::ReflectComponent,
    world::DeferredWorld,
};
use bevy_reflect::Reflect;

pub use compositor::*;
use manual_texture_view::ManualTextureViews;
pub use render_target::*;
use tracing::warn;
pub use view::*;

use crate::{
    extract_resource::ExtractResourcePlugin,
    render_graph::{InternedRenderSubGraph, RenderGraphApp, RenderSubGraph},
    RenderApp,
};

pub struct CompositionPlugin;

impl Plugin for CompositionPlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<RenderGraphDriver>()
            .init_resource::<ManualTextureViews>();

        app.add_plugins(ExtractResourcePlugin::<ManualTextureViews>::default());
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
            //TODO: ideally we don't want to mention cameras in this module, since they'll be
            //separated out into their own crate soon
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
