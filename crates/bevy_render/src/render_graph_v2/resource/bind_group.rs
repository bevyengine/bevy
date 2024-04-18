use std::marker::PhantomData;

use bevy_utils::HashMap;
use wgpu::BindGroupLayoutEntry;

use crate::{
    prelude::Image,
    render_asset::RenderAssets,
    render_graph_v2::NodeContext,
    render_resource::{AsBindGroup, AsBindGroupError, BindGroup, BindGroupLayout},
    renderer::RenderDevice,
    texture::{FallbackImage, GpuImage},
};

#[derive(Default)]
pub struct RenderBindGroups {
    layouts: HashMap<Box<[BindGroupLayoutEntry]>, BindGroupLayout>,
    bind_groups: HashMap<u16, BindGroup>,
    queued_bind_groups: HashMap<u16, Box<dyn AsRenderBindGroup>>,
}

#[derive(Copy, Clone, PartialEq, Eq, Hash)]
pub struct RenderBindGroup<'a> {
    pub(super) id: u16,
    data: PhantomData<&'a BindGroup>,
}

impl<'a> RenderBindGroup<'a> {
    pub(crate) fn as_unsafe(&self) -> UnsafeRenderBindGroup {
        UnsafeRenderBindGroup { id: self.id }
    }
}

pub(crate) struct UnsafeRenderBindGroup {
    pub(super) id: u16,
}

pub trait AsRenderBindGroup: Send + Sync + 'static {
    fn label(&self) -> Option<&'static str>;

    fn bind_group_layout(&self, render_device: &RenderDevice) -> BindGroupLayout {
        render_device
            .create_bind_group_layout(self.label(), &self.bind_group_layout_entries(render_device))
    }

    fn bind_group_layout_entries(&self, render_device: &RenderDevice) -> Vec<BindGroupLayoutEntry>;

    fn bind_group(
        self,
        node_context: NodeContext<'_, ()>, //TODO: PROPER DEPENDENCIES AAAAAAAA
        render_device: &RenderDevice,
        layout: &BindGroupLayout,
    ) -> Result<BindGroup, AsBindGroupError>;
}

impl<B: AsBindGroup + Send + Sync + 'static> AsRenderBindGroup for B {
    fn label(&self) -> Option<&'static str> {
        <B as AsBindGroup>::label()
    }

    fn bind_group_layout_entries(&self, render_device: &RenderDevice) -> Vec<BindGroupLayoutEntry> {
        <B as AsBindGroup>::bind_group_layout_entries(render_device)
    }

    fn bind_group(
        self,
        node_context: NodeContext<'_, ()>,
        render_device: &RenderDevice,
        layout: &BindGroupLayout,
    ) -> Result<BindGroup, AsBindGroupError> {
        let images = node_context
            .get_world_resource::<RenderAssets<GpuImage>>()
            .ok_or(AsBindGroupError::RetryNextUpdate)?;
        let fallback_image = node_context
            .get_world_resource::<FallbackImage>()
            .ok_or(AsBindGroupError::RetryNextUpdate)?;
        Ok(
            <B as AsBindGroup>::as_bind_group(
                &self,
                layout,
                render_device,
                images,
                fallback_image,
            )?
            .bind_group,
        )
    }
}

impl<
        F: FnOnce(NodeContext<'_, ()>, &RenderDevice, &BindGroupLayout) -> BindGroup
            + Send
            + Sync
            + 'static,
    > AsRenderBindGroup for (&'static str, &'static [BindGroupLayoutEntry], F)
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
        node_context: NodeContext<'_, ()>,
        render_device: &RenderDevice,
        layout: &BindGroupLayout,
    ) -> Result<BindGroup, AsBindGroupError> {
        Ok((self.2)(node_context, render_device, layout))
    }
}

impl<
        F: FnOnce(NodeContext<'_, ()>, &RenderDevice, &BindGroupLayout) -> BindGroup
            + Send
            + Sync
            + 'static,
    > AsRenderBindGroup for (&'static [BindGroupLayoutEntry], F)
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
        node_context: NodeContext<'_, ()>,
        render_device: &RenderDevice,
        layout: &BindGroupLayout,
    ) -> Result<BindGroup, AsBindGroupError> {
        Ok((self.1)(node_context, render_device, layout))
    }
}
