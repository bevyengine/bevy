//! A module for the [`GizmoConfig<T>`] [`Resource`].

use bevy_ecs::{component::Component, system::Resource};
use bevy_reflect::TypePath;
use bevy_render::{color::Color, view::RenderLayers};

/// A trait used for custom gizmo configs.
/// 
/// Make sure to derive [`Default`], [`Clone`], [`TypePath`] and register in the app using `app.add_gizmos::<T>()`
pub trait GizmoConfigExtension: 'static + Default + Clone + TypePath + Send + Sync {}

/// The default gizmo config.
#[derive(Default, Clone, TypePath)]
pub struct Global;
impl GizmoConfigExtension for Global {}

/// A struct that stores configuration for gizmos.
#[derive(Resource, Clone)]
pub struct GizmoConfig<T: GizmoConfigExtension = Global> {
    /// Set to `false` to stop drawing gizmos.
    ///
    /// Defaults to `true`.
    pub enabled: bool,
    /// Line width specified in pixels.
    ///
    /// If `line_perspective` is `true` then this is the size in pixels at the camera's near plane.
    ///
    /// Defaults to `2.0`.
    pub line_width: f32,
    /// Apply perspective to gizmo lines.
    ///
    /// This setting only affects 3D, non-orthographic cameras.
    ///
    /// Defaults to `false`.
    pub line_perspective: bool,
    /// How closer to the camera than real geometry the line should be.
    ///
    /// Value between -1 and 1 (inclusive).
    /// * 0 means that there is no change to the line position when rendering
    /// * 1 means it is furthest away from camera as possible
    /// * -1 means that it will always render in front of other things.
    ///
    /// This is typically useful if you are drawing wireframes on top of polygons
    /// and your wireframe is z-fighting (flickering on/off) with your main model.
    /// You would set this value to a negative number close to 0.0.
    pub depth_bias: f32,
    /// Describes which rendering layers gizmos will be rendered to.
    ///
    /// Gizmos will only be rendered to cameras with intersecting layers.
    pub render_layers: RenderLayers,
    /// Extended configuration provided by the respective plugin.
    pub extended: T,
}

impl<T: GizmoConfigExtension> Default for GizmoConfig<T> {
    fn default() -> Self {
        Self {
            enabled: true,
            line_width: 2.,
            line_perspective: false,
            depth_bias: 0.,
            render_layers: Default::default(),
            extended: Default::default(),
        }
    }
}

// We need to get rid of the generic extended settings in the extract system to unify the rendering systems.
// It is a component because multiple resources of the same type are not allowed.
#[derive(Component, Debug, Clone)]
#[allow(unused)]
pub(crate) struct ExtractedGizmoConfig {
    pub enabled: bool,
    pub line_width: f32,
    pub line_perspective: bool,
    pub depth_bias: f32,
    pub render_layers: RenderLayers,
}

impl<T: GizmoConfigExtension> From<&GizmoConfig<T>> for ExtractedGizmoConfig {
    fn from(other: &GizmoConfig<T>) -> Self {
        Self {
            enabled: other.enabled,
            line_width: other.line_width,
            line_perspective: other.line_perspective,
            depth_bias: other.depth_bias,
            render_layers: other.render_layers,
        }
    }
}

/// Configuration for drawing the [`Aabb`] component on entities.
#[derive(Clone, Default, TypePath)]
pub struct AabbGizmos {
    /// Draws all bounding boxes in the scene when set to `true`.
    ///
    /// To draw a specific entity's bounding box, you can add the [`AabbGizmo`] component.
    ///
    /// Defaults to `false`.
    pub draw_all: bool,
    /// The default color for bounding box gizmos.
    ///
    /// A random color is chosen per box if `None`.
    ///
    /// Defaults to `None`.
    pub default_color: Option<Color>,
}

impl GizmoConfigExtension for AabbGizmos {}
