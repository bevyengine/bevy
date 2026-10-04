//! A source inspecting a separate running app over the Bevy Remote Protocol.
//!
//! Each remote entity is mirrored by a local proxy entity, spawned [`Disabled`] so that the
//! systems of the app running the inspector skip it. A proxy only carries what the entity tree
//! needs: the remote id, a resolved label and its parent proxy. Remote component values are never
//! inserted into the local world, so no local hook or observer runs for them. The components of
//! the selected entity are fetched separately and deserialized into [`RemoteDetails`] for the
//! details panel instead.
//!
//! The entity tree is polled with `world.query` for the components it needs only: [`Name`],
//! [`ChildOf`] and the label-defining components. A full poll reading every component runs when
//! entities the inspector has not seen appear, and at least every 10 seconds. It tells which
//! entities hold a reflected component, since the others, such as observers and systems, are not
//! shown.
//!
//! Remote entities with the [`Disabled`] component are not shown either, since `world.query`
//! skips them.
//!
//! [`Name`]: bevy_ecs::name::Name

use alloc::{
    string::{String, ToString},
    sync::Arc,
    vec::Vec,
};
use core::{any::TypeId, time::Duration};

use bevy_dev_tools::inspection::label_resolution::{
    resolve_label, ComponentLabelData, LabelResolutionRegistry,
};
use bevy_ecs::{
    change_detection::Mut,
    component::{Component, ComponentId},
    entity::Entity,
    entity_disabling::Disabled,
    hierarchy::{ChildOf, Children},
    query::{Allow, With},
    reflect::{AppTypeRegistry, ReflectComponent},
    resource::Resource,
    system::{Res, ResMut},
    world::World,
};
use bevy_log::{info, warn};
use bevy_platform::collections::{HashMap, HashSet};
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

pub mod details;

use crate::{
    details_panel::DetailsPanelSync, entity_tree::EntityTreeSync, InspectorSelection,
    InspectorSource,
};
use details::RemoteDetails;

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
    /// Every label-defining type.
    labels: HashSet<String>,
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

/// One remote entity, as the entity tree needs it.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct RemoteRow {
    /// The value of its `Name`.
    pub name: Option<String>,
    /// The remote entity its `ChildOf` points to.
    pub parent: Option<Entity>,
    /// The sorted type paths of the label-defining components it holds.
    pub labels: Vec<String>,
}

/// One row of a `world.query` answer, reduced to what the entity tree needs.
#[derive(Debug)]
struct PolledRow {
    entity: Entity,
    row: RemoteRow,
    /// The types `has` reported present, or `None` if the poll did not ask for `has`.
    detected: Option<Vec<String>>,
    /// Whether the poll serialized or detected any component of the entity.
    reflected: bool,
}

/// The entities of the remote world, as last reported by `world.query`.
#[derive(Resource, Debug, Default)]
pub struct RemoteSnapshot {
    rows: HashMap<Entity, RemoteRow>,
    order: Vec<Entity>,
    detected: HashMap<Entity, Vec<String>>,
    known: HashSet<Entity>,
    reflected: HashSet<Entity>,
    dirty: bool,
}

impl RemoteSnapshot {
    /// The tree data of `remote`.
    pub fn row(&self, remote: Entity) -> Option<&RemoteRow> {
        self.rows.get(&remote)
    }

    /// The number of remote entities in the snapshot.
    pub fn len(&self) -> usize {
        self.order.len()
    }

    /// Whether the snapshot holds no entities.
    pub fn is_empty(&self) -> bool {
        self.order.is_empty()
    }

