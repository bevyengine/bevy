//! A source inspecting a separate running app over the Bevy Remote Protocol.
//!
//! The remote entities are mirrored into a separate [`World`], the [`RemoteWorld`], at their
//! remote ids. The entity tree and the details panel read that world instead of the local one, so
//! no local hook, observer or system ever sees remote data. See [`world`] for what is mirrored.
//!
//! The entity tree is polled with `world.query` for the components it needs only: [`Name`],
//! [`ChildOf`] and the label-defining components. A full poll reading every component runs when
//! entities the inspector has not seen appear, and at least every 10 seconds. It tells which
//! entities hold a reflected component, since the others, such as observers and systems, are not
//! mirrored. Answers are parsed off the main thread, and only what changed is written.
//!
//! Remote entities with the [`Disabled`] component are not shown, since `world.query` skips them.
//!
//! [`Name`]: bevy_ecs::name::Name
//! [`ChildOf`]: bevy_ecs::hierarchy::ChildOf
//! [`Disabled`]: bevy_ecs::entity_disabling::Disabled

use alloc::{
    string::{String, ToString},
    sync::Arc,
    vec::Vec,
};
use core::{any::TypeId, time::Duration};

use bevy_dev_tools::inspection::label_resolution::LabelResolutionRegistry;
use bevy_ecs::{
    change_detection::Mut,
    entity::Entity,
    reflect::{AppTypeRegistry, ReflectComponent},
    resource::Resource,
    system::{Res, ResMut},
    world::World,
};
use bevy_log::{info, warn};
use bevy_platform::collections::HashSet;
use bevy_reflect::{
    enums::VariantInfo, prelude::ReflectDefault, serde::ReflectSerializeWithRegistry, Reflect,
    ReflectSerialize, TypeInfo, TypeRegistry,
};
use bevy_remote::{
    builtin_methods::{
        BrpAppInfoResponse, BrpQuery, BrpQueryFilter, BrpQueryParams, BrpQueryResponse,
        BrpQueryRow, ComponentSelector, BRP_APP_INFO_METHOD, BRP_QUERY_METHOD,
    },
    client::{BrpClient, BrpClientError},
    http::DEFAULT_PORT,
};
use bevy_tasks::{block_on, poll_once, IoTaskPool, Task, TaskPool};
use bevy_time::{Real, Time};
use serde::Deserialize;
use serde_json::Value;

pub mod world;

use crate::{
    details_panel::{DetailsCollapsed, DetailsColumnSplits, DetailsPanelSync},
    entity_tree::{clear_rows, EntityTreeSync},
    InspectorSelection, InspectorSource,
};
pub use world::{RemoteComponents, RemoteWorld};
use world::{SpawnRemoteError, TreeComponents};

const NAME: &str = "bevy_ecs::name::Name";
const CHILD_OF: &str = "bevy_ecs::hierarchy::ChildOf";
const IS_RESOURCE: &str = "bevy_ecs::resource::IsResource";

/// The shortest time between two `world.query` polls.
const POLL_INTERVAL: Duration = Duration::from_millis(500);
/// The time to wait before reconnecting after a failed request.
const RETRY_INTERVAL: Duration = Duration::from_secs(2);
/// The time after which a request without an answer is dropped and the connection marked failed.
const REQUEST_TIMEOUT: Duration = Duration::from_secs(10);
/// The longest time between two full polls.
///
/// Full polls otherwise only run when unseen entities appear, so this bounds how long a change
/// only a full poll reports, such as an unserializable label-defining component inserted into an
/// existing entity, goes undetected.
const FULL_REFRESH_INTERVAL: Duration = Duration::from_secs(10);
/// The shortest time between two full polls, so that a remote app spawning entities all the time
/// is not fully polled every time.
const FULL_MIN_INTERVAL: Duration = Duration::from_secs(2);

/// The address of the remote app the inspector reads from.
#[derive(Debug, Clone, PartialEq, Eq, Reflect)]
#[reflect(Debug, Default, Clone, PartialEq)]
pub struct RemoteSource {
    /// The host the remote app serves the Bevy Remote Protocol on.
    pub host: String,
    /// The port the remote app serves the Bevy Remote Protocol on.
    pub port: u16,
}

impl Default for RemoteSource {
    fn default() -> Self {
        Self::localhost(DEFAULT_PORT)
    }
}

impl RemoteSource {
    /// A source reading from `host:port`.
    pub fn new(host: impl Into<String>, port: u16) -> Self {
        Self {
            host: host.into(),
            port,
        }
    }

    /// A source reading from `127.0.0.1:port`.
    pub fn localhost(port: u16) -> Self {
        Self::new("127.0.0.1", port)
    }
}

/// The state of the connection to the remote app.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub enum RemoteConnectionState {
    /// The inspector reads from the local world, or has not contacted the remote app yet.
    #[default]
    Disconnected,
    /// An `app.info` request is in flight.
    Connecting,
    /// The remote app answered `app.info` and is polled for its entities.
    Connected {
        /// The name the remote app reports.
        app_name: String,
        /// The Bevy version the remote app reports.
        bevy_version: String,
    },
    /// The last request failed. The connection is retried after a short delay.
    Failed(String),
}

/// The component types a poll asks for.
#[derive(Debug, Default)]
struct PollTypes {
    /// [`NAME`], [`CHILD_OF`] and the serializable label-defining types, read on every poll.
    read: Vec<String>,
    /// The label-defining types that cannot be serialized, asked for with `has` on full polls.
    has: Vec<String>,
}

/// A request the connection is due to send.
#[derive(Debug)]
enum Request {
    Info,
    Query { params: Value, full: bool },
}

/// An answer, parsed off the main thread.
#[derive(Debug)]
enum Reply {
    Info(BrpAppInfoResponse),
    Rows(Vec<PolledRow>),
}

struct PendingCall {
    task: Task<Result<Reply, BrpClientError>>,
    started: Duration,
    full: bool,
}

/// The connection to the remote app.
///
/// At most one poll is in flight at a time, so polls never pile up when the remote app answers
/// slower than the poll interval. Dropping the in-flight task cancels it.
#[derive(Resource, Default)]
pub struct RemoteConnection {
    /// The state of the connection.
    pub state: RemoteConnectionState,
    source: Option<RemoteSource>,
    client: Option<BrpClient>,
    pending: Option<PendingCall>,
    next_poll: Duration,
    types: Arc<PollTypes>,
    next_full: Duration,
    last_full: Duration,
}

