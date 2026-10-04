//! The components of the selected remote entity, fetched and deserialized for the details panel.
//!
//! The selected entity's components are listed with `world.list_components` and read with
//! `world.get_components` when the selection changes, and then twice a second. They are only
//! deserialized when they change, and never inserted into the local world. Components the
//! inspector cannot deserialize are shown as the JSON the remote app sent.
//!
//! `world.list_components` reports component names, which the remote app only knows when built
//! with the `debug` feature, as Bevy is by default.

use alloc::{
    boxed::Box,
    string::{String, ToString},
    vec::Vec,
};
use core::{fmt::Write, time::Duration};

use bevy_ecs::{
    component::{Component, ComponentId},
    entity::Entity,
    reflect::{AppTypeRegistry, ReflectComponent},
    resource::Resource,
    world::World,
};
use bevy_log::warn;
use bevy_reflect::{
    serde::TypedReflectDeserializer, PartialReflect, ReflectFromReflect, TypeRegistry,
};
use bevy_remote::{
    builtin_methods::{
        BrpGetComponentsParams, BrpGetComponentsResponse, BrpListComponentsParams,
        BRP_GET_COMPONENTS_METHOD, BRP_LIST_COMPONENTS_METHOD,
    },
    client::{BrpClient, BrpClientError},
};
use bevy_tasks::{block_on, poll_once, Task};
use bevy_time::{Real, Time};
use bevy_utils::prelude::ShortName;
use serde::de::DeserializeSeed;
use serde_json::Value;

use super::{
    remote_entity, spawn, RemoteConnection, RemoteConnectionState, POLL_INTERVAL, REQUEST_TIMEOUT,
};
use crate::{
    component_short_name,
    details_panel::{
        field_entries, read_only, ComponentDetails, DetailsPanelSync, FieldEntry, FieldValue,
    },
    InspectorSelection,
};

/// The time between two fetches of the selected entity's components.
const DETAILS_INTERVAL: Duration = POLL_INTERVAL;

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

/// The components of one remote entity, as the remote app reported them.
#[derive(Debug, Default, Clone, PartialEq)]
pub struct RemoteEntityData {
    /// The serialized components and their full type paths, sorted by type path.
    pub components: Vec<(String, Value)>,
    /// The sorted names of every component the entity holds, serialized or not.
    pub listed: Vec<String>,
}

struct DetailsCall {
    task: Task<Result<RemoteEntityData, BrpClientError>>,
    started: Duration,
}

/// The components of the selected remote entity.
#[derive(Resource, Default)]
pub struct RemoteDetails {
    /// The proxy the components belong to.
    pub proxy: Option<Entity>,
    /// The serialized components, sorted by type path.
    pub components: Vec<RemoteComponent>,
    /// The components the remote app holds but did not serialize, as their local
    /// [`ComponentId`] and full type path.
    pub detected: Vec<(ComponentId, String)>,
    listed: Vec<String>,
    pending: Option<DetailsCall>,
    next_fetch: Duration,
    error: Option<String>,
}

/// Fetches the components of the selected remote entity, and deserializes them into
/// [`RemoteDetails`] when they change.
pub fn sync_remote_details(world: &mut World) {
    let selection = world.resource::<InspectorSelection>().0;
    let target = selection.and_then(|proxy| Some((proxy, remote_entity(world, proxy)?)));
    let proxy = target.map(|(proxy, _)| proxy);
    if world.resource::<RemoteDetails>().proxy != proxy {
        *world.resource_mut::<RemoteDetails>() = RemoteDetails {
            proxy,
            ..RemoteDetails::default()
        };
        world.resource_mut::<DetailsPanelSync>().set_dirty();
    }
    let Some((_, remote)) = target else {
        return;
    };
    let now = world
        .get_resource::<Time<Real>>()
        .map(Time::elapsed)
        .unwrap_or_default();

    if let Some(data) = finish_fetch(&mut world.resource_mut::<RemoteDetails>(), now) {
        receive_entity_data(world, data);
    }

    let details = world.resource::<RemoteDetails>();
    if details.pending.is_some() || now < details.next_fetch {
        return;
    }
    let connection = world.resource::<RemoteConnection>();
    if !matches!(connection.state, RemoteConnectionState::Connected { .. }) {
        return;
    }
    let Some(client) = connection.client().cloned() else {
        return;
    };
    world.resource_mut::<RemoteDetails>().pending = Some(DetailsCall {
        task: spawn(fetch_entity(client, remote)),
        started: now,
    });
}

