use crate::{
    render_graph::{Node, ResourceSlotInfo, ResourceSlots},
    renderer::{RenderContext, RenderResourceId, RenderResourceType},
};
use bevy_app::{Events, ManualEventReader};
use bevy_ecs::world::World;
use bevy_window::{WindowCreated, WindowId, WindowResized, Windows};
use std::borrow::Cow;

pub struct WindowSwapChainNode {
    window_id: WindowId,
    window_created_event_reader: ManualEventReader<WindowCreated>,
    window_resized_event_reader: ManualEventReader<WindowResized>,
}

impl WindowSwapChainNode {
    pub const OUT_TEXTURE: &'static str = "texture";

    pub fn new(window_id: WindowId) -> Self {
        WindowSwapChainNode {
            window_id,
            window_created_event_reader: Default::default(),
            window_resized_event_reader: Default::default(),
        }
    }
}

impl Node for WindowSwapChainNode {
    fn output(&self) -> &[ResourceSlotInfo] {
        static OUTPUT: &[ResourceSlotInfo] = &[ResourceSlotInfo {
            name: Cow::Borrowed(WindowSwapChainNode::OUT_TEXTURE),
            resource_type: RenderResourceType::Texture,
        }];
        OUTPUT
    }

    fn update(
        &mut self,
        world: &World,
        render_context: &mut dyn RenderContext,
        _input: &ResourceSlots,
        output: &mut ResourceSlots,
    ) {
        const WINDOW_TEXTURE: usize = 0;
        let window_created_events = world.get_resource::<Events<WindowCreated>>().unwrap();
        let window_resized_events = world.get_resource::<Events<WindowResized>>().unwrap();
        let windows = world.get_resource::<Windows>().unwrap();

        let window = windows
            .get(self.window_id)
            .expect("Window swapchain node refers to a non-existent window.");

        let render_resource_context = render_context.resources_mut();

        // create window swapchain when window is resized or created
        if self
            .window_created_event_reader
            .iter(window_created_events)
            .any(|e| e.id == window.id())
            || self
                .window_resized_event_reader
                .iter(window_resized_events)
                .any(|e| e.id == window.id())
        {
            render_resource_context.create_swap_chain(window);
        }

        let swap_chain_texture = render_resource_context.next_swap_chain_texture(window);
        output.set(
            WINDOW_TEXTURE,
            RenderResourceId::Texture(swap_chain_texture),
        );
    }
}
