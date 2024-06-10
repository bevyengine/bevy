use std::ops::Deref;

use bevy_render::render_resource::{
    Buffer, BufferUsages, ImageCopyBuffer, ImageDataLayout, Texture, TextureUsages,
};

use crate::{
    core::{RenderGraphBuilder, RenderHandle, UsagesRenderResource},
    deps,
};

use super::SrcDst;

///Performs a copy operation between two textures which must be of the same size and format.
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

///Performs a copy operation between a texture and a buffer. The copy must not overrun the
///destination buffer. If `layout` is `None` the graph will check if the buffer has an associated
///`ImageDataLayout` in its metadata. If not, the function will `panic!`
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
        .expect("ImageDataLayout not provided for copy operation");

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

///Performs a copy operation between a texture and a buffer. The copy must not overrun the
///destination texture. If `layout` is `None` the graph will check if the buffer has an associated
///`ImageDataLayout` in its metadata. If not, the function will `panic!`
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

///Performs a copy operation between two buffers. The copy must not overrun the destination buffer.
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

///Denotes a resource whose data can be copied to others of type `Dst`
pub trait CopyResource<Dst: UsagesRenderResource>: UsagesRenderResource {
    ///Perfoms a copy from a resource of type `Self` to a resource of type `Dst` in the render
    ///graph
    fn copy_resource<'g>(graph: &mut RenderGraphBuilder<'_, 'g>, src_dst: SrcDst<'g, Self, Dst>);
}

impl CopyResource<Texture> for Texture {
    fn copy_resource<'g>(graph: &mut RenderGraphBuilder<'_, 'g>, src_dst: SrcDst<'g, Self>) {
        copy_texture_to_texture(graph, src_dst);
    }
}

impl CopyResource<Buffer> for Texture {
    fn copy_resource<'g>(
        graph: &mut RenderGraphBuilder<'_, 'g>,
        src_dst: SrcDst<'g, Self, Buffer>,
    ) {
        copy_texture_to_buffer(graph, src_dst, None);
    }
}

impl CopyResource<Texture> for Buffer {
    fn copy_resource<'g>(
        graph: &mut RenderGraphBuilder<'_, 'g>,
        src_dst: SrcDst<'g, Self, Texture>,
    ) {
        copy_buffer_to_texture(graph, src_dst, None);
    }
}

impl CopyResource<Buffer> for Buffer {
    fn copy_resource<'g>(graph: &mut RenderGraphBuilder<'_, 'g>, src_dst: SrcDst<'g, Self>) {
        copy_buffer_to_buffer(graph, src_dst);
    }
}

///Performs a copy operation between two resources. If copying between a texture and a buffer (or
///vice-versa) the buffer must have been created with an associated `ImageDataLayout` in its
///metadata.
pub fn copy<'g, Src: CopyResource<Dst>, Dst: UsagesRenderResource>(
    graph: &mut RenderGraphBuilder<'_, 'g>,
    src_dst: SrcDst<'g, Src, Dst>,
) {
    CopyResource::copy_resource(graph, src_dst);
}

///Clones a resource by creating a new resource from the original's metadata, and performing a copy
///from the original resource to the new one.
pub fn clone<'g, R: CopyResource<R>>(
    graph: &mut RenderGraphBuilder<'_, 'g>,
    resource: RenderHandle<'g, R>,
) -> RenderHandle<'g, R> {
    let meta = graph.meta(resource).clone();
    let new_resource = graph.new_resource(meta);
    copy(
        graph,
        SrcDst {
            src: resource,
            dst: new_resource,
        },
    );
    new_resource
}
