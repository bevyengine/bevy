use crate::{
    render_graph_2::{Node, ResourceBindings, ResourceSlot},
    render_resource::ResourceInfo,
    renderer_2::RenderContext,
};
use bevy_app::{EventReader, Events};
use bevy_window::{WindowCreated, WindowId, WindowResized, Windows};
use legion::prelude::*;

pub enum SwapChainWindowSource {
    Primary,
    Id(WindowId),
}

impl Default for SwapChainWindowSource {
    fn default() -> Self {
        SwapChainWindowSource::Primary
    }
}

pub struct WindowSwapChainNode {
    source_window: SwapChainWindowSource,
    window_created_event_reader: EventReader<WindowCreated>,
    window_resized_event_reader: EventReader<WindowResized>,
}

impl WindowSwapChainNode {
    pub fn new(
        source_window: SwapChainWindowSource,
        window_created_event_reader: EventReader<WindowCreated>,
        window_resized_event_reader: EventReader<WindowResized>,
    ) -> Self {
        WindowSwapChainNode {
            source_window,
            window_created_event_reader,
            window_resized_event_reader,
        }
    }
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
        let window = match self.source_window {
            SwapChainWindowSource::Primary => {
                windows.get_primary().expect("No primary window exists")
            }
            SwapChainWindowSource::Id(id) => windows
                .get(id)
                .expect("Received window resized event for non-existent window"),
        };

        // create window swapchain
        if let Some(_) = window_created_events
            .find_latest(&mut self.window_created_event_reader, |e| e.id == window.id)
        {
            render_resources.create_swap_chain(window);
        }

        // resize window swapchain
        if let Some(_) = window_resized_events
            .find_latest(&mut self.window_resized_event_reader, |e| e.id == window.id)
        {
            render_resources.create_swap_chain(window);
        }

        let swap_chain_texture = render_resources.next_swap_chain_texture(window.id);
        output.set(WINDOW_TEXTURE, swap_chain_texture);
    }
}