    /// Drops every entity.
    fn clear(&mut self) {
        *self = Self {
            dirty: true,
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
    fn set(&mut self, rows: Vec<PolledRow>, full: bool) -> bool {
        let mut previous = core::mem::take(&mut self.detected);
        if full {
            self.known.clear();
            self.reflected.clear();
        }
        let mut unseen = false;
        let mut shown = HashMap::with_capacity(rows.len());
        let mut order = Vec::with_capacity(rows.len());
        for polled in rows {
            let entity = polled.entity;
            let detected = polled
                .detected
                .unwrap_or_else(|| previous.remove(&entity).unwrap_or_default());
            if full {
                self.known.insert(entity);
                if polled.reflected {
                    self.reflected.insert(entity);
                }
            } else if !self.known.contains(&entity) {
                unseen = true;
            }
            if !polled.reflected && !self.reflected.contains(&entity) {
                continue;
            }
            let mut row = polled.row;
            if !detected.is_empty() {
                row.labels.extend(detected.iter().cloned());
                row.labels.sort_unstable();
                row.labels.dedup();
                self.detected.insert(entity, detected);
            }
            order.push(entity);
            shown.insert(entity, row);
        }
        if shown != self.rows {
            self.rows = shown;
            self.dirty = true;
        }
        self.order = order;
        unseen
    }
}

/// Maps remote entities to the local proxy entities mirroring them.
#[derive(Resource, Debug, Default)]
pub struct RemoteProxyIndex {
    proxies: HashMap<Entity, Entity>,
}

impl RemoteProxyIndex {
    /// The proxy mirroring `remote`, if one exists.
    pub fn proxy(&self, remote: Entity) -> Option<Entity> {
        self.proxies.get(&remote).copied()
    }

    /// The number of proxies currently tracked.
    pub fn len(&self) -> usize {
        self.proxies.len()
    }

    /// Whether no proxies are currently tracked.
    pub fn is_empty(&self) -> bool {
        self.proxies.is_empty()
    }
}

/// A local proxy entity mirroring one entity of the remote world.
#[derive(Component, Debug, Clone, Copy, Reflect)]
#[reflect(Component, Debug, Clone)]
pub struct RemoteEntityProxy {
    /// The id of the mirrored entity in the remote world.
    pub remote: Entity,
}

/// The label the entity tree shows for a proxy.
#[derive(Component, Debug, Default, Clone, PartialEq, Eq, Reflect)]
#[reflect(Component, Debug, Default, Clone, PartialEq)]
pub struct RemoteLabel(pub String);

/// Whether the inspector currently reads from a remote app.
pub(crate) fn is_remote(world: &World) -> bool {
    matches!(
        world.get_resource::<InspectorSource>(),
        Some(InspectorSource::Remote(_))
    )
}

/// The remote id mirrored by `entity`, if it is a proxy.
pub(crate) fn remote_entity(world: &World, entity: Entity) -> Option<Entity> {
    world
        .get::<RemoteEntityProxy>(entity)
        .map(|proxy| proxy.remote)
}

/// The label the entity tree shows for `entity`, if it is a proxy.
pub(crate) fn proxy_label(world: &World, entity: Entity) -> Option<String> {
    let remote = remote_entity(world, entity)?;
    Some(
        world
            .get::<RemoteLabel>(entity)
            .map(|label| label.0.clone())
            .unwrap_or_else(|| remote.to_string()),
    )
}

/// Sets the connection up for the current [`InspectorSource`] when it changes, clearing every
/// proxy and the selection.
pub fn sync_remote_source(world: &mut World) {
    let source = match world.get_resource::<InspectorSource>() {
        Some(InspectorSource::Remote(source)) => Some(source),
        _ => None,
    };
    if world.resource::<RemoteConnection>().source.as_ref() == source {
        return;
    }
    let source = source.cloned();

    clear_proxies(world);
    *world.resource_mut::<RemoteSnapshot>() = RemoteSnapshot::default();
    *world.resource_mut::<RemoteDetails>() = RemoteDetails::default();
    world.resource_mut::<InspectorSelection>().0 = None;

    let mut connection = world.resource_mut::<RemoteConnection>();
    connection.pending = None;
    connection.state = RemoteConnectionState::Disconnected;
    connection.next_poll = Duration::ZERO;
    connection.next_full = Duration::ZERO;
    connection.client = source
        .as_ref()
        .map(|source| BrpClient::new(source.host.clone(), source.port));
    connection.source = source;

    world.resource_mut::<EntityTreeSync>().set_dirty();
    world.resource_mut::<DetailsPanelSync>().set_dirty();
}

/// Despawns every proxy.
fn clear_proxies(world: &mut World) {
    let proxies: Vec<Entity> = world
        .query_filtered::<Entity, (With<RemoteEntityProxy>, Allow<Disabled>)>()
        .iter(world)
        .collect();
    for proxy in proxies {
        if let Ok(proxy) = world.get_entity_mut(proxy) {
            proxy.despawn();
        }
    }
    world.resource_mut::<RemoteProxyIndex>().proxies.clear();
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
    let Some(priorities) = priorities else {
        return types;
    };
    for registration in registry.iter() {
        if registration.data::<ReflectComponent>().is_none()
            || priorities
                .get_priority_by_type_id(registration.type_id())
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
            types.read.push(path.clone());
        } else {
            types.has.push(path.clone());
        }
        types.labels.insert(path);
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

/// Parses a `world.query` answer into the rows the entity tree needs.
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
        mut components,
        has,
    } = row;
    if components.contains_key(IS_RESOURCE) {
        return None;
    }
    let detected: Vec<String> = has
        .into_iter()
        .filter(|(_, present)| *present == Value::Bool(true))
        .map(|(path, _)| path)
        .collect();
    let reflected = !components.is_empty() || !detected.is_empty();
    let name = match components.remove(NAME) {
        Some(Value::String(name)) => Some(name),
        _ => None,
    };
    let parent = components
        .get(CHILD_OF)
        .and_then(|parent| Entity::deserialize(parent).ok());
    let mut labels: Vec<String> = components
        .into_keys()
        .filter(|path| types.labels.contains(path))
        .collect();
    labels.sort_unstable();
    Some(PolledRow {
        entity,
        row: RemoteRow {
            name,
            parent,
            labels,
        },
        detected: full.then_some(detected),
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

/// Spawns, reparents, relabels and despawns the proxies so that they match the latest snapshot.
pub fn apply_remote_snapshot(world: &mut World) {
    if !is_remote(world) || !world.resource::<RemoteSnapshot>().dirty {
        return;
    }
    let changed = world.resource_scope(|world, mut snapshot: Mut<RemoteSnapshot>| {
        snapshot.dirty = false;
        let despawned = despawn_vanished(world, &snapshot);
        let spawned = spawn_missing(world, &snapshot);
        let reparented = apply_hierarchy(world, &snapshot);
        let relabelled = apply_labels(world, &snapshot);
        despawned || spawned || reparented || relabelled
    });
    if changed {
        world.resource_mut::<EntityTreeSync>().set_dirty();
    }
}

fn despawn_vanished(world: &mut World, snapshot: &RemoteSnapshot) -> bool {
    let stale: Vec<(Entity, Entity)> = world
        .resource::<RemoteProxyIndex>()
        .proxies
        .iter()
        .filter(|(remote, _)| !snapshot.rows.contains_key(*remote))
        .map(|(remote, proxy)| (*remote, *proxy))
        .collect();

    for (remote, proxy) in &stale {
        let children: Vec<Entity> = world
            .get::<Children>(*proxy)
            .map(|children| children.iter().copied().collect())
            .unwrap_or_default();
        for child in children {
            if let Ok(mut child) = world.get_entity_mut(child) {
                child.remove::<ChildOf>();
            }
        }
        if let Ok(proxy) = world.get_entity_mut(*proxy) {
            proxy.despawn();
        }
        world
            .resource_mut::<RemoteProxyIndex>()
            .proxies
            .remove(remote);
    }
    !stale.is_empty()
}

fn spawn_missing(world: &mut World, snapshot: &RemoteSnapshot) -> bool {
    let mut spawned = false;
    for remote in &snapshot.order {
        let known = world
            .resource::<RemoteProxyIndex>()
            .proxy(*remote)
            .is_some_and(|proxy| world.get_entity(proxy).is_ok());
        if known {
            continue;
        }
        let proxy = world
            .spawn((RemoteEntityProxy { remote: *remote }, Disabled))
            .id();
        world
            .resource_mut::<RemoteProxyIndex>()
            .proxies
            .insert(*remote, proxy);
        spawned = true;
    }
    spawned
}

fn apply_hierarchy(world: &mut World, snapshot: &RemoteSnapshot) -> bool {
    let mut changed = false;
    for remote in &snapshot.order {
        let index = world.resource::<RemoteProxyIndex>();
        let Some(proxy) = index.proxy(*remote) else {
            continue;
        };
        let parent = snapshot.rows[remote]
            .parent
            .and_then(|parent| index.proxy(parent))
            .filter(|parent| *parent != proxy);
        if world.get::<ChildOf>(proxy).map(ChildOf::parent) == parent {
            continue;
        }
        let Ok(mut proxy) = world.get_entity_mut(proxy) else {
            continue;
        };
        match parent {
            Some(parent) => {
                proxy.insert(ChildOf(parent));
            }
            None => {
                proxy.remove::<ChildOf>();
            }
        }
        changed = true;
    }
    changed
}

fn apply_labels(world: &mut World, snapshot: &RemoteSnapshot) -> bool {
    let registry = world.resource::<AppTypeRegistry>().clone();
    let registry = registry.read();
    let mut changed = false;
    for remote in &snapshot.order {
        let Some(proxy) = world.resource::<RemoteProxyIndex>().proxy(*remote) else {
            continue;
        };
        let label = remote_label(world, &registry, proxy, &snapshot.rows[remote])
            .unwrap_or_else(|| remote.to_string());
        if world
            .get::<RemoteLabel>(proxy)
            .map(|label| label.0.as_str())
            != Some(label.as_str())
        {
            world.entity_mut(proxy).insert(RemoteLabel(label));
            changed = true;
        }
    }
    changed
}

/// Resolves the label of a remote entity the way the local tree does: its [`Name`], or else the
/// label-defining components it holds that are registered locally.
///
/// [`Name`]: bevy_ecs::name::Name
fn remote_label(
    world: &World,
    registry: &TypeRegistry,
    proxy: Entity,
    row: &RemoteRow,
) -> Option<String> {
    if let Some(name) = &row.name {
        return Some(name.clone());
    }
    let priorities = world.get_resource::<LabelResolutionRegistry>()?;
    let labels: Vec<(&str, _)> = row
        .labels
        .iter()
        .filter_map(|type_path| {
            let registration = registry.get_with_type_path(type_path)?;
            let priority = priorities.get_priority_by_type_id(registration.type_id())?;
            Some((
                registration.type_info().type_path_table().short_path(),
                priority,
            ))
        })
        .collect();
    let data: Vec<ComponentLabelData> = labels
        .iter()
        .map(|(short_name, priority)| ComponentLabelData {
            component_id: ComponentId::new(0),
            short_name,
            label_definition_priority: Some(*priority),
        })
        .collect();
    resolve_label(world, proxy, &data).map(|label| label.label.as_str().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy_dev_tools::inspection::label_resolution::LabelDefinitionPriority;
    use bevy_ecs::name::Name;
    use bevy_reflect::{TypePath, TypeRegistryArc};
    use serde_json::json;

    pub(crate) fn test_world() -> World {
        let mut world = World::new();
        let registry = AppTypeRegistry(TypeRegistryArc::default());
        {
            let mut registry = registry.write();
            registry.register::<Name>();
            registry.register::<ChildOf>();
        }
        world.insert_resource(registry);
        world.insert_resource(InspectorSource::Remote(RemoteSource::localhost(
            DEFAULT_PORT,
        )));
        world.init_resource::<InspectorSelection>();
        world.init_resource::<RemoteConnection>();
        world.init_resource::<RemoteProxyIndex>();
        world.init_resource::<RemoteSnapshot>();
        world.init_resource::<RemoteDetails>();
        world.init_resource::<EntityTreeSync>();
        world.init_resource::<DetailsPanelSync>();
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

    fn poll(world: &mut World, rows: Vec<Value>, full: bool) {
        let rows = parse_rows(Value::Array(rows), &world_types(world), full).unwrap();
        world.resource_mut::<RemoteSnapshot>().set(rows, full);
        apply_remote_snapshot(world);
    }

    pub(crate) fn apply(world: &mut World, rows: Vec<Value>) {
        poll(world, rows, true);
        details::sync_remote_details(world);
    }

    pub(crate) fn proxy(world: &World, remote: Entity) -> Entity {
        world.resource::<RemoteProxyIndex>().proxy(remote).unwrap()
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
    fn mirrors_remote_entities_as_disabled_proxies() {
        let mut world = test_world();
        let parent = remote(1);
        let child = remote(2);
        apply(
            &mut world,
            alloc::vec![
                row(parent, json!({ NAME: "Parent" })),
                row(child, json!({ NAME: "Child", CHILD_OF: parent })),
            ],
        );

        let parent_proxy = proxy(&world, parent);
        let child_proxy = proxy(&world, child);
        assert_ne!(parent_proxy, parent);
        assert!(world.get::<Disabled>(parent_proxy).is_some());
        assert!(world.get::<Name>(parent_proxy).is_none());
        assert_eq!(
            world.get::<ChildOf>(child_proxy).map(ChildOf::parent),
            Some(parent_proxy)
        );
        assert_eq!(proxy_label(&world, child_proxy).as_deref(), Some("Child"));
    }

    #[test]
    fn labels_unnamed_entities_with_their_remote_id() {
        let mut world = test_world();
        apply(
            &mut world,
            alloc::vec![row(remote(7), json!({ "demo::Unknown": 1 }))],
        );
        assert_eq!(
            proxy_label(&world, proxy(&world, remote(7))).as_deref(),
            Some("7v0")
        );
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
    fn rows_keep_only_what_the_tree_needs() {
        let world = label_world();
        let rows = parse_rows(
            json!([row(
                remote(1),
                json!({
                    NAME: "Cube",
                    CHILD_OF: remote(2),
                    Labelled::type_path(): null,
                    "demo::Transform": { "x": 1.0 },
                }),
            )]),
            &world_types(&world),
            false,
        )
        .unwrap();
        assert_eq!(
            rows[0].row,
            RemoteRow {
                name: Some("Cube".to_string()),
                parent: Some(remote(2)),
                labels: alloc::vec![Labelled::type_path().to_string()],
            }
        );
        assert_eq!(rows[0].detected, None);
    }

    #[test]
    fn unseen_entities_without_tree_components_wait_for_a_full_poll() {
        let mut world = label_world();
        poll(
            &mut world,
            alloc::vec![
                row(remote(1), json!({ NAME: "A", "demo::Transform": 1 })),
                row(remote(2), json!({ "demo::Transform": 1 })),
                row(remote(3), json!({})),
            ],
            true,
        );
        assert_eq!(world.resource::<RemoteProxyIndex>().len(), 2);

        let unseen = parse_rows(
            json!([
                row(remote(1), json!({ NAME: "A" })),
                row(remote(2), json!({})),
                row(remote(3), json!({})),
                row(remote(4), json!({})),
                row(remote(5), json!({ NAME: "E" })),
            ]),
            &world_types(&world),
            false,
        )
        .unwrap();
        assert!(world.resource_mut::<RemoteSnapshot>().set(unseen, false));
        apply_remote_snapshot(&mut world);
        let index = world.resource::<RemoteProxyIndex>();
        assert!(index.proxy(remote(1)).is_some());
        assert!(
            index.proxy(remote(2)).is_some(),
            "known reflected entities stay"
        );
        assert!(
            index.proxy(remote(3)).is_none(),
            "known internal entities stay hidden"
        );
        assert!(
            index.proxy(remote(4)).is_none(),
            "unseen entities wait for a full poll"
        );
        assert!(
            index.proxy(remote(5)).is_some(),
            "named entities are shown at once"
        );

        poll(
            &mut world,
            alloc::vec![
                row(remote(1), json!({ NAME: "A" })),
                row(remote(2), json!({ "demo::Transform": 1 })),
                row(remote(3), json!({})),
                row(remote(4), json!({ "demo::Transform": 1 })),
                row(remote(5), json!({ NAME: "E" })),
            ],
            true,
        );
        assert_eq!(world.resource::<RemoteProxyIndex>().len(), 4);
        assert!(world
            .resource::<RemoteProxyIndex>()
            .proxy(remote(4))
            .is_some());
    }

    #[test]
    fn labels_entities_with_components_only_reported_by_has() {
        let mut world = label_world();
        let path = Labelled::type_path();
        apply(
            &mut world,
            alloc::vec![json!({
                "entity": remote(3),
                "components": {},
                "has": { path: true },
            })],
        );
        assert_eq!(
            proxy_label(&world, proxy(&world, remote(3))).as_deref(),
            Some("Labelled")
        );

        poll(&mut world, alloc::vec![row(remote(3), json!({}))], false);
        assert_eq!(
            proxy_label(&world, proxy(&world, remote(3))).as_deref(),
            Some("Labelled"),
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
                json!({ "entity": remote(1), "components": {}, "has": { path: false } }),
                row(remote(2), json!({ IS_RESOURCE: {}, NAME: "Resource" })),
                row(remote(3), json!({ NAME: "Shown" })),
            ],
        );
        let index = world.resource::<RemoteProxyIndex>();
        assert_eq!(index.len(), 1);
        assert!(index.proxy(remote(3)).is_some());
    }

    #[test]
    fn an_unchanged_poll_does_not_dirty_the_tree() {
        let mut world = test_world();
        let rows = || {
            alloc::vec![
                row(remote(1), json!({ NAME: "A" })),
                row(remote(2), json!({ NAME: "B", CHILD_OF: remote(1) })),
            ]
        };
        poll(&mut world, rows(), true);
        world.resource_mut::<EntityTreeSync>().timer.reset();

        poll(&mut world, rows(), false);
        assert!(!world.resource::<RemoteSnapshot>().dirty);
        assert_eq!(
            world.resource::<EntityTreeSync>().timer.elapsed(),
            Duration::ZERO
        );

        poll(
            &mut world,
            alloc::vec![
                row(remote(1), json!({ NAME: "A" })),
                row(remote(2), json!({ NAME: "Renamed", CHILD_OF: remote(1) })),
            ],
            false,
        );
        assert_ne!(
            world.resource::<EntityTreeSync>().timer.elapsed(),
            Duration::ZERO
        );
        assert_eq!(
            proxy_label(&world, proxy(&world, remote(2))).as_deref(),
            Some("Renamed")
        );
    }

    #[test]
    fn follows_remote_despawns_and_reparenting() {
        let mut world = test_world();
        let a = remote(1);
        let b = remote(2);
        let c = remote(3);
        apply(
            &mut world,
            alloc::vec![
                row(a, json!({ NAME: "A" })),
                row(b, json!({ NAME: "B", CHILD_OF: a })),
                row(c, json!({ NAME: "C", CHILD_OF: b })),
            ],
        );
        let b_proxy = proxy(&world, b);
        let c_proxy = proxy(&world, c);

        apply(
            &mut world,
            alloc::vec![
                row(a, json!({ NAME: "A" })),
                row(c, json!({ NAME: "C", CHILD_OF: a })),
            ],
        );

        assert!(world.get_entity(b_proxy).is_err());
        assert!(world.get_entity(c_proxy).is_ok());
        assert_eq!(
            world.get::<ChildOf>(c_proxy).map(ChildOf::parent),
            Some(proxy(&world, a))
        );
        assert_eq!(world.resource::<RemoteProxyIndex>().len(), 2);
    }

    #[test]
    fn a_remote_id_colliding_with_a_local_entity_gets_its_own_proxy() {
        let mut world = test_world();
        let local = world.spawn(Name::new("Local")).id();
        apply(
            &mut world,
            alloc::vec![row(local, json!({ NAME: "Remote" }))],
        );

        let proxy = proxy(&world, local);
        assert_ne!(proxy, local);
        assert_eq!(world.get::<Name>(local).map(Name::as_str), Some("Local"));
        assert_eq!(proxy_label(&world, proxy).as_deref(), Some("Remote"));
    }

    #[test]
    fn a_new_generation_replaces_the_proxy() {
        let mut world = test_world();
        let old = remote(4);
        let new =
            Entity::from_index_and_generation(old.index(), old.generation().after_versions(1));
        apply(&mut world, alloc::vec![row(old, json!({ NAME: "Old" }))]);
        let old_proxy = proxy(&world, old);
        world.resource_mut::<InspectorSelection>().0 = Some(old_proxy);

        apply(&mut world, alloc::vec![row(new, json!({ NAME: "New" }))]);

        assert!(world.get_entity(old_proxy).is_err());
        assert_ne!(proxy(&world, new), old_proxy);
    }

    #[test]
    fn switching_to_local_despawns_the_proxies() {
        let mut world = test_world();
        sync_remote_source(&mut world);
        apply(
            &mut world,
            alloc::vec![row(remote(1), json!({ NAME: "A" }))],
        );
        let proxy = proxy(&world, remote(1));
        world.resource_mut::<InspectorSelection>().0 = Some(proxy);

        world.insert_resource(InspectorSource::Local);
        sync_remote_source(&mut world);

        assert!(world.get_entity(proxy).is_err());
        assert!(world.resource::<RemoteProxyIndex>().is_empty());
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
            alloc::vec![row(remote(1), json!({ NAME: "A" }))],
        );
        let proxy = proxy(&world, remote(1));
        world.resource_mut::<InspectorSelection>().0 = Some(proxy);
        details::sync_remote_details(&mut world);
        assert_eq!(world.resource::<RemoteDetails>().proxy, Some(proxy));

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
        apply_remote_snapshot(&mut world);
        details::sync_remote_details(&mut world);

        assert!(world.resource::<RemoteSnapshot>().is_empty());
        assert!(world.get_entity(proxy).is_err());
        assert!(world.resource::<RemoteProxyIndex>().is_empty());
        assert_eq!(world.resource::<RemoteDetails>().proxy, None);
    }

    #[test]
    fn an_unanswered_request_times_out() {
        let mut connection = connected();
        let mut snapshot = RemoteSnapshot::default();
        snapshot.set(
            parse_rows(
                json!([row(remote(1), json!({ NAME: "A" }))]),
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
            json!([{ "entity": remote(1), "components": {}, "has": { path: true } }]),
            now,
        );

        now += POLL_INTERVAL;
        assert!(!connection.polls_full(now));
        query(
            &mut connection,
            &mut snapshot,
            json!([row(remote(1), json!({}))]),
            now,
        );
        assert_eq!(snapshot.row(remote(1)).unwrap().labels, [path]);
        assert_eq!(snapshot.len(), 1);

        now += POLL_INTERVAL;
        assert!(!connection.polls_full(now));
        query(
            &mut connection,
            &mut snapshot,
            json!([
                row(remote(1), json!({})),
                row(remote(2), json!({ NAME: "New" }))
            ]),
            now,
        );
        assert_eq!(snapshot.row(remote(1)).unwrap().labels, [path]);

        assert!(!connection.polls_full(now + POLL_INTERVAL));
        now = start + FULL_MIN_INTERVAL;
        assert!(connection.polls_full(now));
        query(
            &mut connection,
            &mut snapshot,
            json!([
                { "entity": remote(1), "components": {}, "has": { path: true } },
                { "entity": remote(2), "components": { NAME: "New" }, "has": { path: false } },
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
        let reply = json!([row(remote(1), json!({ NAME: "A" }))]);
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
