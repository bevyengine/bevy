use std::num::NonZero;

use variadics_please::all_tuples_with_size;

use crate::{
    frame_graph::{
        FrameGraphBuffer, FrameGraphTexture, GraphResourceNodeHandle, PassNodeBuilder,
        TextureViewInfo,
    },
    render_resource::Sampler,
};

use super::{
    BindGroupEntryBinding, BindingResourceBufferRef, BindingResourceHelper, BindingResourceRef,
    BindingResourceTextureViewRef, IntoBindingResourceRef,
};

#[derive(Clone)]
pub struct BindGroupEntryHandle {
    pub binding: u32,
    pub resource: BindingResourceHandle,
}

impl BindGroupEntryHandle {
    pub fn get_ref(&self, pass_node_builder: &mut PassNodeBuilder) -> BindGroupEntryBinding {
        let resource = self.resource.make_binding_resource_ref(pass_node_builder);

        BindGroupEntryBinding {
            binding: self.binding,
            resource,
        }
    }
}

#[derive(Clone)]
pub enum BindingResourceHandle {
    Buffer(BindingResourceBufferHandle),
    Sampler(Sampler),
    TextureView(BindingResourceTextureViewHandle),
    TextureViewArray(Vec<BindingResourceTextureViewHandle>),
}

#[derive(Clone)]
pub struct BindingResourceBufferHandle {
    pub buffer: GraphResourceNodeHandle<FrameGraphBuffer>,
    pub size: Option<NonZero<u64>>,
}

#[derive(Clone)]
pub struct BindingResourceTextureViewHandle {
    pub texture: GraphResourceNodeHandle<FrameGraphTexture>,
    pub texture_view_info: TextureViewInfo,
}

impl BindingResourceHelper for BindingResourceHandle {
    fn make_binding_resource_ref(
        &self,
        pass_node_builder: &mut PassNodeBuilder,
    ) -> BindingResourceRef {
        match &self {
            BindingResourceHandle::Buffer(handle) => {
                let buffer = pass_node_builder.read(handle.buffer.clone());
                BindingResourceBufferRef {
                    buffer,
                    size: handle.size,
                }
                .into_binding()
            }
            BindingResourceHandle::Sampler(info) => BindingResourceRef::Sampler(info.clone()),
            BindingResourceHandle::TextureView(handle) => {
                let texture = pass_node_builder.read(handle.texture.clone());
                BindingResourceRef::TextureView {
                    texture,
                    texture_view_info: handle.texture_view_info.clone(),
                }
            }
            BindingResourceHandle::TextureViewArray(handles) => {
                let mut target = vec![];
                for handle in handles.iter() {
                    let texture = pass_node_builder.read(handle.texture.clone());
                    target.push(BindingResourceTextureViewRef {
                        texture,
                        texture_view_info: handle.texture_view_info.clone(),
                    });
                }

                BindingResourceRef::TextureViewArray(target)
            }
        }
    }
}

pub trait IntoBindingResourceHandle {
    fn into_binding(self) -> BindingResourceHandle;
}

impl IntoBindingResourceHandle for &BindingResourceHandle {
    fn into_binding(self) -> BindingResourceHandle {
        self.clone()
    }
}

impl IntoBindingResourceHandle for BindingResourceHandle {
    fn into_binding(self) -> BindingResourceHandle {
        self
    }
}

impl IntoBindingResourceHandle for &[BindingResourceTextureViewHandle] {
    fn into_binding(self) -> BindingResourceHandle {
        BindingResourceHandle::TextureViewArray(self.to_vec())
    }
}

impl IntoBindingResourceHandle for BindingResourceTextureViewHandle {
    fn into_binding(self) -> BindingResourceHandle {
        BindingResourceHandle::TextureView(self)
    }
}

impl IntoBindingResourceHandle for &BindingResourceTextureViewHandle {
    fn into_binding(self) -> BindingResourceHandle {
        BindingResourceHandle::TextureView(self.clone())
    }
}

impl IntoBindingResourceHandle for GraphResourceNodeHandle<FrameGraphBuffer> {
    fn into_binding(self) -> BindingResourceHandle {
        BindingResourceHandle::Buffer(BindingResourceBufferHandle {
            buffer: self,
            size: None,
        })
    }
}

impl IntoBindingResourceHandle for &GraphResourceNodeHandle<FrameGraphBuffer> {
    fn into_binding(self) -> BindingResourceHandle {
        BindingResourceHandle::Buffer(BindingResourceBufferHandle {
            buffer: self.clone(),
            size: None,
        })
    }
}

