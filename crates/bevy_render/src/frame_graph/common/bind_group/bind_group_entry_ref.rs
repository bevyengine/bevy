use std::num::NonZero;

use variadics_please::all_tuples_with_size;

use crate::frame_graph::{
    FrameGraphBuffer, FrameGraphTexture, ResourceRead, ResourceRef, SamplerInfo, TextureViewInfo,
};

#[derive(Clone)]
pub struct BindGroupEntryRef {
    pub binding: u32,
    pub resource: BindingResourceRef,
}

#[derive(Clone)]
pub enum BindingResourceRef {
    Buffer {
        buffer: ResourceRef<FrameGraphBuffer, ResourceRead>,
        size: Option<NonZero<u64>>,
    },
    Sampler(SamplerInfo),
    TextureView {
        texture: ResourceRef<FrameGraphTexture, ResourceRead>,
        texture_view_info: TextureViewInfo,
    },
}

pub struct BindGroupEntryRefs<const N: usize = 1> {
    entries: [BindGroupEntryRef; N],
}

impl<const N: usize> BindGroupEntryRefs<N> {
    #[inline]
    pub fn sequential(resources: impl IntoBindingResourceRefArray<N>) -> Self {
        let mut i = 0;
        Self {
            entries: resources.into_array().map(|resource| {
                let binding = i;
                i += 1;
                BindGroupEntryRef { binding, resource }
            }),
        }
    }

    #[inline]
    pub fn with_indices(indexed_resources: impl IntoIndexedBindingResourceRefArray<N>) -> Self {
        Self {
            entries: indexed_resources
                .into_array()
                .map(|(binding, resource)| BindGroupEntryRef { binding, resource }),
        }
    }
}

impl BindGroupEntryRefs<1> {
    pub fn single(resource: impl IntoBindingResourceRef) -> [BindGroupEntryRef; 1] {
        [BindGroupEntryRef {
            binding: 0,
            resource: resource.into_binding(),
        }]
    }
}

impl<const N: usize> core::ops::Deref for BindGroupEntryRefs<N> {
    type Target = [BindGroupEntryRef];

    fn deref(&self) -> &[BindGroupEntryRef] {
        &self.entries
    }
}

pub trait IntoBindingResourceRef {
    fn into_binding(self) -> BindingResourceRef;
}

impl IntoBindingResourceRef for &ResourceRef<FrameGraphBuffer, ResourceRead> {
    fn into_binding(self) -> BindingResourceRef {
        BindingResourceRef::Buffer {
            buffer: self.clone(),
            size: None,
        }
    }
}

impl IntoBindingResourceRef for &SamplerInfo {
    fn into_binding(self) -> BindingResourceRef {
        BindingResourceRef::Sampler(self.clone())
    }
}

impl IntoBindingResourceRef for &ResourceRef<FrameGraphTexture, ResourceRead> {
    fn into_binding(self) -> BindingResourceRef {
        BindingResourceRef::TextureView {
            texture: self.clone(),
            texture_view_info: TextureViewInfo::default(),
        }
    }
}

impl IntoBindingResourceRef
    for (
        &ResourceRef<FrameGraphTexture, ResourceRead>,
        &TextureViewInfo,
    )
{
    fn into_binding(self) -> BindingResourceRef {
        BindingResourceRef::TextureView {
            texture: self.0.clone(),
            texture_view_info: self.1.clone(),
        }
    }
}

pub trait IntoBindingResourceRefArray<const N: usize> {
    fn into_array(self) -> [BindingResourceRef; N];
}

macro_rules! impl_to_binding_slice {
    ($N: expr, $(#[$meta:meta])* $(($T: ident, $I: ident)),*) => {
        $(#[$meta])*
        impl< $($T: IntoBindingResourceRef),*> IntoBindingResourceRefArray<$N> for ($($T,)*) {
            #[inline]
            fn into_array(self) -> [BindingResourceRef; $N] {
                let ($($I,)*) = self;
                [$($I.into_binding(), )*]
            }
        }
    }
}

all_tuples_with_size!(impl_to_binding_slice, 1, 32, T, s);

pub trait IntoIndexedBindingResourceRefArray<const N: usize> {
    fn into_array(self) -> [(u32, BindingResourceRef); N];
}

macro_rules! impl_to_indexed_binding_slice {
    ($N: expr, $(($T: ident, $S: ident, $I: ident)),*) => {
        impl< $($T: IntoBindingResourceRef),*> IntoIndexedBindingResourceRefArray< $N> for ($((u32, $T),)*) {
            #[inline]
            fn into_array(self) -> [(u32, BindingResourceRef); $N] {
                let ($(($S, $I),)*) = self;
                [$(($S, $I.into_binding())), *]
            }
        }
    }
}

all_tuples_with_size!(impl_to_indexed_binding_slice, 1, 32, T, n, s);

pub struct DynamicBindGroupEntryRefs {
    entries: Vec<BindGroupEntryRef>,
}

impl DynamicBindGroupEntryRefs {
    pub fn sequential<const N: usize>(entries: impl IntoBindingResourceRefArray<N>) -> Self {
        Self {
            entries: entries
                .into_array()
                .into_iter()
                .enumerate()
                .map(|(ix, resource)| BindGroupEntryRef {
                    binding: ix as u32,
                    resource,
                })
                .collect(),
        }
    }

    pub fn extend_sequential<const N: usize>(
        mut self,
        entries: impl IntoBindingResourceRefArray<N>,
    ) -> Self {
        let start = self.entries.last().unwrap().binding + 1;
        self.entries.extend(
            entries
                .into_array()
                .into_iter()
                .enumerate()
                .map(|(ix, resource)| BindGroupEntryRef {
                    binding: start + ix as u32,
                    resource,
                }),
        );
        self
    }

    pub fn new_with_indices<const N: usize>(
        entries: impl IntoIndexedBindingResourceRefArray<N>,
    ) -> Self {
        Self {
            entries: entries
                .into_array()
                .into_iter()
                .map(|(binding, resource)| BindGroupEntryRef { binding, resource })
                .collect(),
        }
    }

    pub fn extend_with_indices<const N: usize>(
        mut self,
        entries: impl IntoIndexedBindingResourceRefArray<N>,
    ) -> Self {
        self.entries.extend(
            entries
                .into_array()
                .into_iter()
                .map(|(binding, resource)| BindGroupEntryRef { binding, resource }),
        );
        self
    }
}

impl core::ops::Deref for DynamicBindGroupEntryRefs {
    type Target = [BindGroupEntryRef];

    fn deref(&self) -> &[BindGroupEntryRef] {
        &self.entries
    }
}
