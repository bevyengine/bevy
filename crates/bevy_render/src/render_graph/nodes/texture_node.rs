use bevy_asset::HandleUntyped;
use bevy_ecs::world::World;
use std::borrow::Cow;

use crate::{
    render_graph::{Node, ResourceSlotInfo, ResourceSlots},
    renderer::{RenderContext, RenderResourceId, RenderResourceType},
    texture::{SamplerDescriptor, TextureDescriptor, SAMPLER_ASSET_INDEX, TEXTURE_ASSET_INDEX},
};
pub struct TextureNode {
    texture_descriptor: TextureDescriptor,
    sampler_descriptor: Option<SamplerDescriptor>,
    handle: Option<HandleUntyped>,
    has_changed: bool,
}

impl TextureNode {
    pub const OUT_TEXTURE: &'static str = "texture";
    pub const OUT_SAMPLER: &'static str = "sampler";

    pub fn new(
        texture_descriptor: TextureDescriptor,
        sampler_descriptor: Option<SamplerDescriptor>,
        handle: Option<HandleUntyped>,
    ) -> Self {
        Self {
            texture_descriptor,
            sampler_descriptor,
            handle,
            has_changed: true,
        }
    }
}

impl TextureNode {
    pub fn texture_descriptor(&self) -> &TextureDescriptor {
        &self.texture_descriptor
    }

    pub fn texture_descriptor_mut(&mut self) -> &mut TextureDescriptor {
        self.set_changed();
        &mut self.texture_descriptor
    }

    pub fn sampler_descriptor(&self) -> &Option<SamplerDescriptor> {
        &self.sampler_descriptor
    }

    pub fn sampler_descriptor_mut(&mut self) -> &mut Option<SamplerDescriptor> {
        self.set_changed();
        &mut self.sampler_descriptor
    }

    pub fn set_changed(&mut self) {
        self.has_changed = true;
    }
}

impl Node for TextureNode {
    fn output(&self) -> &[ResourceSlotInfo] {
        static WITHOUT_SAMPLER: &[ResourceSlotInfo] = &[ResourceSlotInfo {
            name: Cow::Borrowed(TextureNode::OUT_TEXTURE),
            resource_type: RenderResourceType::Texture,
        }];
        static WITH_SAMPLER: &[ResourceSlotInfo] = &[
            ResourceSlotInfo {
                name: Cow::Borrowed(TextureNode::OUT_TEXTURE),
                resource_type: RenderResourceType::Texture,
            },
            ResourceSlotInfo {
                name: Cow::Borrowed(TextureNode::OUT_SAMPLER),
                resource_type: RenderResourceType::Sampler,
            },
        ];

        if self.sampler_descriptor.is_none() {
            WITHOUT_SAMPLER
        } else {
            WITH_SAMPLER
        }
    }

    fn update(
        &mut self,
        _world: &World,
        render_context: &mut dyn RenderContext,
        _input: &ResourceSlots,
        output: &mut ResourceSlots,
    ) {
        // Need to update
        if self.has_changed {
            let render_resource_context = render_context.resources_mut();

            // First create new texture
            let texture_id = render_resource_context.create_texture(self.texture_descriptor);

            // And remove the old texture
            if let Some(old_texture) = output.get(Self::OUT_TEXTURE) {
                render_resource_context.remove_texture(old_texture.get_texture().unwrap());
            }

            output.set(Self::OUT_TEXTURE, RenderResourceId::Texture(texture_id));

            // And update handle and output
            if let Some(handle) = &self.handle {
                render_resource_context.set_asset_resource_untyped(
                    handle.clone(),
                    RenderResourceId::Texture(texture_id),
                    TEXTURE_ASSET_INDEX,
                );
            }

            // If a sampler is specified
            if let Some(sampler_descriptor) = self.sampler_descriptor {
                // Create the sampler
                let sampler_id = render_resource_context.create_sampler(&sampler_descriptor);

                // And remove the old sampler
                if let Some(old_sampler) = output.get(Self::OUT_SAMPLER) {
                    render_resource_context.remove_sampler(old_sampler.get_sampler().unwrap());
                }

                output.set(Self::OUT_SAMPLER, RenderResourceId::Sampler(sampler_id));

                // And update handle and output
                if let Some(handle) = &self.handle {
                    render_resource_context.set_asset_resource_untyped(
                        handle.clone(),
                        RenderResourceId::Sampler(sampler_id),
                        SAMPLER_ASSET_INDEX,
                    );
                }
            }

            // Remove changed flag
            self.has_changed = false;
        }
    }
}
