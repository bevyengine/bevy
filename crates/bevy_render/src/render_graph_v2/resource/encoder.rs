use bevy_ecs::world::World;

use super::{RenderResource, SimpleResourceStore};
use crate::{
    render_graph_v2::RenderGraph,
    render_resource::{CommandEncoder, CommandEncoderDescriptor},
};

impl RenderResource for CommandEncoder {
    type Descriptor = CommandEncoderDescriptor<'static>;
    type Data = Self;
    type Store = SimpleResourceStore<Self>;

    fn get_store(grpah: &RenderGraph) -> &Self::Store {
        todo!()
    }

    fn get_store_mut(graph: &mut RenderGraph) -> &mut Self::Store {
        todo!()
    }

    fn from_data<'a>(data: &'a Self::Data, world: &'a World) -> Option<&'a Self> {
        Some(data)
    }
}
