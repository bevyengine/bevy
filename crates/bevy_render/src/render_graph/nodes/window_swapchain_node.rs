use crate::{
    render_graph::{Node, ResourceSlotInfo, ResourceSlots},
    renderer::RenderContext, shader::FieldBindType,
};
use bevy_app::{EventReader, Events};
use bevy_window::{WindowCreated, WindowReference, WindowResized, Windows};
use legion::prelude::*;
use std::borrow::Cow;

pub struct WindowSwapChainNode {
    window_reference: WindowReference,
    window_created_event_reader: EventReader<WindowCreated>,
    window_resized_event_reader: EventReader<WindowResized>,
}

impl WindowSwapChainNode {
    pub const OUT_TEXTURE: &'static str = "texture";
    pub fn new(
        window_reference: WindowReference,
        window_created_event_reader: EventReader<WindowCreated>,
        window_resized_event_reader: EventReader<WindowResized>,
    ) -> Self {
        WindowSwapChainNode {
            window_reference,
            window_created_event_reader,
            window_resized_event_reader,
        }
    }
}

impl Node for WindowSwapChainNode {
    fn output(&self) -> &[ResourceSlotInfo] {
        static OUTPUT: &[ResourceSlotInfo] = &[ResourceSlotInfo {
            name: Cow::Borrowed(WindowSwapChainNode::OUT_TEXTURE),
            resource_type: FieldBindType::Texture,
        }];
        OUTPUT
    }

    fn update(
        &mut self,
        _world: &World,
        resources: &Resources,
        render_context: &mut dyn RenderContext,
        _input: &ResourceSlots,
        output: &mut ResourceSlots,
    ) {
        const WINDOW_TEXTURE: usize = 0;
        let window_created_events = resources.get::<Events<WindowCreated>>().unwrap();
        let window_resized_events = resources.get::<Events<WindowResized>>().unwrap();
        let windows = resources.get::<Windows>().unwrap();

        let window = match self.window_reference {
            WindowReference::Primary => windows.get_primary().expect("No primary window exists"),
            WindowReference::Id(id) => windows
                .get(id)
                .expect("Received window resized event for non-existent window"),
        };

        let render_resources = render_context.resources_mut();

        // create window swapchain when window is resized or created
        if self
            .window_created_event_reader
            .find_latest(&window_created_events, |e| e.id == window.id)
            .is_some()
            || self
                .window_resized_event_reader
                .find_latest(&window_resized_events, |e| e.id == window.id)
                .is_some()
        {
            render_resources.create_swap_chain(window);
        }

        let swap_chain_texture = render_resources.next_swap_chain_texture(window.id);
        output.set(WINDOW_TEXTURE, swap_chain_texture);
    }
}
