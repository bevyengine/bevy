//! Inspection methods that extend Bevy's own types.

use bevy_ecs::{
    component::{Component, ComponentId},
    entity::Entity,
    query::SpawnDetails,
    reflect::AppTypeRegistry,
    resource::{IsResource, Resource},
    system::{Commands, EntityCommands},
    world::{EntityWorldMut, World},
};
use bevy_log::{info, warn};
use bevy_utils::memory_size::MemorySize;
use core::any::{type_name, TypeId};

use crate::inspection::{
    component_inspection::{
        ComponentDetailLevel, ComponentInspection, ComponentInspectionError,
        ComponentInspectionSettings, ComponentMetadataMap, ComponentTypeInspection,
        ComponentTypeMetadata,
    },
    entity_inspection::{EntityInspection, EntityInspectionError, EntityInspectionSettings},
    label_resolution::{resolve_label, ComponentLabelData, EntityLabel},
    reflection_tools::{clone_incomplete, component_value_to_string},
    resource_inspection::{
        ResourceInspection, ResourceInspectionError, ResourceInspectionSettings,
    },
};

/// Inspection methods for [`World`], provided as an extension trait.
pub trait WorldInspectionExtensionTrait {
    /// Inspects the given entity, computing component type metadata on the fly.
    fn inspect(
        &self,
        entity: Entity,
        settings: EntityInspectionSettings,
    ) -> Result<EntityInspection, EntityInspectionError>;

    /// Inspects the given entity, reusing the component type metadata in `metadata_map`.
    fn inspect_cached(
        &self,
        entity: Entity,
        settings: &EntityInspectionSettings,
        metadata_map: &ComponentMetadataMap,
    ) -> Result<EntityInspection, EntityInspectionError>;

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

    /// Inspects the resource of type `R`.
    fn inspect_resource<R: Resource>(
        &self,
        settings: ResourceInspectionSettings,
    ) -> Result<ResourceInspection, ResourceInspectionError>;

    /// Inspects the resource with the given [`ComponentId`],
    /// the dynamically-typed variant of [`inspect_resource`](Self::inspect_resource).
    fn inspect_resource_by_id(
        &self,
        component_id: ComponentId,
        settings: ResourceInspectionSettings,
    ) -> Result<ResourceInspection, ResourceInspectionError>;

    /// Inspects every resource currently present in the world.
    fn inspect_all_resources(
        &self,
        settings: ResourceInspectionSettings,
    ) -> Vec<ResourceInspection>;
}

impl WorldInspectionExtensionTrait for World {
    fn inspect(
        &self,
        entity: Entity,
        settings: EntityInspectionSettings,
    ) -> Result<EntityInspection, EntityInspectionError> {
        let metadata_map = ComponentMetadataMap::for_entity(self, entity);

        self.inspect_cached(entity, &settings, &metadata_map)
    }

    fn inspect_cached(
        &self,
        entity: Entity,
        settings: &EntityInspectionSettings,
        metadata_map: &ComponentMetadataMap,
    ) -> Result<EntityInspection, EntityInspectionError> {
        let entity_ref = self
            .get_entity(entity)
            .map_err(|_| EntityInspectionError::EntityNotFound(entity))?;

        let spawn_details = self
            .try_query::<SpawnDetails>()
            .and_then(|mut query| query.get(self, entity).ok());

        let (components, total_memory_size) = if settings.include_components {
            let components: Vec<ComponentInspection> = entity_ref
                .archetype()
                .components()
                .iter()
                .filter_map(|component_id| {
                    let metadata = metadata_map.get(component_id)?;

                    self.inspect_component_by_id(
                        *component_id,
                        entity,
                        metadata,
                        settings.component_settings,
                    )
                    .ok()
                })
                .collect();

            let total_bytes = components
                .iter()
                .fold(0u64, |acc, component| acc + component.memory_size.0);

            (Some(components), Some(MemorySize(total_bytes)))
        } else {
            (None, None)
        };

        let label = resolve_entity_label(self, entity, components.as_deref(), metadata_map);

        Ok(EntityInspection {
            entity,
            label,
            total_memory_size,
            components,
            spawn_details,
        })
    }

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

    fn inspect_resource<R: Resource>(
        &self,
        settings: ResourceInspectionSettings,
    ) -> Result<ResourceInspection, ResourceInspectionError> {
        let component_id = self.components().component_id::<R>().ok_or(
            ResourceInspectionError::ResourceNotRegistered(type_name::<R>()),
        )?;

        self.inspect_resource_by_id(component_id, settings)
    }

