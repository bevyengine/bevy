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
    /// when the underlying texture is multi-layer (e.g., a multiview camera's
    /// main texture array). Populated all-at-once on first per-layer access.
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
                    None,
                    "color_attachment_resolve_layer_view",
                )
            });
            let target_views = self.per_layer_views.get_or_init(|| {
                build_per_layer_d2_views(&self.texture.texture, None, "color_attachment_layer_view")
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
            build_per_layer_d2_views(&self.texture.texture, None, "color_attachment_layer_view")
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

/// Synthesizes a single-layer `D2` [`TextureView`] of each layer of a
/// (possibly multi-layer) texture. Used to back per-layer attachment access in
/// [`ColorAttachment`] and [`DepthStencilViewAttachment`].
///
/// `aspect` selects the texture aspect for the synthesized views; `None` uses
/// the descriptor default (all aspects), which is what color attachments and
/// combined depth-stencil render targets need.
fn build_per_layer_d2_views(
    texture: &Texture,
    aspect: Option<wgpu::TextureAspect>,
    label: &'static str,
) -> Vec<TextureView> {
    let layer_count = texture.depth_or_array_layers();
    (0..layer_count)
        .map(|layer| {
            texture.create_view(&TextureViewDescriptor {
                label: Some(label),
                base_array_layer: layer,
                array_layer_count: Some(1),
                dimension: Some(TextureViewDimension::D2),
                aspect: aspect.unwrap_or_default(),
                ..Default::default()
            })
        })
        .collect()
}

/// Build a vector of first-call latches (one per layer of `texture`) all set
/// to `initial`. Used to back per-layer attachment access in
/// [`ColorAttachment::per_layer_first_call`] and
/// [`DepthStencilViewAttachment`].
fn init_per_layer_first_call(texture: &Texture, initial: bool) -> Vec<AtomicBool> {
    (0..texture.depth_or_array_layers())
        .map(|_| AtomicBool::new(initial))
        .collect()
}

/// Flip the whole-texture latch (seeding the per-layer slots from its current
/// value on first access) and then flip the requested layer's own slot,
/// returning whether that layer still needs a clear.
///
/// Flipping the whole-texture latch is what makes a later legacy
/// [`DepthStencilViewAttachment::get_attachment`] call load rather than
/// re-clear depth that per-eye dispatch already wrote.
fn layer_latch(
    slots: &OnceLock<Vec<AtomicBool>>,
    global: &AtomicBool,
    texture: &Texture,
    layer: u32,
    store: StoreOp,
) -> bool {
    let initial = global.fetch_and(store != StoreOp::Store, Ordering::Relaxed);
    let per_layer = slots.get_or_init(|| init_per_layer_first_call(texture, initial));
    per_layer[layer as usize].fetch_and(store != StoreOp::Store, Ordering::Relaxed)
}

#[derive(Clone)]
enum DepthStencilData {
    DepthOnly {
        depth_clear_value: Option<f32>,
        depth_is_first_call: Arc<AtomicBool>,
    },
    StencilOnly {
        stencil_clear_value: Option<u32>,
        stencil_is_first_call: Arc<AtomicBool>,
    },
    DepthStencil {
        depth_clear_value: Option<f32>,
        stencil_clear_value: Option<u32>,
        depth_is_first_call: Arc<AtomicBool>,
        stencil_is_first_call: Arc<AtomicBool>,
    },
}

/// Depth and stencil views of a depth texture.
///
/// Texture views with single depth or stencil aspect can be read in shaders.
/// Texture views with all aspect is renderable.
/// See <https://gpuweb.github.io/gpuweb/#depth-formats>
/// and <https://gpuweb.github.io/gpuweb/#abstract-opdef-renderable-texture-view>.
#[derive(Clone)]
pub enum DepthStencilViews {
    /// The texture only has depth aspect.
    DepthOnly { depth_view: TextureView },
    /// The texture only has stencil aspect.
    StencilOnly { stencil_view: TextureView },
    /// The texture has combined depth and stencil.
    DepthStencil {
        /// A texture view with both depth and stencil aspects, renderable, but can't be used as a binding resource.
        combined_view: TextureView,
        /// A texture view with depth only aspect, sample type `unfilterable-float` or `depth` in shaders, not renderable.
        depth_view: TextureView,
        /// A texture view with stencil only aspect, sample type `uint` in shaders, not renderable.
        stencil_view: TextureView,
    },
}

impl DepthStencilViews {
    /// Return [`DepthStencilViews::DepthStencil::depth_view`] or [`DepthStencilViews::DepthOnly::depth_view`] or `None`.
    pub fn depth_only_view(&self) -> Option<&TextureView> {
        match self {
            DepthStencilViews::DepthStencil { depth_view, .. }
            | DepthStencilViews::DepthOnly { depth_view } => Some(depth_view),
            DepthStencilViews::StencilOnly { .. } => None,
        }
    }

    /// Return [`DepthStencilViews::DepthStencil::stencil_view`] or [`DepthStencilViews::StencilOnly::stencil_view`] or `None`.
    pub fn stencil_only_view(&self) -> Option<&TextureView> {
        match self {
            DepthStencilViews::DepthOnly { .. } => None,
            DepthStencilViews::DepthStencil { stencil_view, .. }
            | DepthStencilViews::StencilOnly { stencil_view } => Some(stencil_view),
        }
    }

    /// Return [`DepthStencilViews::DepthStencil::combined_view`] or [`DepthStencilViews::DepthOnly::depth_view`] or [`DepthStencilViews::StencilOnly::stencil_view`].
    pub fn attachment_view(&self) -> &TextureView {
        match self {
            DepthStencilViews::DepthOnly { depth_view } => depth_view,
            DepthStencilViews::StencilOnly { stencil_view } => stencil_view,
            DepthStencilViews::DepthStencil { combined_view, .. } => combined_view,
        }
    }

    /// Create the appropriate views for a depth-stencil texture based on its texture format.
    fn from_texture(texture: &CachedTexture) -> Self {
        let depth_view = || {
            texture.texture.create_view(&TextureViewDescriptor {
                aspect: wgpu::TextureAspect::DepthOnly,
                ..Default::default()
            })
        };

        let stencil_view = || {
            texture.texture.create_view(&TextureViewDescriptor {
                aspect: wgpu::TextureAspect::StencilOnly,
                ..Default::default()
            })
        };

        match texture.texture.format().channels() {
            wgpu_types::TextureChannel::DEPTH_STENCIL => DepthStencilViews::DepthStencil {
                combined_view: texture
                    .texture
                    .create_view(&TextureViewDescriptor::default()),
                depth_view: depth_view(),
                stencil_view: stencil_view(),
            },
            wgpu_types::TextureChannel::DEPTH => DepthStencilViews::DepthOnly {
                depth_view: depth_view(),
            },
            wgpu_types::TextureChannel::STENCIL => DepthStencilViews::StencilOnly {
                stencil_view: stencil_view(),
            },
            _ => {
                panic!(
                    "Can't create depth attachment. Texture format is not a depth-stencil format."
                )
            }
        }
    }
}

/// A wrapper for a [`CachedTexture`] that is used as a depth [`RenderPassDepthStencilAttachment`].
#[derive(Clone)]
pub struct DepthStencilAttachment {
    pub texture: CachedTexture,
    pub previous_frame_texture: Option<CachedTexture>,
    depth_stencil_view_attachment: DepthStencilViewAttachment,
    pub previous_frame_depth_stencil_views: Option<DepthStencilViews>,
}

impl DepthStencilAttachment {
    pub fn depth_stencil_views(&self) -> &DepthStencilViews {
        &self.depth_stencil_view_attachment.depth_stencil_views
    }

    pub fn new(
        texture: CachedTexture,
        previous_frame_texture: Option<CachedTexture>,
        depth_clear_value: Option<f32>,
        stencil_clear_value: Option<u32>,
    ) -> Self {
        let depth_stencil_views = DepthStencilViews::from_texture(&texture);

        let previous_frame_depth_stencil_views = previous_frame_texture
            .as_ref()
            .map(DepthStencilViews::from_texture);

        Self {
            texture,
            previous_frame_texture,
            depth_stencil_view_attachment: DepthStencilViewAttachment::new(
                depth_stencil_views,
                depth_clear_value,
                stencil_clear_value,
            ),
            previous_frame_depth_stencil_views,
        }
    }

    /// Get this texture view as an attachment. The attachment will be cleared with a value of
    /// `clear_value` if this is the first time calling this function with `store` == [`StoreOp::Store`],
    /// and a clear value was provided, otherwise it will be loaded.
    pub fn get_attachment(&self, store: StoreOp) -> RenderPassDepthStencilAttachment<'_> {
        self.depth_stencil_view_attachment.get_attachment(store)
    }

    /// Get an attachment targeting a single layer of the underlying (possibly
    /// multi-layer) depth texture. Used by per-eye dispatch in the prepass /
    /// deferred render-graph nodes under multiview.
    ///
    /// For single-layer textures, pass `layer = 0` — the synthesized view
    /// addresses the same subresource [`Self::get_attachment`] would.
    pub fn get_attachment_for_layer(
        &self,
        layer: u32,
        store: StoreOp,
    ) -> RenderPassDepthStencilAttachment<'_> {
        self.depth_stencil_view_attachment
            .get_attachment_for_layer(&self.texture.texture, layer, store)
    }

    /// Marks this depth attachment as unused this frame so that it'll be
    /// cleared at first use.
    pub fn prepare_for_new_frame(&self) {
        self.depth_stencil_view_attachment.prepare_for_new_frame();
    }
}

