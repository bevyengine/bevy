use bevy_ecs::world::{EntityRef, World};
use bevy_math::FloatOrd;
use bevy_utils::HashMap;
use std::borrow::Borrow;
use std::hash::Hash;
use wgpu::{TextureUsages, TextureViewDescriptor};

use crate::{
    render_graph_v2::{NodeContext, RenderGraph, RenderGraphBuilder},
    render_resource::{Sampler, SamplerDescriptor, Texture, TextureDescriptor, TextureView},
};

use super::{
    ref_eq::RefEq, DescribedRenderResource, FromDescriptorRenderResource, IntoRenderResource,
    NewRenderResource, RenderDependencies, RenderHandle, RenderResource, RenderResourceId,
    RenderResourceMeta, ResourceTracker, ResourceType, UsagesRenderResource, WriteRenderResource,
};

impl RenderResource for Texture {
    const RESOURCE_TYPE: ResourceType = ResourceType::Texture;

    #[inline]
    fn new_direct<'g>(
        graph: &mut RenderGraphBuilder<'g>,
        resource: RefEq<'g, Self>,
    ) -> RenderHandle<'g, Self> {
        graph.new_texture_direct(None, resource)
    }

    #[inline]
    fn get_from_store<'a>(
        context: &'a NodeContext,
        resource: RenderHandle<'a, Self>,
    ) -> Option<&'a Self> {
        context.get_texture(resource)
    }
}

impl WriteRenderResource for Texture {}

impl DescribedRenderResource for Texture {
    type Descriptor = TextureDescriptor<'static>;

    #[inline]
    fn new_with_descriptor<'g>(
        graph: &mut RenderGraphBuilder<'g>,
        descriptor: Self::Descriptor,
        resource: RefEq<'g, Self>,
    ) -> RenderHandle<'g, Self> {
        graph.new_texture_direct(Some(descriptor), resource)
    }

    fn get_descriptor<'a, 'g: 'a>(
        graph: &'a RenderGraphBuilder<'g>,
        resource: RenderHandle<'g, Self>,
    ) -> Option<&'a Self::Descriptor> {
        graph.get_texture_descriptor(resource)
    }
}

impl FromDescriptorRenderResource for Texture {
    #[inline]
    fn new_from_descriptor<'g>(
        graph: &mut RenderGraphBuilder<'g>,
        descriptor: Self::Descriptor,
    ) -> RenderHandle<'g, Self> {
        graph.new_texture_descriptor(descriptor)
    }
}

impl UsagesRenderResource for Texture {
    type Usages = TextureUsages;

    #[inline]
    fn get_descriptor_mut<'a, 'g: 'a>(
        graph: &'a mut RenderGraphBuilder<'g>,
        resource: RenderHandle<'g, Self>,
    ) -> Option<&'a mut Self::Descriptor> {
        graph.get_texture_descriptor_mut(resource)
    }

    #[inline]
    fn has_usages<'g>(descriptor: &Self::Descriptor, usages: &Self::Usages) -> bool {
        descriptor.usage.contains(*usages)
    }

    #[inline]
    fn add_usages<'g>(descriptor: &mut Self::Descriptor, usages: Self::Usages) {
        descriptor.usage.insert(usages);
    }
}

impl<'g> IntoRenderResource<'g> for TextureDescriptor<'static> {
    type Resource = Texture;

    #[inline]
    fn into_render_resource(
        self,
        graph: &mut RenderGraphBuilder<'g>,
    ) -> RenderHandle<'g, Self::Resource> {
        graph.new_texture_descriptor(self)
    }
}

pub struct RenderGraphTextureView<'g> {
    texture: RenderHandle<'g, Texture>,
    descriptor: TextureViewDescriptor<'static>,
}

#[derive(Default)]
pub struct RenderGraphTextureViews<'g> {
    texture_views: HashMap<RenderResourceId, RenderResourceMeta<'g, TextureView>>,
    queued_texture_views: HashMap<RenderResourceId, RenderGraphTextureView<'g>>,
    existing_borrows: HashMap<*const TextureView, RenderResourceId>,
}

impl<'g> RenderGraphTextureViews<'g> {
    pub fn new() -> Self {
        Default::default()
    }

