use crate::render_resource::{RenderResource, RenderResourceIterator, RenderResources};

impl RenderResources for bevy_transform::prelude::Transform {
    fn render_resources_len(&self) -> usize {
        1
    }

    fn get_render_resource(&self, index: usize) -> Option<&dyn RenderResource> {
        if index == 0 {
            Some(&self.value)
        } else {
            None
        }
    }

    fn get_render_resource_name(&self, index: usize) -> Option<&str> {
        if index == 0 {
            Some("Transform")
        } else {
            None
        }
    }

    fn iter_render_resources(&self) -> RenderResourceIterator {
        RenderResourceIterator::new(self)
    }
}
