//! A module for the [`GizmoConfig<T>`] [`Resource`].
//!
//! This module provides the configuration system for Bevy gizmos, including
//! line width, style, and rendering settings. The key innovation here is the
//! use of logical pixels via the `Val` enum, which ensures consistent line
//! thickness across different display scales and DPI settings.
//!
//! ## Architecture Overview
//!
//! The gizmo configuration system consists of several key components:
//!
//! 1. **GizmoConfig**: Main configuration struct containing line settings
//! 2. **GizmoLineConfig**: Specific configuration for line rendering
//! 3. **GizmoConfigGroup**: Trait for grouping related configurations
//! 4. **GizmoConfigStore**: Resource that manages all configuration instances
//!
//! ## Logical Pixel Implementation
//!
//! The line width is now specified using `Val` enum values instead of raw f32:
//! - `Val::Px(f32)`: Logical pixels that scale with DPI
//! - `Val::Vw(f32)`: Percentage of viewport width
//! - `Val::Vh(f32)`: Percentage of viewport height
//!
//! This ensures that gizmo lines maintain consistent visual thickness
//! regardless of the display's scale factor or resolution.

pub use bevy_gizmos_macros::GizmoConfigGroup;

#[cfg(all(
    feature = "bevy_render",
    any(feature = "bevy_pbr", feature = "bevy_sprite")
))]
use {crate::GizmoAsset, bevy_asset::Handle, bevy_ecs::component::Component};

use bevy_ecs::{reflect::ReflectResource, resource::Resource};
use bevy_reflect::{std_traits::ReflectDefault, Reflect, TypePath};
use bevy_ui::Val;
use bevy_utils::TypeIdMap;
use core::{
    any::TypeId,
    hash::Hash,
    ops::{Deref, DerefMut},
    panic,
};

/// An enum configuring how line joints will be drawn.
/// 
/// Line joints determine how two connected line segments are rendered at their
/// intersection point. This affects the visual appearance of complex line shapes
/// like polygons or multi-segment paths.
/// 
/// # Variants
/// 
/// - `None`: No special joint rendering (lines meet at sharp angles)
/// - `Miter`: Sharp pointed joints (lines extend to meet at a point)
/// - `Round(u32)`: Rounded corners with specified triangle resolution
/// - `Bevel`: Straight beveled joints (flat connection between lines)
#[derive(Debug, Default, Copy, Clone, Reflect, PartialEq, Eq, Hash)]
#[reflect(Default, PartialEq, Hash, Clone)]
pub enum GizmoLineJoint {
    /// Does not draw any line joints.
    #[default]
    None,
    /// Extends both lines at the joining point until they meet in a sharp point.
    Miter,
    /// Draws a round corner with the specified resolution between the two lines.
    ///
    /// The resolution determines the amount of triangles drawn per joint,
    /// e.g. `GizmoLineJoint::Round(4)` will draw 4 triangles at each line joint.
    Round(u32),
    /// Draws a bevel, a straight line in this case, to connect the ends of both lines.
    Bevel,
}

/// An enum used to configure the style of gizmo lines, similar to CSS line-style.
/// 
/// This enum controls the visual appearance of gizmo lines, allowing for different
/// line patterns and styles. The styles are similar to those found in CSS and
/// other graphics systems.
/// 
/// # Variants
/// 
/// - `Solid`: Continuous line without breaks or patterns
/// - `Dotted`: Line made up of evenly spaced dots
/// - `Dashed`: Line with alternating visible and invisible segments
/// 
/// # Examples
/// 
/// ```rust
/// use bevy_gizmos::config::GizmoLineStyle;
/// 
/// // Solid line (default)
/// let solid = GizmoLineStyle::Solid;
/// 
/// // Dotted line
/// let dotted = GizmoLineStyle::Dotted;
/// 
/// // Dashed line with custom gap and line lengths
/// let dashed = GizmoLineStyle::Dashed {
///     gap_scale: 2.0,    // Gap is 2x the line width
///     line_scale: 1.5,   // Visible segment is 1.5x the line width
/// };
/// ```
#[derive(Copy, Clone, Debug, Default, PartialEq, Reflect)]
#[reflect(Default, PartialEq, Hash, Clone)]
#[non_exhaustive]
pub enum GizmoLineStyle {
    /// A solid line without any decorators
    #[default]
    Solid,
    /// A dotted line
    Dotted,
    /// A dashed line with configurable gap and line sizes
    Dashed {
        /// The length of the gap in `line_width`s
        gap_scale: f32,
        /// The length of the visible line in `line_width`s
        line_scale: f32,
    },
}

