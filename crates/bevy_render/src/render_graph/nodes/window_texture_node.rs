use crate::{
    render_graph::{Node, ResourceSlotInfo, ResourceSlots},
    renderer::{RenderContext, RenderResourceId, RenderResourceType},
    texture::TextureDescriptor,
};
use bevy_app::{prelude::Events, ManualEventReader};
use bevy_ecs::{Resources, World};
use bevy_window::{WindowCreated, WindowId, WindowResized, Windows};
use std::borrow::Cow;

pub struct XRWindowTextureNode {
    descriptor: TextureDescriptor,
    have_texture: bool,
}

impl XRWindowTextureNode {
    pub fn new(descriptor: TextureDescriptor) -> Self {
        XRWindowTextureNode {
            descriptor,
            have_texture: false,
        }
    }
}

impl Node for XRWindowTextureNode {
    fn output(&self) -> &[ResourceSlotInfo] {
        static OUTPUT: &[ResourceSlotInfo] = &[ResourceSlotInfo {
            name: Cow::Borrowed(WindowTextureNode::OUT_TEXTURE),
            resource_type: RenderResourceType::Texture,
        }];
        OUTPUT
    }

    fn update(
        &mut self,
        _world: &World,
        _resources: &Resources,
        render_context: &mut dyn RenderContext,
        _input: &ResourceSlots,
        output: &mut ResourceSlots,
    ) {
        const WINDOW_TEXTURE: usize = 0;

        if !self.have_texture {
            let render_resource_context = render_context.resources_mut();
            if let Some(RenderResourceId::Texture(old_texture)) = output.get(WINDOW_TEXTURE) {
                render_resource_context.remove_texture(old_texture);
            }

            #[cfg(target_os = "android")]
            {
                self.descriptor.size.width = 1440;
                self.descriptor.size.height = 1584;
            }

            #[cfg(not(target_os = "android"))]
            {
                self.descriptor.size.width = 1344;
                self.descriptor.size.height = 1512;
            }
            self.descriptor.size.depth = 2; // two eyes

            let texture_resource = render_resource_context.create_texture(self.descriptor);
            output.set(WINDOW_TEXTURE, RenderResourceId::Texture(texture_resource));

            self.have_texture = true;
        }
    }
}

pub struct WindowTextureNode {
    window_id: WindowId,
    descriptor: TextureDescriptor,
    window_created_event_reader: ManualEventReader<WindowCreated>,
    window_resized_event_reader: ManualEventReader<WindowResized>,
}

impl WindowTextureNode {
    pub const OUT_TEXTURE: &'static str = "texture";

    pub fn new(window_id: WindowId, descriptor: TextureDescriptor) -> Self {
        WindowTextureNode {
            window_id,
            descriptor,
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

            self.descriptor.size.width = window.physical_width();
            self.descriptor.size.height = window.physical_height();
            let texture_resource = render_resource_context.create_texture(self.descriptor);
            output.set(WINDOW_TEXTURE, RenderResourceId::Texture(texture_resource));
        }
    }
}
