use std::hash::Hash;
use std::{borrow::Borrow, ptr};

use crate::render_resource::{
    BindGroup, BindGroupLayout, Buffer, ComputePipeline, RenderPipeline, Sampler, Texture,
    TextureView,
};

//Note: I never ended up using much of the actual functionality of this, aside from it being Cow
//without requiring Clone.
pub enum RefEq<'a, T> {
    Borrowed(&'a T),
    Owned(T),
}

impl<'a, T> RefEq<'a, T> {
    pub fn reborrow(&'a self) -> Self {
        match self {
            Self::Borrowed(r) => Self::Borrowed(r),
            Self::Owned(t) => Self::Borrowed(t),
        }
    }
}

impl<'a, T: PartialEq> PartialEq for RefEq<'a, T> {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Borrowed(l), Self::Borrowed(r)) => ptr::eq(*l, *r),
            (Self::Owned(l), Self::Owned(r)) => l == r,
            _ => false,
        }
    }
}

impl<'a, T: Eq> Eq for RefEq<'a, T> {}

impl<'a, T: Hash> Hash for RefEq<'a, T> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        core::mem::discriminant(self).hash(state);
        match self {
            RefEq::Borrowed(r) => (*r as *const T).hash(state),
            RefEq::Owned(t) => t.hash(state),
        }
    }
}

impl<'a> Hash for RefEq<'a, Texture> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        core::mem::discriminant(self).hash(state);
        match self {
            RefEq::Borrowed(texture) => (*texture as *const Texture).hash(state),
            RefEq::Owned(texture) => texture.id().hash(state),
        }
    }
}

impl<'a> PartialEq for RefEq<'a, Texture> {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Borrowed(l), Self::Borrowed(r)) => ptr::eq(l, r),
            (Self::Owned(l), Self::Owned(r)) => l.id() == r.id(),
            _ => false,
        }
    }
}

impl<'a> Eq for RefEq<'a, Texture> {}

impl<'a> Hash for RefEq<'a, TextureView> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        core::mem::discriminant(self).hash(state);
        match self {
            RefEq::Borrowed(texture_view) => (*texture_view as *const TextureView).hash(state),
            RefEq::Owned(texture_view) => texture_view.id().hash(state),
        }
    }
}

impl<'a> PartialEq for RefEq<'a, TextureView> {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Borrowed(l), Self::Borrowed(r)) => ptr::eq(l, r),
            (Self::Owned(l), Self::Owned(r)) => l.id() == r.id(),
            _ => false,
        }
    }
}

impl<'a> Eq for RefEq<'a, TextureView> {}

impl<'a> Hash for RefEq<'a, Sampler> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        core::mem::discriminant(self).hash(state);
        match self {
            RefEq::Borrowed(sampler) => (*sampler as *const Sampler).hash(state),
            RefEq::Owned(sampler) => sampler.id().hash(state),
        }
    }
}

impl<'a> PartialEq for RefEq<'a, Sampler> {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Borrowed(l), Self::Borrowed(r)) => ptr::eq(l, r),
            (Self::Owned(l), Self::Owned(r)) => l.id() == r.id(),
            _ => false,
        }
    }
}

impl<'a> Eq for RefEq<'a, Sampler> {}

impl<'a> Hash for RefEq<'a, Buffer> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        core::mem::discriminant(self).hash(state);
        match self {
            RefEq::Borrowed(buffer) => (*buffer as *const Buffer).hash(state),
            RefEq::Owned(buffer) => buffer.id().hash(state),
        }
    }
}

impl<'a> PartialEq for RefEq<'a, Buffer> {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Borrowed(l), Self::Borrowed(r)) => ptr::eq(l, r),
            (Self::Owned(l), Self::Owned(r)) => l.id() == r.id(),
            _ => false,
        }
    }
}

impl<'a> Eq for RefEq<'a, Buffer> {}

impl<'a> Hash for RefEq<'a, RenderPipeline> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        core::mem::discriminant(self).hash(state);
        match self {
            RefEq::Borrowed(render_pipeline) => {
                (*render_pipeline as *const RenderPipeline).hash(state);
            }
            RefEq::Owned(render_pipeline) => render_pipeline.id().hash(state),
        }
    }
}

impl<'a> PartialEq for RefEq<'a, RenderPipeline> {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Borrowed(l), Self::Borrowed(r)) => ptr::eq(l, r),
            (Self::Owned(l), Self::Owned(r)) => l.id() == r.id(),
            _ => false,
        }
    }
}

impl<'a> Eq for RefEq<'a, RenderPipeline> {}

impl<'a> Hash for RefEq<'a, ComputePipeline> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        core::mem::discriminant(self).hash(state);
        match self {
            RefEq::Borrowed(compute_pipeline) => {
                (*compute_pipeline as *const ComputePipeline).hash(state);
            }
            RefEq::Owned(compute_pipeline) => compute_pipeline.id().hash(state),
        }
    }
}

impl<'a> PartialEq for RefEq<'a, ComputePipeline> {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Borrowed(l), Self::Borrowed(r)) => ptr::eq(l, r),
            (Self::Owned(l), Self::Owned(r)) => l.id() == r.id(),
            _ => false,
        }
    }
}

impl<'a> Eq for RefEq<'a, ComputePipeline> {}

impl<'a> Hash for RefEq<'a, BindGroup> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        core::mem::discriminant(self).hash(state);
        match self {
            RefEq::Borrowed(bind_group) => {
                (*bind_group as *const BindGroup).hash(state);
            }
            RefEq::Owned(bind_group) => bind_group.id().hash(state),
        }
    }
}

impl<'a> PartialEq for RefEq<'a, BindGroup> {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Borrowed(l), Self::Borrowed(r)) => ptr::eq(l, r),
            (Self::Owned(l), Self::Owned(r)) => l.id() == r.id(),
            _ => false,
        }
    }
}

impl<'a> Eq for RefEq<'a, BindGroup> {}

impl<T> Borrow<T> for RefEq<'_, T> {
    fn borrow(&self) -> &T {
        match self {
            RefEq::Borrowed(r) => r,
            RefEq::Owned(t) => t,
        }
    }
}

impl<T: Clone> Clone for RefEq<'_, T> {
    fn clone(&self) -> Self {
        match self {
            Self::Borrowed(r) => Self::Borrowed(*r),
            Self::Owned(t) => Self::Owned(t.clone()),
        }
    }
}

impl<'a, T> From<T> for RefEq<'a, T> {
    fn from(value: T) -> Self {
        Self::Owned(value)
    }
}

impl<'a, T> From<&'a T> for RefEq<'a, T> {
    fn from(value: &'a T) -> Self {
        Self::Borrowed(value)
    }
}