impl Eq for GizmoLineStyle {}

impl Hash for GizmoLineStyle {
    fn hash<H: core::hash::Hasher>(&self, state: &mut H) {
        match self {
            Self::Solid => {
                0u64.hash(state);
            }
            Self::Dotted => 1u64.hash(state),
            Self::Dashed {
                gap_scale,
                line_scale,
            } => {
                2u64.hash(state);
                gap_scale.to_bits().hash(state);
                line_scale.to_bits().hash(state);
            }
        }
    }
}

/// A trait used to create gizmo config groups.
///
/// This trait allows you to create custom configuration groups for gizmos that
/// can store additional settings beyond the standard `GizmoConfig`. This is useful
/// for creating specialized gizmo systems with their own configuration needs.
///
/// ## Implementation Requirements
///
/// - Must derive `Default` + `Reflect` 
/// - Must be registered in the app using `app.init_gizmo_group::<T>()`
///
/// ## Example
///
/// ```rust
/// use bevy_gizmos::config::GizmoConfigGroup;
/// use bevy_reflect::{Reflect, TypePath};
/// 
/// #[derive(Default, Reflect, TypePath)]
/// struct MyCustomGizmoConfig {
///     pub custom_setting: bool,
///     pub custom_value: f32,
/// }
/// 
/// impl GizmoConfigGroup for MyCustomGizmoConfig {}
/// ```
pub trait GizmoConfigGroup: Reflect + TypePath + Default {}

/// The default gizmo config group.
#[derive(Default, Reflect, GizmoConfigGroup)]
#[reflect(Default)]
pub struct DefaultGizmoConfigGroup;

/// Used when the gizmo config group needs to be type-erased.
/// Also used for retained gizmos, which can't have a gizmo config group.
#[derive(Default, Reflect, GizmoConfigGroup, Debug, Clone)]
#[reflect(Default, Clone)]
pub struct ErasedGizmoConfigGroup;

/// A [`Resource`] storing [`GizmoConfig`] and [`GizmoConfigGroup`] structs.
///
/// This resource acts as a central registry for all gizmo configuration groups.
/// It maintains a type-safe mapping between `TypeId`s and their corresponding
/// configuration instances, allowing for multiple independent gizmo systems
/// with different settings.
///
/// ## Architecture
///
/// The store uses a `TypeIdMap` to maintain the invariant that each `TypeId`
/// maps to the correct configuration type. This enables type-safe access to
/// configurations while supporting dynamic configuration groups.
///
/// ## Usage
///
/// ```rust
/// use bevy_gizmos::config::{GizmoConfigStore, DefaultGizmoConfigGroup};
/// 
/// // Get configuration for the default group
/// let (config, _) = config_store.config::<DefaultGizmoConfigGroup>();
/// 
/// // Modify configuration
/// let (config, _) = config_store.config_mut::<DefaultGizmoConfigGroup>();
/// config.line.width = Val::Px(4.0);
/// ```
///
/// Use `app.init_gizmo_group::<T>()` to register a custom config group.
#[derive(Reflect, Resource, Default)]
#[reflect(Resource, Default)]
pub struct GizmoConfigStore {
    // INVARIANT: must map TypeId::of::<T>() to correct type T
    #[reflect(ignore)]
    store: TypeIdMap<(GizmoConfig, Box<dyn Reflect>)>,
}

impl GizmoConfigStore {
    /// Returns [`GizmoConfig`] and [`GizmoConfigGroup`] associated with [`TypeId`] of a [`GizmoConfigGroup`]
    pub fn get_config_dyn(&self, config_type_id: &TypeId) -> Option<(&GizmoConfig, &dyn Reflect)> {
        let (config, ext) = self.store.get(config_type_id)?;
        Some((config, ext.deref()))
    }

