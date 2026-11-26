//! The section of the details panel showing where the selected entity is stored and where it was
//! spawned.

use alloc::{
    string::{String, ToString},
    vec::Vec,
};

use bevy_ecs::{
    entity::Entity,
    entity_disabling::{DefaultQueryFilters, Disabled},
    world::World,
};

use super::{ComponentDetails, ComponentKey, FieldAnnotations, FieldEntry, FieldValue};

pub(super) const SECTION_KEY: &str = "#entity";

pub(super) const LOCATION_UNTRACKED: &str = "track_location feature is off";

pub(super) fn is_entity_section(key: &ComponentKey) -> bool {
    &*key.0 == SECTION_KEY
}

pub(super) fn prepend_entity_section(
    world: &World,
    entity: Entity,
    components: &mut Vec<ComponentDetails>,
) {
    if let Some(section) = entity_section(world, entity) {
        components.insert(0, section);
    }
}

fn entity_section(world: &World, entity: Entity) -> Option<ComponentDetails> {
    let entity_ref = world.get_entity(entity).ok()?;
    let location = entity_ref.location();
    let spawned_by = entity_ref
        .spawned_by()
        .into_option()
        .map(ToString::to_string)
        .unwrap_or_else(|| LOCATION_UNTRACKED.to_string());
    let disabled = match world.get_resource::<DefaultQueryFilters>() {
        Some(filters) => filters.disabling_ids().any(|id| entity_ref.contains_id(id)),
        None => entity_ref.contains::<Disabled>(),
    };
    let rows = [
        ("index", entity.index().to_string()),
        ("generation", entity.generation().to_string()),
        ("archetype", location.archetype_id.index().to_string()),
        ("archetype row", location.archetype_row.index().to_string()),
        ("table", location.table_id.as_usize().to_string()),
        ("table row", location.table_row.index().to_string()),
        (
            "archetype size",
            entity_ref.archetype().component_count().to_string(),
        ),
        ("spawn tick", entity_ref.spawn_tick().get().to_string()),
        ("spawned by", spawned_by),
        ("disabled", disabled.to_string()),
    ];
    Some(ComponentDetails {
        key: ComponentKey::from(SECTION_KEY),
        name: "Entity".to_string(),
        memory: String::new(),
        fields: rows.into_iter().map(row).collect(),
    })
}

fn row((label, value): (&str, String)) -> FieldEntry {
    FieldEntry {
        path: label.replace(' ', "_"),
        label: label.to_string(),
        depth: 0,
        value: FieldValue::Label(value),
        limit: None,
        annotations: FieldAnnotations {
            read_only: true,
            ..Default::default()
        },
        unit_enum: false,
    }
}