/// A wrapper for a [`TextureView`] that is used as a [`RenderPassDepthStencilAttachment`].
#[derive(Clone)]
pub struct DepthStencilViewAttachment {
    pub depth_stencil_views: DepthStencilViews,
    depth_stencil_data: DepthStencilData,
    /// Lazily-populated single-layer `D2` views of the backing texture, one
    /// per array layer, used by [`Self::get_attachment_for_layer`]. The aspect
    /// mirrors whichever view [`DepthStencilViews::attachment_view`] would
    /// return, so per-layer attachments stay renderable in the same way the
    /// whole-texture attachment is.
    per_layer_views: Arc<OnceLock<Vec<TextureView>>>,
    /// One depth first-call latch per array layer. See
    /// [`ColorAttachment::per_layer_first_call`] for the seeding rationale.
    per_layer_depth_first_call: Arc<OnceLock<Vec<AtomicBool>>>,
    /// One stencil first-call latch per array layer.
    per_layer_stencil_first_call: Arc<OnceLock<Vec<AtomicBool>>>,
}

impl DepthStencilViewAttachment {
    pub fn new(
        depth_stencil_views: DepthStencilViews,
        depth_clear_value: Option<f32>,
        stencil_clear_value: Option<u32>,
    ) -> Self {
        let depth_stencil_data = match &depth_stencil_views {
            DepthStencilViews::DepthStencil { .. } => DepthStencilData::DepthStencil {
                depth_clear_value,
                stencil_clear_value,
                depth_is_first_call: Arc::new(AtomicBool::new(depth_clear_value.is_some())),
                stencil_is_first_call: Arc::new(AtomicBool::new(stencil_clear_value.is_some())),
            },
            DepthStencilViews::DepthOnly { .. } => DepthStencilData::DepthOnly {
                depth_clear_value,
                depth_is_first_call: Arc::new(AtomicBool::new(depth_clear_value.is_some())),
            },
            DepthStencilViews::StencilOnly { .. } => DepthStencilData::StencilOnly {
                stencil_clear_value,
                stencil_is_first_call: Arc::new(AtomicBool::new(stencil_clear_value.is_some())),
            },
        };
        Self {
            depth_stencil_views,
            depth_stencil_data,
            per_layer_views: Arc::new(OnceLock::new()),
            per_layer_depth_first_call: Arc::new(OnceLock::new()),
            per_layer_stencil_first_call: Arc::new(OnceLock::new()),
        }
    }

