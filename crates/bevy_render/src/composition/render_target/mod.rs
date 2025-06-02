mod manual_texture_view;
mod window;
use bevy_app::{App, Plugin};
pub use manual_texture_view::*;
pub use window::*;

use bevy_asset::{AssetId, Assets, Handle};
use bevy_ecs::{
    component::{Component, HookContext},
    entity::{ContainsEntity, Entity},
    world::DeferredWorld,
};
use bevy_image::Image;
use bevy_math::{FloatOrd, UVec2};
use bevy_platform::collections::HashSet;
use bevy_reflect::Reflect;
use bevy_window::{NormalizedWindowRef, Window, WindowRef};
use derive_more::derive::From;
use tracing::warn;
use wgpu::TextureFormat;

use crate::{
    extract_resource::ExtractResourcePlugin, render_asset::RenderAssets,
    render_resource::TextureView, texture::GpuImage,
};

use super::{Compositor, CompositorEvent};

pub struct RenderTargetPlugin;

impl Plugin for RenderTargetPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<ManualTextureViews>().add_plugins((
            WindowRenderPlugin,
            ExtractResourcePlugin::<ManualTextureViews>::default(),
        ));
    }
}

/// Information about the current [`RenderTarget`].
#[derive(Default, Debug, Clone)]
pub struct RenderTargetInfo {
    /// The physical size of this render target (in physical pixels, ignoring scale factor).
    pub physical_size: UVec2,
    /// The scale factor of this render target.
    ///
    /// When rendering to a window, typically it is a value greater or equal than 1.0,
    /// representing the ratio between the size of the window in physical pixels and the logical size of the window.
    pub scale_factor: f32,
}

/// The "target" that a [`Camera`] will render to. For example, this could be a [`Window`]
/// swapchain or an [`Image`].
#[derive(Component, Debug, Clone, Reflect, From)]
#[component(immutable, on_insert = Self::on_insert, on_remove = Self::on_remove)]
#[reflect(Clone)]
pub enum RenderTarget {
    /// Window to which the camera's view is rendered.
    Window(WindowRef),
    /// Image to which the camera's view is rendered.
    Image(ImageRenderTarget),
    /// Texture View to which the camera's view is rendered.
    /// Useful when the texture view needs to be created outside of Bevy, for example OpenXR.
    TextureView(ManualTextureViewHandle),
}

impl RenderTarget {
    fn on_insert(mut world: DeferredWorld, ctx: HookContext) {
        world.trigger_targets(CompositorEvent::CompositorChanged, ctx.entity);
    }

    fn on_remove(mut world: DeferredWorld, ctx: HookContext) {
        world.trigger_targets(CompositorEvent::CompositorChanged, ctx.entity);

        if world
            .get_entity(ctx.entity)
            .is_ok_and(|e| e.get::<Compositor>().is_some())
        {
            world
                .commands()
                .entity(ctx.entity)
                .insert(RenderTarget::default());

            warn!(
                "{}Entity {} has a Compositor component but its RenderTarget was removed. Reinserting a default RenderTarget.",
                ctx.caller.map(|location| format!("{location}: ")).unwrap_or_default(), ctx.entity,
            );
        }
    }
}

/// A render target that renders to an [`Image`].
#[derive(Debug, Clone, Reflect, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[reflect(Clone, PartialEq, Hash)]
pub struct ImageRenderTarget {
    /// The image to render to.
    pub handle: Handle<Image>,
    /// The scale factor of the render target image, corresponding to the scale
    /// factor for a window target. This should almost always be 1.0.
    pub scale_factor: FloatOrd,
}

impl From<Handle<Image>> for RenderTarget {
    fn from(handle: Handle<Image>) -> Self {
        Self::Image(handle.into())
    }
}

impl From<Handle<Image>> for ImageRenderTarget {
    fn from(handle: Handle<Image>) -> Self {
        Self {
            handle,
            scale_factor: FloatOrd(1.0),
        }
    }
}

impl Default for RenderTarget {
    fn default() -> Self {
        Self::Window(Default::default())
    }
}

