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
    BindGroupEntryBinding, BindGroupResourceBinding, BindGroupResourceHelper,
    BindingResourceBuffer, BindingResourceTextureView, IntoBindGroupResourceBinding,
};

#[derive(Clone)]
pub struct BindGroupEntryHandle {
    pub binding: u32,
    pub resource: BindGroupResourceHandle,
}

impl BindGroupEntryHandle {
    pub fn get_ref(&self, pass_node_builder: &mut PassNodeBuilder) -> BindGroupEntryBinding {
        let resource = self
            .resource
            .make_binding_group_resource_binding(pass_node_builder);

        BindGroupEntryBinding {
            binding: self.binding,
            resource,
        }
    }
}

#[derive(Clone)]
pub enum BindGroupResourceHandle {
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

impl BindGroupResourceHelper for BindGroupResourceHandle {
    fn make_binding_group_resource_binding(
        &self,
        pass_node_builder: &mut PassNodeBuilder,
    ) -> BindGroupResourceBinding {
        match &self {
            BindGroupResourceHandle::Buffer(handle) => {
                let buffer = pass_node_builder.read(handle.buffer.clone());
                BindingResourceBuffer {
                    buffer,
                    size: handle.size,
                }
                .into_binding()
            }
            BindGroupResourceHandle::Sampler(info) => {
                BindGroupResourceBinding::Sampler(info.clone())
            }
            BindGroupResourceHandle::TextureView(handle) => {
                let texture = pass_node_builder.read(handle.texture.clone());

                BindGroupResourceBinding::TextureView(BindingResourceTextureView {
                    texture,
                    texture_view_info: handle.texture_view_info.clone(),
                })
            }
            BindGroupResourceHandle::TextureViewArray(handles) => {
                let mut target = vec![];
                for handle in handles.iter() {
                    let texture = pass_node_builder.read(handle.texture.clone());
                    target.push(BindingResourceTextureView {
                        texture,
                        texture_view_info: handle.texture_view_info.clone(),
                    });
                }

                BindGroupResourceBinding::TextureViewArray(target)
            }
        }
    }
}

pub trait IntoBindGroupResourceHandle {
    fn into_binding(self) -> BindGroupResourceHandle;
}

impl IntoBindGroupResourceHandle for &BindGroupResourceHandle {
    fn into_binding(self) -> BindGroupResourceHandle {
        self.clone()
    }
}

impl IntoBindGroupResourceHandle for BindGroupResourceHandle {
    fn into_binding(self) -> BindGroupResourceHandle {
        self
    }
}

impl IntoBindGroupResourceHandle for &[BindingResourceTextureViewHandle] {
    fn into_binding(self) -> BindGroupResourceHandle {
        BindGroupResourceHandle::TextureViewArray(self.to_vec())
    }
}

impl IntoBindGroupResourceHandle for BindingResourceTextureViewHandle {
    fn into_binding(self) -> BindGroupResourceHandle {
        BindGroupResourceHandle::TextureView(self)
    }
}

impl IntoBindGroupResourceHandle for &BindingResourceTextureViewHandle {
    fn into_binding(self) -> BindGroupResourceHandle {
        BindGroupResourceHandle::TextureView(self.clone())
    }
}

impl IntoBindGroupResourceHandle for GraphResourceNodeHandle<FrameGraphBuffer> {
    fn into_binding(self) -> BindGroupResourceHandle {
        BindGroupResourceHandle::Buffer(BindingResourceBufferHandle {
            buffer: self,
            size: None,
        })
    }
}

impl IntoBindGroupResourceHandle for &GraphResourceNodeHandle<FrameGraphBuffer> {
    fn into_binding(self) -> BindGroupResourceHandle {
        BindGroupResourceHandle::Buffer(BindingResourceBufferHandle {
            buffer: self.clone(),
            size: None,
        })
    }
}

impl IntoBindGroupResourceHandle for BindingResourceBufferHandle {
    fn into_binding(self) -> BindGroupResourceHandle {
        BindGroupResourceHandle::Buffer(self)
    }
}

impl IntoBindGroupResourceHandle for Sampler {
    fn into_binding(self) -> BindGroupResourceHandle {
        BindGroupResourceHandle::Sampler(self)
    }
}

impl IntoBindGroupResourceHandle for &Sampler {
    fn into_binding(self) -> BindGroupResourceHandle {
        BindGroupResourceHandle::Sampler(self.clone())
    }
}

impl IntoBindGroupResourceHandle for GraphResourceNodeHandle<FrameGraphTexture> {
    fn into_binding(self) -> BindGroupResourceHandle {
        BindGroupResourceHandle::TextureView(BindingResourceTextureViewHandle {
            texture: self,
            texture_view_info: TextureViewInfo::default(),
        })
    }
}

impl IntoBindGroupResourceHandle
    for (
        &GraphResourceNodeHandle<FrameGraphTexture>,
        &TextureViewInfo,
    )
{
    fn into_binding(self) -> BindGroupResourceHandle {
        BindGroupResourceHandle::TextureView(BindingResourceTextureViewHandle {
            texture: self.0.clone(),
            texture_view_info: self.1.clone(),
        })
    }
}

pub trait IntoBindGroupResourceHandleArray<const N: usize> {
    fn into_array(self) -> [BindGroupResourceHandle; N];
}

macro_rules! impl_to_binding_slice {
    ($N: expr, $(#[$meta:meta])* $(($T: ident, $I: ident)),*) => {
        $(#[$meta])*
        impl< $($T: IntoBindGroupResourceHandle),*> IntoBindGroupResourceHandleArray<$N> for ($($T,)*) {
            #[inline]
            fn into_array(self) -> [BindGroupResourceHandle; $N] {
                let ($($I,)*) = self;
                [$($I.into_binding(), )*]
            }
        }
    }
}

all_tuples_with_size!(impl_to_binding_slice, 1, 32, T, s);

pub trait IntoIndexedBindGroupResourceHandleArray<const N: usize> {
    fn into_array(self) -> [(u32, BindGroupResourceHandle); N];
}

macro_rules! impl_to_indexed_binding_slice {
    ($N: expr, $(($T: ident, $S: ident, $I: ident)),*) => {
        impl< $($T: IntoBindGroupResourceHandle),*> IntoIndexedBindGroupResourceHandleArray< $N> for ($((u32, $T),)*) {
            #[inline]
            fn into_array(self) -> [(u32, BindGroupResourceHandle); $N] {
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
    pub fn sequential<const N: usize>(entries: impl IntoBindGroupResourceHandleArray<N>) -> Self {
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
        entries: impl IntoBindGroupResourceHandleArray<N>,
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
        entries: impl IntoIndexedBindGroupResourceHandleArray<N>,
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
        entries: impl IntoIndexedBindGroupResourceHandleArray<N>,
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
