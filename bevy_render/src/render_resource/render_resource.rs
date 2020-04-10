use crate::{mesh::Mesh, texture::Texture};
use bevy_asset::Handle;
use std::collections::HashMap;
use uuid::Uuid;

// TODO: Rename to RenderResourceId
#[derive(Copy, Clone, Hash, Debug, Eq, PartialEq)]
pub struct RenderResource(Uuid);

impl RenderResource {
    pub fn new() -> Self {
        RenderResource(Uuid::new_v4())
    }
}

// TODO: consider scoping breaking these mappings up by type: Texture, Sampler, etc
// the overlap could cause accidents.
#[derive(Default)]
pub struct RenderResources {
    pub texture_to_resource: HashMap<Handle<Texture>, RenderResource>,
    pub texture_to_sampler_resource: HashMap<Handle<Texture>, RenderResource>,
    pub mesh_to_vertices_resource: HashMap<Handle<Mesh>, RenderResource>,
    pub mesh_to_indices_resource: HashMap<Handle<Mesh>, RenderResource>,
}

impl RenderResources {
    pub fn consume(&mut self, render_resources: RenderResources) {
        // TODO: this is brittle. consider a single change-stream-based approach instead?
        self.texture_to_resource.extend(render_resources.texture_to_resource);        
        self.texture_to_sampler_resource.extend(render_resources.texture_to_sampler_resource);        
        self.mesh_to_vertices_resource.extend(render_resources.mesh_to_vertices_resource);        
        self.mesh_to_indices_resource.extend(render_resources.mesh_to_indices_resource);        
    } 

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
}
