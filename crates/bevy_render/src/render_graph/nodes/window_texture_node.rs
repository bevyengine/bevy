use crate::{
    render_graph::{Node, ResourceSlotInfo, ResourceSlots},
    renderer::RenderContext,
    texture::{SamplerDescriptor, TextureDescriptor},
};
use bevy_app::{Events, ManualEventReader};
use bevy_asset::HandleUntyped;
use bevy_ecs::world::World;
use bevy_window::{WindowCreated, WindowId, WindowResized, Windows};

use super::TextureNode;

pub struct WindowTextureNode {
    inner: TextureNode,
    window_id: WindowId,
    window_created_event_reader: ManualEventReader<WindowCreated>,
    window_resized_event_reader: ManualEventReader<WindowResized>,
}

impl WindowTextureNode {
    pub const OUT_TEXTURE: &'static str = TextureNode::OUT_TEXTURE;
    pub const OUT_SAMPLER: &'static str = TextureNode::OUT_SAMPLER;

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
        self.inner.output()
    }

    fn update(
        &mut self,
        world: &World,
        render_context: &mut dyn RenderContext,
        input: &ResourceSlots,
        output: &mut ResourceSlots,
    ) {
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
            // Update TextureNode descriptor
            let mut texture_descriptor = self.inner.texture_descriptor_mut();
            texture_descriptor.size.width = window.physical_width();
            texture_descriptor.size.height = window.physical_height();

            // Pass through into TextureNode
            self.inner.update(world, render_context, input, output);
        }
    }
}
