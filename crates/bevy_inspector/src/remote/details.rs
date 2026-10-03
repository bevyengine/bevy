//! The components of the selected remote entity, deserialized for the details panel.
//!
//! Only the selected entity's components are deserialized, and never inserted into the local
//! world. Components the inspector cannot deserialize are shown as the JSON the remote app sent.

use alloc::{
    boxed::Box,
    string::{String, ToString},
    vec::Vec,
};

use bevy_ecs::{
    component::{Component, ComponentId},
    entity::Entity,
    reflect::{AppTypeRegistry, ReflectComponent},
    resource::Resource,
    world::World,
};
use bevy_reflect::{serde::TypedReflectDeserializer, PartialReflect, ReflectFromReflect};
use bevy_utils::prelude::ShortName;
use serde::de::DeserializeSeed;
use serde_json::{Map, Value};

use super::{remote_entity, RemoteSnapshot};
use crate::{
    component_short_name,
    details_panel::{
        field_entries, read_only, ComponentDetails, FieldEntry, FieldValue,
    },
    InspectorSelection,
};

/// The number of characters of raw JSON shown for a component the inspector cannot describe.
const RAW_JSON_CHARS: usize = 160;

/// Keys the details panel group listing the components that are not registered locally.
#[derive(Component)]
struct UnregisteredComponents;

/// One serialized component of the selected remote entity.
#[derive(Debug)]
pub struct RemoteComponent {
    /// The local [`ComponentId`] of the component type, or `None` if it is not registered
    /// locally for reflection.
    pub id: Option<ComponentId>,
    /// The full type path of the component.
    pub type_path: String,
    /// The value as the remote app serialized it.
    pub json: Value,
    /// The value deserialized with the local type registration, or why that failed.
    pub value: Result<Box<dyn PartialReflect>, String>,
}

/// The components of the selected remote entity.
#[derive(Resource, Debug, Default)]
pub struct RemoteDetails {
    /// The proxy the components belong to.
    pub proxy: Option<Entity>,
    /// The serialized components, in the order the remote app sent them.
    pub components: Vec<RemoteComponent>,
    /// The components the remote app reported through `has` but did not serialize, as their
    /// local [`ComponentId`] and full type path.
    pub detected: Vec<(ComponentId, String)>,
    revision: u64,
}

/// Deserializes the components of the selected remote entity into [`RemoteDetails`] when the
/// selection or the snapshot changes.
pub fn sync_remote_details(world: &mut World) {
    let selection = world.resource::<InspectorSelection>().0;
    let target = selection.and_then(|proxy| Some((proxy, remote_entity(world, proxy)?)));
    let revision = world.resource::<RemoteSnapshot>().revision;
    let details = world.resource::<RemoteDetails>();
    let proxy = target.map(|(proxy, _)| proxy);
    if details.proxy == proxy && (proxy.is_none() || details.revision == revision) {
        return;
    }

    let (components, detected) = match target {
        Some((_, remote)) => {
            let snapshot = world.resource::<RemoteSnapshot>();
            let rows = snapshot.components(remote).cloned().unwrap_or_default();
            let detected = snapshot.detected(remote).to_vec();
            (
                deserialize_components(world, rows),
                detected_components(world, detected),
            )
        }
        None => (Vec::new(), Vec::new()),
    };
    *world.resource_mut::<RemoteDetails>() = RemoteDetails {
        proxy,
        components,
        detected,
        revision,
    };
}

fn deserialize_components(world: &mut World, rows: Map<String, Value>) -> Vec<RemoteComponent> {
    let registry = world.resource::<AppTypeRegistry>().clone();
    let registry = registry.read();
    let mut components = Vec::new();
    for (type_path, json) in rows {
        let Some((registration, reflect_component)) = registry
            .get_with_type_path(&type_path)
            .and_then(|registration| {
                Some((registration, registration.data::<ReflectComponent>()?))
            })
        else {
            world.register_component::<UnregisteredComponents>();
            components.push(RemoteComponent {
                id: None,
                type_path,
                json,
                value: Err("not registered locally".to_string()),
            });
            continue;
        };

        let value = TypedReflectDeserializer::new(registration, &registry)
            .deserialize(&json)
            .map_err(|error| error.to_string())
            .map(|value| {
                match registration
                    .data::<ReflectFromReflect>()
                    .and_then(|from_reflect| from_reflect.from_reflect(value.as_ref()))
                {
                    Some(concrete) => concrete.into_partial_reflect(),
                    None => value,
                }
            });
        components.push(RemoteComponent {
            id: Some(reflect_component.register_component(world)),
            type_path,
            json,
            value,
        });
    }
    components
}

