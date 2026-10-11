//! The components of the selected remote entity, fetched into the [`RemoteWorld`] for the
//! details panel.
//!
//! The selected entity's components are listed with `world.list_components` and read with
//! `world.get_components` when the selection changes, and then twice a second. They are
//! deserialized off the main thread and written into the [`RemoteWorld`] when they change. The
//! details panel shows the values the remote app sent, notes the components it could not
//! serialize, and shows the JSON of those the inspector cannot insert.
//!
//! `world.list_components` reports component names, which the remote app only knows when built
//! with the `debug` feature, as Bevy is by default.

use alloc::{
    string::{String, ToString},
    vec::Vec,
};
use core::{fmt::Write, time::Duration};

use bevy_ecs::{
    change_detection::Mut, entity::Entity, reflect::AppTypeRegistry, resource::Resource,
    world::World,
};
use bevy_log::warn;
use bevy_reflect::TypeRegistryArc;
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
use serde_json::Value;

use super::{
    source::{spawn, POLL_INTERVAL, REQUEST_TIMEOUT},
    world::{AsideReason, Coverage, PolledComponent, UnregisteredComponents},
    RemoteComponents, RemoteConnectionState, RemoteConnections, RemoteWorlds,
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

/// The components of one remote entity, deserialized off the main thread.
#[derive(Debug, Default)]
pub(crate) struct FetchedEntity {
    /// The serialized components.
    pub(crate) components: Vec<PolledComponent>,
    /// The full type paths of the components the remote app holds but did not serialize.
    pub(crate) unserialized: Vec<String>,
}

struct DetailsCall {
    task: Task<Result<FetchedEntity, BrpClientError>>,
    started: Duration,
}

#[derive(Resource, Default)]
pub struct RemoteEntityFetchs {
    pub main: RemoteEntityFetch,
    pub render: RemoteEntityFetch,
}

/// The fetch of the selected remote entity's components.
#[derive(Default)]
pub struct RemoteEntityFetch {
    entity: Option<Entity>,
    pending: Option<DetailsCall>,
    next_fetch: Duration,
    error: Option<String>,
}

impl RemoteEntityFetch {
    /// The remote entity whose components are fetched.
    pub fn entity(&self) -> Option<Entity> {
        self.entity
    }

    /// Whether a fetch is in flight.
    pub fn is_fetching(&self) -> bool {
        self.pending.is_some()
    }
}

/// Fetches the components of the selected remote entity, and writes them into the
/// [`RemoteWorld`] when they change.
pub fn sync_remote_details(world: &mut World) {
    // CHAIN 4
    _sync_remote_details(world, true);
    _sync_remote_details(world, false);
}

fn _sync_remote_details(world: &mut World, is_main: bool) {
    let rw = world.resource::<RemoteWorlds>();
    let w = if is_main { &rw.main } else { &rw.render };

    let selection = world
        .resource::<InspectorSelection>()
        .0
        .filter(|selection| w.contains(selection.entity));
    let selection_entity = selection.map_or(None, |s| Some(s.entity));

    let fs = world.resource::<RemoteEntityFetchs>();
    let f = if is_main { &fs.main } else { &fs.render };
    if f.entity != selection_entity {
        let mut fetchs = world.resource_mut::<RemoteEntityFetchs>();
        if is_main {
            fetchs.main = RemoteEntityFetch {
                entity: selection_entity,
                ..RemoteEntityFetch::default()
            };
        } else {
            fetchs.render = RemoteEntityFetch {
                entity: selection_entity,
                ..RemoteEntityFetch::default()
            };
        }
    }

    let Some(selection) = selection else {
        return;
    };
    if selection.is_main != is_main {
        return;
    }

    let now = world
        .get_resource::<Time<Real>>()
        .map(Time::elapsed)
        .unwrap_or_default();

    let mut fetchs = world.resource_mut::<RemoteEntityFetchs>();
    let mut fetch = if is_main {
        &mut fetchs.main
    } else {
        &mut fetchs.render
    };
    if let Some(fetched) = finish_fetch(&mut fetch, now) {
        receive_entity(world, selection.entity, fetched, is_main);
    }

    let fetchs = world.resource::<RemoteEntityFetchs>();
    let fetch = if is_main {
        &fetchs.main
    } else {
        &fetchs.render
    };
    if fetch.pending.is_some() || now < fetch.next_fetch {
        return;
    }
    let connections = world.resource::<RemoteConnections>();
    let connection = if is_main {
        &connections.main
    } else {
        &connections.render
    };
    if !matches!(connection.state, RemoteConnectionState::Connected { .. }) {
        return;
    }
    let Some(client) = connection.client().cloned() else {
        return;
    };
    let registry = world.resource::<AppTypeRegistry>().0.clone();
    let mut fetchs = world.resource_mut::<RemoteEntityFetchs>();
    let mut fetch = if is_main {
        &mut fetchs.main
    } else {
        &mut fetchs.render
    };
    fetch.pending = Some(DetailsCall {
        task: spawn(fetch_entity(client, selection.entity, registry)),
        started: now,
    });
}

/// Writes the fetched components of `remote` into the [`RemoteWorld`].
pub(crate) fn receive_entity(
    world: &mut World,
    remote: Entity,
    fetched: FetchedEntity,
    is_main: bool,
) {
    let written = world.resource_scope(|_, mut remote_worlds: Mut<RemoteWorlds>| {
        let remote_world = if is_main {
            &mut remote_worlds.main
        } else {
            &mut remote_worlds.render
        };
        remote_world.register(
            fetched
                .components
                .iter()
                .map(|component| component.type_path.as_str()),
        );
        let written = remote_world.write(
            remote,
            fetched.components,
            &fetched.unserialized,
            Coverage::Entity,
            false,
        );
        if written.tree {
            remote_world.order_children();
        }
        remote_world.take_garbage();
        written
    });
    if written.changed {
        world.resource_mut::<DetailsPanelSync>().set_dirty();
    }
    if written.tree {
        world
            .resource_mut::<crate::entity_tree::EntityTreeSync>()
            .set_dirty();
    }
}

/// Handles the fetch in flight if it was answered or timed out, returning the fetched data.
fn finish_fetch(fetch: &mut RemoteEntityFetch, now: Duration) -> Option<FetchedEntity> {
    let mut call = fetch.pending.take()?;
    let result = match block_on(poll_once(&mut call.task)) {
        Some(result) => result,
        None if now.saturating_sub(call.started) >= REQUEST_TIMEOUT => Err(
            BrpClientError::InvalidResponse("the remote app did not answer in time".into()),
        ),
        None => {
            fetch.pending = Some(call);
            return None;
        }
    };

    fetch.next_fetch = now + DETAILS_INTERVAL;
    match result {
        Ok(fetched) => {
            fetch.error = None;
            Some(fetched)
        }
        Err(error) => {
            let error = error.to_string();
            if fetch.error.as_ref() != Some(&error) {
                warn!("the remote inspector could not read the selected entity: {error}");
                fetch.error = Some(error);
            }
            None
        }
    }
}

async fn fetch_entity(
    client: BrpClient,
    remote: Entity,
    registry: TypeRegistryArc,
) -> Result<FetchedEntity, BrpClientError> {
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
    Ok(decode_entity(
        &registry,
        components.into_iter().collect(),
        listed,
    ))
}

/// Deserializes the `components` of an entity, and lists the `listed` types without a value as
/// unserialized.
pub(crate) fn decode_entity(
    registry: &TypeRegistryArc,
    components: Vec<(String, Value)>,
    listed: Vec<String>,
) -> FetchedEntity {
    let unserialized = listed
        .into_iter()
        .filter(|type_path| !components.iter().any(|(path, _)| path == type_path))
        .collect();
    let registry = registry.read();
    FetchedEntity {
        components: components
            .into_iter()
            .map(|(type_path, json)| PolledComponent::decode(&registry, type_path, json))
            .collect(),
        unserialized,
    }
}

/// Filters and completes the `components` the details panel found on a mirrored entity, using
/// its [`RemoteComponents`] record.
///
/// Only components whose value came from the remote app keep their value. Those it holds without
/// serializing them show a note, and the others, such as required components only added locally,
/// are hidden. Components kept aside for having hooks show their raw JSON with a note. Other
/// components kept aside show their deserialized value, or else their raw JSON.
/// Components that are not registered locally are gathered into one last group.
pub(crate) fn annotate(
    world: &World,
    record: &RemoteComponents,
    components: Vec<ComponentDetails>,
) -> Vec<ComponentDetails> {
    let mut groups: Vec<ComponentDetails> = components
        .into_iter()
        .filter(|component| record.is_serialized(component.id))
        .collect();
    for (id, type_path) in record.unserialized() {
        let Some(id) = *id else {
            continue;
        };
        groups.push(ComponentDetails {
            id,
            name: component_short_name(world, id),
            type_path: type_path.clone(),
            memory: String::new(),
            fields: alloc::vec![value_entry("not serializable".to_string())],
        });
    }
    let registry = world.resource::<AppTypeRegistry>().read();
    let mut unregistered = Vec::new();
    for aside in record.aside() {
        let Some(id) = aside.id else {
            unregistered.push(aside);
            continue;
        };
        let fields = match (&aside.reason, &aside.value) {
            (AsideReason::Failed(error), _) => {
                alloc::vec![value_entry(raw_value(&aside.json, Some(error.as_str())))]
            }
            (AsideReason::Hooked, _) => {
                alloc::vec![value_entry(raw_value(&aside.json, Some(SERVER_DATA)))]
            }
            (_, Some(value)) => field_entries(value.as_partial_reflect(), &registry)
                .into_iter()
                .map(|entry| FieldEntry {
                    value: read_only(entry.value),
                    limit: None,
                    ..entry
                })
                .collect(),
            (_, None) => alloc::vec![value_entry(raw_value(&aside.json, None))],
        };
        groups.push(ComponentDetails {
            id,
            name: component_short_name(world, id),
            type_path: aside.type_path.clone(),
            memory: String::new(),
            fields,
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
    groups
}

/// The note on the raw JSON of components that are not inserted for having hooks.
const SERVER_DATA: &str = "shown from server data";

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
fn raw_value(json: &Value, note: Option<&str>) -> String {
    let mut text = BoundedText::default();
    if write!(text, "{json}").is_err() {
        text.text.push_str("...");
    }
    match note {
        Some(note) => alloc::format!("{} ({note})", text.text),
        None => text.text,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        _InspectorSelection,
        details_panel::inspect_components,
        remote::tests::{apply, mirrored, remote, row, test_world},
    };
    use bevy_ecs::{
        component::Component, lifecycle::HookContext, reflect::ReflectComponent,
        world::DeferredWorld,
    };
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

    #[derive(Component, Reflect, Default)]
    #[reflect(Component, Default)]
    #[require(Shadow)]
    struct Visible(bool);

    #[derive(Component, Reflect, Default)]
    #[reflect(Component, Default)]
    struct Shadow(u8);

    #[derive(Component, Reflect, Default)]
    #[component(on_add = no_op)]
    #[reflect(Component, Default)]
    struct Hooked {
        level: u8,
    }

    fn no_op(_: DeferredWorld, _: HookContext) {}

    fn details_world() -> World {
        let world = test_world();
        {
            let registry = world.resource::<AppTypeRegistry>();
            let mut registry = registry.write();
            registry.register::<Health>();
            registry.register::<Team>();
            registry.register::<Handled>();
            registry.register::<Visible>();
            registry.register::<Shadow>();
            registry.register::<Hooked>();
        }
        world
    }

    /// Mirrors `target` with a name, and selects it.
    fn select(world: &mut World, target: Entity) {
        apply(world, alloc::vec![row(target, json!({ NAME: "Selected" }))]);
        let mut sel = world.resource_mut::<InspectorSelection>();
        sel.0 = Some(_InspectorSelection {
            entity: target,
            is_main: true,
        });
        sync_remote_details(world);
    }

    /// Receives `components` as the data of `target`, listed along with `unserialized`.
    fn fetched(world: &mut World, target: Entity, components: Value, unserialized: &[&str]) {
        let Value::Object(components) = components else {
            panic!("components are a map");
        };
        let components: Vec<(String, Value)> = components.into_iter().collect();
        let listed: Vec<String> = components
            .iter()
            .map(|(type_path, _)| type_path.clone())
            .chain(unserialized.iter().map(ToString::to_string))
            .collect();
        let registry = world.resource::<AppTypeRegistry>().0.clone();
        receive_entity(world, target, decode_entity(&registry, components, listed));
    }

    fn groups(world: &World, target: Entity) -> Vec<ComponentDetails> {
        inspect_components(
            mirrored(world),
            Some(_InspectorSelection {
                entity: target,
                is_main: true,
            }),
        )
    }

    fn names(groups: &[ComponentDetails]) -> Vec<&str> {
        groups.iter().map(|group| group.name.as_str()).collect()
    }

    #[test]
    fn a_despawned_selection_shows_no_components() {
        let mut world = details_world();
        let target = remote(11);
        select(&mut world, target);
        fetched(&mut world, target, json!({ NAME: "A" }), &[]);
        assert_eq!(names(&groups(&world, target)), ["Name"]);

        apply(&mut world, alloc::vec![]);
        sync_remote_details(&mut world);

        assert!(mirrored(&world).get_entity(target).is_err());
        assert!(groups(&world, target).is_empty());
        assert_eq!(world.resource::<RemoteEntityFetchs>().main.entity(), None);
    }

    #[test]
    fn shows_the_values_the_remote_app_sent() {
        let mut world = details_world();
        let target = remote(11);
        select(&mut world, target);
        fetched(
            &mut world,
            target,
            json!({
                NAME: "Player",
                Health::type_path(): { "current": 3.5, "owner": "9v0" },
                Team::type_path(): 2,
                "demo::Unknown": { "value": 3 },
                "demo::Other": 1,
            }),
            &[],
        );

        assert!(world.query::<&Health>().iter(&world).next().is_none());
        let inspected = mirrored(&world);
        assert_eq!(
            inspected
                .get::<Health>(target)
                .and_then(|health| health.owner),
            Some(remote(9))
        );

        let groups = groups(&world, target);
        assert_eq!(names(&groups), ["Health", "Name", "Team", "Unregistered"]);
        assert_eq!(
            groups[0].id,
            inspected.component_id::<Health>().unwrap(),
            "components are keyed by their id in the remote world"
        );
        assert!(!groups[0].memory.is_empty());
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
        let target = remote(11);
        select(&mut world, target);
        fetched(
            &mut world,
            target,
            json!({ Health::type_path(): { "renamed": 1.0 } }),
            &[],
        );

        let groups = groups(&world, target);
        assert_eq!(names(&groups), ["Health"]);
        let FieldValue::Label(text) = &groups[0].fields[0].value else {
            panic!("raw components are read-only");
        };
        assert!(text.starts_with(r#"{"renamed":1.0}"#));
    }

    #[test]
    fn components_with_hooks_show_their_raw_json() {
        let mut world = details_world();
        let target = remote(11);
        select(&mut world, target);
        fetched(
            &mut world,
            target,
            json!({ Hooked::type_path(): { "level": 4 } }),
            &[],
        );

        assert!(!mirrored(&world).entity(target).contains::<Hooked>());
        let groups = groups(&world, target);
        assert_eq!(names(&groups), ["Hooked"]);
        let FieldValue::Label(text) = &groups[0].fields[0].value else {
            panic!("components with hooks are read-only");
        };
        assert_eq!(text, r#"{"level":4} (shown from server data)"#);
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
    fn unserialized_components_are_listed_with_a_note() {
        let mut world = details_world();
        let target = remote(11);
        select(&mut world, target);
        fetched(
            &mut world,
            target,
            json!({ NAME: "Cube" }),
            &[Handled::type_path(), "demo::Internal"],
        );

        let groups = groups(&world, target);
        assert_eq!(names(&groups), ["Handled", "Name"]);
        assert_eq!(
            groups[0].id,
            mirrored(&world).component_id::<Handled>().unwrap()
        );
        assert_eq!(
            groups[0].fields,
            [value_entry("not serializable".to_string())]
        );
        assert!(mirrored(&world).get::<Handled>(target).is_none());
    }

    #[test]
    fn required_defaults_are_not_shown_as_values() {
        let mut world = details_world();
        let target = remote(11);
        select(&mut world, target);
        fetched(
            &mut world,
            target,
            json!({ Visible::type_path(): true }),
            &[],
        );
        assert!(mirrored(&world).get::<Shadow>(target).is_some());
        assert_eq!(names(&groups(&world, target)), ["Visible"]);

        fetched(
            &mut world,
            target,
            json!({ Visible::type_path(): true }),
            &[Shadow::type_path()],
        );
        let groups = groups(&world, target);
        assert_eq!(names(&groups), ["Shadow", "Visible"]);
        assert_eq!(
            groups[0].fields,
            [value_entry("not serializable".to_string())]
        );
    }

    #[test]
    fn unchanged_data_does_not_refresh_the_panel() {
        let mut world = details_world();
        let target = remote(11);
        select(&mut world, target);
        let data = |team: u8| {
            json!({
                Health::type_path(): { "current": 3.5, "owner": null },
                Team::type_path(): team,
            })
        };
        fetched(&mut world, target, data(1), &[]);

        world.resource_mut::<DetailsPanelSync>().timer.reset();
        fetched(&mut world, target, data(1), &[]);
        assert_eq!(
            world.resource::<DetailsPanelSync>().timer.elapsed(),
            Duration::ZERO
        );

        fetched(&mut world, target, data(2), &[]);
        assert_ne!(
            world.resource::<DetailsPanelSync>().timer.elapsed(),
            Duration::ZERO
        );
        assert_eq!(
            mirrored(&world).get::<Team>(target).map(|team| team.0),
            Some(2)
        );
    }
}
