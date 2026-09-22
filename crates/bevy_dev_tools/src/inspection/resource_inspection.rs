//! Types describing resources, gathered by inspecting a [`World`](bevy_ecs::world::World).

use bevy_ecs::component::ComponentId;
use bevy_reflect::TypeRegistration;
use bevy_utils::{memory_size::MemorySize, prelude::DebugName};
use core::{
    any::TypeId,
    fmt::{Display, Formatter},
};

/// The result of inspecting a resource, summarized by its [`Display`] implementation.
#[derive(Clone, Debug)]
pub struct ResourceInspection {
    /// The [`ComponentId`] of the resource.
    pub component_id: ComponentId,
    /// The type name of the resource.
    pub name: DebugName,
    /// The value of the resource as a string, gathered via reflection.
    pub value: String,
    /// The [`TypeId`] of the resource, or `None` for dynamic types.
    pub type_id: Option<TypeId>,
    /// The shallow size of the resource in memory, excluding heap allocations.
    pub memory_size: MemorySize,
    /// The registered type information of the resource, if it is reflected and registered.
    pub type_registration: Option<TypeRegistration>,
}

impl Display for ResourceInspection {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        write!(
            f,
            "{} ({}): {}",
            self.name.shortname(),
            self.memory_size,
            self.value
        )
    }
}

/// An error that can occur when attempting to inspect a resource.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ResourceInspectionError {
    /// The resource type was not registered in the world.
    ResourceNotRegistered(&'static str),
    /// The resource ID provided was not registered in the world.
    ResourceIdNotRegistered(ComponentId),
}

impl Display for ResourceInspectionError {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        match self {
            ResourceInspectionError::ResourceNotRegistered(name) => {
                write!(f, "Resource type {name} not registered in world")
            }
            ResourceInspectionError::ResourceIdNotRegistered(component_id) => {
                write!(f, "{component_id:?} not registered in world")
            }
        }
    }
}

impl core::error::Error for ResourceInspectionError {}

/// Settings for inspecting a resource.
#[derive(Clone, Copy, Debug)]
#[cfg_attr(feature = "serialize", derive(serde::Serialize, serde::Deserialize))]
pub struct ResourceInspectionSettings {
    /// Whether type paths in the value string are kept in full.
    /// When false, every `::` in the formatted value is collapsed, including inside string values.
    pub full_type_names: bool,
}

impl Default for ResourceInspectionSettings {
    fn default() -> Self {
        Self {
            full_type_names: true,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::inspection::extension_methods::WorldInspectionExtensionTrait;
    use bevy_ecs::{reflect::AppTypeRegistry, resource::Resource, world::World};
    use bevy_reflect::Reflect;

    #[derive(Resource, Reflect, Debug)]
    struct Score(u32);

    #[derive(Resource, Debug)]
    struct Unregistered;

    fn test_world() -> World {
        let mut world = World::new();
        world.init_resource::<AppTypeRegistry>();
        world
            .resource_mut::<AppTypeRegistry>()
            .write()
            .register::<Score>();
        world.insert_resource(Score(7));
        world
    }

    #[test]
    fn inspect_resource_reports_name_and_value() {
        let world = test_world();

        let inspection = world
            .inspect_resource::<Score>(ResourceInspectionSettings::default())
            .unwrap();

        assert_eq!(inspection.name.shortname().to_string(), "Score");
        assert!(inspection.value.contains('7'), "{}", inspection.value);
        assert_eq!(inspection.type_id, Some(TypeId::of::<Score>()));
        assert_eq!(inspection.memory_size, MemorySize::new(4));
    }

    #[test]
    fn inspect_all_resources_includes_inserted_resource() {
        let world = test_world();

        let inspections = world.inspect_all_resources(ResourceInspectionSettings::default());

        assert!(
            inspections
                .iter()
                .any(|inspection| inspection.type_id == Some(TypeId::of::<Score>())),
            "the inserted resource should be inspected"
        );
    }

    #[test]
    fn unregistered_resource_returns_error() {
        let world = test_world();

        let result = world.inspect_resource::<Unregistered>(ResourceInspectionSettings::default());

        assert!(matches!(
            result.unwrap_err(),
            ResourceInspectionError::ResourceNotRegistered(_)
        ));
    }

    #[test]
    fn display_contains_name_and_value() {
        let world = test_world();

        let displayed = world
            .inspect_resource::<Score>(ResourceInspectionSettings::default())
            .unwrap()
            .to_string();

        assert!(displayed.contains("Score"), "{displayed}");
        assert!(displayed.contains('7'), "{displayed}");
    }

    #[test]
    fn missing_type_registry_is_not_required() {
        let mut world = World::new();
        world.insert_resource(Score(7));

        let inspection = world
            .inspect_resource::<Score>(ResourceInspectionSettings::default())
            .unwrap();

        assert!(inspection.type_registration.is_none());
    }
}