impl IntoBindingResourceHandle for BindingResourceBufferHandle {
    fn into_binding(self) -> BindingResourceHandle {
        BindingResourceHandle::Buffer(self)
    }
}

impl IntoBindingResourceHandle for Sampler {
    fn into_binding(self) -> BindingResourceHandle {
        BindingResourceHandle::Sampler(self)
    }
}

impl IntoBindingResourceHandle for &Sampler {
    fn into_binding(self) -> BindingResourceHandle {
        BindingResourceHandle::Sampler(self.clone())
    }
}

impl IntoBindingResourceHandle for GraphResourceNodeHandle<FrameGraphTexture> {
    fn into_binding(self) -> BindingResourceHandle {
        BindingResourceHandle::TextureView(BindingResourceTextureViewHandle {
            texture: self,
            texture_view_info: TextureViewInfo::default(),
        })
    }
}

impl IntoBindingResourceHandle
    for (
        &GraphResourceNodeHandle<FrameGraphTexture>,
        &TextureViewInfo,
    )
{
    fn into_binding(self) -> BindingResourceHandle {
        BindingResourceHandle::TextureView(BindingResourceTextureViewHandle {
            texture: self.0.clone(),
            texture_view_info: self.1.clone(),
        })
    }
}

pub trait IntoBindingResourceHandleArray<const N: usize> {
    fn into_array(self) -> [BindingResourceHandle; N];
}

macro_rules! impl_to_binding_slice {
    ($N: expr, $(#[$meta:meta])* $(($T: ident, $I: ident)),*) => {
        $(#[$meta])*
        impl< $($T: IntoBindingResourceHandle),*> IntoBindingResourceHandleArray<$N> for ($($T,)*) {
            #[inline]
            fn into_array(self) -> [BindingResourceHandle; $N] {
                let ($($I,)*) = self;
                [$($I.into_binding(), )*]
            }
        }
    }
}

all_tuples_with_size!(impl_to_binding_slice, 1, 32, T, s);

pub trait IntoIndexedBindingResourceHandleArray<const N: usize> {
    fn into_array(self) -> [(u32, BindingResourceHandle); N];
}

macro_rules! impl_to_indexed_binding_slice {
    ($N: expr, $(($T: ident, $S: ident, $I: ident)),*) => {
        impl< $($T: IntoBindingResourceHandle),*> IntoIndexedBindingResourceHandleArray< $N> for ($((u32, $T),)*) {
            #[inline]
            fn into_array(self) -> [(u32, BindingResourceHandle); $N] {
                let ($(($S, $I),)*) = self;
                [$(($S, $I.into_binding())), *]
            }
        }
    }
}

all_tuples_with_size!(impl_to_indexed_binding_slice, 1, 32, T, n, s);

pub struct DynamicBindGroupEntryHandles {
    entries: Vec<BindGroupEntryHandle>,
}

impl DynamicBindGroupEntryHandles {
    pub fn sequential<const N: usize>(entries: impl IntoBindingResourceHandleArray<N>) -> Self {
        Self {
            entries: entries
                .into_array()
                .into_iter()
                .enumerate()
                .map(|(ix, resource)| BindGroupEntryHandle {
                    binding: ix as u32,
                    resource,
                })
                .collect(),
        }
    }

    pub fn extend_sequential<const N: usize>(
        mut self,
        entries: impl IntoBindingResourceHandleArray<N>,
    ) -> Self {
        let start = self.entries.last().unwrap().binding + 1;
        self.entries.extend(
            entries
                .into_array()
                .into_iter()
                .enumerate()
                .map(|(ix, resource)| BindGroupEntryHandle {
                    binding: start + ix as u32,
                    resource,
                }),
        );
        self
    }

    pub fn new_with_indices<const N: usize>(
        entries: impl IntoIndexedBindingResourceHandleArray<N>,
    ) -> Self {
        Self {
            entries: entries
                .into_array()
                .into_iter()
                .map(|(binding, resource)| BindGroupEntryHandle { binding, resource })
                .collect(),
        }
    }

    pub fn extend_with_indices<const N: usize>(
        mut self,
        entries: impl IntoIndexedBindingResourceHandleArray<N>,
    ) -> Self {
        self.entries.extend(
            entries
                .into_array()
                .into_iter()
                .map(|(binding, resource)| BindGroupEntryHandle { binding, resource }),
        );
        self
    }
}

impl core::ops::Deref for DynamicBindGroupEntryHandles {
    type Target = [BindGroupEntryHandle];

    fn deref(&self) -> &[BindGroupEntryHandle] {
        &self.entries
    }
}
