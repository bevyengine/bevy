//! Rules and strategies for determining the inspection-displayed label of an entity.

use bevy_animation::AnimationPlayer;
use bevy_app::{App, Plugin};
use bevy_audio::{AudioPlayer, AudioSink};
use bevy_camera::{Camera, Camera2d};
use bevy_core_pipeline::Skybox;
use bevy_ecs::{
    component::ComponentId, entity::Entity, name::Name, observer::Observer, resource::Resource,
    system::SystemIdMarker, world::World,
};
use bevy_input::gamepad::Gamepad;
use bevy_light::{
    AmbientLight, Atmosphere, DirectionalLight, FogVolume, IrradianceVolume, LightProbe,
    PointLight, SpotLight, SunDisk,
};
use bevy_mesh::{Mesh2d, Mesh3d};
use bevy_pbr::{wireframe::Wireframe, DistanceFog, Lightmap};
use bevy_picking::pointer::PointerId;
use bevy_platform::collections::HashMap;
use bevy_sprite::{Sprite, Text2d};
use bevy_text::TextSpan;
use bevy_ui::{
    widget::{Button, ImageNode, Text, ViewportNode},
    Node,
};
use bevy_window::{Monitor, Window};
use core::{
    any::TypeId,
    ops::{Deref, DerefMut},
};

/// The priority level for label-defining components.
///
/// Higher values indicate higher priority when determining an entity's label.
///
/// # Priority Conventions
///
/// - User-defined label-defining components should use [`USER`](Self::USER) priority (`0`).
/// - Library-defined components (in Bevy, or in third-party Bevy crates)
///   that are label-defining should use [`LIBRARY`](Self::LIBRARY) priority (`-10`).
/// - Fallback components (e.g. [`Camera`]) should use [`FALLBACK`](Self::FALLBACK) priority (`-20`).
///
/// Leaving space between these priority levels allows for future expansion
/// and customization in tricky edge cases.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serialize", derive(serde::Serialize, serde::Deserialize))]
pub struct LabelDefinitionPriority(pub i8);

impl LabelDefinitionPriority {
    /// The recommended priority for user-defined label-defining components.
    pub const USER: Self = Self(0);

    /// The recommended priority for library-defined label-defining components
    /// (e.g. Bevy built-ins or third-party crate types).
    pub const LIBRARY: Self = Self(-10);

    /// The recommended priority for fallback label-defining components
    /// that should only be used when no better label is available
    /// (e.g. [`Camera`], [`Node`]).
    pub const FALLBACK: Self = Self(-20);
}

/// Summary of a component on an entity, used for label resolution in [`resolve_label`].
#[derive(Clone, Copy, Debug)]
pub struct ComponentLabelData<'a> {
    /// The [`ComponentId`] of the component.
    pub component_id: ComponentId,
    /// The short (i.e. unqualified) name of the component type.
    pub short_name: &'a str,
    /// The label-defining priority, if this component is registered as label-defining.
    pub label_definition_priority: Option<LabelDefinitionPriority>,
}

/// The label of an inspected entity, used for inspection.
///
/// This is distinct from the entity's [`Name`],
/// which is an explicitly set component that is used in priority
/// over any [label defining](LabelDefinitionPriority) components
/// when determining the entity's label.
///
/// This data is produced by [`resolve_label`].
#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serialize", derive(serde::Serialize, serde::Deserialize))]
pub struct EntityLabel {
    /// The resolved label to display for the entity.
    ///
    /// Stored as a [`Name`] to take advantage of the optimizations and conveniences of that type,
    /// even though not all labels are [`Name`]-derived.
    pub label: Name,
    /// How the label was determined, which can be used to inform display decisions.
    pub origin: LabelOrigin,
}

impl Deref for EntityLabel {
    type Target = Name;

    fn deref(&self) -> &Self::Target {
        &self.label
    }
}

impl DerefMut for EntityLabel {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.label
    }
}

impl EntityLabel {
    /// Constructs a [`Custom`] entity label.
    ///
    /// [`Custom`]: LabelOrigin::Custom
    pub fn custom(label: &str) -> Self {
        Self {
            label: Name::new(label.to_owned()),
            origin: LabelOrigin::Custom,
        }
    }

    /// Constructs a [`Resolved`] entity label.
    ///
    /// [`Resolved`]: LabelOrigin::Resolved
    pub fn resolved(label: &str) -> Self {
        Self {
            label: Name::new(label.to_owned()),
            origin: LabelOrigin::Resolved,
        }
    }

    /// Constructs a [`Fallback`] entity label.
    ///
    /// [`Fallback`]: LabelOrigin::Fallback
    pub fn fallback(label: &str) -> Self {
        Self {
            label: Name::new(label.to_owned()),
            origin: LabelOrigin::Fallback,
        }
    }
}

