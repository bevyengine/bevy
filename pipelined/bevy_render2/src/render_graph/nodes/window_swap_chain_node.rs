use crate::{
    render_graph::{Node, ResourceSlotInfo, ResourceSlots},
    render_resource::{RenderResourceId, RenderResourceType, SwapChainDescriptor},
    renderer::RenderContext,
};
use bevy_ecs::world::World;
use bevy_utils::HashMap;
use bevy_window::WindowId;
use raw_window_handle::{HasRawWindowHandle, RawWindowHandle};
use std::borrow::Cow;
use std::ops::{Deref, DerefMut};

pub struct ExtractedWindow {
    pub id: WindowId,
    pub handle: RawWindowHandleWrapper,
    pub physical_width: u32,
    pub physical_height: u32,
    pub vsync: bool,
}

#[derive(Default)]
pub struct ExtractedWindows {
    pub windows: HashMap<WindowId, ExtractedWindow>,
}

impl Deref for ExtractedWindows {
    type Target = HashMap<WindowId, ExtractedWindow>;

    fn deref(&self) -> &Self::Target {
        &self.windows
    }
}

impl DerefMut for ExtractedWindows {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.windows
    }
}

pub struct RawWindowHandleWrapper(pub RawWindowHandle);

// TODO: THIS IS NOT SAFE ... ONLY FOR PROTOTYPING
unsafe impl Send for RawWindowHandleWrapper {}
unsafe impl Sync for RawWindowHandleWrapper {}

// TODO: safe?
unsafe impl HasRawWindowHandle for RawWindowHandleWrapper {
    fn raw_window_handle(&self) -> RawWindowHandle {
        self.0.clone()
    }
}

pub struct WindowSwapChainNode {
    window_id: WindowId,
}

impl WindowSwapChainNode {
    pub const OUT_TEXTURE: &'static str = "texture";

    pub fn new(window_id: WindowId) -> Self {
        WindowSwapChainNode { window_id }
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
        let windows = world.get_resource::<ExtractedWindows>().unwrap();
        let window = windows
            .get(&self.window_id)
            .expect("Window swapchain node refers to a non-existent window.");

        let render_resource_context = render_context.resources_mut();
        let swap_chain_descriptor = SwapChainDescriptor {
            window_id: window.id,
            format: crate::texture::TextureFormat::Bgra8UnormSrgb,
            width: window.physical_width,
            height: window.physical_height,
            vsync: window.vsync,
        };

        let swap_chain_texture =
            render_resource_context.next_swap_chain_texture(&swap_chain_descriptor);
        output.set(0, RenderResourceId::Texture(swap_chain_texture));
    }
}
