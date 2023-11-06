//! A module for the [`GizmoConfig<T>`] [`Resource`].

use core::panic;
use std::ops::{Deref, DerefMut};

use bevy_ecs::{component::Component, system::Resource};
use bevy_reflect::{Reflect, TypePath};
use bevy_render::{color::Color, view::RenderLayers};
use bevy_utils::HashMap;

/// A trait used for custom gizmo configs.
///
/// Here you can store additional configuration for you gizmos not covered by [`GizmoConfig`]
///
/// Make sure to derive [`Default`] + [`Reflect`], and register in the app using `app.init_gizmo_config::<T>()`
pub trait CustomGizmoConfig: Reflect + TypePath + Default {}

/// The default gizmo config.
#[derive(Default, Reflect)]
pub struct DefaultGizmoConfig;
impl CustomGizmoConfig for DefaultGizmoConfig {}

/// A [`Resource`] storing [`GizmoConfig`] and [`CustomGizmoConfig`] structs
///
/// Use `app.init_gizmo_config::<T>()` to register a custom config.
#[derive(Resource, Default)]
pub struct GizmoConfigStore {
    // INVARIANT: store must map TypeId::of::<T>() to correct type T
    store: HashMap<&'static str, (GizmoConfig, Box<dyn Reflect>)>,
}

impl GizmoConfigStore {
    /// Returns [`GizmoConfig`] and `&dyn` [`CustomGizmoConfig`] associated with [`TypePath`] of a [`CustomGizmoConfig`]
    pub fn get_dyn(&self, config_type_path: &str) -> (&GizmoConfig, &dyn Reflect) {
        let Some((config, ext)) = self.store.get(config_type_path) else {
            panic!("Requested config {} does not exist in `GizmoConfigStore`! Did you forget to add it using `app.init_gizmo_config<T>()`?", config_type_path);
        };
        (config, ext.deref())
    }

    /// Returns [`GizmoConfig`] and [`CustomGizmoConfig`] associated with a [`CustomGizmoConfig`] `T`
    pub fn get<T: CustomGizmoConfig>(&self) -> (&GizmoConfig, &T) {
        let (config, ext) = self.get_dyn(T::type_path());
        // hash map invariant guarantees that `&dyn CustomGizmoConfig` is of correct type T
        let ext = ext.as_any().downcast_ref().unwrap();
        (config, ext)
    }

    /// Returns mutable [`GizmoConfig`] and `&dyn` [`CustomGizmoConfig`] associated with [`TypePath`] of a [`CustomGizmoConfig`]
    pub fn get_mut_dyn(&mut self, config_type_path: &str) -> (&mut GizmoConfig, &mut dyn Reflect) {
        let Some((config, ext)) = self.store.get_mut(config_type_path) else {
            panic!("Requested config {} does not exist in `GizmoConfigStore`! Did you forget to add it using `app.init_gizmo_config<T>()`?", config_type_path);
        };
        (config, ext.deref_mut())
    }

    /// Returns mutable [`GizmoConfig`] and [`CustomGizmoConfig`] associated with a [`CustomGizmoConfig`] `T`
    pub fn get_mut<T: CustomGizmoConfig>(&mut self) -> (&mut GizmoConfig, &mut T) {
        let (config, ext) = self.get_mut_dyn(T::type_path());
        // hash map invariant guarantees that `&dyn CustomGizmoConfig` is of correct type T
        let ext = ext.as_any_mut().downcast_mut().unwrap();
        (config, ext)
    }

    /// Returns an iterator over all [`GizmoConfig`]s.
    pub fn iter(&self) -> impl Iterator<Item = (&str, &GizmoConfig, &dyn Reflect)> + '_ {
        self.store
            .iter()
            .map(|(&id, (config, ext))| (id, config, ext.deref()))
    }

    /// Returns an iterator over all [`GizmoConfig`]s, by mutable reference.
    pub fn iter_mut(
        &mut self,
    ) -> impl Iterator<Item = (&str, &mut GizmoConfig, &mut dyn Reflect)> + '_ {
        self.store
            .iter_mut()
            .map(|(&id, (config, ext))| (id, config, ext.deref_mut()))
    }

    /// Inserts [`GizmoConfig`] and [`CustomGizmoConfig`] replacing old values
    pub fn insert<T: CustomGizmoConfig>(&mut self, config: GizmoConfig, ext_config: T) {
        // INVARIANT: hash map must only map TypeId::of::<T>() to Box<T>
        self.store
            .insert(T::type_path(), (config, Box::new(ext_config)));
    }

    pub(crate) fn regsiter<T: CustomGizmoConfig>(&mut self) {
        self.insert::<T>(GizmoConfig::default(), T::default());
    }
}

/// A struct that stores configuration for gizmos.
#[derive(Component, Clone)]
pub struct GizmoConfig {
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
}

impl Default for GizmoConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            line_width: 2.,
            line_perspective: false,
            depth_bias: 0.,
            render_layers: Default::default(),
        }
    }
}

/// Configuration for drawing the [Aabb](bevy_render::primitives::Aabb) component on entities.
#[derive(Default, Reflect)]
pub struct AabbGizmoConfig {
    /// Draws all bounding boxes in the scene when set to `true`.
    ///
    /// To draw a specific entity's bounding box, you can add the [ShowAabbGizmo](crate::ShowAabbGizmo) component.
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

impl CustomGizmoConfig for AabbGizmoConfig {}