/// Normalized version of the render target.
///
/// Once we have this we shouldn't need to resolve it down anymore.
#[derive(Debug, Clone, Reflect, PartialEq, Eq, Hash, PartialOrd, Ord, From)]
#[reflect(Clone, PartialEq, Hash)]
pub enum NormalizedRenderTarget {
    /// Window to which the camera's view is rendered.
    Window(NormalizedWindowRef),
    /// Image to which the camera's view is rendered.
    Image(ImageRenderTarget),
    /// Texture View to which the camera's view is rendered.
    /// Useful when the texture view needs to be created outside of Bevy, for example OpenXR.
    TextureView(ManualTextureViewHandle),
}

impl RenderTarget {
    /// Normalize the render target down to a more concrete value, mostly used for equality comparisons.
    pub fn normalize(&self, primary_window: Option<Entity>) -> Option<NormalizedRenderTarget> {
        match self {
            RenderTarget::Window(window_ref) => window_ref
                .normalize(primary_window)
                .map(NormalizedRenderTarget::Window),
            RenderTarget::Image(handle) => Some(NormalizedRenderTarget::Image(handle.clone())),
            RenderTarget::TextureView(id) => Some(NormalizedRenderTarget::TextureView(*id)),
        }
    }

    /// Get a handle to the render target's image,
    /// or `None` if the render target is another variant.
    pub fn as_image(&self) -> Option<&Handle<Image>> {
        if let Self::Image(image_target) = self {
            Some(&image_target.handle)
        } else {
            None
        }
    }
}

impl NormalizedRenderTarget {
    pub fn get_texture_view<'a>(
        &self,
        windows: &'a ExtractedWindows,
        images: &'a RenderAssets<GpuImage>,
        manual_texture_views: &'a ManualTextureViews,
    ) -> Option<&'a TextureView> {
        match self {
            NormalizedRenderTarget::Window(window_ref) => windows
                .get(&window_ref.entity())
                .and_then(|window| window.swap_chain_texture_view.as_ref()),
            NormalizedRenderTarget::Image(image_target) => images
                .get(&image_target.handle)
                .map(|image| &image.texture_view),
            NormalizedRenderTarget::TextureView(id) => {
                manual_texture_views.get(id).map(|tex| &tex.texture_view)
            }
        }
    }

    /// Retrieves the [`TextureFormat`] of this render target, if it exists.
    pub fn get_texture_format<'a>(
        &self,
        windows: &'a ExtractedWindows,
        images: &'a RenderAssets<GpuImage>,
        manual_texture_views: &'a ManualTextureViews,
    ) -> Option<TextureFormat> {
        match self {
            NormalizedRenderTarget::Window(window_ref) => windows
                .get(&window_ref.entity())
                .and_then(|window| window.swap_chain_texture_format),
            NormalizedRenderTarget::Image(image_target) => images
                .get(&image_target.handle)
                .map(|image| image.texture_format),
            NormalizedRenderTarget::TextureView(id) => {
                manual_texture_views.get(id).map(|tex| tex.format)
            }
        }
    }

    pub fn get_render_target_info<'a>(
        &self,
        resolutions: impl IntoIterator<Item = (Entity, &'a Window)>,
        images: &Assets<Image>,
        manual_texture_views: &ManualTextureViews,
    ) -> Option<RenderTargetInfo> {
        match self {
            NormalizedRenderTarget::Window(window_ref) => resolutions
                .into_iter()
                .find(|(entity, _)| *entity == window_ref.entity())
                .map(|(_, window)| RenderTargetInfo {
                    physical_size: window.physical_size(),
                    scale_factor: window.resolution.scale_factor(),
                }),
            NormalizedRenderTarget::Image(image_target) => {
                let image = images.get(&image_target.handle)?;
                Some(RenderTargetInfo {
                    physical_size: image.size(),
                    scale_factor: image_target.scale_factor.0,
                })
            }
            NormalizedRenderTarget::TextureView(id) => {
                manual_texture_views.get(id).map(|tex| RenderTargetInfo {
                    physical_size: tex.size,
                    scale_factor: 1.0,
                })
            }
        }
    }

    // Check if this render target is contained in the given changed windows or images.
    fn is_changed(
        &self,
        changed_window_ids: &HashSet<Entity>,
        changed_image_handles: &HashSet<&AssetId<Image>>,
    ) -> bool {
        match self {
            NormalizedRenderTarget::Window(window_ref) => {
                changed_window_ids.contains(&window_ref.entity())
            }
            NormalizedRenderTarget::Image(image_target) => {
                changed_image_handles.contains(&image_target.handle.id())
            }
            NormalizedRenderTarget::TextureView(_) => true,
        }
    }
}
