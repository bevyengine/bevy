use super::{GraphRawResourceNodeHandle, Pass, ResourceNode, TypeHandle};

pub struct PassNode {
    pub name: String,
    pub handle: TypeHandle<PassNode>,
    pub writes: Vec<GraphRawResourceNodeHandle>,
    pub reads: Vec<GraphRawResourceNodeHandle>,
    pub resource_request_array: Vec<TypeHandle<ResourceNode>>,
    pub resource_release_array: Vec<TypeHandle<ResourceNode>>,
    pub pass: Option<Pass>,
}

impl PassNode {
    pub fn new(name: &str, handle: TypeHandle<PassNode>) -> Self {
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
