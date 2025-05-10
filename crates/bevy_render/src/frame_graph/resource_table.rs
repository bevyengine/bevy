use bevy_platform::collections::HashMap;

use crate::renderer::RenderDevice;

use super::{
    AnyFrameGraphResource, FrameGraphResourceCreator, GraphResource, ImportedResource,
    ResourceNode, ResourceRef, ResourceRelease, ResourceRequese, ResourceView,
    TransientResourceCache, TypeHandle, VirtualResource,
};

#[derive(Default)]
pub struct ResourceTable {
    resources: HashMap<TypeHandle<ResourceNode>, AnyFrameGraphResource>,
}

impl ResourceTable {
    pub fn get_resource<ResourceType: GraphResource, ViewType: ResourceView>(
        &self,
        resource_ref: &ResourceRef<ResourceType, ViewType>,
    ) -> Option<&ResourceType> {
        self.resources
            .get(&resource_ref.handle)
            .map(|res| GraphResource::borrow_resource(res))
    }

    pub fn request_resource(
        &mut self,
        request: &ResourceRequese,
        device: &RenderDevice,
        transient_resource_cache: &mut TransientResourceCache,
    ) {
        let handle = request.handle;
        let resource = match &request.resource {
            VirtualResource::Imported(resource) => match &resource {
                ImportedResource::Texture(resource) => {
                    AnyFrameGraphResource::ImportedTexture(resource.clone())
                }
                ImportedResource::Buffer(resource) => {
                    AnyFrameGraphResource::ImportedBuffer(resource.clone())
                }
            },
            VirtualResource::Setuped(desc) => transient_resource_cache
                .get_resource(desc)
                .unwrap_or_else(|| device.create_resource(desc)),
        };

        self.resources.insert(handle, resource);
    }

    pub fn release_resource(
        &mut self,
        release: &ResourceRelease,
        transient_resource_cache: &mut TransientResourceCache,
    ) {
        if let Some(resource) = self.resources.remove(&release.handle) {
            match resource {
                AnyFrameGraphResource::OwnedBuffer(buffer) => {
                    transient_resource_cache.insert_resource(
                        buffer.desc.clone().into(),
                        AnyFrameGraphResource::OwnedBuffer(buffer),
                    );
                }
                AnyFrameGraphResource::OwnedTexture(texture) => {
                    transient_resource_cache.insert_resource(
                        texture.desc.clone().into(),
                        AnyFrameGraphResource::OwnedTexture(texture),
                    );
                }
                _ => {}
            }
        }
    }
}
