use super::{handle::TypeHandle, AnyFrameGraphResource, AnyFrameGraphResourceDescriptor, PassNode};

pub struct ResourceNode {
    pub handle: TypeHandle<ResourceNode>,
    pub name: String,
    pub first_use_pass: Option<TypeHandle<PassNode>>,
    pub last_user_pass: Option<TypeHandle<PassNode>>,
    version: u32,
    pub resource: VirtualResource,
}

pub enum VirtualResource {
    Setuped(AnyFrameGraphResourceDescriptor),
    Imported(AnyFrameGraphResource),
}

impl ResourceNode {
    pub fn new(name: &str, handle: TypeHandle<ResourceNode>, resource: VirtualResource) -> Self {
        ResourceNode {
            name: name.to_string(),
            handle,
            version: 0,
            first_use_pass: None,
            last_user_pass: None,
            resource,
        }
    }
}

impl ResourceNode {
    pub fn version(&self) -> u32 {
        self.version
    }

    pub fn new_version(&mut self) {
        self.version += 1
    }

    pub fn update_lifetime(&mut self, handle: TypeHandle<PassNode>) {
        if self.first_use_pass.is_none() {
            self.first_use_pass = Some(handle);
        }

        self.last_user_pass = Some(handle)
    }
}
