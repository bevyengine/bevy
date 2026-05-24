use super::CachedTexture;
use crate::render_resource::{Texture, TextureFormat, TextureView};
use alloc::sync::Arc;
use bevy_color::LinearRgba;
use bevy_platform::sync::OnceLock;
use core::sync::atomic::{AtomicBool, Ordering};
use wgpu::{
    Color as WgpuColor, LoadOp, Operations, RenderPassColorAttachment,
    RenderPassDepthStencilAttachment, StoreOp, TextureViewDescriptor, TextureViewDimension,
};

/// A wrapper for a [`CachedTexture`] that is used as a [`RenderPassColorAttachment`].
#[derive(Clone)]
pub struct ColorAttachment {
    pub texture: CachedTexture,
    pub resolve_target: Option<CachedTexture>,
    pub previous_frame_texture: Option<CachedTexture>,
    clear_color: Option<WgpuColor>,
    is_first_call: Arc<AtomicBool>,
    /// Lazily-populated per-layer `D2` views of [`Self::texture`], for use by
    /// [`Self::get_attachment_for_layer`] / [`Self::get_unsampled_attachment_for_layer`]
    /// when the underlying texture is multi-layer (e.g., post-C2 multiview
    /// prepass). Populated all-at-once on first per-layer access.
    per_layer_views: Arc<OnceLock<Vec<TextureView>>>,
    /// Lazily-populated per-layer `D2` views of [`Self::resolve_target`].
    per_layer_resolve_views: Arc<OnceLock<Vec<TextureView>>>,
    /// One first-call latch per layer of [`Self::texture`], populated lazily
    /// on first per-layer attachment access. Each per-layer slot is flipped
    /// independently so per-eye dispatch (one render pass per layer) clears
    /// each layer exactly once per frame instead of only layer 0.
    per_layer_first_call: Arc<OnceLock<Vec<AtomicBool>>>,
}

impl ColorAttachment {
    pub fn new(
        texture: CachedTexture,
        resolve_target: Option<CachedTexture>,
        previous_frame_texture: Option<CachedTexture>,
        clear_color: Option<WgpuColor>,
    ) -> Self {
        Self {
            texture,
            resolve_target,
            previous_frame_texture,
            clear_color,
            is_first_call: Arc::new(AtomicBool::new(true)),
            per_layer_views: Arc::new(OnceLock::new()),
            per_layer_resolve_views: Arc::new(OnceLock::new()),
            per_layer_first_call: Arc::new(OnceLock::new()),
        }
    }