    fn inspect_resource_by_id(
        &self,
        component_id: ComponentId,
        settings: ResourceInspectionSettings,
    ) -> Result<ResourceInspection, ResourceInspectionError> {
        let component_info = self.components().get_info(component_id).ok_or(
            ResourceInspectionError::ResourceIdNotRegistered(component_id),
        )?;
        let memory_size = MemorySize::new(component_info.layout().size());
        let name = component_info.name();
        let type_id = component_info.type_id();

        let type_registration = type_id.and_then(|type_id| {
            self.get_resource::<AppTypeRegistry>()
                .and_then(|type_registry| type_registry.read().get(type_id).cloned())
        });

        let value = match self.resource_entities().get(component_id) {
            Some(resource_entity) => {
                component_value_to_string(self, resource_entity, type_id, settings.full_type_names)
            }
            None => "<Resource not present>".to_string(),
        };

        Ok(ResourceInspection {
            component_id,
            name,
            value,
            type_id,
            memory_size,
            type_registration,
        })
    }

    fn inspect_all_resources(
        &self,
        settings: ResourceInspectionSettings,
    ) -> Vec<ResourceInspection> {
        self.resource_entities()
            .iter()
            .filter_map(|(component_id, _entity)| {
                self.inspect_resource_by_id(component_id, settings).ok()
            })
            .collect()
    }
}

/// Determines the label of an inspected entity from its inspected components. Entities that back
/// a resource take the short name of that resource.
fn resolve_entity_label(
    world: &World,
    entity: Entity,
    components: Option<&[ComponentInspection]>,
    metadata_map: &ComponentMetadataMap,
) -> Option<EntityLabel> {
    let Some(components) = components else {
        return resolve_label(world, entity, &[]);
    };

    let is_resource = components.iter().any(|component| {
        metadata_map
            .get(&component.component_id)
            .and_then(|metadata| metadata.type_id)
            == Some(TypeId::of::<IsResource>())
    });

    if is_resource {
        return components
            .iter()
            .find(|component| {
                world
                    .resource_entities()
                    .get(component.component_id)
                    .is_some()
            })
            .map(|component| EntityLabel::resolved(&component.name.shortname().to_string()));
    }

    let short_names: Vec<String> = components
        .iter()
        .map(|component| component.name.shortname().to_string())
        .collect();

    let label_data: Vec<ComponentLabelData> = components
        .iter()
        .zip(short_names.iter())
        .map(|(component, short_name)| ComponentLabelData {
            component_id: component.component_id,
            short_name: short_name.as_str(),
            label_definition_priority: metadata_map
                .get(&component.component_id)
                .and_then(|metadata| metadata.label_definition_priority),
        })
        .collect();

    resolve_label(world, entity, &label_data)
}

/// Inspection methods for [`EntityCommands`], provided as an extension trait.
pub trait EntityCommandsInspectionExtensionTrait {
    /// Inspects this entity, logging the result at the info level.
    fn inspect(&mut self, settings: EntityInspectionSettings);

    /// Inspects the component of type `C` on this entity, logging the result at the info level.
    fn inspect_component<C: Component>(&mut self, settings: ComponentInspectionSettings);
}

impl EntityCommandsInspectionExtensionTrait for EntityCommands<'_> {
    fn inspect(&mut self, settings: EntityInspectionSettings) {
        let entity = self.id();

        self.queue(move |entity_world_mut: EntityWorldMut| {
            let world = entity_world_mut.world();
            match world.inspect(entity, settings) {
                Ok(inspection) => info!("{inspection}"),
                Err(error) => warn!("Failed to inspect entity: {error}"),
            }
        });
    }

    fn inspect_component<C: Component>(&mut self, settings: ComponentInspectionSettings) {
        let entity = self.id();

        self.queue(move |entity_world_mut: EntityWorldMut| {
            let world = entity_world_mut.world();
            match world.inspect_component::<C>(entity, settings) {
                Ok(inspection) => info!("{inspection}"),
                Err(error) => warn!("Failed to inspect component: {error}"),
            }
        });
    }
}

/// Inspection methods for [`Commands`], provided as an extension trait.
pub trait CommandsInspectionExtensionTrait {
    /// Inspects the resource of type `R`, logging the result at the info level.
    fn inspect_resource<R: Resource>(&mut self, settings: ResourceInspectionSettings);

    /// Inspects every resource in the world, logging the results at the info level.
    fn inspect_all_resources(&mut self, settings: ResourceInspectionSettings);
}

impl CommandsInspectionExtensionTrait for Commands<'_, '_> {
    fn inspect_resource<R: Resource>(&mut self, settings: ResourceInspectionSettings) {
        self.queue(
            move |world: &mut World| match world.inspect_resource::<R>(settings) {
                Ok(inspection) => info!("{inspection}"),
                Err(error) => warn!("Failed to inspect resource: {error}"),
            },
        );
    }

    fn inspect_all_resources(&mut self, settings: ResourceInspectionSettings) {
        self.queue(move |world: &mut World| {
            let mut inspections = world.inspect_all_resources(settings);
            inspections.sort_by_key(|inspection| inspection.name.shortname().to_string());

            let mut log_string = format!("Inspecting all resources ({} found):", inspections.len());
            for inspection in &inspections {
                log_string.push_str(&format!("\n- {inspection}"));
            }

            info!("{log_string}");
        });
    }
}