    /// Get this texture view as an attachment. The attachment will be cleared with a value of
    /// `clear_value` if this is the first time calling this function with `store` == [`StoreOp::Store`],
    /// and a clear value was provided, otherwise it will be loaded.
    pub fn get_attachment(&self, store: StoreOp) -> RenderPassDepthStencilAttachment<'_> {
        match (&self.depth_stencil_data, &self.depth_stencil_views) {
            (
                DepthStencilData::DepthStencil {
                    depth_clear_value,
                    stencil_clear_value,
                    depth_is_first_call,
                    stencil_is_first_call,
                },
                DepthStencilViews::DepthStencil { combined_view, .. },
            ) => {
                let depth_is_first_call =
                    depth_is_first_call.fetch_and(store != StoreOp::Store, Ordering::Relaxed);
                let stencil_is_first_call =
                    stencil_is_first_call.fetch_and(store != StoreOp::Store, Ordering::Relaxed);
                RenderPassDepthStencilAttachment {
                    view: combined_view,
                    depth_ops: Some(Operations {
                        load: if depth_is_first_call {
                            // If first_call is true, then a clear value will always have been provided in the constructor
                            LoadOp::Clear(depth_clear_value.unwrap())
                        } else {
                            LoadOp::Load
                        },
                        store,
                    }),
                    stencil_ops: Some(Operations {
                        load: if stencil_is_first_call {
                            // If first_call is true, then a clear value will always have been provided in the constructor
                            LoadOp::Clear(stencil_clear_value.unwrap())
                        } else {
                            LoadOp::Load
                        },
                        store,
                    }),
                }
            }
            (
                DepthStencilData::DepthOnly {
                    depth_clear_value,
                    depth_is_first_call,
                },
                DepthStencilViews::DepthOnly { depth_view },
            ) => {
                let depth_is_first_call =
                    depth_is_first_call.fetch_and(store != StoreOp::Store, Ordering::Relaxed);
                RenderPassDepthStencilAttachment {
                    view: depth_view,
                    depth_ops: Some(Operations {
                        load: if depth_is_first_call {
                            // If first_call is true, then a clear value will always have been provided in the constructor
                            LoadOp::Clear(depth_clear_value.unwrap())
                        } else {
                            LoadOp::Load
                        },
                        store,
                    }),
                    stencil_ops: None,
                }
            }
            (
                DepthStencilData::StencilOnly {
                    stencil_clear_value,
                    stencil_is_first_call,
                },
                DepthStencilViews::StencilOnly { stencil_view },
            ) => {
                let stencil_is_first_call =
                    stencil_is_first_call.fetch_and(store != StoreOp::Store, Ordering::Relaxed);
                RenderPassDepthStencilAttachment {
                    view: stencil_view,
                    depth_ops: None,
                    stencil_ops: Some(Operations {
                        load: if stencil_is_first_call {
                            // If first_call is true, then a clear value will always have been provided in the constructor
                            LoadOp::Clear(stencil_clear_value.unwrap())
                        } else {
                            LoadOp::Load
                        },
                        store,
                    }),
                }
            }

            _ => unreachable!("Mismatched depth stencil data and views"),
        }
    }

    /// Per-layer counterpart to [`Self::get_attachment`], targeting a single
    /// array layer of `texture` (which must be the texture these views were
    /// built from).
    ///
    /// The per-layer first-call latches are seeded from the CURRENT value of
    /// the whole-texture latches, so a consumer that runs after an earlier
    /// pass in the same frame already flipped them (e.g. the transmissive pass
    /// after the main opaque pass) loads instead of re-clearing.
    pub fn get_attachment_for_layer(
        &self,
        texture: &Texture,
        layer: u32,
        store: StoreOp,
    ) -> RenderPassDepthStencilAttachment<'_> {
        // Mirror whichever aspect `DepthStencilViews::attachment_view` would
        // hand out, so the per-layer view stays renderable the same way.
        let aspect = match &self.depth_stencil_views {
            DepthStencilViews::DepthOnly { .. } => Some(wgpu::TextureAspect::DepthOnly),
            DepthStencilViews::StencilOnly { .. } => Some(wgpu::TextureAspect::StencilOnly),
            DepthStencilViews::DepthStencil { .. } => None,
        };
        let per_layer = self.per_layer_views.get_or_init(|| {
            build_per_layer_d2_views(texture, aspect, "depth_stencil_attachment_layer_view")
        });
        let view = &per_layer[layer as usize];

        match &self.depth_stencil_data {
            DepthStencilData::DepthStencil {
                depth_clear_value,
                stencil_clear_value,
                depth_is_first_call,
                stencil_is_first_call,
            } => {
                let depth_first = layer_latch(
                    &self.per_layer_depth_first_call,
                    depth_is_first_call,
                    texture,
                    layer,
                    store,
                );
                let stencil_first = layer_latch(
                    &self.per_layer_stencil_first_call,
                    stencil_is_first_call,
                    texture,
                    layer,
                    store,
                );
                RenderPassDepthStencilAttachment {
                    view,
                    depth_ops: Some(Operations {
                        load: if depth_first {
                            LoadOp::Clear(depth_clear_value.unwrap())
                        } else {
                            LoadOp::Load
                        },
                        store,
                    }),
                    stencil_ops: Some(Operations {
                        load: if stencil_first {
                            LoadOp::Clear(stencil_clear_value.unwrap())
                        } else {
                            LoadOp::Load
                        },
                        store,
                    }),
                }
            }
            DepthStencilData::DepthOnly {
                depth_clear_value,
                depth_is_first_call,
            } => {
                let depth_first = layer_latch(
                    &self.per_layer_depth_first_call,
                    depth_is_first_call,
                    texture,
                    layer,
                    store,
                );
                RenderPassDepthStencilAttachment {
                    view,
                    depth_ops: Some(Operations {
                        load: if depth_first {
                            LoadOp::Clear(depth_clear_value.unwrap())
                        } else {
                            LoadOp::Load
                        },
                        store,
                    }),
                    stencil_ops: None,
                }
            }
            DepthStencilData::StencilOnly {
                stencil_clear_value,
                stencil_is_first_call,
            } => {
                let stencil_first = layer_latch(
                    &self.per_layer_stencil_first_call,
                    stencil_is_first_call,
                    texture,
                    layer,
                    store,
                );
                RenderPassDepthStencilAttachment {
                    view,
                    depth_ops: None,
                    stencil_ops: Some(Operations {
                        load: if stencil_first {
                            LoadOp::Clear(stencil_clear_value.unwrap())
                        } else {
                            LoadOp::Load
                        },
                        store,
                    }),
                }
            }
        }
    }

    /// Marks this depth attachment as unused this frame so that it'll be
    /// cleared at first use.
    pub fn prepare_for_new_frame(&self) {
        for slots in [
            &self.per_layer_depth_first_call,
            &self.per_layer_stencil_first_call,
        ] {
            if let Some(per_layer) = slots.get() {
                for slot in per_layer {
                    slot.store(true, Ordering::Relaxed);
                }
            }
        }
        match &self.depth_stencil_data {
            DepthStencilData::DepthStencil {
                depth_is_first_call,
                stencil_is_first_call,
                ..
            } => {
                depth_is_first_call.store(true, Ordering::Relaxed);
                stencil_is_first_call.store(true, Ordering::Relaxed);
            }
            DepthStencilData::DepthOnly {
                depth_is_first_call,
                ..
            } => {
                depth_is_first_call.store(true, Ordering::Relaxed);
            }
            DepthStencilData::StencilOnly {
                stencil_is_first_call,
                ..
            } => {
                stencil_is_first_call.store(true, Ordering::Relaxed);
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
