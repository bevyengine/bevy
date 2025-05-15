use crate::{
    frame_graph::{
        ColorAttachment, ColorAttachmentOwner, DepthStencilAttachmentDrawing, FrameGraphTexture, PassBuilder, ResourceMeta, TextureViewDrawing, TextureViewInfo
    },
    render_resource::{TextureFormat, TextureView},
};
use alloc::sync::Arc;
use bevy_color::LinearRgba;
use core::sync::atomic::{AtomicBool, Ordering};
use std::ops::Deref;
use wgpu::{Color, LoadOp, Operations, StoreOp};

#[derive(Clone)]
pub struct ColorAttachmentHandle {
    pub texture: ResourceMeta<FrameGraphTexture>,
    pub resolve_target: Option<ResourceMeta<FrameGraphTexture>>,
    clear_color: Option<LinearRgba>,
    is_first_call: Arc<AtomicBool>,
}

impl ColorAttachmentHandle {
    pub fn get_unsampled_attachment(
        &self,
        pass_builder: &mut PassBuilder,
    ) -> ColorAttachment {
        let first_call = self.is_first_call.fetch_and(false, Ordering::SeqCst);
        let view = TextureViewDrawing {
            texture: pass_builder.write_material(&self.texture),
            desc: TextureViewInfo::default(),
        };

        ColorAttachment {
            view,
            resolve_target: None,
            ops: Operations {
                load: match (self.clear_color, first_call) {
                    (Some(clear_color), true) => LoadOp::Clear(clear_color.into()),
                    (None, _) | (Some(_), false) => LoadOp::Load,
                },
                store: StoreOp::Store,
            },
        }
    }

    pub fn get_color_attachment(&self, pass_builder: &mut PassBuilder) -> ColorAttachment {
        let view;

        let mut resolve_target = None;

        if self.resolve_target.is_none() {
            view = TextureViewDrawing {
                texture: pass_builder.write_material(&self.texture),
                desc: TextureViewInfo::default(),
            };
        } else {
            view = TextureViewDrawing {
                texture: pass_builder.write_material(self.resolve_target.as_ref().unwrap()),
                desc: TextureViewInfo::default(),
            };

            resolve_target = Some(TextureViewDrawing {
                texture: pass_builder.write_material(&self.texture),
                desc: TextureViewInfo::default(),
            })
        }

        let first_call = self.is_first_call.fetch_and(false, Ordering::SeqCst);

        ColorAttachment {
            view,
            resolve_target,
            ops: Operations {
                load: match (self.clear_color, first_call) {
                    (Some(clear_color), true) => LoadOp::Clear(clear_color.into()),
                    (None, _) | (Some(_), false) => LoadOp::Load,
                },
                store: StoreOp::Store,
            },
        }
    }

    pub fn new(
        texture: ResourceMeta<FrameGraphTexture>,
        resolve_target: Option<ResourceMeta<FrameGraphTexture>>,
        clear_color: Option<LinearRgba>,
    ) -> Self {
        Self {
            texture,
            resolve_target,
            clear_color,
            is_first_call: Arc::new(AtomicBool::new(true)),
        }
    }

    pub(crate) fn mark_as_cleared(&self) {
        self.is_first_call.store(false, Ordering::SeqCst);
    }
}

#[derive(Clone)]
pub struct DepthAttachmentHandle {
    pub texture: ResourceMeta<FrameGraphTexture>,
    pub texture_view_info: TextureViewInfo,
    clear_value: Option<f32>,
    is_first_call: Arc<AtomicBool>,
}

impl DepthAttachmentHandle {
    pub fn new(
        texture: ResourceMeta<FrameGraphTexture>,
        texture_view_info: TextureViewInfo,
        clear_value: Option<f32>,
    ) -> Self {
        Self {
            texture,
            texture_view_info,
            clear_value,
            is_first_call: Arc::new(AtomicBool::new(clear_value.is_some())),
        }
    }

    pub fn get_depth_ops(&self, store: StoreOp) -> Option<Operations<f32>> {
        let first_call = self
            .is_first_call
            .fetch_and(store != StoreOp::Store, Ordering::SeqCst);

        Some(Operations {
            load: if first_call {
                // If first_call is true, then a clear value will always have been provided in the constructor
                LoadOp::Clear(self.clear_value.unwrap())
            } else {
                LoadOp::Load
            },
            store,
        })
    }

    pub fn get_attachment(
        &self,
        store: StoreOp,
        pass_builder: &mut PassBuilder,
    ) -> DepthStencilAttachmentDrawing {
        let first_call = self
            .is_first_call
            .fetch_and(store != StoreOp::Store, Ordering::SeqCst);

        let texture = pass_builder.write_material(&self.texture);

        DepthStencilAttachmentDrawing {
            view: TextureViewDrawing {
                texture,
                desc: self.texture_view_info.clone(),
            },
            depth_ops: Some(Operations {
                load: if first_call {
                    // If first_call is true, then a clear value will always have been provided in the constructor
                    LoadOp::Clear(self.clear_value.unwrap())
                } else {
                    LoadOp::Load
                },
                store,
            }),
            stencil_ops: None,
        }
    }
}

/// A wrapper for a [`TextureView`] that is used as a [`RenderPassColorAttachment`] for a view
/// target's final output texture.
#[derive(Clone)]
pub struct OutputColorAttachment {
    pub view: TextureView,
    pub format: TextureFormat,
    is_first_call: Arc<AtomicBool>,
}

impl OutputColorAttachment {
    pub fn new(view: TextureView, format: TextureFormat) -> Self {
        Self {
            view,
            format,
            is_first_call: Arc::new(AtomicBool::new(true)),
        }
    }

    pub fn get_attachment_operations(&self, clear_color: Option<LinearRgba>) -> Operations<Color> {
        let first_call = self.is_first_call.fetch_and(false, Ordering::SeqCst);
        Operations {
            load: match (clear_color, first_call) {
                (Some(clear_color), true) => LoadOp::Clear(clear_color.into()),
                (None, _) | (Some(_), false) => LoadOp::Load,
            },
            store: StoreOp::Store,
        }
    }

    /// Get this texture view as an attachment. The attachment will be cleared with a value of
    /// the provided `clear_color` if this is the first time calling this function, otherwise it
    /// will be loaded.
    pub fn get_attachment(&self, clear_color: Option<LinearRgba>) -> ColorAttachmentOwner {
        let first_call = self.is_first_call.fetch_and(false, Ordering::SeqCst);

        ColorAttachmentOwner {
            view: self.view.deref().clone(),
            resolve_target: None,
            ops: Operations {
                load: match (clear_color, first_call) {
                    (Some(clear_color), true) => LoadOp::Clear(clear_color.into()),
                    (None, _) | (Some(_), false) => LoadOp::Load,
                },
                store: StoreOp::Store,
            },
        }
    }
}