    pub fn new_direct(
        &mut self,
        tracker: &mut ResourceTracker,
        descriptor: Option<TextureViewDescriptor<'static>>,
        resource: RefEq<'g, TextureView>,
    ) -> RenderResourceId {
        match resource {
            RefEq::Borrowed(texture_view) => {
                if let Some(id) = self
                    .existing_borrows
                    .get(&(texture_view as *const TextureView))
                {
                    *id
                } else {
                    let id = tracker.new_resource(ResourceType::TextureView, None);
                    self.texture_views.insert(
                        id,
                        RenderResourceMeta {
                            descriptor,
                            resource: RefEq::Borrowed(texture_view),
                        },
                    );
                    self.existing_borrows
                        .insert(texture_view as *const TextureView, id);
                    id
                }
            }
            RefEq::Owned(texture_view) => {
                let id = tracker.new_resource(ResourceType::TextureView, None);
                self.texture_views.insert(
                    id,
                    RenderResourceMeta {
                        descriptor,
                        resource: RefEq::Owned(texture_view),
                    },
                );
                id
            }
        }
    }

    pub fn new_from_descriptor(
        &mut self,
        tracker: &mut ResourceTracker<'g>,
        descriptor: RenderGraphTextureView<'g>,
    ) -> RenderResourceId {
        let id = tracker.new_resource(
            ResourceType::TextureView,
            Some(RenderDependencies::of(&descriptor.texture)),
        );
        self.queued_texture_views.insert(id, descriptor);
        id
    }

    pub fn create_queued_resources(
        &mut self,
        graph: &RenderGraph<'g>,
        world: &World,
        // view_entity: EntityRef,
    ) {
        for (id, queued_view) in self.queued_texture_views.drain() {
            let dependencies = RenderDependencies::of(&queued_view.texture);
            let context = NodeContext {
                graph,
                world,
                dependencies,
                // entity: view_entity,
            };
            let texture_view = context
                .get(queued_view.texture)
                .create_view(&queued_view.descriptor);
            self.texture_views.insert(
                id,
                RenderResourceMeta {
                    descriptor: Some(queued_view.descriptor),
                    resource: RefEq::Owned(texture_view),
                },
            );
        }
    }

    pub fn get_descriptor(&self, id: RenderResourceId) -> Option<&TextureViewDescriptor<'static>> {
        let check_normal = self
            .texture_views
            .get(&id)
            .and_then(|meta| meta.descriptor.as_ref());
        let check_queued = self
            .queued_texture_views
            .get(&id)
            .map(|queued_view| &queued_view.descriptor);
        check_normal.or(check_queued)
    }

    pub fn get(&self, id: RenderResourceId) -> Option<&TextureView> {
        self.texture_views
            .get(&id)
            .map(|meta| meta.resource.borrow())
    }
}

impl RenderResource for TextureView {
    const RESOURCE_TYPE: ResourceType = ResourceType::TextureView;

    #[inline]
    fn new_direct<'g>(
        graph: &mut RenderGraphBuilder<'g>,
        resource: RefEq<'g, Self>,
    ) -> RenderHandle<'g, Self> {
        graph.new_texture_view_direct(None, resource)
    }

    #[inline]
    fn get_from_store<'a>(
        context: &'a NodeContext,
        resource: RenderHandle<'a, Self>,
    ) -> Option<&'a Self> {
        context.get_texture_view(resource)
    }
}

impl WriteRenderResource for TextureView {}

impl DescribedRenderResource for TextureView {
    type Descriptor = TextureViewDescriptor<'static>;

    #[inline]
    fn new_with_descriptor<'g>(
        graph: &mut RenderGraphBuilder<'g>,
        descriptor: Self::Descriptor,
        resource: RefEq<'g, Self>,
    ) -> RenderHandle<'g, Self> {
        graph.new_texture_view_direct(Some(descriptor), resource)
    }

    #[inline]
    fn get_descriptor<'a, 'g: 'a>(
        graph: &'a RenderGraphBuilder<'g>,
        resource: RenderHandle<'g, Self>,
    ) -> Option<&'a Self::Descriptor> {
        graph.get_texture_view_descriptor(resource)
    }
}

impl<'g> IntoRenderResource<'g> for RenderGraphTextureView<'g> {
    type Resource = TextureView;

    fn into_render_resource(
        self,
        graph: &mut RenderGraphBuilder<'g>,
    ) -> RenderHandle<'g, Self::Resource> {
        graph.new_texture_view_descriptor(self)
    }
}

