use crate::frame_graph::GraphResource;

pub struct ResourceMeta<ResourceType: GraphResource> {
    pub key: String,
    pub desc: <ResourceType as GraphResource>::Descriptor,
}
