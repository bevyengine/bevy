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

pub fn copy_texture_to_texture<'g>(
    graph: &mut RenderGraphBuilder<'_, 'g>,
    src: RenderHandle<'g, Texture>,
    mut dst: RenderHandle<'g, Texture>,
) {
    graph.add_usages(src, TextureUsages::COPY_SRC);
    graph.add_usages(dst, TextureUsages::COPY_DST);

    //wgpu asserts copies are the same size;
    let size = graph.meta(src).size;

    graph.add_node(
        Some("copy_texture_to_texture".into()),
        deps![&src, &mut dst],
        move |ctx, cmds, _| {
            cmds.copy_texture_to_texture(
                ctx.get(src).as_image_copy(),
                ctx.get(dst).as_image_copy(),
                size,
            );
        },
    );
}

pub fn copy_texture_to_buffer<'g>(
    graph: &mut RenderGraphBuilder<'_, 'g>,
    src: RenderHandle<'g, Texture>,
    mut dst: RenderHandle<'g, Buffer>,
    layout: Option<ImageDataLayout>,
) {
    graph.add_usages(src, TextureUsages::COPY_SRC);
    graph.add_usages(dst, BufferUsages::COPY_DST);

    let size = graph.meta(src).size;
    let layout = layout
        .or(graph.meta(dst).layout)
        .expect("ImageDataLayout not provided");

    graph.add_node(
        Some("copy_texture_to_buffer".into()),
        deps![&src, &mut dst],
        move |ctx, cmds, _| {
            cmds.copy_texture_to_buffer(
                ctx.get(src).as_image_copy(),
                ImageCopyBuffer {
                    buffer: ctx.get(dst).deref(),
                    layout,
                },
                size,
            );
        },
    );
}

pub fn copy_buffer_to_texture<'g>(
    graph: &mut RenderGraphBuilder<'_, 'g>,
    src: RenderHandle<'g, Buffer>,
    mut dst: RenderHandle<'g, Texture>,
    layout: Option<ImageDataLayout>,
) {
    graph.add_usages(src, BufferUsages::COPY_SRC);
    graph.add_usages(dst, TextureUsages::COPY_DST);

    let layout = layout
        .or(graph.meta(src).layout)
        .expect("ImageDataLayout not provided");
    let size = graph.meta(dst).size;

    graph.add_node(
        Some("copy_buffer_to_texture".into()),
        deps![&src, &mut dst],
        move |ctx, cmds, _| {
            cmds.copy_buffer_to_texture(
                ImageCopyBuffer {
                    buffer: ctx.get(src).deref(),
                    layout,
                },
                ctx.get(dst).as_image_copy(),
                size,
            );
        },
    );
}

pub fn copy_buffer_to_buffer<'g>(
    graph: &mut RenderGraphBuilder<'_, 'g>,
    src: RenderHandle<'g, Buffer>,
    mut dst: RenderHandle<'g, Buffer>,
) {
    graph.add_usages(src, BufferUsages::COPY_SRC);
    graph.add_usages(dst, BufferUsages::COPY_DST);

    let size = graph.meta(src).descriptor.size;

    graph.add_node(
        Some("copy_buffer_to_buffer".into()),
        deps![&src, &mut dst],
        move |ctx, cmds, _| {
            cmds.copy_buffer_to_buffer(ctx.get(src).deref(), 0, ctx.get(dst).deref(), 0, size);
            //TODO: size and offsets are probably incorrect considering alignment, need to round up?
        },
    );
}

pub trait CopyTo<Dst: UsagesRenderResource>: UsagesRenderResource {
    fn copy_to<'g>(
        graph: &mut RenderGraphBuilder<'_, 'g>,
        src: RenderHandle<'g, Self>,
        dst: RenderHandle<'g, Dst>,
    );
}

impl CopyTo<Texture> for Texture {
    fn copy_to<'g>(
        graph: &mut RenderGraphBuilder<'_, 'g>,
        src: RenderHandle<'g, Texture>,
        dst: RenderHandle<'g, Texture>,
    ) {
        copy_texture_to_texture(graph, src, dst);
    }
}

impl CopyTo<Buffer> for Texture {
    fn copy_to<'g>(
        graph: &mut RenderGraphBuilder<'_, 'g>,
        src: RenderHandle<'g, Texture>,
        dst: RenderHandle<'g, Buffer>,
    ) {
        copy_texture_to_buffer(graph, src, dst, None);
    }
}

impl CopyTo<Texture> for Buffer {
    fn copy_to<'g>(
        graph: &mut RenderGraphBuilder<'_, 'g>,
        src: RenderHandle<'g, Self>,
        dst: RenderHandle<'g, Texture>,
    ) {
        copy_buffer_to_texture(graph, src, dst, None);
    }
}

impl CopyTo<Buffer> for Buffer {
    fn copy_to<'g>(
        graph: &mut RenderGraphBuilder<'_, 'g>,
        src: RenderHandle<'g, Self>,
        dst: RenderHandle<'g, Buffer>,
    ) {
        copy_buffer_to_buffer(graph, src, dst);
    }
}

pub fn copy_to<'g, Src: CopyTo<Dst>, Dst: UsagesRenderResource>(
    graph: &mut RenderGraphBuilder<'_, 'g>,
    src: RenderHandle<'g, Src>,
    dst: RenderHandle<'g, Dst>,
) {
    CopyTo::copy_to(graph, src, dst);
}

pub fn clone<'g, R: CopyTo<R>>(
    graph: &mut RenderGraphBuilder<'_, 'g>,
    resource: RenderHandle<'g, R>,
) -> RenderHandle<'g, R> {
    let meta = graph.meta(resource).clone();
    let new_resource = graph.new_resource(meta);
    copy_to(graph, resource, new_resource);
    new_resource
}
