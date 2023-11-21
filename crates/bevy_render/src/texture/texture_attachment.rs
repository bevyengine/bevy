use crate::prelude::Color;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};
use wgpu::{
    LoadOp, Operations, RenderPassColorAttachment, RenderPassDepthStencilAttachment, TextureView,
};

/// A wrapper for a [TextureView] that is used as a [RenderPassColorAttachment].
pub struct ColorAttachment {
    pub view: TextureView,
    pub resolve_target: Option<TextureView>,
    clear_color: Color,
    is_first_call: Arc<AtomicBool>,
}

impl ColorAttachment {
    pub fn new(view: TextureView, resolve_target: Option<TextureView>, clear_color: Color) -> Self {
        Self {
            view,
            resolve_target,
            clear_color,
            is_first_call: Arc::new(AtomicBool::new(true)),
        }
    }

    /// Get this texture view as an attachment. The attachment will be cleared with a value of
    /// `clear_color` if this is the first time calling this function, otherwise the it will be loaded.
    ///
    /// The returned attachment will always have writing enabled (`store: true`).
    pub fn get_attachment(&self) -> RenderPassColorAttachment {
        let first_call = self.is_first_call.fetch_and(false, Ordering::SeqCst);

        RenderPassColorAttachment {
            view: &self.view,
            resolve_target: self.resolve_target.as_ref(),
            ops: Operations {
                load: if first_call {
                    LoadOp::Clear(self.clear_color.into())
                } else {
                    LoadOp::Load
                },
                store: true,
            },
        }
    }
}

/// A wrapper for a [TextureView] that is used as a depth-only [RenderPassDepthStencilAttachment].
pub struct DepthAttachment {
    pub view: TextureView,
    clear_value: f32,
    is_first_call: Arc<AtomicBool>,
}

impl DepthAttachment {
    pub fn new(view: TextureView, clear_value: f32) -> Self {
        Self {
            view,
            clear_value,
            is_first_call: Arc::new(AtomicBool::new(true)),
        }
    }

    /// Get this texture view as an attachment. The attachment will be cleared with a value of
    /// `clear_value` if this is the first time calling this function, otherwise the it will be loaded.
    pub fn get_attachment(&self, store: bool) -> RenderPassDepthStencilAttachment {
        let first_call = self.is_first_call.fetch_and(false, Ordering::SeqCst);

        RenderPassDepthStencilAttachment {
            view: &self.view,
            depth_ops: Some(Operations {
                load: if first_call {
                    LoadOp::Clear(self.clear_value)
                } else {
                    LoadOp::Load
                },
                store,
            }),
            stencil_ops: None,
        }
    }
}
