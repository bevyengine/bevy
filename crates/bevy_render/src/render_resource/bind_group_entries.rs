use bevy_utils::all_tuples_with_size;
use wgpu::{BindGroupEntry, BindingResource};

use super::{TextureView, Sampler};

/// Helper for constructing bindgroups.
/// 
/// Allows constructing the descriptor's entries as:
/// ```
/// render_device.create_bind_group(&BindGroupDescriptor {
///     label: Some("my_bind_group"),
///     layout: &my_layout,
///     entries: BindGroupEntries::with_indexes((
///         (2, &my_sampler),
///         (3, my_uniform),
///     )).as_slice(),
/// });
/// ```
/// 
/// instead of
/// 
/// ```
/// render_device.create_bind_group(&BindGroupDescriptor {
///     label: Some("my_bind_group"),
///     layout: &my_layout,
///     entries: &[
///         BindGroupEntry {
///             binding: 2,
///             resource: BindingResource::Sampler(&my_sampler),
///         },
///         BindGroupEntry {
///             binding: 3,
///             resource: my_uniform,
///         },
///     ],
/// });
/// ```
/// 
/// or
/// 
/// ```
/// render_device.create_bind_group(&BindGroupDescriptor {
///     label: Some("my_bind_group"),
///     layout: &my_layout,
///     entries: BindGroupEntries::sequential((
///         &my_sampler,
///         my_uniform,
///     )).as_slice(),
/// });
/// ```
/// 
/// instead of 
/// 
/// ```
/// render_device.create_bind_group(&BindGroupDescriptor {
///     label: Some("my_bind_group"),
///     layout: &my_layout,
///     entries: &[
///         BindGroupEntry {
///             binding: 0,
///             resource: BindingResource::Sampler(&my_sampler),
///         },
///         BindGroupEntry {
///             binding: 1,
///             resource: my_uniform,
///         },
///     ],
/// });
/// ```

pub struct BindGroupEntries<'b, const N: usize> {
    entries: [BindGroupEntry<'b>; N]
}

impl<'b, const N: usize> BindGroupEntries<'b, N> {
    #[inline]
    pub fn sequential(resources: impl AsBindingArray<'b, N>) -> Self {
        let mut i = 0;
        Self { 
            entries: resources.as_array().map(|resource| {
                let binding = i;
                i += 1;
                BindGroupEntry{ binding, resource }
            })
        }
    }

    #[inline]
    pub fn with_indexes(indexed_resources: impl AsIndexedBindingArray<'b, N>) -> Self {
        Self { 
            entries: indexed_resources.as_array().map(|(binding, resource)| {
                BindGroupEntry{ binding, resource }
            })
        }
    }
}

impl<'b, const N: usize> std::ops::Deref for BindGroupEntries<'b, N> {
    type Target = [BindGroupEntry<'b>];

    fn deref(&self) -> &[BindGroupEntry<'b>] { 
        &self.entries
    }
}

pub trait AsBinding<'a> {
    fn as_binding(self) -> BindingResource<'a>;
}

impl<'a> AsBinding<'a> for &'a TextureView {
    #[inline]
    fn as_binding(self) -> BindingResource<'a> {
        BindingResource::TextureView(self)
    }
}

impl<'a> AsBinding<'a> for &'a[&'a wgpu::TextureView] {
    #[inline]
    fn as_binding(self) -> BindingResource<'a> {
        BindingResource::TextureViewArray(self)
    }
}

impl<'a> AsBinding<'a> for &'a Sampler {
    #[inline]
    fn as_binding(self) -> BindingResource<'a> {
        BindingResource::Sampler(self)
    }
}

impl<'a> AsBinding<'a> for BindingResource<'a> {
    #[inline]
    fn as_binding(self) -> BindingResource<'a> {
        self
    }
}

pub trait AsBindingArray<'b, const N: usize> {
    fn as_array(self) -> [BindingResource<'b>; N];
}



macro_rules! impl_to_binding_slice {
    ($N: expr, $(($T: ident, $I: ident)),*) => {
        impl<'b, $($T: AsBinding<'b>),*> AsBindingArray<'b, $N> for ($($T,)*) {
            #[inline]
            fn as_array(self) -> [BindingResource<'b>; $N] {
                let ($($I,)*) = self;
                [$($I.as_binding(), )*]
            }
        }
    }
}

all_tuples_with_size!(impl_to_binding_slice, 1, 32, T, s);

pub trait AsIndexedBindingArray<'b, const N: usize> {
    fn as_array(self) -> [(u32, BindingResource<'b>); N];
}

macro_rules! impl_to_indexed_binding_slice {
    ($N: expr, $(($T: ident, $S: ident, $I: ident)),*) => {
        impl<'b, $($T: AsBinding<'b>),*> AsIndexedBindingArray<'b, $N> for ($((u32, $T),)*) {
            #[inline]
            fn as_array(self) -> [(u32, BindingResource<'b>); $N] {
                let ($(($S, $I),)*) = self;
                [$(($S, $I.as_binding())), *]
            }
        }
    }
}

all_tuples_with_size!(impl_to_indexed_binding_slice, 1, 32, T, n, s);

pub struct DynamicBindGroupEntries<'b> {
    entries: Vec<BindGroupEntry<'b>>,
}

impl<'b> DynamicBindGroupEntries<'b> {
    pub fn sequential<const N: usize>(entries: impl AsBindingArray<'b, N>) -> Self {
        Self {
            entries: entries.as_array().into_iter().enumerate().map(|(ix, resource)| BindGroupEntry { binding: ix as u32, resource }).collect()
        }
    }

    pub fn extend_sequential<const N: usize>(mut self, entries: impl AsBindingArray<'b, N>) -> Self {
        let start = self.entries.last().unwrap().binding + 1;
        self.entries.extend(entries.as_array().into_iter().enumerate().map(|(ix, resource)| BindGroupEntry { binding: start + ix as u32, resource }));
        self
    }

    pub fn new_with_indexes<const N: usize>(entries: impl AsIndexedBindingArray<'b, N>) -> Self {
        Self {
            entries: entries.as_array().into_iter().map(|(binding, resource)| BindGroupEntry { binding, resource }).collect()
        }
    }

    pub fn extend_with_indexes<const N: usize>(mut self, entries: impl AsIndexedBindingArray<'b, N>) -> Self {
        self.entries.extend(entries.as_array().into_iter().map(|(binding, resource)| BindGroupEntry { binding, resource }));
        self
    }
}

impl<'b> std::ops::Deref for DynamicBindGroupEntries<'b> {
    type Target = [BindGroupEntry<'b>];

    fn deref(&self) -> &[BindGroupEntry<'b>] { 
        &self.entries
    }
}