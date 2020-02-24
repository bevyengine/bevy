use crate::asset::{Handle, Texture};
use std::collections::HashMap;

#[derive(Copy, Clone, Hash, Debug, Eq, PartialEq)]
pub struct RenderResource(pub u64);

#[derive(Default)]
pub struct RenderResources {
    pub name_to_resource: HashMap<String, RenderResource>,
    pub texture_to_resource: HashMap<Handle<Texture>, RenderResource>,
    pub resource_index: u64,
}

impl RenderResources {
    pub fn set_named_resource(&mut self, name: &str, resource: RenderResource) {
        self.name_to_resource.insert(name.to_string(), resource);
    }

    pub fn get_named_resource(&self, name: &str) -> Option<RenderResource> {
        self.name_to_resource.get(name).cloned()
    }

    pub fn set_texture_resource(&mut self, texture: Handle<Texture>, resource: RenderResource) {
        self.texture_to_resource.insert(texture, resource);
    }

    pub fn get_texture_resource(&self, texture: Handle<Texture>) -> Option<RenderResource> {
        self.texture_to_resource.get(&texture).cloned()
    }

    pub fn get_next_resource(&mut self) -> RenderResource {
        let resource = self.resource_index;
        self.resource_index += 1;

        RenderResource(resource)
    }
}
