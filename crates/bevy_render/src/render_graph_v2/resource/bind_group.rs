use bevy_utils::HashMap;
use wgpu::BindGroupLayoutEntry;

use crate::{
    prelude::Image,
    render_asset::RenderAssets,
    render_graph_v2::NodeContext,
    render_resource::{AsBindGroup, AsBindGroupError, BindGroup, BindGroupLayout},
    renderer::RenderDevice,
    texture::FallbackImage,
};

pub struct RenderBindGroups {
    layouts: HashMap<Box<[BindGroupLayoutEntry]>, BindGroupLayout>,
    bind_groups: HashMap<RenderBindGroup, BindGroup>,
    queued_bind_groups: HashMap<RenderBindGroup, ()>, //todo: bind group job type;
}

#[derive(Copy, Clone, PartialEq, Eq, Hash)]
pub struct RenderBindGroup {
    id: u16,
}

pub trait AsRenderBindGroup {
    fn label(&self) -> Option<&'static str>;

    fn bind_group_layout(&self, render_device: &RenderDevice) -> BindGroupLayout {
        render_device
            .create_bind_group_layout(self.label(), &self.bind_group_layout_entries(render_device))
    }

    fn bind_group_layout_entries(&self, render_device: &RenderDevice) -> Vec<BindGroupLayoutEntry>;

    fn bind_group(
        self,
        node_context: NodeContext,
        layout: &BindGroupLayout,
        render_device: &RenderDevice,
    ) -> Result<BindGroup, AsBindGroupError>;
}

impl<B: AsBindGroup> AsRenderBindGroup for B {
    fn label(&self) -> Option<&'static str> {
        <B as AsBindGroup>::label()
    }

    fn bind_group_layout_entries(&self, render_device: &RenderDevice) -> Vec<BindGroupLayoutEntry> {
        <B as AsBindGroup>::bind_group_layout_entries(render_device)
    }

    fn bind_group(
        self,
        node_context: NodeContext,
        layout: &BindGroupLayout,
        render_device: &RenderDevice,
    ) -> Result<BindGroup, AsBindGroupError> {
        // let images = node_context
        //     .get_world_resource::<RenderAssets<Image>>()
        //     .ok_or(AsBindGroupError::RetryNextUpdate)?;
        // let fallback_image = node_context
        //     .get_world_resource::<FallbackImage>()
        //     .ok_or(AsBindGroupError::RetryNextUpdate)?;
        // Ok(
        //     <B as AsBindGroup>::as_bind_group(
        //         &self,
        //         layout,
        //         render_device,
        //         images,
        //         fallback_image,
        //     )?
        //     .bind_group,
        // )
        todo!()
    }
}

impl<
        F: FnOnce(NodeContext, &BindGroupLayout, &RenderDevice) -> Result<BindGroup, AsBindGroupError>,
    > AsRenderBindGroup for (&'static str, &[BindGroupLayoutEntry], F)
{
    fn label(&self) -> Option<&'static str> {
        Some(self.0)
    }

    fn bind_group_layout_entries(
        &self,
        _render_device: &RenderDevice,
    ) -> Vec<BindGroupLayoutEntry> {
        let mut entries = Vec::new();
        entries.extend_from_slice(self.1);
        entries
    }

    fn bind_group(
        self,
        node_context: NodeContext,
        layout: &BindGroupLayout,
        render_device: &RenderDevice,
    ) -> Result<BindGroup, AsBindGroupError> {
        (self.2)(node_context, layout, render_device)
    }
}

impl<
        F: FnOnce(NodeContext, &BindGroupLayout, &RenderDevice) -> Result<BindGroup, AsBindGroupError>,
    > AsRenderBindGroup for (&[BindGroupLayoutEntry], F)
{
    fn label(&self) -> Option<&'static str> {
        None
    }

    fn bind_group_layout_entries(
        &self,
        _render_device: &RenderDevice,
    ) -> Vec<BindGroupLayoutEntry> {
        let mut entries = Vec::new();
        entries.extend_from_slice(self.0);
        entries
    }

    fn bind_group(
        self,
        node_context: NodeContext,
        layout: &BindGroupLayout,
        render_device: &RenderDevice,
    ) -> Result<BindGroup, AsBindGroupError> {
        (self.1)(node_context, layout, render_device)
    }
}
