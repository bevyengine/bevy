mod compute_builder;
mod extract;
mod node;
mod plugin;

pub use compute_builder::ComputeCommandBuilder;
pub use node::RenderTaskEncoder;
pub use plugin::RenderTaskPlugin;

use crate::{
    render_graph::{InternedRenderLabel, RenderLabel, RenderSubGraph},
    settings::{WgpuFeatures, WgpuLimits},
};
use bevy_app::{App, SubApp};
use bevy_ecs::{
    component::Component,
    query::{QueryItem, ReadOnlyQueryData},
    world::World,
};

pub trait RenderTask: Component + Clone {
    type RenderNodeLabel: RenderLabel + Default;
    type RenderNodeSubGraph: RenderSubGraph + Default;
    fn render_node_ordering<'a>() -> &'a [InternedRenderLabel]; // TODO: This API is not actually usable

    const REQUIRED_FEATURES: WgpuFeatures = WgpuFeatures::empty();
    const REQUIRED_LIMITS: WgpuLimits = WgpuLimits::downlevel_webgl2_defaults();

    #[expect(unused_variables)]
    fn plugin_app_build(app: &mut App) {}

    #[expect(unused_variables)]
    fn plugin_render_app_build(render_app: &mut SubApp) {}

    type ExtraQueryData: ReadOnlyQueryData;

    fn encode_commands(
        &self,
        encoder: &mut RenderTaskEncoder,
        extra_query_data: QueryItem<Self::ExtraQueryData>,
        world: &World,
    );
}