/// The local [`ComponentId`] of each detected type path.
///
/// Detected types are asked for by their local registration, so each one has a
/// [`ReflectComponent`].
fn detected_components(world: &mut World, detected: Vec<String>) -> Vec<(ComponentId, String)> {
    let registry = world.resource::<AppTypeRegistry>().clone();
    let registry = registry.read();
    detected
        .into_iter()
        .filter_map(|type_path| {
            let reflect_component = registry
                .get_with_type_path(&type_path)?
                .data::<ReflectComponent>()?;
            Some((reflect_component.register_component(world), type_path))
        })
        .collect()
}

/// The details panel groups of `entity`, if it is a proxy.
///
/// Components registered locally are keyed by their local [`ComponentId`]. Those the remote app
/// did not serialize show a note, and those that fail to deserialize show their raw JSON.
/// Components that are not registered locally are gathered into one last group showing their raw
/// JSON.
pub(crate) fn proxy_components(world: &World, entity: Entity) -> Option<Vec<ComponentDetails>> {
    remote_entity(world, entity)?;
    let details = world.resource::<RemoteDetails>();
    if details.proxy != Some(entity) {
        return Some(Vec::new());
    }

    let registry = world.resource::<AppTypeRegistry>().read();
    let mut groups = Vec::new();
    let mut unregistered = Vec::new();
    for component in &details.components {
        let Some(id) = component.id else {
            unregistered.push(component);
            continue;
        };
        let fields = match &component.value {
            Ok(value) => field_entries(value.as_ref(), &registry)
                .into_iter()
                .map(|entry| FieldEntry {
                    value: read_only(entry.value),
                    limit: None,
                    ..entry
                })
                .collect(),
            Err(error) => alloc::vec![value_entry(raw_value(&component.json, Some(error)))],
        };
        groups.push(ComponentDetails {
            id,
            name: component_short_name(world, id),
            type_path: component.type_path.clone(),
            memory: String::new(),
            fields,
        });
    }
    for (id, type_path) in &details.detected {
        groups.push(ComponentDetails {
            id: *id,
            name: component_short_name(world, *id),
            type_path: type_path.clone(),
            memory: String::new(),
            fields: alloc::vec![value_entry("not serializable".to_string())],
        });
    }
    groups.sort_by(|left, right| (&left.name, left.id).cmp(&(&right.name, right.id)));
    unregistered.sort_by(|left, right| left.type_path.cmp(&right.type_path));

    if !unregistered.is_empty()
        && let Some(id) = world.component_id::<UnregisteredComponents>()
    {
        groups.push(ComponentDetails {
            id,
            name: "Unregistered".to_string(),
            type_path: String::new(),
            memory: String::new(),
            fields: unregistered
                .into_iter()
                .map(|component| FieldEntry {
                    path: component.type_path.clone(),
                    label: ShortName(&component.type_path).to_string(),
                    depth: 0,
                    value: FieldValue::Label(raw_value(&component.json, None)),
                    limit: None,
                })
                .collect(),
        });
    }
    Some(groups)
}

fn value_entry(text: String) -> FieldEntry {
    FieldEntry {
        path: String::new(),
        label: "value".to_string(),
        depth: 0,
        value: FieldValue::Label(text),
        limit: None,
    }
}

