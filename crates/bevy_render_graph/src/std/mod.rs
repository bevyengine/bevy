use std::any::type_name;
use std::hash::Hash;
use std::mem;

mod builder;
use bevy_render::render_resource::{TextureFormat, TextureView};
pub use builder::*;

mod copy;
pub use copy::*;

mod fullscreen;
pub use fullscreen::*;

use crate::core::debug::{RenderGraphDebug, RenderGraphDebugContext};
use crate::{
    core::{
        resource::{
            IntoRenderDependencies, IntoRenderResource, RenderDependencies, RenderHandle,
            RenderResource, WriteRenderResource,
        },
        RenderGraphBuilder,
    },
    extend_deps,
};

pub struct Swap<'g, R: WriteRenderResource> {
    current: RenderHandle<'g, R>,
    next: RenderHandle<'g, R>,
}

impl<'g, R: WriteRenderResource> Swap<'g, R> {
    pub fn new(
        graph: &mut RenderGraphBuilder<'_, 'g>,
        descriptor: impl IntoRenderResource<'g, Resource = R> + Clone,
    ) -> Self {
        Self {
            current: graph.new_resource(descriptor.clone()),
            next: graph.new_resource(descriptor.clone()),
        }
    }

    pub fn current(&self) -> RenderHandle<'g, R> {
        self.current
    }

    pub fn swap(&mut self) -> SrcDst<'g, R> {
        let src_dst = SrcDst {
            src: self.current,
            dst: self.next,
        };
        mem::swap(&mut self.current, &mut self.next);
        src_dst
    }
}

impl<'g, R: WriteRenderResource> RenderGraphDebug<'g> for Swap<'g, R> {
    fn fmt(
        &self,
        ctx: RenderGraphDebugContext<'_, 'g>,
        f: &mut std::fmt::Formatter,
    ) -> std::fmt::Result {
        f.debug_struct(type_name::<Self>())
            .field("current", &ctx.debug(&self.current))
            .field("next", &ctx.debug(&self.next))
            .finish()
    }
}

pub struct SrcDst<'g, Src: RenderResource, Dst: WriteRenderResource = Src> {
    pub src: RenderHandle<'g, Src>,
    pub dst: RenderHandle<'g, Dst>,
}

impl<'g, Src: RenderResource, Dst: WriteRenderResource> Copy for SrcDst<'g, Src, Dst> {}

impl<'g, Src: RenderResource, Dst: WriteRenderResource> Clone for SrcDst<'g, Src, Dst> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<'g, Src: RenderResource, Dst: WriteRenderResource> PartialEq for SrcDst<'g, Src, Dst> {
    fn eq(&self, other: &Self) -> bool {
        self.src == other.src && self.dst == other.dst
    }
}

impl<'g, Src: RenderResource, Dst: WriteRenderResource> Eq for SrcDst<'g, Src, Dst> {}

impl<'g, Src: RenderResource, Dst: WriteRenderResource> Hash for SrcDst<'g, Src, Dst> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.src.hash(state);
        self.dst.hash(state);
    }
}

impl<'g, Src: RenderResource, Dst: WriteRenderResource> RenderGraphDebug<'g>
    for SrcDst<'g, Src, Dst>
{
    fn fmt(
        &self,
        ctx: RenderGraphDebugContext<'_, 'g>,
        f: &mut std::fmt::Formatter<'_>,
    ) -> std::fmt::Result {
        f.debug_struct(type_name::<Self>())
            .field("src", &ctx.debug(&self.src))
            .field("dst", &ctx.debug(&self.dst))
            .finish()
    }
}

impl<'g, Src: RenderResource, Dst: WriteRenderResource> IntoRenderDependencies<'g>
    for SrcDst<'g, Src, Dst>
{
    fn into_render_dependencies(mut self, dependencies: &mut RenderDependencies<'g>) {
        extend_deps!(dependencies, &self.src, &mut self.dst);
    }
}

pub fn texture_view_format<'g>(
    graph: &RenderGraphBuilder<'_, 'g>,
    texture_view: RenderHandle<'g, TextureView>,
) -> TextureFormat {
    let texture_view_meta = graph.meta(texture_view);
    texture_view_meta
        .descriptor
        .format
        .unwrap_or_else(|| graph.meta(texture_view_meta.texture).format)
}

//many utilities to be added here later
