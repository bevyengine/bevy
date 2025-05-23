use core::marker::PhantomData;

use super::{
    index::TypeIndex, AnyFrameGraphResourceDescriptor, ArcTransientResource, PassNode,
    TransientResource,
};

pub struct Ref<ResourceType, VieType> {
    pub index: TypeIndex<ResourceNode>,
    _marker: PhantomData<(ResourceType, VieType)>,
}

impl<ResourceType, VieType> Ref<ResourceType, VieType> {
    pub fn new(index: TypeIndex<ResourceNode>) -> Self {
        Self {
            index,
            _marker: PhantomData,
        }
    }
}

impl<ResourceType, VieType> Clone for Ref<ResourceType, VieType> {
    fn clone(&self) -> Self {
        Self {
            index: self.index,
            _marker: PhantomData,
        }
    }
}

impl<ResourceType, VieType> PartialEq for Ref<ResourceType, VieType> {
    fn eq(&self, other: &Self) -> bool {
        self.index == other.index
    }
}

impl<ResourceType, VieType> Eq for Ref<ResourceType, VieType> {}

pub trait ResourceView {}

pub struct ResourceRead;
pub struct ResourceWrite;

impl ResourceView for ResourceRead {}
impl ResourceView for ResourceWrite {}

pub struct Handle<ResourceType: TransientResource> {
    pub raw: GraphRawResourceHandle,
    _marker: PhantomData<ResourceType>,
}

impl<ResourceType: TransientResource> Clone for Handle<ResourceType> {
    fn clone(&self) -> Self {
        Handle {
            raw: self.raw.clone(),
            _marker: PhantomData,
        }
    }
}

impl<ResourceType: TransientResource> Handle<ResourceType> {
    pub fn new(index: TypeIndex<ResourceNode>, version: u32) -> Self {
        Self {
            raw: GraphRawResourceHandle { index, version },
            _marker: PhantomData,
        }
    }
}

#[derive(PartialEq, Eq, Hash, Clone)]
pub struct GraphRawResourceHandle {
    pub index: TypeIndex<ResourceNode>,
    pub version: u32,
}

pub struct ResourceNode {
    pub handle: TypeIndex<ResourceNode>,
    pub name: String,
    pub first_use_pass: Option<TypeIndex<PassNode>>,
    pub last_user_pass: Option<TypeIndex<PassNode>>,
    version: u32,
    pub resource: VirtualResource,
}

pub struct ResourceRequese {
    pub handle: TypeIndex<ResourceNode>,
    pub resource: VirtualResource,
}

pub struct ResourceRelease {
    pub handle: TypeIndex<ResourceNode>,
}

#[derive(Clone)]
pub enum VirtualResource {
    Setuped(AnyFrameGraphResourceDescriptor),
    Imported(ArcTransientResource),
}

impl ResourceNode {
    pub fn new(name: &str, handle: TypeIndex<ResourceNode>, resource: VirtualResource) -> Self {
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

    pub fn update_lifetime(&mut self, handle: TypeIndex<PassNode>) {
        if self.first_use_pass.is_none() {
            self.first_use_pass = Some(handle);
        }

        self.last_user_pass = Some(handle);
    }
}