fn raw_value(json: &Value, error: Option<&String>) -> String {
    let mut text = json.to_string();
    if text.chars().count() > RAW_JSON_CHARS {
        text = text.chars().take(RAW_JSON_CHARS).collect();
        text.push_str("...");
    }
    match error {
        Some(error) => alloc::format!("{text} ({error})"),
        None => text,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        details_panel::inspect_components,
        remote::{
            tests::{apply, proxy, remote, row, test_world},
            RemoteEntityProxy, RemoteLabel,
        },
    };
    use bevy_ecs::{entity_disabling::Disabled, name::Name};
    use bevy_reflect::{prelude::ReflectDefault, Reflect, TypePath};
    use serde_json::json;

    const NAME: &str = "bevy_ecs::name::Name";

    #[derive(Component, Reflect, Default)]
    #[reflect(Component, Default)]
    struct Health {
        current: f32,
        owner: Option<Entity>,
    }

    #[derive(Component, Reflect, Default)]
    #[reflect(Component, Default)]
    struct Team(u8);

    #[derive(Component, Reflect, Default)]
    #[reflect(Component, Default)]
    struct Handled;

    fn details_world() -> World {
        let world = test_world();
        {
            let registry = world.resource::<AppTypeRegistry>();
            let mut registry = registry.write();
            registry.register::<Health>();
            registry.register::<Team>();
            registry.register::<Handled>();
            registry.register::<Disabled>();
            registry.register::<RemoteEntityProxy>();
            registry.register::<RemoteLabel>();
        }
        world
    }

    fn select(world: &mut World, remote: Entity) -> Entity {
        let proxy = proxy(world, remote);
        world.resource_mut::<InspectorSelection>().0 = Some(proxy);
        sync_remote_details(world);
        proxy
    }

    fn names(groups: &[ComponentDetails]) -> Vec<&str> {
        groups.iter().map(|group| group.name.as_str()).collect()
    }

    #[test]
    fn the_details_of_a_despawned_proxy_are_cleared() {
        let mut world = details_world();
        apply(
            &mut world,
            alloc::vec![row(remote(1), json!({ NAME: "A" }))],
        );
        let selected = select(&mut world, remote(1));
        assert_eq!(world.resource::<RemoteDetails>().proxy, Some(selected));

        apply(&mut world, alloc::vec![]);

        assert_eq!(world.resource::<RemoteDetails>().proxy, None);
        assert!(world.resource::<RemoteDetails>().components.is_empty());
    }

    #[test]
    fn deserializes_the_selected_entity_without_inserting_components() {
        let mut world = details_world();
        let target = remote(1);
        apply(
            &mut world,
            alloc::vec![row(
                target,
                json!({
                    NAME: "Player",
                    Health::type_path(): { "current": 3.5, "owner": "9v0" },
                    Team::type_path(): 2,
                    "demo::Unknown": { "value": 3 },
                    "demo::Other": 1,
                }),
            )],
        );
        let proxy = select(&mut world, target);

        assert!(world.get::<Health>(proxy).is_none());
        let health = world
            .resource::<RemoteDetails>()
            .components
            .iter()
            .find(|component| component.type_path == Health::type_path())
            .unwrap();
        assert_eq!(
            health
                .value
                .as_ref()
                .unwrap()
                .try_downcast_ref::<Health>()
                .map(|health| health.owner),
            Some(Some(remote(9)))
        );

        let groups = proxy_components(&world, proxy).unwrap();
        assert_eq!(names(&groups), ["Health", "Name", "Team", "Unregistered"]);
        assert_eq!(
            groups[0].id,
            world.component_id::<Health>().unwrap(),
            "registered components are keyed by their local id"
        );
        let owner = groups[0]
            .fields
            .iter()
            .find(|entry| entry.path == "owner.0")
            .unwrap();
        assert_eq!(owner.value, FieldValue::Label("9v0".to_string()));
        assert!(groups[0]
            .fields
            .iter()
            .all(|entry| matches!(entry.value, FieldValue::Label(_))));
        assert_eq!(
            groups[1].fields[0].value,
            FieldValue::Label("Player".into())
        );
        let unregistered: Vec<&str> = groups[3]
            .fields
            .iter()
            .map(|entry| entry.label.as_str())
            .collect();
        assert_eq!(unregistered, ["Other", "Unknown"]);
    }

    #[test]
    fn a_mismatched_local_definition_falls_back_to_raw_json() {
        let mut world = details_world();
        let target = remote(1);
        apply(
            &mut world,
            alloc::vec![row(
                target,
                json!({ Health::type_path(): { "renamed": 1.0 } })
            )],
        );
        let proxy = select(&mut world, target);

        let groups = proxy_components(&world, proxy).unwrap();
        assert_eq!(names(&groups), ["Health"]);
        let FieldValue::Label(text) = &groups[0].fields[0].value else {
            panic!("raw components are read-only");
        };
        assert!(text.starts_with(r#"{"renamed":1.0}"#));
    }

    #[test]
    fn detected_components_are_listed_with_a_note() {
        let mut world = details_world();
        let target = remote(1);
        apply(
            &mut world,
            alloc::vec![json!({
                "entity": target,
                "components": { NAME: "Cube" },
                "has": { Handled::type_path(): true, Team::type_path(): false },
            })],
        );
        let proxy = select(&mut world, target);

        let groups = proxy_components(&world, proxy).unwrap();
        assert_eq!(names(&groups), ["Handled", "Name"]);
        assert_eq!(groups[0].id, world.component_id::<Handled>().unwrap());
        assert_eq!(
            groups[0].fields,
            [value_entry("not serializable".to_string())]
        );
    }

    #[test]
    fn the_proxy_components_are_never_shown() {
        let mut world = details_world();
        let target = remote(1);
        apply(
            &mut world,
            alloc::vec![row(target, json!({ NAME: "Cube" }))],
        );
        let proxy = select(&mut world, target);
        assert!(world.get::<RemoteLabel>(proxy).is_some());

        let groups = inspect_components(&world, Some(proxy));
        assert_eq!(names(&groups), ["Name"]);
        assert!(world.get::<Name>(proxy).is_none());
    }
}