    /// Get this texture view as an attachment. The attachment will be cleared with a value of
    /// `clear_color` if this is the first time calling this function, otherwise it will be loaded.
    ///
    /// The returned attachment will always have writing enabled (`store: StoreOp::Load`).
    pub fn get_attachment(&self) -> RenderPassColorAttachment<'_> {
        if let Some(resolve_target) = self.resolve_target.as_ref() {
            let first_call = self.is_first_call.fetch_and(false, Ordering::SeqCst);

            RenderPassColorAttachment {
                view: &resolve_target.default_view,
                depth_slice: None,
                resolve_target: Some(&self.texture.default_view),
                ops: Operations {
                    load: match (self.clear_color, first_call) {
                        (Some(clear_color), true) => LoadOp::Clear(clear_color),
                        (None, _) | (Some(_), false) => LoadOp::Load,
                    },
                    store: StoreOp::Store,
                },
            }
        } else {
            self.get_unsampled_attachment()
        }
    }

    /// Get this texture view as an attachment, without the resolve target. The attachment will be cleared with
    /// a value of `clear_color` if this is the first time calling this function, otherwise it will be loaded.
    ///
    /// The returned attachment will always have writing enabled (`store: StoreOp::Load`).
    pub fn get_unsampled_attachment(&self) -> RenderPassColorAttachment<'_> {
        let first_call = self.is_first_call.fetch_and(false, Ordering::SeqCst);

        RenderPassColorAttachment {
            view: &self.texture.default_view,
            depth_slice: None,
            resolve_target: None,
            ops: Operations {
                load: match (self.clear_color, first_call) {
                    (Some(clear_color), true) => LoadOp::Clear(clear_color),
                    (None, _) | (Some(_), false) => LoadOp::Load,
                },
                store: StoreOp::Store,
            },
        }
    }

    /// Get this texture view as an attachment targeting a single layer of the
    /// underlying texture (and resolve target, if any). For single-layer
    /// textures, pass `layer = 0` — the synthesized view is bit-identical to
    /// what [`Self::get_attachment`] returns.
    ///
    /// Used by per-eye dispatch in the prepass / deferred render-graph nodes
    /// under multiview, where each eye is rendered to its own layer in a
    /// separate render pass.
    pub fn get_attachment_for_layer(&self, layer: u32) -> RenderPassColorAttachment<'_> {
        if let Some(resolve_target) = self.resolve_target.as_ref() {
            let first_call = self.first_call_for_layer(layer);
            let resolve_views = self.per_layer_resolve_views.get_or_init(|| {
                build_per_layer_d2_views(
                    &resolve_target.texture,
                    "color_attachment_resolve_layer_view",
                )
            });
            let target_views = self.per_layer_views.get_or_init(|| {
                build_per_layer_d2_views(&self.texture.texture, "color_attachment_layer_view")
            });

            RenderPassColorAttachment {
                view: &resolve_views[layer as usize],
                depth_slice: None,
                resolve_target: Some(&target_views[layer as usize]),
                ops: Operations {
                    load: match (self.clear_color, first_call) {
                        (Some(clear_color), true) => LoadOp::Clear(clear_color),
                        (None, _) | (Some(_), false) => LoadOp::Load,
                    },
                    store: StoreOp::Store,
                },
            }
        } else {
            self.get_unsampled_attachment_for_layer(layer)
        }
    }

    /// Per-layer counterpart to [`Self::get_unsampled_attachment`]. See
    /// [`Self::get_attachment_for_layer`].
    pub fn get_unsampled_attachment_for_layer(&self, layer: u32) -> RenderPassColorAttachment<'_> {
        let first_call = self.first_call_for_layer(layer);
        let target_views = self.per_layer_views.get_or_init(|| {
            build_per_layer_d2_views(&self.texture.texture, "color_attachment_layer_view")
        });

        RenderPassColorAttachment {
            view: &target_views[layer as usize],
            depth_slice: None,
            resolve_target: None,
            ops: Operations {
                load: match (self.clear_color, first_call) {
                    (Some(clear_color), true) => LoadOp::Clear(clear_color),
                    (None, _) | (Some(_), false) => LoadOp::Load,
                },
                store: StoreOp::Store,
            },
        }
    }

    /// Flip the per-layer first-call latch for `layer`, and mark the global
    /// latch as touched so any subsequent legacy `get_attachment` /
    /// `get_unsampled_attachment` call loads instead of re-clearing the
    /// already-per-layer-cleared texture.
    ///
    /// The per-layer slots are seeded from the CURRENT value of the global
    /// latch on first access — a consumer that runs after an earlier-in-the
    /// -frame pass already flipped the global to false (e.g. the transmissive
    /// pass running after the main opaque pass) sees `false` on its first
    /// per-layer access and loads instead of re-clearing the already-rendered
    /// attachment.
    fn first_call_for_layer(&self, layer: u32) -> bool {
        let initial = self.is_first_call.fetch_and(false, Ordering::SeqCst);
        let per_layer_first = self
            .per_layer_first_call
            .get_or_init(|| init_per_layer_first_call(&self.texture.texture, initial));
        per_layer_first[layer as usize].fetch_and(false, Ordering::SeqCst)
    }

    pub(crate) fn mark_as_cleared(&self) {
        self.is_first_call.store(false, Ordering::SeqCst);
        if let Some(per_layer) = self.per_layer_first_call.get() {
            for slot in per_layer {
                slot.store(false, Ordering::SeqCst);
            }
        }
    }
}

/// Synthesizes a single-layer `D2` `TextureView` of each layer of a (possibly
/// multi-layer) texture. Used to back per-layer attachment access in
/// [`ColorAttachment`] and [`DepthAttachment`].
fn build_per_layer_d2_views(texture: &Texture, label: &'static str) -> Vec<TextureView> {
    let layer_count = texture.depth_or_array_layers();
    (0..layer_count)
        .map(|layer| {
            texture.create_view(&TextureViewDescriptor {
                label: Some(label),
                base_array_layer: layer,
                array_layer_count: Some(1),
                dimension: Some(TextureViewDimension::D2),
                ..Default::default()
            })
        })
        .collect()
}

/// Build a vector of first-call latches (one per layer of `texture`) all set
/// to `initial`. Used to back per-layer attachment access in
/// [`ColorAttachment::per_layer_first_call`] and
/// [`DepthAttachment::per_layer_first_call`].
fn init_per_layer_first_call(texture: &Texture, initial: bool) -> Vec<AtomicBool> {
    (0..texture.depth_or_array_layers())
        .map(|_| AtomicBool::new(initial))
        .collect()
}

/// A wrapper for a [`TextureView`] that is used as a depth-only [`RenderPassDepthStencilAttachment`].
#[derive(Clone)]
pub struct DepthAttachment {
    pub view: TextureView,
    /// Underlying multi-layer texture handle, populated only when constructed
    /// via [`Self::new_multi_layer`]. Required by
    /// [`Self::get_attachment_for_layer`] to synthesize per-layer `D2` views.
    multi_layer_texture: Option<Texture>,
    clear_value: Option<f32>,
    is_first_call: Arc<AtomicBool>,
    per_layer_views: Arc<OnceLock<Vec<TextureView>>>,
    /// One first-call latch per layer of [`Self::multi_layer_texture`],
    /// populated lazily on first per-layer attachment access. See
    /// [`ColorAttachment::per_layer_first_call`] for rationale.
    per_layer_first_call: Arc<OnceLock<Vec<AtomicBool>>>,
}