impl RemoteConnection {
    /// The client talking to the current source, if the inspector reads from a remote app.
    pub fn client(&self) -> Option<&BrpClient> {
        self.client.as_ref()
    }

    /// The source the connection was set up for.
    pub fn source(&self) -> Option<&RemoteSource> {
        self.source.as_ref()
    }

    /// Whether a poll is in flight.
    pub fn is_polling(&self) -> bool {
        self.pending.is_some()
    }

    /// Whether a poll started at `now` is a full poll.
    fn polls_full(&self, now: Duration) -> bool {
        now >= self.next_full
    }

    /// Marks the connection failed, clearing the remote entities if it was connected so that
    /// stale data is not shown as live.
    fn fail(&mut self, snapshot: &mut RemoteSnapshot, error: String, now: Duration) {
        if !matches!(&self.state, RemoteConnectionState::Failed(previous) if *previous == error) {
            warn!("the remote inspector lost its connection: {error}");
        }
        if matches!(self.state, RemoteConnectionState::Connected { .. }) {
            snapshot.clear();
        }
        self.state = RemoteConnectionState::Failed(error);
        self.next_poll = now + RETRY_INTERVAL;
    }

    /// Handles the request in flight if it was answered or timed out. Returns whether no request
    /// is in flight anymore.
    fn finish_pending(&mut self, snapshot: &mut RemoteSnapshot, now: Duration) -> bool {
        let Some(mut pending) = self.pending.take() else {
            return true;
        };
        let elapsed = now.saturating_sub(pending.started);
        match block_on(poll_once(&mut pending.task)) {
            Some(result) => finish_call(self, snapshot, pending.full, result, now, elapsed),
            None if elapsed >= REQUEST_TIMEOUT => self.fail(
                snapshot,
                "the remote app did not answer in time".to_string(),
                now,
            ),
            None => {
                self.pending = Some(pending);
                return false;
            }
        }
        true
    }

    /// The request to send at `now`, if one is due. A full poll first refreshes the polled types
    /// with `types`, so that types registered late are picked up.
    fn next_request(
        &mut self,
        now: Duration,
        types: impl FnOnce() -> PollTypes,
    ) -> Option<Request> {
        if self.pending.is_some() || self.client.is_none() || now < self.next_poll {
            return None;
        }
        if matches!(self.state, RemoteConnectionState::Connected { .. }) {
            let full = self.polls_full(now);
            if full {
                self.types = Arc::new(types());
            }
            return Some(Request::Query {
                params: query_params(&self.types, full),
                full,
            });
        }
        if self.state == RemoteConnectionState::Disconnected {
            self.state = RemoteConnectionState::Connecting;
        }
        Some(Request::Info)
    }
}

/// One row of a `world.query` answer.
#[derive(Debug)]
struct PolledRow {
    entity: Entity,
    /// The components the [`RemoteWorld`] mirrors.
    components: TreeComponents,
    /// Whether the poll serialized or detected any component of the entity.
    reflected: bool,
}

/// The entities of the remote world, as last reported by `world.query`.
#[derive(Resource, Debug, Default)]
pub struct RemoteSnapshot {
    shown: Vec<Entity>,
    pending: Option<Vec<PolledRow>>,
    known: HashSet<Entity>,
    reflected: HashSet<Entity>,
    reset: bool,
}

impl RemoteSnapshot {
    /// The remote entities the last poll reported and the inspector shows.
    pub fn shown(&self) -> &[Entity] {
        &self.shown
    }

    /// The number of remote entities shown.
    pub fn len(&self) -> usize {
        self.shown.len()
    }

    /// Whether no remote entity is shown.
    pub fn is_empty(&self) -> bool {
        self.shown.is_empty()
    }

    /// Drops every entity, and has the [`RemoteWorld`] rebuilt.
    fn clear(&mut self) {
        *self = Self {
            reset: true,
            ..Self::default()
        };
    }

    /// Replaces the snapshot with `rows`, which come from a full poll if `full` is set.
    ///
    /// A full poll records which entities hold a reflected component. The others, such as
    /// observers and systems, are skipped until a later full poll reports one. Other polls keep
    /// what the last full poll reported, and show the entities it did not see only once they hold
    /// a polled component.
    ///
    /// Returns whether the rows hold entities the last full poll did not see.
    fn set(&mut self, polled: Vec<PolledRow>, full: bool) -> bool {
        if full {
            self.known.clear();
            self.reflected.clear();
        }
        let mut unseen = false;
        let mut shown = Vec::with_capacity(polled.len());
        for polled in polled {
            let entity = polled.entity;
            if full {
                self.known.insert(entity);
                if polled.reflected {
                    self.reflected.insert(entity);
                }
            } else if !self.known.contains(&entity) {
                unseen = true;
            }
            if polled.reflected || self.reflected.contains(&entity) {
                shown.push(polled);
            }
        }
        self.shown = shown.iter().map(|row| row.entity).collect();
        self.pending = Some(shown);
        unseen
    }
}

/// Whether the inspector currently reads from a remote app.
pub(crate) fn is_remote(world: &World) -> bool {
    matches!(
        world.get_resource::<InspectorSource>(),
        Some(InspectorSource::Remote(_))
    )
}

/// Sets the connection up for the current [`InspectorSource`] when it changes, clearing the
/// [`RemoteWorld`] and the selection.
pub fn sync_remote_source(world: &mut World) {
    let source = match world.get_resource::<InspectorSource>() {
        Some(InspectorSource::Remote(source)) => Some(source),
        _ => None,
    };
    if world.resource::<RemoteConnection>().source.as_ref() == source {
        return;
    }
    let source = source.cloned();

    *world.resource_mut::<RemoteSnapshot>() = RemoteSnapshot::default();
    reset_inspected_world(world);

    let mut connection = world.resource_mut::<RemoteConnection>();
    connection.pending = None;
    connection.state = RemoteConnectionState::Disconnected;
    connection.next_poll = Duration::ZERO;
    connection.next_full = Duration::ZERO;
    connection.client = source
        .as_ref()
        .map(|source| BrpClient::new(source.host.clone(), source.port));
    connection.source = source;
}

