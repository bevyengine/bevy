use std::{any::TypeId, marker::PhantomData};

use bevy_ecs::world::World;
use bevy_utils::all_tuples_with_size;
use wgpu::BindGroupLayoutEntry;

use crate::{
    prelude::Image,
    render_asset::RenderAssets,
    render_resource::{AsBindGroup, AsBindGroupError, BindGroup, BindGroupLayout},
    renderer::RenderDevice,
    texture::FallbackImage,
};

use super::NodeContext;

// /// Handle to a resource for use within a [`super::RenderGraph`].
// #[derive(Clone)] // TODO: Should this be Copy?
// pub struct RenderGraphResource {
//     /// Uniquely identifies a resource within the render graph.
//     pub(crate) id: RenderGraphResourceId,
//     /// Counter starting at 0 that gets incremented every time the resource is modified.
//     pub(crate) generation: u16,
// }
//
// /// Uniquely identifies a resource within a [`super::RenderGraph`].
// pub type RenderGraphResourceId = u16;
//
// /// Usage of a [`RenderGraphResource`] within a [`RenderGraphNode`].
// pub struct RenderGraphResourceUsage {
//     /// The resource used by the node.
//     pub resource: RenderGraphResource,
//     /// How the resource is used by the node.
//     pub usage_type: RenderGraphResourceUsageType,
// }
//
// /// Type of resource usage for a [`RenderGraphResourceUsage`].
// pub enum RenderGraphResourceUsageType {
//     /// Corresponds to [`wgpu::BindingType::Texture`].
//     ReadTexture,
//     /// Corresponds to [`wgpu::BindingType::StorageTexture`] with [`wgpu::StorageTextureAccess::WriteOnly`].
//     WriteTexture,
//     /// Corresponds to [`wgpu::BindingType::StorageTexture`] with [`wgpu::StorageTextureAccess::ReadWrite`].
//     ReadWriteTexture,
// }

pub trait RenderResource: Send + Sync + 'static {}

pub trait IntoRenderResource {
    type Resource: RenderResource;
    fn into_render_resource(self, render_device: &RenderDevice, world: &World) -> Self::Resource;
}

impl<R: RenderResource, F: FnOnce(&RenderDevice) -> R> IntoRenderResource for F {
    type Resource = R;

    fn into_render_resource(self, render_device: &RenderDevice, _world: &World) -> Self::Resource {
        (self)(render_device)
    }
}

pub struct RenderHandle<T: RenderResource> {
    index: u16,
    generation: u16,
    data: PhantomData<T>,
}

impl<T: RenderResource> RenderHandle<T> {
    pub fn r(&self) -> RenderResourceId {
        todo!()
    }

    pub fn w(&mut self) -> RenderResourceId {
        //get id before incremenet
        self.generation += 1;
        todo!()
    }
}

impl<T: RenderResource> Copy for RenderHandle<T> {}
impl<T: RenderResource> Clone for RenderHandle<T> {
    fn clone(&self) -> Self {
        *self
    }
}

pub struct RenderResourceId {
    type_id: TypeId,
    index: u16,
    generation: u16,
    writes: bool,
}

impl<R: RenderResource> Into<RenderResourceId> for &RenderHandle<R> {
    fn into(self) -> RenderResourceId {
        RenderResourceId {
            type_id: TypeId::of::<R>(),
            index: self.index,
            generation: self.generation,
            writes: false,
        }
    }
}

impl<R: RenderResource> Into<RenderResourceId> for &mut RenderHandle<R> {
    fn into(self) -> RenderResourceId {
        RenderResourceId {
            type_id: TypeId::of::<R>(),
            index: self.index,
            generation: self.generation,
            writes: true,
        }
    }
}

pub trait IntoRenderResourceIds<const N: usize> {
    fn into_resource_ids(self) -> [RenderResourceId; N];
}

impl<T: Into<RenderResourceId>> IntoRenderResourceIds<1> for T {
    fn into_resource_ids(self) -> [RenderResourceId; 1] {
        [self.into()]
    }
}

macro_rules! impl_into_render_resource_ids {
    ($N: literal, $(($T: ident, $t: ident)),*) => {
        impl <$($T: Into<RenderResourceId>),*> IntoRenderResourceIds<$N> for ($($T,)*) {
            fn into_resource_ids(self) -> [RenderResourceId; $N] {
                let ($($t,)*) = self;
                [$($t.into()),*]
            }
        }
    };
}

all_tuples_with_size!(impl_into_render_resource_ids, 0, 16, T, t);

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
        let images = node_context
            .get_resource::<RenderAssets<Image>>()
            .ok_or(AsBindGroupError::RetryNextUpdate)?;
        let fallback_image = node_context
            .get_resource::<FallbackImage>()
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