/// Identifies how the inspected entity's label was determined.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serialize", derive(serde::Serialize, serde::Deserialize))]
pub enum LabelOrigin {
    /// The entity label comes from the [`Name`] component.
    Custom,
    /// The label was resolved from label-defining components via [`resolve_label`].
    Resolved,
    /// No [`Name`] component or label-defining component was found;
    /// the caller provided a fallback label.
    Fallback,
}

/// Determines the label to display for the given `entity`.
///
/// If the [`Name`] component is present, its value will be used as the label.
///
/// If any component marked as "label-defining" is present
/// (i.e., has a [`LabelDefinitionPriority`]), its label will be used.
/// If multiple label-defining components with the same highest priority are present,
/// they will be joined in alphabetical order,
/// separated by a "|" character.
///
/// Otherwise, [`None`] is returned.
/// The caller can then fall back to a default label such as "Entity".
///
/// # Arguments
///
/// * `world` - The world to query for the entity's [`Name`] component.
/// * `entity` - The entity to resolve the label for.
/// * `components` - A slice of [`ComponentLabelData`] describing each component on the entity.
///   Callers should assemble these from whatever component data they have.
pub fn resolve_label(
    world: &World,
    entity: Entity,
    components: &[ComponentLabelData],
) -> Option<EntityLabel> {
    if let Some(custom_label) = world.get::<Name>(entity).cloned() {
        return Some(EntityLabel::custom(custom_label.as_str()));
    }

    let mut label_resolution_priorities: Vec<(&str, LabelDefinitionPriority)> = components
        .iter()
        .filter_map(|c| c.label_definition_priority.map(|p| (c.short_name, p)))
        .collect();

    if label_resolution_priorities.is_empty() {
        return None;
    }

    // Sort by priority (higher priority first), then alphabetically
    label_resolution_priorities.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(b.0)));

    // Only include labels with the highest priority
    // PERF: we could do this more efficiently by combining the sort and filter steps
    let highest_priority = label_resolution_priorities[0].1;
    label_resolution_priorities.retain(|&(_, priority)| priority == highest_priority);

    let resolved_label = label_resolution_priorities
        .into_iter()
        .map(|(label, _)| label)
        .collect::<Vec<&str>>()
        .join(" | ");

    Some(EntityLabel::resolved(&resolved_label))
}

/// Stores the registered label-defining component types and their priorities.
///
/// When determining an entity's label via [`resolve_label`], components with higher priority values
/// will take precedence over those with lower priority values.
///
/// Entities with an explicit [`Name`] component will always take precedence over label-defining components.
///
/// # Priority Conventions
///
/// See the associated constants on [`LabelDefinitionPriority`] for recommended priority levels.
///
/// # Usage
///
/// Components that should be "label-defining" should be registered in this registry
/// using [`LabelResolutionRegistry::register_label_defining_type`],
/// typically in the plugin that defines the component.
#[derive(Debug, Resource, Default)]
pub struct LabelResolutionRegistry {
    /// A mapping of label-defining component [`TypeId`]s to their priority levels.
    label_defining_types: HashMap<TypeId, LabelDefinitionPriority>,
}

impl LabelResolutionRegistry {
    /// Creates a new, empty [`LabelResolutionRegistry`].
    pub const fn new() -> Self {
        Self {
            label_defining_types: HashMap::new(),
        }
    }

    /// Registers a label-defining component type with the given priority.
    ///
    /// Higher priority components will take precedence when determining an entity's label.
    pub fn register_label_defining_type<T: 'static>(&mut self, priority: LabelDefinitionPriority) {
        let type_id = TypeId::of::<T>();
        self.label_defining_types.insert(type_id, priority);
    }

    /// Gets the priority of a label-defining component type, if registered.
    pub fn get_priority<T: 'static>(&self) -> Option<LabelDefinitionPriority> {
        let type_id = TypeId::of::<T>();
        self.get_priority_by_type_id(type_id)
    }

    /// Gets the priority of a label-defining component type by its [`TypeId`], if registered.
    pub fn get_priority_by_type_id(&self, type_id: TypeId) -> Option<LabelDefinitionPriority> {
        self.label_defining_types.get(&type_id).copied()
    }

    /// Removes a label-defining component type from the registry.
    pub fn unregister_label_defining_type<T: 'static>(&mut self) {
        let type_id = TypeId::of::<T>();
        self.unregister_label_defining_type_by_type_id(type_id);
    }

    /// Removes a label-defining component type from the registry by its [`TypeId`].
    pub fn unregister_label_defining_type_by_type_id(&mut self, type_id: TypeId) {
        self.label_defining_types.remove(&type_id);
    }
}