impl RenderResource for Sampler {
    const RESOURCE_TYPE: ResourceType = ResourceType::Sampler;

    #[inline]
    fn new_direct<'g>(
        graph: &mut RenderGraphBuilder<'g>,
        resource: RefEq<'g, Self>,
    ) -> RenderHandle<'g, Self> {
        graph.new_sampler_direct(None, resource)
    }

    #[inline]
    fn get_from_store<'a>(
        context: &'a NodeContext,
        resource: RenderHandle<'a, Self>,
    ) -> Option<&'a Self> {
        context.get_sampler(resource)
    }
}

impl DescribedRenderResource for Sampler {
    type Descriptor = RenderGraphSamplerDescriptor;

    #[inline]
    fn new_with_descriptor<'g>(
        graph: &mut RenderGraphBuilder<'g>,
        descriptor: Self::Descriptor,
        resource: RefEq<'g, Self>,
    ) -> RenderHandle<'g, Self> {
        graph.new_sampler_direct(Some(descriptor), resource)
    }

    fn get_descriptor<'a, 'g: 'a>(
        graph: &'a RenderGraphBuilder<'g>,
        resource: RenderHandle<'g, Self>,
    ) -> Option<&'a Self::Descriptor> {
        graph.get_sampler_descriptor(resource)
    }
}

impl FromDescriptorRenderResource for Sampler {
    #[inline]
    fn new_from_descriptor<'g>(
        graph: &mut RenderGraphBuilder<'g>,
        descriptor: Self::Descriptor,
    ) -> RenderHandle<'g, Self> {
        graph.new_sampler_descriptor(descriptor)
    }
}

#[derive(Clone, Debug)]
pub struct RenderGraphSamplerDescriptor(pub SamplerDescriptor<'static>);

impl Hash for RenderGraphSamplerDescriptor {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        let SamplerDescriptor {
            label: _, //Note: labels aren't hashed, which might result in mismatched labels but otherwise should be fine
            address_mode_u,
            address_mode_v,
            address_mode_w,
            mag_filter,
            min_filter,
            mipmap_filter,
            lod_min_clamp,
            lod_max_clamp,
            compare,
            anisotropy_clamp,
            border_color,
        } = self.0;

        address_mode_u.hash(state);
        address_mode_v.hash(state);
        address_mode_w.hash(state);
        mag_filter.hash(state);
        min_filter.hash(state);
        mipmap_filter.hash(state);
        FloatOrd(lod_min_clamp).hash(state);
        FloatOrd(lod_max_clamp).hash(state);
        compare.hash(state);
        anisotropy_clamp.hash(state);
        border_color.hash(state);
    }
}

impl PartialEq for RenderGraphSamplerDescriptor {
    fn eq(&self, other: &Self) -> bool {
        let s = &self.0;
        let o = &other.0;

        s.address_mode_u == o.address_mode_u
            && s.address_mode_v == o.address_mode_v
            && s.address_mode_w == o.address_mode_w
            && s.mag_filter == o.mag_filter
            && s.min_filter == o.min_filter
            && s.mipmap_filter == o.mipmap_filter
            && FloatOrd(s.lod_min_clamp) == FloatOrd(o.lod_min_clamp)
            && FloatOrd(s.lod_max_clamp) == FloatOrd(o.lod_max_clamp)
            && s.compare == o.compare
            && s.anisotropy_clamp == o.anisotropy_clamp
            && s.border_color == o.border_color
    }
}

impl Eq for RenderGraphSamplerDescriptor {}

impl<'g> IntoRenderResource<'g> for RenderGraphSamplerDescriptor {
    type Resource = Sampler;

    fn into_render_resource(
        self,
        graph: &mut RenderGraphBuilder<'g>,
    ) -> RenderHandle<'g, Self::Resource> {
        graph.new_resource(NewRenderResource::FromDescriptor(self))
    }
}

impl<'g> IntoRenderResource<'g> for SamplerDescriptor<'static> {
    type Resource = Sampler;

    fn into_render_resource(
        self,
        graph: &mut RenderGraphBuilder<'g>,
    ) -> RenderHandle<'g, Self::Resource> {
        graph.new_resource(NewRenderResource::FromDescriptor(
            RenderGraphSamplerDescriptor(self),
        ))
    }
}