    /// Returns [`GizmoConfig`] and [`GizmoConfigGroup`] associated with [`GizmoConfigGroup`] `T`
    pub fn config<T: GizmoConfigGroup>(&self) -> (&GizmoConfig, &T) {
        let Some((config, ext)) = self.get_config_dyn(&TypeId::of::<T>()) else {
            panic!("Requested config {} does not exist in `GizmoConfigStore`! Did you forget to add it using `app.init_gizmo_group<T>()`?", T::type_path());
        };
        // hash map invariant guarantees that &dyn Reflect is of correct type T
        let ext = ext.as_any().downcast_ref().unwrap();
        (config, ext)
    }

    /// Returns mutable [`GizmoConfig`] and [`GizmoConfigGroup`] associated with [`TypeId`] of a [`GizmoConfigGroup`]
    pub fn get_config_mut_dyn(
        &mut self,
        config_type_id: &TypeId,
    ) -> Option<(&mut GizmoConfig, &mut dyn Reflect)> {
        let (config, ext) = self.store.get_mut(config_type_id)?;
        Some((config, ext.deref_mut()))
    }

    /// Returns mutable [`GizmoConfig`] and [`GizmoConfigGroup`] associated with [`GizmoConfigGroup`] `T`
    pub fn config_mut<T: GizmoConfigGroup>(&mut self) -> (&mut GizmoConfig, &mut T) {
        let Some((config, ext)) = self.get_config_mut_dyn(&TypeId::of::<T>()) else {
            panic!("Requested config {} does not exist in `GizmoConfigStore`! Did you forget to add it using `app.init_gizmo_group<T>()`?", T::type_path());
        };
        // hash map invariant guarantees that &dyn Reflect is of correct type T
        let ext = ext.as_any_mut().downcast_mut().unwrap();
        (config, ext)
    }

    /// Returns an iterator over all [`GizmoConfig`]s.
    pub fn iter(&self) -> impl Iterator<Item = (&TypeId, &GizmoConfig, &dyn Reflect)> + '_ {
        self.store
            .iter()
            .map(|(id, (config, ext))| (id, config, ext.deref()))
    }

    /// Returns an iterator over all [`GizmoConfig`]s, by mutable reference.
    pub fn iter_mut(
        &mut self,
    ) -> impl Iterator<Item = (&TypeId, &mut GizmoConfig, &mut dyn Reflect)> + '_ {
        self.store
            .iter_mut()
            .map(|(id, (config, ext))| (id, config, ext.deref_mut()))
    }

    /// Inserts [`GizmoConfig`] and [`GizmoConfigGroup`] replacing old values
    pub fn insert<T: GizmoConfigGroup>(&mut self, config: GizmoConfig, ext_config: T) {
        // INVARIANT: hash map must correctly map TypeId::of::<T>() to &dyn Reflect of type T
        self.store
            .insert(TypeId::of::<T>(), (config, Box::new(ext_config)));
    }

    pub(crate) fn register<T: GizmoConfigGroup>(&mut self) {
        self.insert(GizmoConfig::default(), T::default());
    }
}

/// A struct that stores configuration for gizmos.
/// 
/// This is the main configuration struct for gizmo rendering. It contains all
/// the settings that control how gizmos are drawn, including line properties,
/// depth behavior, and rendering layers.
/// 
/// ## Key Features
/// 
/// - **Line Configuration**: Controls line width, style, and joint appearance
/// - **Depth Bias**: Adjusts rendering order to prevent z-fighting
/// - **Render Layers**: Controls which cameras can see the gizmos
/// - **Enable/Disable**: Global toggle for gizmo rendering
/// 
/// ## Usage
/// 
/// ```rust
/// use bevy_gizmos::config::GizmoConfig;
/// 
/// let config = GizmoConfig {
///     enabled: true,
///     line: GizmoLineConfig::default(),
///     depth_bias: -0.1, // Render slightly in front
///     ..Default::default()
/// };
/// ```
#[derive(Clone, Reflect, Debug)]
#[reflect(Clone, Default)]
pub struct GizmoConfig {
    /// Set to `false` to stop drawing gizmos.
    ///
    /// Defaults to `true`.
    pub enabled: bool,
    /// Line settings.
    pub line: GizmoLineConfig,
    /// How closer to the camera than real geometry the gizmos should be.
    ///
    /// In 2D this setting has no effect and is effectively always -1.
    ///
    /// Value between -1 and 1 (inclusive).
    /// * 0 means that there is no change to the line position when rendering
    /// * 1 means it is furthest away from camera as possible
    /// * -1 means that it will always render in front of other things.
    ///
    /// This is typically useful if you are drawing wireframes on top of polygons
    /// and your wireframe is z-fighting (flickering on/off) with your main model.
    /// You would set this value to a negative number close to 0.
    pub depth_bias: f32,
    /// Describes which rendering layers gizmos will be rendered to.
    ///
    /// Gizmos will only be rendered to cameras with intersecting layers.
    #[cfg(feature = "bevy_render")]
    pub render_layers: bevy_render::view::RenderLayers,
}

