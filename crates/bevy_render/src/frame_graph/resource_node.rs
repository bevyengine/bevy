use core::marker::PhantomData;

use super::{handle::TypeHandle, AnyFrameGraphResourceDescriptor, ImportedResource, PassNode};

pub trait ResourceView {}

pub struct ResourceRef<ResourceType, VieType> {
    pub handle: TypeHandle<ResourceNode>,
    _marker: PhantomData<(ResourceType, VieType)>,
}

impl<ResourceType, VieType> ResourceRef<ResourceType, VieType> {
    pub fn new(handle: TypeHandle<ResourceNode>) -> Self {
        Self {
            handle,
            _marker: PhantomData,
        }
    }
}

pub struct ResourceRead;
pub struct ResourceWrite;

impl ResourceView for ResourceRead {}
impl ResourceView for ResourceWrite {}

pub struct GraphResourceNodeHandle<ResourceType> {
    pub handle: TypeHandle<ResourceNode>,
    pub version: u32,
    _marker: PhantomData<ResourceType>,
}

impl<ResourceType> GraphResourceNodeHandle<ResourceType> {
    pub fn raw(&self) -> GraphRawResourceNodeHandle {
        GraphRawResourceNodeHandle {
            handle: self.handle,
            version: self.version,
        }
    }

    pub fn new(handle: TypeHandle<ResourceNode>, version: u32) -> Self {
        Self {
            handle,
            version,
            _marker: PhantomData,
        }
    }
}

#[derive(PartialEq, Eq, Hash, Clone)]
pub struct GraphRawResourceNodeHandle {
    pub handle: TypeHandle<ResourceNode>,
    pub version: u32,
}

pub struct ResourceNode {
    pub handle: TypeHandle<ResourceNode>,
    pub name: String,
    pub first_use_pass: Option<TypeHandle<PassNode>>,
    pub last_user_pass: Option<TypeHandle<PassNode>>,
    version: u32,
    pub resource: VirtualResource,
}

pub struct ResourceRequese {
    pub handle: TypeHandle<ResourceNode>,
    pub resource: VirtualResource,
}

pub struct ResourceRelease {
    pub handle: TypeHandle<ResourceNode>,
}

#[derive(Clone)]
pub enum VirtualResource {
    Setuped(AnyFrameGraphResourceDescriptor),
    Imported(ImportedResource),
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
    pub fn request(&self) -> ResourceRequese {
        ResourceRequese {
            handle: self.handle,
            resource: self.resource.clone(),
        }
    }

    pub fn release(&self) -> ResourceRelease {
        ResourceRelease {
            handle: self.handle,
        }
    }

    pub fn version(&self) -> u32 {
        self.version
    }

    pub fn new_version(&mut self) {
        self.version += 1;
    }

    pub fn update_lifetime(&mut self, handle: TypeHandle<PassNode>) {
        if self.first_use_pass.is_none() {
            self.first_use_pass = Some(handle);
        }

        self.last_user_pass = Some(handle);
    }
}
