//! A module for the [`GizmoConfig<T>`] [`Resource`].

use core::panic;
use std::{
    any::TypeId,
    ops::{Deref, DerefMut},
};

use bevy_ecs::{component::Component, system::Resource};
use bevy_render::{color::Color, view::RenderLayers};
use bevy_utils::HashMap;

/// A trait used for custom gizmo configs.
///
/// Here you can store additional configuration for you gizmos not covered by [`GizmoConfig`]
///
/// Make sure to derive [`Default`] and register in the app using `app.init_gizmo_config::<T>()`
pub trait CustomGizmoConfig: 'static + Send + Sync {}

/// The default gizmo config.
#[derive(Default)]
pub struct DefaultGizmoConfig;
impl CustomGizmoConfig for DefaultGizmoConfig {}

/// A [`Resource`] storing [`GizmoConfig`] and [`CustomGizmoConfig`] structs
///
/// Use `app.init_gizmo_config::<T>()` to register a custom config.
#[derive(Resource, Default)]
pub struct GizmoConfigStore {
    store: HashMap<TypeId, GizmoConfig>,
    // SAFETY: store_ext must map TypeId::of::<T>() to correct type T
    store_ext: HashMap<TypeId, Box<dyn CustomGizmoConfig>>,
}

impl GizmoConfigStore {
    /// Returns [`GizmoConfig`] and [`CustomGizmoConfig`] associated with [`CustomGizmoConfig`] `T`
    pub fn get<T: CustomGizmoConfig>(&self) -> (&GizmoConfig, &T) {
        let Some(config) = self.store.get(&TypeId::of::<T>()) else {
            panic!("Requested config {} does not exist in `GizmoConfigStore`! Did you forget to add it using `app.init_gizmo_config<T>()`?", std::any::type_name::<T>());
        };
        let ext = self.store_ext.get(&TypeId::of::<T>()).unwrap().deref();
        // SAFETY: hash map invariant guarantees that `&dyn CustomGizmoConfig` is of correct type T
        let ext = unsafe { &*(ext as *const dyn CustomGizmoConfig as *const T) };
        (config, ext)
    }

    /// Returns mutable [`GizmoConfig`] and [`CustomGizmoConfig`] associated with [`CustomGizmoConfig`] `T`
    pub fn get_mut<T: CustomGizmoConfig>(&mut self) -> (&mut GizmoConfig, &mut T) {
        let Some(config) = self.store.get_mut(&TypeId::of::<T>()) else {
            panic!("Requested config {} does not exist in `GizmoConfigStore`! Did you forget to add it using `app.init_gizmo_config<T>()`?", std::any::type_name::<T>());
        };
        let ext = self
            .store_ext
            .get_mut(&TypeId::of::<T>())
            .unwrap()
            .deref_mut();
        // SAFETY: hash map invariant guarantees that `&dyn CustomGizmoConfig` is of correct type T
        let ext = unsafe { &mut *(ext as *mut dyn CustomGizmoConfig as *mut T) };
        (config, ext)
    }

    /// Returns an iterator over all [`GizmoConfigs`]s.
    pub fn iter(&self) -> impl Iterator<Item = &GizmoConfig> + '_ {
        self.store.values()
    }

    /// Returns an iterator over all [`GizmoConfigs`]s, by mutable reference.
    pub fn iter_mut(&mut self) -> impl Iterator<Item = &mut GizmoConfig> + '_ {
        self.store.values_mut()
    }

    pub(crate) fn insert<T: CustomGizmoConfig>(&mut self, config: GizmoConfig, ext_config: T) {
        self.store.insert(TypeId::of::<T>(), config);
        // SAFETY: hash map must only map TypeId::of::<T>() to Box<T>
        self.store_ext
            .insert(TypeId::of::<T>(), Box::new(ext_config));
    }

    pub(crate) fn regsiter<T: CustomGizmoConfig + Default>(&mut self) {
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

/// Configuration for drawing the [`Aabb`] component on entities.
#[derive(Default)]
pub struct AabbGizmoConfig {
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

impl CustomGizmoConfig for AabbGizmoConfig {}
