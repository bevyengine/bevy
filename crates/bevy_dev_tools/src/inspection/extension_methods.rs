//! Inspection methods that extend Bevy's own types.

use bevy_ecs::{component::Component, component::ComponentId, entity::Entity, world::World};
use bevy_utils::memory_size::MemorySize;
use core::any::type_name;

use crate::inspection::{
    component_inspection::{
        ComponentDetailLevel, ComponentInspection, ComponentInspectionError,
        ComponentInspectionSettings, ComponentTypeInspection, ComponentTypeMetadata,
    },
    reflection_tools::{clone_incomplete, component_value_to_string},
};

/// Inspection methods for [`World`], provided as an extension trait.
pub trait WorldInspectionExtensionTrait {
    /// Inspects the component with the given [`ComponentId`] on the given entity,
    /// using the caller-provided [`ComponentTypeMetadata`].
    fn inspect_component_by_id(
        &self,
        component_id: ComponentId,
        entity: Entity,
        metadata: &ComponentTypeMetadata,
        settings: ComponentInspectionSettings,
    ) -> Result<ComponentInspection, ComponentInspectionError>;

    /// Inspects the component of type `C` on the given entity,
    /// computing its [`ComponentTypeMetadata`] on the fly.
    fn inspect_component<C: Component>(
        &self,
        entity: Entity,
        settings: ComponentInspectionSettings,
    ) -> Result<ComponentInspection, ComponentInspectionError>;

    /// Inspects the component type `C` itself, rather than a component on a specific entity.
    fn inspect_component_type<C: Component>(
        &self,
    ) -> Result<ComponentTypeInspection, ComponentInspectionError>;

    /// Inspects the component type with the given [`ComponentId`],
    /// the dynamically-typed variant of [`inspect_component_type`](Self::inspect_component_type).
    fn inspect_component_type_by_id(
        &self,
        component_id: ComponentId,
    ) -> Result<ComponentTypeInspection, ComponentInspectionError>;
}

impl WorldInspectionExtensionTrait for World {
    fn inspect_component_by_id(
        &self,
        component_id: ComponentId,
        entity: Entity,
        metadata: &ComponentTypeMetadata,
        settings: ComponentInspectionSettings,
    ) -> Result<ComponentInspection, ComponentInspectionError> {
        let component_info = self.components().get_info(component_id).ok_or(
            ComponentInspectionError::ComponentIdNotRegistered(component_id),
        )?;
        let memory_size = MemorySize::new(component_info.layout().size());
        let name = component_info.name();

        let entity_ref = self
            .get_entity(entity)
            .map_err(|_| ComponentInspectionError::ComponentNotFound(component_id))?;

        if !entity_ref.contains_id(component_id) {
            return Err(ComponentInspectionError::ComponentNotFound(component_id));
        }

        let value = if settings.detail_level == ComponentDetailLevel::Names {
            None
        } else {
            Some(component_value_to_string(
                self,
                entity,
                metadata.type_id,
                settings.full_type_names,
            ))
        };

        let reflected_value = if settings.store_reflected_value {
            metadata
                .type_id
                .and_then(|type_id| self.get_reflect(entity, type_id).ok())
                .and_then(|reflected| clone_incomplete(reflected.as_partial_reflect()).ok())
        } else {
            None
        };

        Ok(ComponentInspection {
            entity,
            component_id,
            name,
            memory_size,
            value,
            reflected_value,
        })
    }

    fn inspect_component<C: Component>(
        &self,
        entity: Entity,
        settings: ComponentInspectionSettings,
    ) -> Result<ComponentInspection, ComponentInspectionError> {
        let component_id = self.components().valid_component_id::<C>().ok_or(
            ComponentInspectionError::ComponentNotRegistered(type_name::<C>()),
        )?;

        let metadata = ComponentTypeMetadata::new(self, component_id)?;

        self.inspect_component_by_id(component_id, entity, &metadata, settings)
    }

    fn inspect_component_type<C: Component>(
        &self,
    ) -> Result<ComponentTypeInspection, ComponentInspectionError> {
        let component_id = self.components().valid_component_id::<C>().ok_or(
            ComponentInspectionError::ComponentNotRegistered(type_name::<C>()),
        )?;

        self.inspect_component_type_by_id(component_id)
    }

    fn inspect_component_type_by_id(
        &self,
        component_id: ComponentId,
    ) -> Result<ComponentTypeInspection, ComponentInspectionError> {
        let metadata = ComponentTypeMetadata::new(self, component_id)?;

        let mut entity_count = 0;
        for archetype in self.archetypes().iter() {
            if archetype.contains(component_id) {
                entity_count += archetype.len() as usize;
            }
        }

        Ok(ComponentTypeInspection {
            entity_count,
            metadata,
        })
    }
}