impl Default for GizmoConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            line: Default::default(),
            depth_bias: 0.,
            #[cfg(feature = "bevy_render")]
            render_layers: Default::default(),
        }
    }
}

/// A struct that stores configuration for gizmo line rendering.
/// 
/// This struct contains all the settings that control how individual gizmo lines
/// are rendered, including width, style, joints, and perspective behavior.
/// 
/// ## Logical Pixel Implementation
/// 
/// The `width` field now uses `Val` enum instead of raw `f32` values. This enables
/// logical pixel rendering that automatically scales with the display's DPI factor,
/// ensuring consistent line thickness across different screens and resolutions.
/// 
/// ### Val Types Supported
/// 
/// - `Val::Px(f32)`: Logical pixels that scale with DPI
/// - `Val::Vw(f32)`: Percentage of viewport width
/// - `Val::Vh(f32)`: Percentage of viewport height
/// - `Val::Auto`: Automatic sizing (falls back to 2.0 pixels)
/// 
/// ## Example
/// 
/// ```rust
/// use bevy_gizmos::config::GizmoLineConfig;
/// use bevy_ui::Val;
/// 
/// let config = GizmoLineConfig {
///     width: Val::Px(4.0),        // 4 logical pixels
///     perspective: false,          // No perspective scaling
///     style: GizmoLineStyle::Solid,
///     joints: GizmoLineJoint::None,
/// };
/// ```
#[derive(Clone, Reflect, Debug)]
#[reflect(Clone, Default)]
pub struct GizmoLineConfig {
    /// Line width specified using logical pixels via the `Val` enum.
    ///
    /// This field supports various units for specifying line width:
    /// - `Val::Px(f32)`: Logical pixels that scale with DPI
    /// - `Val::Vw(f32)`: Percentage of viewport width  
    /// - `Val::Vh(f32)`: Percentage of viewport height
    /// - `Val::Auto`: Automatic sizing (defaults to 2.0 pixels)
    ///
    /// If `perspective` is `true` then this is the size in pixels at the camera's near plane.
    ///
    /// Defaults to `Val::Px(2.0)`.
    pub width: Val, 
    /// Apply perspective to gizmo lines.
    ///
    /// This setting only affects 3D, non-orthographic cameras.
    ///
    /// Defaults to `false`.
    pub perspective: bool,
    /// Determine the style of gizmo lines.
    pub style: GizmoLineStyle,
    /// Describe how lines should join.
    pub joints: GizmoLineJoint,
}

impl Default for GizmoLineConfig {
    fn default() -> Self {
        Self {
            // width: 2.,
            // replace with Val alternative
            width: Val::Px(2.0),
            perspective: false,
            style: GizmoLineStyle::Solid,
            joints: GizmoLineJoint::None,
        }
    }
}

#[cfg(all(
    feature = "bevy_render",
    any(feature = "bevy_pbr", feature = "bevy_sprite")
))]
#[derive(Component)]
pub(crate) struct GizmoMeshConfig {
    pub line_perspective: bool,
    pub line_style: GizmoLineStyle,
    pub line_joints: GizmoLineJoint,
    pub render_layers: bevy_render::view::RenderLayers,
    pub handle: Handle<GizmoAsset>,
}