/// Handles the fetch in flight if it was answered or timed out, returning the fetched data.
fn finish_fetch(details: &mut RemoteDetails, now: Duration) -> Option<RemoteEntityData> {
    let mut call = details.pending.take()?;
    let result = match block_on(poll_once(&mut call.task)) {
        Some(result) => result,
        None if now.saturating_sub(call.started) >= REQUEST_TIMEOUT => Err(
            BrpClientError::InvalidResponse("the remote app did not answer in time".into()),
        ),
        None => {
            details.pending = Some(call);
            return None;
        }
    };
    details.next_fetch = now + DETAILS_INTERVAL;
    match result {
        Ok(data) => {
            details.error = None;
            Some(data)
        }
        Err(error) => {
            let error = error.to_string();
            if details.error.as_ref() != Some(&error) {
                warn!("the remote inspector could not read the selected entity: {error}");
                details.error = Some(error);
            }
            None
        }
    }
}

async fn fetch_entity(
    client: BrpClient,
    remote: Entity,
) -> Result<RemoteEntityData, BrpClientError> {
    let params = serde_json::to_value(BrpListComponentsParams { entity: remote })?;
    let value = client
        .call(BRP_LIST_COMPONENTS_METHOD, Some(params))
        .await?;
    let listed: Vec<String> = serde_json::from_value(value)?;

    let params = serde_json::to_value(BrpGetComponentsParams {
        entity: remote,
        components: listed.clone(),
        strict: false,
    })?;
    let value = client.call(BRP_GET_COMPONENTS_METHOD, Some(params)).await?;
    let (BrpGetComponentsResponse::Lenient { components, .. }
    | BrpGetComponentsResponse::Strict(components)) = serde_json::from_value(value)?;
    let mut components: Vec<(String, Value)> = components.into_iter().collect();
    components.sort_unstable_by(|(left, _), (right, _)| left.cmp(right));
    Ok(RemoteEntityData { components, listed })
}

/// Deserializes `data` into [`RemoteDetails`] unless it equals the data already shown.
///
/// Components whose value did not change keep their deserialized value.
pub(crate) fn receive_entity_data(world: &mut World, data: RemoteEntityData) {
    let details = world.resource::<RemoteDetails>();
    if details.listed == data.listed
        && details.components.len() == data.components.len()
        && details
            .components
            .iter()
            .zip(&data.components)
            .all(|(component, (type_path, json))| {
                component.type_path == *type_path && component.json == *json
            })
    {
        return;
    }

    let registry = world.resource::<AppTypeRegistry>().clone();
    let registry = registry.read();
    let mut previous = core::mem::take(&mut world.resource_mut::<RemoteDetails>().components);
    let mut components = Vec::with_capacity(data.components.len());
    for (type_path, json) in data.components {
        match previous
            .iter()
            .position(|component| component.type_path == type_path && component.json == json)
        {
            Some(index) => components.push(previous.swap_remove(index)),
            None => components.push(deserialize_component(world, &registry, type_path, json)),
        }
    }
    let detected = detected_components(world, &registry, &data.listed, &components);

    let mut details = world.resource_mut::<RemoteDetails>();
    details.components = components;
    details.detected = detected;
    details.listed = data.listed;
    world.resource_mut::<DetailsPanelSync>().set_dirty();
}

