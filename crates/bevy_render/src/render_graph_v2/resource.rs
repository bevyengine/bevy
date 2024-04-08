use std::{any::TypeId, marker::PhantomData};

use bevy_ecs::world::World;
use bevy_utils::all_tuples_with_size;

use crate::renderer::RenderDevice;

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

pub struct RenderResourceId {
    type_id: TypeId,
    index: u16,
    generation: u8,
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
