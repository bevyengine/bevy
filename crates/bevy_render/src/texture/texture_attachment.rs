use crate::{
    frame_graph::{
        BluePrintProvider, ColorAttachmentRef, FrameGraphError, PassNodeBuilder, ResourceBoardKey,
        TextureViewInfo, TextureViewRef,
    },
    render_resource::{TextureFormat, TextureView},
};
use alloc::sync::Arc;
use bevy_color::LinearRgba;
use core::sync::atomic::{AtomicBool, Ordering};
use wgpu::{
    Color, LoadOp, Operations, RenderPassColorAttachment, RenderPassDepthStencilAttachment, StoreOp,
};

#[derive(Clone)]
pub struct ColorAttachmentProvider {
    pub texture: ResourceBoardKey,
    pub resolve_target: Option<ResourceBoardKey>,
    clear_color: Option<LinearRgba>,
    is_first_call: Arc<AtomicBool>,
}

impl BluePrintProvider for ColorAttachmentProvider {
    type BluePrint = ColorAttachmentRef;
    fn make_blue_print(
        &self,
        pass_node_builder: &mut PassNodeBuilder,
    ) -> Result<Self::BluePrint, FrameGraphError> {
        let view_ref;

        let mut resolve_target = None;

        if self.resolve_target.is_none() {
            view_ref = TextureViewRef {
                texture_ref: pass_node_builder.read_from_board(&self.texture)?,
                desc: TextureViewInfo::default(),
            };
        } else {
            view_ref = TextureViewRef {
                texture_ref: pass_node_builder
                    .read_from_board(self.resolve_target.as_ref().unwrap())?,
                desc: TextureViewInfo::default(),
            };

            resolve_target = Some(TextureViewRef {
                texture_ref: pass_node_builder.read_from_board(&self.texture)?,
                desc: TextureViewInfo::default(),
            })
        }

        let first_call = self.is_first_call.fetch_and(false, Ordering::SeqCst);

        Ok(ColorAttachmentRef {
            view_ref,
            resolve_target,
            ops: Operations {
                load: match (self.clear_color, first_call) {
                    (Some(clear_color), true) => LoadOp::Clear(clear_color.into()),
                    (None, _) | (Some(_), false) => LoadOp::Load,
                },
                store: StoreOp::Store,
            },
        })
    }
}

impl ColorAttachmentProvider {
    pub fn new(
        texture: ResourceBoardKey,
        resolve_target: Option<ResourceBoardKey>,
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

/// A wrapper for a [`TextureView`] that is used as a depth-only [`RenderPassDepthStencilAttachment`].
#[derive(Clone)]
pub struct DepthAttachment {
    pub view: TextureView,
    clear_value: Option<f32>,
    is_first_call: Arc<AtomicBool>,
}

impl DepthAttachment {
    pub fn new(view: TextureView, clear_value: Option<f32>) -> Self {
        Self {
            view,
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

    /// Get this texture view as an attachment. The attachment will be cleared with a value of
    /// `clear_value` if this is the first time calling this function with `store` == [`StoreOp::Store`],
    /// and a clear value was provided, otherwise it will be loaded.
    pub fn get_attachment(&self, store: StoreOp) -> RenderPassDepthStencilAttachment {
        let first_call = self
            .is_first_call
            .fetch_and(store != StoreOp::Store, Ordering::SeqCst);

        RenderPassDepthStencilAttachment {
            view: &self.view,
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
    pub fn get_attachment(&self, clear_color: Option<LinearRgba>) -> RenderPassColorAttachment {
        let first_call = self.is_first_call.fetch_and(false, Ordering::SeqCst);

        RenderPassColorAttachment {
            view: &self.view,
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