/// Rebuilds the [`RemoteWorld`] empty, and clears what refers to its entities or component ids:
/// the selection, the tree rows, and the collapsed and column split state of the details panel.
fn reset_inspected_world(world: &mut World) {
    let registry = world
        .get_resource::<AppTypeRegistry>()
        .cloned()
        .unwrap_or_default();
    world.resource_mut::<RemoteWorld>().reset(registry);
    world.resource_mut::<InspectorSelection>().0 = None;
    if let Some(mut collapsed) = world.get_resource_mut::<DetailsCollapsed>() {
        collapsed.0.clear();
    }
    if let Some(mut splits) = world.get_resource_mut::<DetailsColumnSplits>() {
        splits.0.clear();
    }
    clear_rows(world);
    world.resource_mut::<EntityTreeSync>().set_dirty();
    world.resource_mut::<DetailsPanelSync>().set_dirty();
}

/// Polls the request in flight and starts the next one when it is due.
pub fn poll_remote_connection(
    time: Option<Res<Time<Real>>>,
    registry: Option<Res<AppTypeRegistry>>,
    priorities: Option<Res<LabelResolutionRegistry>>,
    mut connection: ResMut<RemoteConnection>,
    mut snapshot: ResMut<RemoteSnapshot>,
) {
    if connection.client.is_none() {
        return;
    }
    let now = time.map(|time| time.elapsed()).unwrap_or_default();
    let connection = &mut *connection;
    if !connection.finish_pending(&mut snapshot, now) {
        return;
    }
    let Some(request) = connection.next_request(now, || match &registry {
        Some(registry) => poll_types(&registry.read(), priorities.as_deref()),
        None => PollTypes::default(),
    }) else {
        return;
    };
    let Some(client) = connection.client.clone() else {
        return;
    };
    let (task, full) = match request {
        Request::Info => (
            spawn(async move {
                let value = client.call(BRP_APP_INFO_METHOD, None).await?;
                Ok(Reply::Info(serde_json::from_value(value)?))
            }),
            false,
        ),
        Request::Query { params, full } => {
            let types = connection.types.clone();
            (
                spawn(async move {
                    let value = client.call(BRP_QUERY_METHOD, Some(params)).await?;
                    Ok(Reply::Rows(parse_rows(value, &types, full)?))
                }),
                full,
            )
        }
    };
    connection.pending = Some(PendingCall {
        task,
        started: now,
        full,
    });
}

fn spawn<T: Send + 'static>(future: impl Future<Output = T> + Send + 'static) -> Task<T> {
    IoTaskPool::get_or_init(TaskPool::default).spawn(future)
}

/// The types each poll asks for, from the label-defining component types registered locally for
/// reflection.
///
/// `world.query` leaves the types that cannot be serialized, such as those holding a `Handle`,
/// out of its results, so they are asked for with `has` instead. Since `has` reports every type for
/// every entity, it is only asked for on full polls.
fn poll_types(registry: &TypeRegistry, priorities: Option<&LabelResolutionRegistry>) -> PollTypes {
    let mut types = PollTypes {
        read: alloc::vec![NAME.to_string(), CHILD_OF.to_string()],
        ..PollTypes::default()
    };
    for registration in registry.iter() {
        if registration.data::<ReflectComponent>().is_none()
            || priorities
                .and_then(|priorities| priorities.get_priority_by_type_id(registration.type_id()))
                .is_none()
        {
            continue;
        }
        let path = registration.type_info().type_path().to_string();
        if serializable(
            registry,
            Some(registration.type_info()),
            &mut HashSet::new(),
        ) {
            types.read.push(path);
        } else {
            types.has.push(path);
        }
    }
    types.read.sort_unstable();
    types.has.sort_unstable();
    types
}

/// Whether values of the type described by `info` can be serialized through reflection.
///
/// Types the walk cannot see into, such as fields without type information, count as not
/// serializable.
fn serializable(
    registry: &TypeRegistry,
    info: Option<&TypeInfo>,
    visiting: &mut HashSet<TypeId>,
) -> bool {
    let Some(info) = info else {
        return false;
    };
    let type_id = info.type_id();
    if registry
        .get_type_data::<ReflectSerialize>(type_id)
        .is_some()
        || registry
            .get_type_data::<ReflectSerializeWithRegistry>(type_id)
            .is_some()
        || !visiting.insert(type_id)
    {
        return true;
    }
    let result = match info {
        TypeInfo::Struct(info) => info
            .iter()
            .all(|field| serializable(registry, field.type_info(), visiting)),
        TypeInfo::TupleStruct(info) => info
            .iter()
            .all(|field| serializable(registry, field.type_info(), visiting)),
        TypeInfo::Tuple(info) => info
            .iter()
            .all(|field| serializable(registry, field.type_info(), visiting)),
        TypeInfo::List(info) => serializable(registry, info.item_info(), visiting),
        TypeInfo::Array(info) => serializable(registry, info.item_info(), visiting),
        TypeInfo::Map(info) => {
            serializable(registry, info.key_info(), visiting)
                && serializable(registry, info.value_info(), visiting)
        }
        TypeInfo::Set(info) => serializable(
            registry,
            registry.get_type_info(info.value_ty().id()),
            visiting,
        ),
        TypeInfo::Enum(info) => info.iter().all(|variant| match variant {
            VariantInfo::Struct(variant) => variant
                .iter()
                .all(|field| serializable(registry, field.type_info(), visiting)),
            VariantInfo::Tuple(variant) => variant
                .iter()
                .all(|field| serializable(registry, field.type_info(), visiting)),
            VariantInfo::Unit(_) => true,
        }),
        TypeInfo::Opaque(_) => false,
    };
    visiting.remove(&type_id);
    result
}

/// The `world.query` parameters of a poll. A full poll reads every component and asks for the
/// unserializable label-defining types with `has`; other polls read only [`PollTypes::read`].
fn query_params(types: &PollTypes, full: bool) -> Value {
    let params = BrpQueryParams {
        data: BrpQuery {
            components: Vec::new(),
            option: if full {
                ComponentSelector::All
            } else {
                ComponentSelector::Paths(types.read.clone())
            },
            has: if full { types.has.clone() } else { Vec::new() },
        },
        filter: BrpQueryFilter {
            without: alloc::vec![IS_RESOURCE.to_string()],
            with: Vec::new(),
        },
        strict: false,
    };
    serde_json::to_value(params).unwrap_or_default()
}

/// Parses a `world.query` answer, keeping the components the [`RemoteWorld`] mirrors.
fn parse_rows(
    value: Value,
    types: &PollTypes,
    full: bool,
) -> Result<Vec<PolledRow>, serde_json::Error> {
    let rows: BrpQueryResponse = serde_json::from_value(value)?;
    Ok(rows
        .into_iter()
        .filter_map(|row| polled_row(row, types, full))
        .collect())
}