fn deserialize_component(
    world: &mut World,
    registry: &TypeRegistry,
    type_path: String,
    json: Value,
) -> RemoteComponent {
    let Some((registration, reflect_component)) = registry
        .get_with_type_path(&type_path)
        .and_then(|registration| Some((registration, registration.data::<ReflectComponent>()?)))
    else {
        world.register_component::<UnregisteredComponents>();
        return RemoteComponent {
            id: None,
            type_path,
            json,
            value: Err("not registered locally".to_string()),
        };
    };

    let value = TypedReflectDeserializer::new(registration, registry)
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
    RemoteComponent {
        id: Some(reflect_component.register_component(world)),
        type_path,
        json,
        value,
    }
}

/// The local [`ComponentId`] and type path of each listed component that is registered locally
/// but was not serialized.
fn detected_components(
    world: &mut World,
    registry: &TypeRegistry,
    listed: &[String],
    components: &[RemoteComponent],
) -> Vec<(ComponentId, String)> {
    listed
        .iter()
        .filter(|type_path| {
            !components
                .iter()
                .any(|component| component.type_path == **type_path)
        })
        .filter_map(|type_path| {
            let reflect_component = registry
                .get_with_type_path(type_path)?
                .data::<ReflectComponent>()?;
            Some((
                reflect_component.register_component(world),
                type_path.clone(),
            ))
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

/// A string that stops accepting text after [`RAW_JSON_CHARS`] characters.
#[derive(Default)]
struct BoundedText {
    text: String,
    chars: usize,
}

impl Write for BoundedText {
    fn write_str(&mut self, text: &str) -> core::fmt::Result {
        for char in text.chars() {
            if self.chars == RAW_JSON_CHARS {
                return Err(core::fmt::Error);
            }
            self.text.push(char);
            self.chars += 1;
        }
        Ok(())
    }
}

/// `json` as text, cut after [`RAW_JSON_CHARS`] characters without formatting the rest.
fn raw_value(json: &Value, error: Option<&String>) -> String {
    let mut text = BoundedText::default();
    if write!(text, "{json}").is_err() {
        text.text.push_str("...");
    }
    match error {
        Some(error) => alloc::format!("{} ({error})", text.text),
        None => text.text,
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

    /// Receives `components` as the selected entity's data, listed along with `unserialized`.
    fn fetched(world: &mut World, components: Value, unserialized: &[&str]) {
        let Value::Object(components) = components else {
            panic!("components are a map");
        };
        let mut components: Vec<(String, Value)> = components.into_iter().collect();
        components.sort_unstable_by(|(left, _), (right, _)| left.cmp(right));
        let mut listed: Vec<String> = components
            .iter()
            .map(|(type_path, _)| type_path.clone())
            .chain(unserialized.iter().map(ToString::to_string))
            .collect();
        listed.sort_unstable();
        receive_entity_data(world, RemoteEntityData { components, listed });
    }

    fn names(groups: &[ComponentDetails]) -> Vec<&str> {
        groups.iter().map(|group| group.name.as_str()).collect()
    }

    fn component<'a>(world: &'a World, type_path: &str) -> &'a RemoteComponent {
        world
            .resource::<RemoteDetails>()
            .components
            .iter()
            .find(|component| component.type_path == type_path)
            .unwrap()
    }

    #[test]
    fn the_details_of_a_despawned_proxy_are_cleared() {
        let mut world = details_world();
        apply(
            &mut world,
            alloc::vec![row(remote(1), json!({ NAME: "A" }))],
        );
        let selected = select(&mut world, remote(1));
        fetched(&mut world, json!({ NAME: "A" }), &[]);
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
            alloc::vec![row(target, json!({ NAME: "Player" }))],
        );
        let proxy = select(&mut world, target);
        fetched(
            &mut world,
            json!({
                NAME: "Player",
                Health::type_path(): { "current": 3.5, "owner": "9v0" },
                Team::type_path(): 2,
                "demo::Unknown": { "value": 3 },
                "demo::Other": 1,
            }),
            &[],
        );

        assert!(world.get::<Health>(proxy).is_none());
        assert_eq!(
            component(&world, Health::type_path())
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
        apply(&mut world, alloc::vec![row(target, json!({ NAME: "A" }))]);
        let proxy = select(&mut world, target);
        fetched(
            &mut world,
            json!({ Health::type_path(): { "renamed": 1.0 } }),
            &[],
        );

        let groups = proxy_components(&world, proxy).unwrap();
        assert_eq!(names(&groups), ["Health"]);
        let FieldValue::Label(text) = &groups[0].fields[0].value else {
            panic!("raw components are read-only");
        };
        assert!(text.starts_with(r#"{"renamed":1.0}"#));
    }

    #[test]
    fn long_raw_json_is_cut() {
        let json = Value::Array((0..100_000).map(Value::from).collect());
        let text = raw_value(&json, None);
        assert_eq!(text.chars().count(), RAW_JSON_CHARS + 3);
        assert!(text.starts_with("[0,1,2,"));
        assert!(text.ends_with("..."));
        assert_eq!(raw_value(&json!([1, 2]), None), "[1,2]");
    }

    #[test]
    fn detected_components_are_listed_with_a_note() {
        let mut world = details_world();
        let target = remote(1);
        apply(
            &mut world,
            alloc::vec![row(target, json!({ NAME: "Cube" }))],
        );
        let proxy = select(&mut world, target);
        fetched(
            &mut world,
            json!({ NAME: "Cube" }),
            &[Handled::type_path(), "demo::Internal"],
        );

        let groups = proxy_components(&world, proxy).unwrap();
        assert_eq!(names(&groups), ["Handled", "Name"]);
        assert_eq!(groups[0].id, world.component_id::<Handled>().unwrap());
        assert_eq!(
            groups[0].fields,
            [value_entry("not serializable".to_string())]
        );
    }

    #[test]
    fn unchanged_data_is_not_deserialized_again() {
        let mut world = details_world();
        let target = remote(1);
        apply(&mut world, alloc::vec![row(target, json!({ NAME: "A" }))]);
        select(&mut world, target);
        let data = |team: u8| {
            json!({
                Health::type_path(): { "current": 3.5, "owner": null },
                Team::type_path(): team,
            })
        };
        let value_address = |world: &World, type_path: &str| {
            let value = component(world, type_path).value.as_ref().unwrap();
            core::ptr::from_ref::<dyn PartialReflect>(value.as_ref()).cast::<u8>()
        };
        fetched(&mut world, data(1), &[]);
        let health = value_address(&world, Health::type_path());
        let team = value_address(&world, Team::type_path());

        world.resource_mut::<DetailsPanelSync>().timer.reset();
        fetched(&mut world, data(1), &[]);
        assert_eq!(
            world.resource::<DetailsPanelSync>().timer.elapsed(),
            Duration::ZERO,
            "unchanged data does not refresh the panel"
        );
        assert_eq!(value_address(&world, Team::type_path()), team);

        fetched(&mut world, data(2), &[]);
        assert_ne!(
            world.resource::<DetailsPanelSync>().timer.elapsed(),
            Duration::ZERO
        );
        assert_eq!(
            value_address(&world, Health::type_path()),
            health,
            "unchanged components are reused"
        );
        assert_eq!(
            component(&world, Team::type_path())
                .value
                .as_ref()
                .unwrap()
                .try_downcast_ref::<Team>()
                .map(|team| team.0),
            Some(2)
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
        fetched(&mut world, json!({ NAME: "Cube" }), &[]);
        assert!(world.get::<RemoteLabel>(proxy).is_some());

        let groups = inspect_components(&world, Some(proxy));
        assert_eq!(names(&groups), ["Name"]);
        assert!(world.get::<Name>(proxy).is_none());
    }
}
