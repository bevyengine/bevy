use super::{GraphRawResourceHandle, Pass, ResourceNode, TypeIndex};

pub struct PassNode {
    pub name: String,
    pub handle: TypeIndex<PassNode>,
    pub writes: Vec<GraphRawResourceHandle>,
    pub reads: Vec<GraphRawResourceHandle>,
    pub resource_request_array: Vec<TypeIndex<ResourceNode>>,
    pub resource_release_array: Vec<TypeIndex<ResourceNode>>,
    pub pass: Option<Pass>,
}

impl PassNode {
    pub fn new(name: &str, handle: TypeIndex<PassNode>) -> Self {
        Self {
            name: name.to_string(),
            handle,
            writes: Default::default(),
            reads: Default::default(),
            resource_request_array: Default::default(),
            resource_release_array: Default::default(),
            pass: Default::default(),
        }
    }
}
