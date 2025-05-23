use bevy_platform::collections::HashMap;

use crate::renderer::RenderDevice;

use super::{
    AnyTransientResource, TransientResourceCreator, TransientResource, ArcTransientResource,
    ResourceNode, Ref, ResourceRelease, ResourceRequese, ResourceView,
    TransientResourceCache, TypeIndex, VirtualResource,
};

#[derive(Default)]
pub struct ResourceTable {
    resources: HashMap<TypeIndex<ResourceNode>, AnyTransientResource>,
}

impl ResourceTable {
    pub fn get_resource<ResourceType: TransientResource, ViewType: ResourceView>(
        &self,
        resource_ref: &Ref<ResourceType, ViewType>,
    ) -> Option<&ResourceType> {
        self.resources
            .get(&resource_ref.index)
            .map(|res| TransientResource::borrow_resource(res))
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
                ArcTransientResource::Texture(resource) => {
                    AnyTransientResource::ImportedTexture(resource.clone())
                }
                ArcTransientResource::Buffer(resource) => {
                    AnyTransientResource::ImportedBuffer(resource.clone())
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
                AnyTransientResource::OwnedBuffer(buffer) => {
                    transient_resource_cache.insert_resource(
                        buffer.desc.clone().into(),
                        AnyTransientResource::OwnedBuffer(buffer),
                    );
                }
                AnyTransientResource::OwnedTexture(texture) => {
                    transient_resource_cache.insert_resource(
                        texture.desc.clone().into(),
                        AnyTransientResource::OwnedTexture(texture),
                    );
                }
                _ => {}
            }
        }
    }
}
