use crate::{
    render_graph_2::{Node, ResourceBindings, ResourceSlot},
    render_resource::{RenderResource, ResourceInfo},
    renderer_2::RenderContext,
    texture::TextureDescriptor,
};
use bevy_app::{EventReader, Events};
use bevy_window::{WindowCreated, WindowId, WindowResized, Windows};
use legion::prelude::*;

pub struct WindowSwapChainNode {
    window_id: WindowId,
    use_primary_window: bool,
    window_resized_event_reader: EventReader<WindowResized>,
    window_created_event_reader: EventReader<WindowCreated>,
    swap_chain_resource: Option<RenderResource>,
}

impl Node for WindowSwapChainNode {
    fn output(&self) -> &[ResourceSlot] {
        static OUTPUT: &[ResourceSlot] = &[ResourceSlot::new(
            "swapchain_texture",
            ResourceInfo::Texture,
        )];
        OUTPUT
    }

    fn update(
        &mut self,
        _world: &World,
        resources: &Resources,
        render_context: &mut dyn RenderContext,
        _input: &ResourceBindings,
        output: &mut ResourceBindings,
    ) {
        const WINDOW_TEXTURE: usize = 0;
        let window_created_events = resources.get::<Events<WindowCreated>>().unwrap();
        let window_resized_events = resources.get::<Events<WindowResized>>().unwrap();
        let windows = resources.get::<Windows>().unwrap();

        let render_resources = render_context.resources_mut();
        let window = if self.use_primary_window {
            windows.get_primary().expect("No primary window exists")
        } else {
            windows
            .get(self.window_id)
            .expect("Received window resized event for non-existent window")
        };

        // create window swapchain
        if let Some(_) = window_created_events
            .find_latest(&mut self.window_created_event_reader, |e| {
                e.id == window.id
            })
        {
            render_resources.create_swap_chain(window);
        }

        // resize window swapchain
        if let Some(_) = window_resized_events
            .find_latest(&mut self.window_resized_event_reader, |e| {
                e.id == window.id
            })
        {
            render_resources.create_swap_chain(window);
        }

        output.set(WINDOW_TEXTURE, self.swap_chain_resource.unwrap());
    }
}