fn polled_row(row: BrpQueryRow, types: &PollTypes, full: bool) -> Option<PolledRow> {
    let BrpQueryRow {
        entity,
        components,
        has,
    } = row;
    if components.contains_key(IS_RESOURCE) {
        return None;
    }
    let mut detected: Vec<String> = has
        .into_iter()
        .filter(|(_, present)| *present == Value::Bool(true))
        .map(|(path, _)| path)
        .collect();
    detected.sort_unstable();
    let reflected = !components.is_empty() || !detected.is_empty();
    let mut labels: Vec<String> = components
        .keys()
        .filter(|path| *path != NAME && *path != CHILD_OF && types.read.binary_search(path).is_ok())
        .cloned()
        .collect();
    labels.sort_unstable();
    Some(PolledRow {
        entity,
        components: TreeComponents {
            name: components
                .get(NAME)
                .and_then(Value::as_str)
                .map(ToString::to_string),
            parent: components
                .get(CHILD_OF)
                .and_then(|parent| Entity::deserialize(parent).ok()),
            labels,
            detected: full.then_some(detected),
        },
        reflected,
    })
}

fn finish_call(
    connection: &mut RemoteConnection,
    snapshot: &mut RemoteSnapshot,
    full: bool,
    result: Result<Reply, BrpClientError>,
    now: Duration,
    elapsed: Duration,
) {
    match result {
        Err(error) => connection.fail(snapshot, error.to_string(), now),
        Ok(Reply::Info(info)) => {
            info!(
                "the remote inspector connected to {} ({})",
                info.app_name, info.bevy_version
            );
            snapshot.clear();
            connection.state = RemoteConnectionState::Connected {
                app_name: info.app_name,
                bevy_version: info.bevy_version,
            };
            connection.next_poll = now;
            connection.next_full = now;
        }
        Ok(Reply::Rows(rows)) => {
            let unseen = snapshot.set(rows, full);
            if full {
                connection.last_full = now;
                connection.next_full = now + FULL_REFRESH_INTERVAL;
            } else if unseen {
                connection.next_full = connection
                    .next_full
                    .min(connection.last_full + FULL_MIN_INTERVAL);
            }
            connection.next_poll = now + POLL_INTERVAL.max(elapsed * 2);
        }
    }
}

/// Writes the latest poll into the [`RemoteWorld`]: despawns the entities it no longer shows,
/// spawns the new ones, and writes the components that changed.
///
/// Rebuilds the [`RemoteWorld`] first if the connection was reset. If a remote id has an earlier
/// generation than the one mirrored, the remote app restarted: the world is rebuilt, the poll
/// written again, and a full poll requested.
pub fn sync_remote_world(world: &mut World) {
    if !is_remote(world) {
        return;
    }
    let mut snapshot = world.resource_mut::<RemoteSnapshot>();
    let reset = core::mem::take(&mut snapshot.reset);
    let pending = snapshot.pending.take();
    if reset {
        reset_inspected_world(world);
    }
    let Some(pending) = pending else {
        return;
    };
    let selection = world.resource::<InspectorSelection>().0;
    let written = world.resource_scope(|_, mut remote: Mut<RemoteWorld>| {
        apply_rows(&mut remote, pending, selection)
    });
    let written = match written {
        Ok(written) => written,
        Err(pending) => {
            warn!("the remote app restarted, rebuilding the remote inspector's copy of it");
            reset_inspected_world(world);
            world.resource_mut::<RemoteConnection>().next_full = Duration::ZERO;
            world
                .resource_scope(|_, mut remote: Mut<RemoteWorld>| {
                    apply_rows(&mut remote, pending, None)
                })
                .unwrap_or_default()
        }
    };
    if written.tree {
        world.resource_mut::<EntityTreeSync>().set_dirty();
    }
    if written.selection {
        world.resource_mut::<DetailsPanelSync>().set_dirty();
    }
}

/// What [`apply_rows`] changed.
#[derive(Debug, Default, Clone, Copy)]
struct Applied {
    /// Whether the entity tree may need a sync.
    tree: bool,
    /// Whether the selected entity changed.
    selection: bool,
}