/// A plugin which registers label-defining components for Bevy's first-party types
/// in the [`LabelResolutionRegistry`] resource.
///
/// When upstreamed, this plugin should not be necessary,
/// as each label-defining component can register itself in its own plugin.
pub struct LabelResolutionPlugin;
impl Plugin for LabelResolutionPlugin {
    fn build(&self, app: &mut App) {
        let mut label_resolution_registry = LabelResolutionRegistry::new();

        // Windowing and input
        label_resolution_registry
            .register_label_defining_type::<Window>(LabelDefinitionPriority::LIBRARY);
        label_resolution_registry
            .register_label_defining_type::<Monitor>(LabelDefinitionPriority::LIBRARY);
        label_resolution_registry
            .register_label_defining_type::<Gamepad>(LabelDefinitionPriority::LIBRARY);
        label_resolution_registry
            .register_label_defining_type::<PointerId>(LabelDefinitionPriority::LIBRARY);

        // UI
        label_resolution_registry
            .register_label_defining_type::<Node>(LabelDefinitionPriority::FALLBACK);
        label_resolution_registry
            .register_label_defining_type::<Button>(LabelDefinitionPriority::LIBRARY);
        label_resolution_registry
            .register_label_defining_type::<Text>(LabelDefinitionPriority::LIBRARY);
        label_resolution_registry
            .register_label_defining_type::<TextSpan>(LabelDefinitionPriority::LIBRARY);
        label_resolution_registry
            .register_label_defining_type::<Text2d>(LabelDefinitionPriority::LIBRARY);
        label_resolution_registry
            .register_label_defining_type::<ImageNode>(LabelDefinitionPriority::LIBRARY);
        label_resolution_registry
            .register_label_defining_type::<ViewportNode>(LabelDefinitionPriority::LIBRARY);

        // Cameras
        label_resolution_registry
            .register_label_defining_type::<Camera>(LabelDefinitionPriority::FALLBACK);
        label_resolution_registry
            .register_label_defining_type::<Camera2d>(LabelDefinitionPriority::LIBRARY);
        label_resolution_registry
            .register_label_defining_type::<Camera2d>(LabelDefinitionPriority::LIBRARY);

        // Lights
        label_resolution_registry
            .register_label_defining_type::<DirectionalLight>(LabelDefinitionPriority::LIBRARY);
        label_resolution_registry
            .register_label_defining_type::<PointLight>(LabelDefinitionPriority::LIBRARY);
        label_resolution_registry
            .register_label_defining_type::<SpotLight>(LabelDefinitionPriority::LIBRARY);
        label_resolution_registry
            .register_label_defining_type::<AmbientLight>(LabelDefinitionPriority::LIBRARY);
        label_resolution_registry
            .register_label_defining_type::<LightProbe>(LabelDefinitionPriority::LIBRARY);
        label_resolution_registry
            .register_label_defining_type::<IrradianceVolume>(LabelDefinitionPriority::LIBRARY);
        label_resolution_registry
            .register_label_defining_type::<SunDisk>(LabelDefinitionPriority::LIBRARY);
        label_resolution_registry
            .register_label_defining_type::<Lightmap>(LabelDefinitionPriority::LIBRARY);

        // Core rendering components
        label_resolution_registry
            .register_label_defining_type::<Sprite>(LabelDefinitionPriority::LIBRARY);
        label_resolution_registry
            .register_label_defining_type::<Mesh2d>(LabelDefinitionPriority::LIBRARY);
        label_resolution_registry
            .register_label_defining_type::<Mesh3d>(LabelDefinitionPriority::LIBRARY);
        label_resolution_registry
            .register_label_defining_type::<Wireframe>(LabelDefinitionPriority::LIBRARY);

        // Atmospherics
        label_resolution_registry
            .register_label_defining_type::<Skybox>(LabelDefinitionPriority::LIBRARY);
        label_resolution_registry
            .register_label_defining_type::<FogVolume>(LabelDefinitionPriority::LIBRARY);
        label_resolution_registry
            .register_label_defining_type::<Atmosphere>(LabelDefinitionPriority::LIBRARY);
        label_resolution_registry
            .register_label_defining_type::<DistanceFog>(LabelDefinitionPriority::LIBRARY);

        // Animation
        label_resolution_registry
            .register_label_defining_type::<AnimationPlayer>(LabelDefinitionPriority::LIBRARY);

        // Audio
        label_resolution_registry
            .register_label_defining_type::<AudioPlayer>(LabelDefinitionPriority::LIBRARY);
        label_resolution_registry
            .register_label_defining_type::<AudioSink>(LabelDefinitionPriority::LIBRARY);

        // System-likes
        label_resolution_registry
            .register_label_defining_type::<Observer>(LabelDefinitionPriority::LIBRARY);
        label_resolution_registry
            .register_label_defining_type::<SystemIdMarker>(LabelDefinitionPriority::LIBRARY);

        app.insert_resource(label_resolution_registry);
    }
}
