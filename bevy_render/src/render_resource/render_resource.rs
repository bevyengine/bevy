use crate::{mesh::Mesh, texture::Texture};
use bevy_asset::Handle;
use std::collections::HashMap;

#[derive(Copy, Clone, Hash, Debug, Eq, PartialEq)]
pub struct RenderResource(pub u64);

// TODO: consider scoping breaking these mappings up by type: Texture, Sampler, etc
// the overlap could cause accidents.
#[derive(Default)]
pub struct RenderResources {
    pub texture_to_resource: HashMap<Handle<Texture>, RenderResource>,
    pub texture_to_sampler_resource: HashMap<Handle<Texture>, RenderResource>,
    pub mesh_to_vertices_resource: HashMap<Handle<Mesh>, RenderResource>,
    pub mesh_to_indices_resource: HashMap<Handle<Mesh>, RenderResource>,
    pub resource_index: u64,
}

impl RenderResources {
    pub fn set_texture_resource(&mut self, texture: Handle<Texture>, resource: RenderResource) {
        self.texture_to_resource.insert(texture, resource);
    }

    pub fn get_texture_resource(&self, texture: Handle<Texture>) -> Option<RenderResource> {
        self.texture_to_resource.get(&texture).cloned()
    }

    pub fn set_mesh_vertices_resource(&mut self, mesh: Handle<Mesh>, resource: RenderResource) {
        self.mesh_to_vertices_resource.insert(mesh, resource);
    }

    pub fn get_mesh_vertices_resource(&self, mesh: Handle<Mesh>) -> Option<RenderResource> {
        self.mesh_to_vertices_resource.get(&mesh).cloned()
    }

    pub fn set_mesh_indices_resource(&mut self, mesh: Handle<Mesh>, resource: RenderResource) {
        self.mesh_to_indices_resource.insert(mesh, resource);
    }

    pub fn get_mesh_indices_resource(&self, mesh: Handle<Mesh>) -> Option<RenderResource> {
        self.mesh_to_indices_resource.get(&mesh).cloned()
    }

    pub fn set_texture_sampler_resource(
        &mut self,
        texture: Handle<Texture>,
        resource: RenderResource,
    ) {
        self.texture_to_sampler_resource.insert(texture, resource);
    }

    pub fn get_texture_sampler_resource(&self, texture: Handle<Texture>) -> Option<RenderResource> {
        self.texture_to_sampler_resource.get(&texture).cloned()
    }

    pub fn get_next_resource(&mut self) -> RenderResource {
        let resource = self.resource_index;
        self.resource_index += 1;

        RenderResource(resource)
    }
}
