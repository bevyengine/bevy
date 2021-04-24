use crate::{
    render_graph::{Node, ResourceSlotInfo, ResourceSlots},
    renderer::{RenderContext, RenderResourceId, RenderResourceType},
    texture::{SamplerDescriptor, TextureDescriptor},
};
use bevy_app::{Events, ManualEventReader};
use bevy_asset::HandleUntyped;
use bevy_ecs::world::World;
use bevy_window::{WindowCreated, WindowId, WindowResized, Windows};
use std::borrow::Cow;

use super::TextureNode;

pub struct WindowTextureNode {
    inner: TextureNode,
    window_id: WindowId,
    window_created_event_reader: ManualEventReader<WindowCreated>,
    window_resized_event_reader: ManualEventReader<WindowResized>,
}

impl WindowTextureNode {
    pub const OUT_TEXTURE: &'static str = TextureNode::OUT_TEXTURE;

    pub fn new(
        window_id: WindowId,
        texture_descriptor: TextureDescriptor,
        sampler_descriptor: Option<SamplerDescriptor>,
        handle: Option<HandleUntyped>,
    ) -> Self {
        WindowTextureNode {
            inner: TextureNode::new(texture_descriptor, sampler_descriptor, handle),
            window_id,
            window_created_event_reader: Default::default(),
            window_resized_event_reader: Default::default(),
        }
    }
}

impl Node for WindowTextureNode {
    fn output(&self) -> &[ResourceSlotInfo] {
        static OUTPUT: &[ResourceSlotInfo] = &[ResourceSlotInfo {
            name: Cow::Borrowed(WindowTextureNode::OUT_TEXTURE),
            resource_type: RenderResourceType::Texture,
        }];
        OUTPUT
    }

    fn update(
        &mut self,
        world: &World,
        render_context: &mut dyn RenderContext,
        input: &ResourceSlots,
        output: &mut ResourceSlots,
    ) {
        const WINDOW_TEXTURE: usize = 0;
        let window_created_events = world.get_resource::<Events<WindowCreated>>().unwrap();
        let window_resized_events = world.get_resource::<Events<WindowResized>>().unwrap();
        let windows = world.get_resource::<Windows>().unwrap();

        let window = windows
            .get(self.window_id)
            .expect("Window texture node refers to a non-existent window.");

        if self
            .window_created_event_reader
            .iter(&window_created_events)
            .any(|e| e.id == window.id())
            || self
                .window_resized_event_reader
                .iter(&window_resized_events)
                .any(|e| e.id == window.id())
        {
            let render_resource_context = render_context.resources_mut();
            if let Some(RenderResourceId::Texture(old_texture)) = output.get(WINDOW_TEXTURE) {
                render_resource_context.remove_texture(old_texture);
            }

            self.inner.texture_descriptor_mut().size.width = window.physical_width();
            self.inner.texture_descriptor_mut().size.height = window.physical_height();

            self.inner.update(world, render_context, input, output);
        }
    }
}