impl DepthAttachment {
    pub fn new(view: TextureView, clear_value: Option<f32>) -> Self {
        Self {
            view,
            multi_layer_texture: None,
            clear_value,
            is_first_call: Arc::new(AtomicBool::new(clear_value.is_some())),
            per_layer_views: Arc::new(OnceLock::new()),
            per_layer_first_call: Arc::new(OnceLock::new()),
        }
    }

    /// Construct a depth attachment backed by a multi-layer texture, enabling
    /// per-layer access via [`Self::get_attachment_for_layer`]. `view` is the
    /// default (multi-layer) view used by [`Self::get_attachment`] — typically
    /// `texture.default_view`.
    pub fn new_multi_layer(
        texture: Texture,
        view: TextureView,
        clear_value: Option<f32>,
    ) -> Self {
        Self {
            view,
            multi_layer_texture: Some(texture),
            clear_value,
            is_first_call: Arc::new(AtomicBool::new(clear_value.is_some())),
            per_layer_views: Arc::new(OnceLock::new()),
            per_layer_first_call: Arc::new(OnceLock::new()),
        }
    }

    /// Get this texture view as an attachment. The attachment will be cleared with a value of
    /// `clear_value` if this is the first time calling this function with `store` == [`StoreOp::Store`],
    /// and a clear value was provided, otherwise it will be loaded.
    pub fn get_attachment(&self, store: StoreOp) -> RenderPassDepthStencilAttachment<'_> {
        let first_call = self
            .is_first_call
            .fetch_and(store != StoreOp::Store, Ordering::Relaxed);

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

    /// Get an attachment targeting a single layer of the underlying multi-layer
    /// depth texture. Used by per-eye dispatch in the prepass / deferred
    /// render-graph nodes under multiview.
    ///
    /// Panics if this attachment was not constructed via
    /// [`Self::new_multi_layer`].
    pub fn get_attachment_for_layer(
        &self,
        layer: u32,
        store: StoreOp,
    ) -> RenderPassDepthStencilAttachment<'_> {
        let texture = self.multi_layer_texture.as_ref().expect(
            "DepthAttachment::get_attachment_for_layer requires the attachment \
             to be constructed with DepthAttachment::new_multi_layer so the \
             underlying multi-layer texture handle is available",
        );
        let per_layer = self
            .per_layer_views
            .get_or_init(|| build_per_layer_d2_views(texture, "depth_attachment_layer_view"));

        // Mark the global latch as touched so any subsequent legacy
        // `get_attachment` call (e.g., main opaque/transparent pass after the
        // prepass per-eye loop) loads instead of re-clearing the depth that
        // was just written per layer. Seed the per-layer slots from the
        // CURRENT global state so a consumer running after an earlier pass
        // already flipped the global (e.g. a main-pass consumer after the
        // prepass per-eye loop) sees `false` on its first per-layer access.
        let initial = self
            .is_first_call
            .fetch_and(store != StoreOp::Store, Ordering::Relaxed);
        let per_layer_first = self.per_layer_first_call.get_or_init(|| {
            init_per_layer_first_call(texture, initial && self.clear_value.is_some())
        });
        let first_call = per_layer_first[layer as usize]
            .fetch_and(store != StoreOp::Store, Ordering::Relaxed);

        RenderPassDepthStencilAttachment {
            view: &per_layer[layer as usize],
            depth_ops: Some(Operations {
                load: if first_call {
                    LoadOp::Clear(self.clear_value.unwrap())
                } else {
                    LoadOp::Load
                },
                store,
            }),
            stencil_ops: None,
        }
    }

    /// Marks this depth attachment as unused this frame so that it'll be
    /// cleared at first use.
    pub fn prepare_for_new_frame(&self) {
        self.is_first_call.store(true, Ordering::Relaxed);
        if let Some(per_layer) = self.per_layer_first_call.get() {
            for slot in per_layer {
                slot.store(true, Ordering::Relaxed);
            }
        }
    }
}

/// A wrapper for a [`TextureView`] that is used as a [`RenderPassColorAttachment`] for a view
/// target's final output texture.
#[derive(Clone)]
pub struct OutputColorAttachment {
    pub view: TextureView,
    pub view_format: TextureFormat,
    is_first_call: Arc<AtomicBool>,
}

impl OutputColorAttachment {
    pub fn new(view: TextureView, view_format: TextureFormat) -> Self {
        Self {
            view,
            view_format,
            is_first_call: Arc::new(AtomicBool::new(true)),
        }
    }

    /// Get this texture view as an attachment. The attachment will be cleared with a value of
    /// the provided `clear_color` if this is the first time calling this function, otherwise it
    /// will be loaded.
    pub fn get_attachment(&self, clear_color: Option<LinearRgba>) -> RenderPassColorAttachment<'_> {
        let first_call = self.is_first_call.fetch_and(false, Ordering::SeqCst);

        RenderPassColorAttachment {
            view: &self.view,
            depth_slice: None,
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

    /// Returns `true` if this attachment has been written to by a render pass.
    // we re-use is_first_call atomic to track usage, which assumes that calls to get_attachment
    // are always consumed by a render pass that writes to the attachment
    pub fn needs_present(&self) -> bool {
        !self.is_first_call.load(Ordering::SeqCst)
    }
}
