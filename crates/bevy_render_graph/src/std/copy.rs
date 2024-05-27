use std::ops::Deref;

use bevy_render::render_resource::{
    Buffer, BufferUsages, ImageCopyBuffer, ImageDataLayout, Texture, TextureUsages,
};

use crate::{
    core::{
        resource::{RenderHandle, UsagesRenderResource},
        RenderGraphBuilder,
    },
    deps,
};

use super::SrcDst;

pub fn copy_texture_to_texture<'g>(
    graph: &mut RenderGraphBuilder<'_, 'g>,
    src_dst: SrcDst<'g, Texture>,
) {
    graph.add_usages(src_dst.src, TextureUsages::COPY_SRC);
    graph.add_usages(src_dst.dst, TextureUsages::COPY_DST);

    //wgpu asserts copies are the same size;
    let size = graph.meta(src_dst.src).size;

    graph.add_node(
        Some("copy_texture_to_texture".into()),
        deps![src_dst],
        move |ctx, cmds, _| {
            cmds.copy_texture_to_texture(
                ctx.get(src_dst.src).as_image_copy(),
                ctx.get(src_dst.dst).as_image_copy(),
                size,
            );
        },
    );
}

pub fn copy_texture_to_buffer<'g>(
    graph: &mut RenderGraphBuilder<'_, 'g>,
    src_dst: SrcDst<'g, Texture, Buffer>,
    layout: Option<ImageDataLayout>,
) {
    graph.add_usages(src_dst.src, TextureUsages::COPY_SRC);
    graph.add_usages(src_dst.dst, BufferUsages::COPY_DST);

    let size = graph.meta(src_dst.src).size;
    let layout = layout
        .or(graph.meta(src_dst.dst).layout)
        .expect("ImageDataLayout not provided");

    graph.add_node(
        Some("copy_texture_to_buffer".into()),
        deps![src_dst],
        move |ctx, cmds, _| {
            cmds.copy_texture_to_buffer(
                ctx.get(src_dst.src).as_image_copy(),
                ImageCopyBuffer {
                    buffer: ctx.get(src_dst.dst).deref(),
                    layout,
                },
                size,
            );
        },
    );
}

pub fn copy_buffer_to_texture<'g>(
    graph: &mut RenderGraphBuilder<'_, 'g>,
    src_dst: SrcDst<'g, Buffer, Texture>,
    layout: Option<ImageDataLayout>,
) {
    graph.add_usages(src_dst.src, BufferUsages::COPY_SRC);
    graph.add_usages(src_dst.dst, TextureUsages::COPY_DST);

    let layout = layout
        .or(graph.meta(src_dst.src).layout)
        .expect("ImageDataLayout not provided");
    let size = graph.meta(src_dst.dst).size;

    graph.add_node(
        Some("copy_buffer_to_texture".into()),
        deps![src_dst],
        move |ctx, cmds, _| {
            cmds.copy_buffer_to_texture(
                ImageCopyBuffer {
                    buffer: ctx.get(src_dst.src).deref(),
                    layout,
                },
                ctx.get(src_dst.dst).as_image_copy(),
                size,
            );
        },
    );
}

pub fn copy_buffer_to_buffer<'g>(
    graph: &mut RenderGraphBuilder<'_, 'g>,
    src_dst: SrcDst<'g, Buffer>,
) {
    graph.add_usages(src_dst.src, BufferUsages::COPY_SRC);
    graph.add_usages(src_dst.dst, BufferUsages::COPY_DST);

    let size = graph.meta(src_dst.src).descriptor.size;

    graph.add_node(
        Some("copy_buffer_to_buffer".into()),
        deps![src_dst],
        move |ctx, cmds, _| {
            cmds.copy_buffer_to_buffer(
                ctx.get(src_dst.src).deref(),
                0,
                ctx.get(src_dst.dst).deref(),
                0,
                size,
            );
            //TODO: size and offsets are probably incorrect considering alignment, need to round up?
        },
    );
}

pub trait CopyTo<Dst: UsagesRenderResource>: UsagesRenderResource {
    fn copy_to<'g>(graph: &mut RenderGraphBuilder<'_, 'g>, src_dst: SrcDst<'g, Self, Dst>);
}

impl CopyTo<Texture> for Texture {
    fn copy_to<'g>(graph: &mut RenderGraphBuilder<'_, 'g>, src_dst: SrcDst<'g, Self>) {
        copy_texture_to_texture(graph, src_dst);
    }
}

impl CopyTo<Buffer> for Texture {
    fn copy_to<'g>(graph: &mut RenderGraphBuilder<'_, 'g>, src_dst: SrcDst<'g, Self, Buffer>) {
        copy_texture_to_buffer(graph, src_dst, None);
    }
}

impl CopyTo<Texture> for Buffer {
    fn copy_to<'g>(graph: &mut RenderGraphBuilder<'_, 'g>, src_dst: SrcDst<'g, Self, Texture>) {
        copy_buffer_to_texture(graph, src_dst, None);
    }
}

impl CopyTo<Buffer> for Buffer {
    fn copy_to<'g>(graph: &mut RenderGraphBuilder<'_, 'g>, src_dst: SrcDst<'g, Self>) {
        copy_buffer_to_buffer(graph, src_dst);
    }
}

pub fn copy_to<'g, Src: CopyTo<Dst>, Dst: UsagesRenderResource>(
    graph: &mut RenderGraphBuilder<'_, 'g>,
    src_dst: SrcDst<'g, Src, Dst>,
) {
    CopyTo::copy_to(graph, src_dst);
}

pub fn clone<'g, R: CopyTo<R>>(
    graph: &mut RenderGraphBuilder<'_, 'g>,
    resource: RenderHandle<'g, R>,
) -> RenderHandle<'g, R> {
    let meta = graph.meta(resource).clone();
    let new_resource = graph.new_resource(meta);
    copy_to(
        graph,
        SrcDst {
            src: resource,
            dst: new_resource,
        },
    );
    new_resource
}