/// Writes `rows` into `remote`. Hands `rows` back if the remote app restarted.
fn apply_rows(
    remote: &mut RemoteWorld,
    rows: Vec<PolledRow>,
    selection: Option<Entity>,
) -> Result<Applied, Vec<PolledRow>> {
    let mut applied = Applied::default();
    let shown: HashSet<Entity> = rows.iter().map(|row| row.entity).collect();
    for entity in remote.entities() {
        if !shown.contains(&entity) {
            remote.despawn(entity);
            applied.tree = true;
            applied.selection |= selection == Some(entity);
        }
    }

    let mut writable = Vec::with_capacity(rows.len());
    for (index, row) in rows.iter().enumerate() {
        match remote.spawn(row.entity) {
            Ok(spawned) => {
                applied.tree |= spawned;
                writable.push(index);
            }
            Err(SpawnRemoteError::Behind) => return Err(rows),
            Err(SpawnRemoteError::Taken) => {
                bevy_log::warn_once!(
                    "the remote inspector cannot mirror {}, its id is taken by a resource",
                    row.entity
                );
            }
            Err(SpawnRemoteError::Ahead) => {}
        }
    }

    let mut writable = writable.into_iter().peekable();
    for (index, row) in rows.into_iter().enumerate() {
        if writable.next_if_eq(&index).is_none() {
            continue;
        }
        let changed = remote.write(row.entity, row.components);
        applied.tree |= changed;
        applied.selection |= changed && selection == Some(row.entity);
    }
    Ok(applied)
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::details_panel::{DetailsCollapsed, DetailsColumnSplits};
    use bevy_dev_tools::inspection::label_resolution::LabelDefinitionPriority;
    use bevy_ecs::{
        component::{Component, ComponentId},
        hierarchy::{ChildOf, Children},
        name::Name,
    };
    use bevy_reflect::{TypePath, TypeRegistryArc};
    use serde_json::json;

    pub(crate) fn test_world() -> World {
        let mut world = World::new();
        let registry = AppTypeRegistry(TypeRegistryArc::default());
        {
            let mut registry = registry.write();
            registry.register::<Name>();
            registry.register::<ChildOf>();
            registry.register::<Children>();
        }
        world.insert_resource(registry);
        world.insert_resource(InspectorSource::Remote(RemoteSource::localhost(
            DEFAULT_PORT,
        )));
        world.init_resource::<InspectorSelection>();
        world.init_resource::<RemoteConnection>();
        world.init_resource::<RemoteSnapshot>();
        world.init_resource::<RemoteWorld>();
        world.init_resource::<EntityTreeSync>();
        world.init_resource::<DetailsPanelSync>();
        world.init_resource::<DetailsCollapsed>();
        world.init_resource::<DetailsColumnSplits>();
        world
    }

    pub(crate) fn remote(index: u32) -> Entity {
        Entity::from_raw_u32(index).unwrap()
    }

    pub(crate) fn row(entity: Entity, components: Value) -> Value {
        json!({ "entity": entity, "components": components })
    }

    fn world_types(world: &World) -> PollTypes {
        poll_types(
            &world.resource::<AppTypeRegistry>().read(),
            world.get_resource::<LabelResolutionRegistry>(),
        )
    }

    fn parse(world: &World, rows: Value, full: bool) -> Vec<PolledRow> {
        parse_rows(rows, &world_types(world), full).unwrap()
    }

    fn poll(world: &mut World, rows: Vec<Value>, full: bool) {
        let rows = parse(world, Value::Array(rows), full);
        world.resource_mut::<RemoteSnapshot>().set(rows, full);
        sync_remote_world(world);
    }

    pub(crate) fn apply(world: &mut World, rows: Vec<Value>) {
        poll(world, rows, true);
    }

    pub(crate) fn mirrored(world: &World) -> &World {
        world.resource::<RemoteWorld>().world()
    }

    fn label(world: &World, entity: Entity) -> String {
        crate::entity_tree::entity_label(
            mirrored(world),
            world.get_resource::<LabelResolutionRegistry>(),
            entity,
        )
    }

    fn rows(rows: Value) -> Result<Reply, BrpClientError> {
        Ok(Reply::Rows(
            parse_rows(rows, &PollTypes::default(), false).unwrap(),
        ))
    }

    fn full_rows(rows: Value) -> Result<Reply, BrpClientError> {
        Ok(Reply::Rows(
            parse_rows(rows, &PollTypes::default(), true).unwrap(),
        ))
    }

    fn info() -> Result<Reply, BrpClientError> {
        Ok(Reply::Info(
            serde_json::from_value(
                json!({ "app_name": "demo", "bevy_version": "0.20.0-dev", "sub_app": "main" }),
            )
            .unwrap(),
        ))
    }

    fn connected() -> RemoteConnection {
        RemoteConnection {
            state: RemoteConnectionState::Connected {
                app_name: "demo".to_string(),
                bevy_version: "0.20.0-dev".to_string(),
            },
            client: Some(BrpClient::localhost(1)),
            ..RemoteConnection::default()
        }
    }

    #[test]
    fn entities_are_serialized_in_display_form() {
        let row = row(remote(5), json!({ CHILD_OF: remote(2) }));
        assert_eq!(row["entity"], json!("5v0"));
        assert_eq!(row["components"][CHILD_OF], json!("2v0"));
    }

    #[test]
    fn mirrors_remote_entities_at_their_ids() {
        let mut world = test_world();
        let parent = remote(20);
        let child = remote(21);
        apply(
            &mut world,
            alloc::vec![
                row(parent, json!({ NAME: "Parent" })),
                row(child, json!({ NAME: "Child", CHILD_OF: parent })),
            ],
        );

        let inspected = mirrored(&world);
        assert_eq!(
            inspected.get::<Name>(parent).map(Name::as_str),
            Some("Parent")
        );
        assert_eq!(
            inspected.get::<ChildOf>(child).map(ChildOf::parent),
            Some(parent)
        );
        assert_eq!(
            inspected
                .get::<Children>(parent)
                .map(|children| children.to_vec()),
            Some(alloc::vec![child])
        );
        assert!(world.get_entity(parent).is_err());
        assert_eq!(label(&world, child), "Child");
    }

    #[test]
    fn labels_unnamed_entities_with_their_remote_id() {
        let mut world = test_world();
        apply(
            &mut world,
            alloc::vec![row(remote(17), json!({ "demo::Unknown": 1 }))],
        );
        assert_eq!(label(&world, remote(17)), "17v0");
    }

    #[derive(Component, Reflect)]
    #[reflect(Component)]
    struct Labelled;

    #[derive(Component, Reflect)]
    struct Unreflected;

    fn label_world() -> World {
        let mut world = test_world();
        {
            let registry = world.resource::<AppTypeRegistry>();
            let mut registry = registry.write();
            registry.register::<Labelled>();
            registry.register::<Unreflected>();
        }
        let mut priorities = LabelResolutionRegistry::new();
        priorities.register_label_defining_type::<Labelled>(LabelDefinitionPriority::LIBRARY);
        priorities.register_label_defining_type::<Unreflected>(LabelDefinitionPriority::LIBRARY);
        world.insert_resource(priorities);
        world
    }

    #[derive(Reflect, Clone)]
    #[reflect(opaque)]
    struct Opaque;

    #[derive(Component, Reflect)]
    #[reflect(Component)]
    struct Holder {
        inner: Option<Opaque>,
    }

    #[derive(Component, Reflect)]
    #[reflect(Component)]
    struct Plain {
        value: f32,
        names: Vec<String>,
    }

    fn register_holder_and_plain(world: &mut World) {
        {
            let registry = world.resource::<AppTypeRegistry>();
            let mut registry = registry.write();
            registry.register::<Holder>();
            registry.register::<Plain>();
        }
        let mut priorities = world.resource_mut::<LabelResolutionRegistry>();
        priorities.register_label_defining_type::<Holder>(LabelDefinitionPriority::LIBRARY);
        priorities.register_label_defining_type::<Plain>(LabelDefinitionPriority::LIBRARY);
    }

    #[test]
    fn polls_read_serializable_label_types_and_ask_for_the_others() {
        let mut world = label_world();
        register_holder_and_plain(&mut world);
        let types = world_types(&world);
        assert_eq!(
            types.read,
            [
                CHILD_OF.to_string(),
                NAME.to_string(),
                Labelled::type_path().to_string(),
                Plain::type_path().to_string(),
            ]
        );
        assert_eq!(types.has, [Holder::type_path().to_string()]);
    }

    #[test]
    fn tree_polls_read_only_the_tree_components() {
        let world = label_world();
        let types = world_types(&world);

        let params: BrpQueryParams = serde_json::from_value(query_params(&types, false)).unwrap();
        assert_eq!(params.data.components, Vec::<String>::new());
        assert_eq!(
            params.data.option,
            ComponentSelector::Paths(types.read.clone())
        );
        assert!(params.data.has.is_empty());
        assert!(!params.strict);
        assert_eq!(params.filter.without, [IS_RESOURCE.to_string()]);

        let params: BrpQueryParams = serde_json::from_value(query_params(&types, true)).unwrap();
        assert_eq!(params.data.option, ComponentSelector::All);
        assert_eq!(params.data.has, types.has);
    }

    #[test]
    fn rows_keep_only_the_tree_components() {
        let world = label_world();
        let rows = parse(
            &world,
            json!([row(
                remote(20),
                json!({
                    NAME: "Cube",
                    CHILD_OF: remote(3),
                    Labelled::type_path(): {},
                    "demo::Transform": { "x": 1.0 },
                }),
            )]),
            false,
        );
        assert_eq!(
            rows[0].components,
            TreeComponents {
                name: Some("Cube".to_string()),
                parent: Some(remote(3)),
                labels: alloc::vec![Labelled::type_path().to_string()],
                detected: None,
            }
        );
    }

    #[test]
    fn unseen_entities_without_tree_components_wait_for_a_full_poll() {
        let mut world = label_world();
        poll(
            &mut world,
            alloc::vec![
                row(remote(11), json!({ NAME: "A", "demo::Transform": 1 })),
                row(remote(12), json!({ "demo::Transform": 1 })),
                row(remote(13), json!({})),
            ],
            true,
        );
        assert_eq!(world.resource_mut::<RemoteWorld>().entities().len(), 2);

        let unseen = parse(
            &world,
            json!([
                row(remote(11), json!({ NAME: "A" })),
                row(remote(12), json!({})),
                row(remote(13), json!({})),
                row(remote(14), json!({})),
                row(remote(15), json!({ NAME: "E" })),
            ]),
            false,
        );
        assert!(world.resource_mut::<RemoteSnapshot>().set(unseen, false));
        sync_remote_world(&mut world);
        let remote_world = world.resource::<RemoteWorld>();
        assert!(remote_world.contains(remote(11)));
        assert!(
            remote_world.contains(remote(12)),
            "known reflected entities stay"
        );
        assert!(
            !remote_world.contains(remote(13)),
            "known internal entities stay hidden"
        );
        assert!(
            !remote_world.contains(remote(14)),
            "unseen entities wait for a full poll"
        );
        assert!(
            remote_world.contains(remote(15)),
            "named entities are shown at once"
        );

        poll(
            &mut world,
            alloc::vec![
                row(remote(11), json!({ NAME: "A" })),
                row(remote(12), json!({ "demo::Transform": 1 })),
                row(remote(13), json!({})),
                row(remote(14), json!({ "demo::Transform": 1 })),
                row(remote(15), json!({ NAME: "E" })),
            ],
            true,
        );
        assert_eq!(world.resource_mut::<RemoteWorld>().entities().len(), 4);
        assert!(world.resource::<RemoteWorld>().contains(remote(14)));
    }

    #[test]
    fn labels_entities_with_components_only_reported_by_has() {
        let mut world = label_world();
        let path = Labelled::type_path();
        apply(
            &mut world,
            alloc::vec![json!({
                "entity": remote(13),
                "components": {},
                "has": { path: true },
            })],
        );
        assert_eq!(label(&world, remote(13)), "Labelled");
        assert!(mirrored(&world).get::<Labelled>(remote(13)).is_none());

        poll(&mut world, alloc::vec![row(remote(13), json!({}))], false);
        assert_eq!(
            label(&world, remote(13)),
            "Labelled",
            "detected labels are kept until the next full poll"
        );
    }

    #[test]
    fn skips_entities_without_a_reflected_or_detected_component() {
        let mut world = label_world();
        let path = Labelled::type_path();
        apply(
            &mut world,
            alloc::vec![
                json!({ "entity": remote(11), "components": {}, "has": { path: false } }),
                row(remote(12), json!({ IS_RESOURCE: {}, NAME: "Resource" })),
                row(remote(13), json!({ NAME: "Shown" })),
            ],
        );
        let entities = world.resource_mut::<RemoteWorld>().entities();
        assert_eq!(entities, [remote(13)]);
    }

    #[test]
    fn an_unchanged_poll_does_not_dirty_the_tree() {
        let mut world = test_world();
        let rows = || {
            alloc::vec![
                row(remote(11), json!({ NAME: "A" })),
                row(remote(12), json!({ NAME: "B", CHILD_OF: remote(11) })),
            ]
        };
        poll(&mut world, rows(), true);
        world.resource_mut::<EntityTreeSync>().timer.reset();

        poll(&mut world, rows(), false);
        assert_eq!(
            world.resource::<EntityTreeSync>().timer.elapsed(),
            Duration::ZERO
        );

        poll(
            &mut world,
            alloc::vec![
                row(remote(11), json!({ NAME: "A" })),
                row(remote(12), json!({ NAME: "Renamed", CHILD_OF: remote(11) })),
            ],
            false,
        );
        assert_ne!(
            world.resource::<EntityTreeSync>().timer.elapsed(),
            Duration::ZERO
        );
        assert_eq!(label(&world, remote(12)), "Renamed");
    }

    #[test]
    fn follows_remote_despawns_and_reparenting() {
        let mut world = test_world();
        let a = remote(11);
        let b = remote(12);
        let c = remote(13);
        apply(
            &mut world,
            alloc::vec![
                row(a, json!({ NAME: "A" })),
                row(b, json!({ NAME: "B", CHILD_OF: a })),
                row(c, json!({ NAME: "C", CHILD_OF: b })),
            ],
        );

        apply(
            &mut world,
            alloc::vec![
                row(a, json!({ NAME: "A" })),
                row(c, json!({ NAME: "C", CHILD_OF: a })),
            ],
        );

        let inspected = mirrored(&world);
        assert!(inspected.get_entity(b).is_err());
        assert!(inspected.get_entity(c).is_ok());
        assert_eq!(inspected.get::<ChildOf>(c).map(ChildOf::parent), Some(a));
        assert_eq!(world.resource_mut::<RemoteWorld>().entities().len(), 2);

        apply(&mut world, alloc::vec![row(c, json!({ NAME: "C" }))]);
        let inspected = mirrored(&world);
        assert!(inspected.get_entity(a).is_err());
        assert!(inspected.get::<ChildOf>(c).is_none());
    }

    #[test]
    fn a_remote_selection_shows_its_mirrored_components() {
        let mut world = test_world();
        let parent = remote(11);
        let child = remote(12);
        apply(
            &mut world,
            alloc::vec![
                row(parent, json!({ NAME: "A", "demo::Transform": 1 })),
                row(child, json!({ CHILD_OF: parent })),
            ],
        );
        let names = |entity| -> Vec<String> {
            crate::details_panel::inspect_components(mirrored(&world), Some(entity))
                .into_iter()
                .map(|component| component.name)
                .collect()
        };
        assert_eq!(names(parent), ["Children", "Name"]);
        assert_eq!(names(child), ["ChildOf"]);
    }

    #[test]
    fn remote_ids_never_touch_local_entities() {
        let mut world = test_world();
        let local = world.spawn(Name::new("Local")).id();
        let id = Entity::from_index_and_generation(remote(30).index(), local.generation());
        apply(&mut world, alloc::vec![row(id, json!({ NAME: "Remote" }))]);
        assert_eq!(world.get::<Name>(local).map(Name::as_str), Some("Local"));
        assert_eq!(label(&world, id), "Remote");
    }

    #[test]
    fn a_new_generation_replaces_the_mirrored_entity() {
        let mut world = test_world();
        let old = remote(14);
        let new =
            Entity::from_index_and_generation(old.index(), old.generation().after_versions(3));
        apply(&mut world, alloc::vec![row(old, json!({ NAME: "Old" }))]);
        apply(&mut world, alloc::vec![row(new, json!({ NAME: "New" }))]);

        let inspected = mirrored(&world);
        assert!(inspected.get_entity(old).is_err());
        assert_eq!(inspected.get::<Name>(new).map(Name::as_str), Some("New"));
    }

    #[test]
    fn an_earlier_generation_rebuilds_the_world() {
        let mut world = test_world();
        world.insert_resource(connected());
        let late = Entity::from_index_and_generation(
            remote(14).index(),
            remote(14).generation().after_versions(3),
        );
        apply(&mut world, alloc::vec![row(late, json!({ NAME: "Old" }))]);
        world.resource_mut::<RemoteConnection>().next_full = Duration::from_secs(60);
        world.resource_mut::<InspectorSelection>().0 = Some(late);

        apply(
            &mut world,
            alloc::vec![row(remote(14), json!({ NAME: "Restarted" }))],
        );

        assert_eq!(
            mirrored(&world).get::<Name>(remote(14)).map(Name::as_str),
            Some("Restarted")
        );
        assert_eq!(world.resource::<InspectorSelection>().0, None);
        assert_eq!(
            world.resource::<RemoteConnection>().next_full,
            Duration::ZERO
        );
    }

    #[test]
    fn switching_to_local_resets_the_remote_world() {
        let mut world = test_world();
        sync_remote_source(&mut world);
        apply(
            &mut world,
            alloc::vec![row(remote(11), json!({ NAME: "A" }))],
        );
        world.resource_mut::<InspectorSelection>().0 = Some(remote(11));
        let id = ComponentId::new(3);
        world.resource_mut::<DetailsCollapsed>().0.insert(id);
        world
            .resource_mut::<DetailsColumnSplits>()
            .0
            .insert(id, 0.3);

        world.insert_resource(InspectorSource::Local);
        sync_remote_source(&mut world);

        assert!(world.resource_mut::<RemoteWorld>().entities().is_empty());
        assert!(world.resource::<DetailsCollapsed>().0.is_empty());
        assert!(world.resource::<DetailsColumnSplits>().0.is_empty());
        assert_eq!(world.resource::<InspectorSelection>().0, None);
        assert!(world.resource::<RemoteConnection>().client().is_none());
    }

    #[test]
    fn every_source_change_clears_the_selection() {
        let mut world = test_world();
        world.insert_resource(InspectorSource::Local);
        let local = world.spawn_empty().id();
        world.resource_mut::<InspectorSelection>().0 = Some(local);

        world.insert_resource(InspectorSource::Remote(RemoteSource::localhost(1)));
        sync_remote_source(&mut world);
        assert_eq!(world.resource::<InspectorSelection>().0, None);

        world.resource_mut::<InspectorSelection>().0 = Some(local);
        sync_remote_source(&mut world);
        assert_eq!(
            world.resource::<InspectorSelection>().0,
            Some(local),
            "an unchanged source keeps the selection"
        );

        world.insert_resource(InspectorSource::Remote(RemoteSource::localhost(2)));
        sync_remote_source(&mut world);
        assert_eq!(world.resource::<InspectorSelection>().0, None);

        world.resource_mut::<InspectorSelection>().0 = Some(local);
        world.insert_resource(InspectorSource::Local);
        sync_remote_source(&mut world);
        assert_eq!(world.resource::<InspectorSelection>().0, None);
    }

    #[test]
    fn failed_requests_mark_the_connection_failed_and_retry_later() {
        let mut connection = RemoteConnection::default();
        let mut snapshot = RemoteSnapshot::default();
        let now = Duration::from_secs(3);
        finish_call(
            &mut connection,
            &mut snapshot,
            false,
            Err(BrpClientError::InvalidResponse("refused".to_string())),
            now,
            Duration::ZERO,
        );
        assert!(matches!(connection.state, RemoteConnectionState::Failed(_)));
        assert_eq!(connection.next_poll, now + RETRY_INTERVAL);

        finish_call(
            &mut connection,
            &mut snapshot,
            false,
            info(),
            now,
            Duration::ZERO,
        );
        assert_eq!(
            connection.state,
            RemoteConnectionState::Connected {
                app_name: "demo".to_string(),
                bevy_version: "0.20.0-dev".to_string(),
            }
        );
    }

    #[test]
    fn losing_the_connection_clears_the_remote_entities() {
        let mut world = test_world();
        world.insert_resource(connected());
        apply(
            &mut world,
            alloc::vec![row(remote(11), json!({ NAME: "A" }))],
        );
        world.resource_mut::<InspectorSelection>().0 = Some(remote(11));

        world.resource_scope(|world, mut connection: Mut<RemoteConnection>| {
            finish_call(
                &mut connection,
                &mut world.resource_mut::<RemoteSnapshot>(),
                false,
                Err(BrpClientError::InvalidResponse("refused".to_string())),
                Duration::from_secs(1),
                Duration::ZERO,
            );
        });
        sync_remote_world(&mut world);

        assert!(world.resource::<RemoteSnapshot>().is_empty());
        assert!(world.resource_mut::<RemoteWorld>().entities().is_empty());
        assert_eq!(world.resource::<InspectorSelection>().0, None);
    }

    #[test]
    fn reconnecting_rebuilds_the_remote_world() {
        let mut world = test_world();
        apply(
            &mut world,
            alloc::vec![row(remote(11), json!({ NAME: "A" }))],
        );
        world.resource_scope(|world, mut connection: Mut<RemoteConnection>| {
            finish_call(
                &mut connection,
                &mut world.resource_mut::<RemoteSnapshot>(),
                false,
                info(),
                Duration::from_secs(1),
                Duration::ZERO,
            );
        });
        sync_remote_world(&mut world);
        assert!(world.resource_mut::<RemoteWorld>().entities().is_empty());
    }

    #[test]
    fn an_unanswered_request_times_out() {
        let mut connection = connected();
        let mut snapshot = RemoteSnapshot::default();
        snapshot.set(
            parse_rows(
                json!([row(remote(11), json!({ NAME: "A" }))]),
                &PollTypes::default(),
                true,
            )
            .unwrap(),
            true,
        );
        let started = Duration::from_secs(1);
        connection.pending = Some(PendingCall {
            task: spawn(core::future::pending()),
            started,
            full: false,
        });

        let almost = started + REQUEST_TIMEOUT - Duration::from_millis(1);
        assert!(!connection.finish_pending(&mut snapshot, almost));
        assert!(connection.is_polling());
        assert!(connection
            .next_request(almost, PollTypes::default)
            .is_none());

        let now = started + REQUEST_TIMEOUT;
        assert!(connection.finish_pending(&mut snapshot, now));
        assert!(!connection.is_polling());
        assert_eq!(
            connection.state,
            RemoteConnectionState::Failed("the remote app did not answer in time".to_string())
        );
        assert_eq!(connection.next_poll, now + RETRY_INTERVAL);
        assert!(snapshot.is_empty());
        assert!(connection.next_request(now, PollTypes::default).is_none());
        assert!(matches!(
            connection.next_request(now + RETRY_INTERVAL, PollTypes::default),
            Some(Request::Info)
        ));
    }

    #[test]
    fn full_polls_pick_up_label_types_registered_late() {
        let mut world = label_world();
        let mut connection = connected();
        let now = Duration::from_secs(1);
        assert!(matches!(
            connection.next_request(now, || world_types(&world)),
            Some(Request::Query { full: true, .. })
        ));
        finish_call(
            &mut connection,
            &mut RemoteSnapshot::default(),
            true,
            full_rows(json!([])),
            now,
            Duration::ZERO,
        );
        register_holder_and_plain(&mut world);

        let Some(Request::Query { params, full }) =
            connection.next_request(now + POLL_INTERVAL, || world_types(&world))
        else {
            panic!("a poll is due");
        };
        assert!(!full);
        assert!(!params.to_string().contains(Plain::type_path()));

        let Some(Request::Query { params, full }) =
            connection.next_request(now + FULL_REFRESH_INTERVAL, || world_types(&world))
        else {
            panic!("a poll is due");
        };
        assert!(full);
        assert!(params.to_string().contains(Holder::type_path()));
        assert!(connection
            .types
            .read
            .contains(&Plain::type_path().to_string()));
    }

    fn query(
        connection: &mut RemoteConnection,
        snapshot: &mut RemoteSnapshot,
        reply: Value,
        now: Duration,
    ) {
        let full = connection.polls_full(now);
        let rows = if full { full_rows(reply) } else { rows(reply) };
        finish_call(connection, snapshot, full, rows, now, Duration::ZERO);
    }

    #[test]
    fn polls_fully_only_when_unseen_entities_appear() {
        let mut connection = RemoteConnection::default();
        let mut snapshot = RemoteSnapshot::default();
        let path = "demo::Handled";
        let start = Duration::from_secs(1);
        let mut now = start;
        assert!(connection.polls_full(now));
        query(
            &mut connection,
            &mut snapshot,
            json!([{ "entity": remote(11), "components": {}, "has": { path: true } }]),
            now,
        );

        now += POLL_INTERVAL;
        assert!(!connection.polls_full(now));
        query(
            &mut connection,
            &mut snapshot,
            json!([row(remote(11), json!({}))]),
            now,
        );
        assert_eq!(snapshot.shown(), [remote(11)]);

        now += POLL_INTERVAL;
        assert!(!connection.polls_full(now));
        query(
            &mut connection,
            &mut snapshot,
            json!([
                row(remote(11), json!({})),
                row(remote(12), json!({ NAME: "New" }))
            ]),
            now,
        );
        assert_eq!(snapshot.shown(), [remote(11), remote(12)]);

        assert!(!connection.polls_full(now + POLL_INTERVAL));
        now = start + FULL_MIN_INTERVAL;
        assert!(connection.polls_full(now));
        query(
            &mut connection,
            &mut snapshot,
            json!([
                { "entity": remote(11), "components": {}, "has": { path: true } },
                { "entity": remote(12), "components": { NAME: "New" }, "has": { path: false } },
            ]),
            now,
        );
        assert!(!connection.polls_full(now + POLL_INTERVAL));
    }

    #[test]
    fn polls_fully_again_after_the_refresh_interval() {
        let mut connection = RemoteConnection::default();
        let mut snapshot = RemoteSnapshot::default();
        let start = Duration::from_secs(1);
        let reply = json!([row(remote(11), json!({ NAME: "A" }))]);
        query(&mut connection, &mut snapshot, reply.clone(), start);

        let mut now = start;
        while now + POLL_INTERVAL < start + FULL_REFRESH_INTERVAL {
            now += POLL_INTERVAL;
            assert!(!connection.polls_full(now));
            query(&mut connection, &mut snapshot, reply.clone(), now);
        }
        assert!(connection.polls_full(start + FULL_REFRESH_INTERVAL));
    }

    #[test]
    fn a_new_connection_polls_fully() {
        let mut connection = RemoteConnection::default();
        let mut snapshot = RemoteSnapshot::default();
        let now = Duration::from_secs(1);
        query(&mut connection, &mut snapshot, json!([]), now);
        assert!(!connection.polls_full(now));

        finish_call(
            &mut connection,
            &mut snapshot,
            false,
            info(),
            now,
            Duration::ZERO,
        );
        assert!(connection.polls_full(now));
    }

    #[test]
    fn slow_answers_stretch_the_poll_interval() {
        let mut connection = RemoteConnection::default();
        let mut snapshot = RemoteSnapshot::default();
        let now = Duration::from_secs(10);
        finish_call(
            &mut connection,
            &mut snapshot,
            true,
            full_rows(json!([])),
            now,
            Duration::from_secs(1),
        );
        assert_eq!(connection.next_poll, now + Duration::from_secs(2));
    }
}
