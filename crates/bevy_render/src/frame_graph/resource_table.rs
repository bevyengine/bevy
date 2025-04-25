use bevy_platform::collections::HashMap;

use super::{
    AnyFrameGraphResource, GraphResource, ResourceNode, ResourceRead, ResourceRef, TypeHandle,
};

pub struct ResourceTable {
    resources: HashMap<TypeHandle<ResourceNode>, AnyFrameGraphResource>,
}

impl ResourceTable {
    pub fn get_resource<ResourceType: GraphResource>(
        &self,
        resource_ref: &ResourceRef<ResourceType, ResourceRead>,
    ) -> Option<&ResourceType> {
        self.resources
            .get(&resource_ref.handle)
            .map(|res| GraphResource::borrow_resource(res))
    }
}
