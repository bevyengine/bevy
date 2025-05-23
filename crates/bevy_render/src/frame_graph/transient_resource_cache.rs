use bevy_ecs::resource::Resource;
use std::collections::HashMap;

use super::{AnyTransientResource, AnyFrameGraphResourceDescriptor};

#[derive(Default, Resource)]
pub struct TransientResourceCache {
    resources: HashMap<AnyFrameGraphResourceDescriptor, Vec<AnyTransientResource>>,
}

impl TransientResourceCache {
    pub fn get_resource(
        &mut self,
        desc: &AnyFrameGraphResourceDescriptor,
    ) -> Option<AnyTransientResource> {
        if let Some(entry) = self.resources.get_mut(desc) {
            entry.pop()
        } else {
            None
        }
    }

    pub fn insert_resource(
        &mut self,
        desc: AnyFrameGraphResourceDescriptor,
        resource: AnyTransientResource,
    ) {
        if let Some(entry) = self.resources.get_mut(&desc) {
            entry.push(resource);
        } else {
            self.resources.insert(desc, vec![resource]);
        }
    }
}
