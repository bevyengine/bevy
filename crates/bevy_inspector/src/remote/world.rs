//! A [`World`] mirroring the entities and components of the remote app.
//!
//! Remote entities are spawned at their remote ids, so the entity tree and the details panel read
//! the mirror like any other world. Component values are inserted through reflection, but only for
//! types that have no component hooks, so that inserting them never spawns entities or depends on
//! state the mirror lacks. Other components are kept aside on the entity's [`RemoteComponents`].
//!
//! Spawning at the remote ids keeps every id, such as the selection or a [`ChildOf`] target, the
//! same in both worlds. [`World::spawn_at`] only accepts the current generation of an index, and a
//! fresh world starts every index at generation 0, so the index is stepped up to the remote
//! generation by spawning and despawning it without freeing. Indices up to the remote ones are
//! reserved, so the allocator never hands them out. A `bevy_ecs` API setting the generation of an
//! index and reserving it would replace this stepping.

use alloc::{
    boxed::Box,
    string::{String, ToString},
    vec::Vec,
};
use core::{
    cmp::Ordering,
    hash::{BuildHasher, Hasher},
};

use bevy_ecs::{
    component::{Component, ComponentId, ComponentInfo},
    entity::{Entity, EntityHashMap, SpawnError},
    error::{warn, FallbackErrorHandler},
    hierarchy::{ChildOf, Children},
    lifecycle::HookContext,
    name::Name,
    query::With,
    reflect::{AppTypeRegistry, ReflectComponent},
    relationship::RelationshipAccessor,
    resource::Resource,
    world::{DeferredWorld, FromWorld, World},
};
use bevy_log::warn_once;
use bevy_platform::{collections::HashMap, hash::FixedHasher};
use bevy_reflect::{
    serde::TypedReflectDeserializer, PartialReflect, Reflect, ReflectFromReflect, ReflectRef,
    TypeRegistry,
};
use serde::de::DeserializeSeed;
use serde_json::Value;

/// The most generations [`spawn_at_remote_id`] steps through for one entity in one call.
const MAX_GENERATION_STEPS: u32 = 4096;

/// Keys the details panel group listing the components that are not registered locally.
#[derive(Component)]
pub(crate) struct UnregisteredComponents;

/// The [`World`] mirroring the remote app.
///
/// It holds a clone of the local [`AppTypeRegistry`], sharing its types, and the remote entities
/// the inspector shows, each with a [`RemoteComponents`] record. Errors of commands queued by
/// component hooks are logged as warnings instead of panicking.
#[derive(Resource)]
pub struct RemoteWorld {
    world: World,
    reserved: u32,
    types: HashMap<u64, Registered>,
    probe: HookProbe,
    garbage: Garbage,
}

/// A component type registered in the [`RemoteWorld`].
#[derive(Clone)]
struct Registered {
    id: ComponentId,
    reflect_component: ReflectComponent,
    /// Whether values of the type are inserted, rather than kept aside for having hooks.
    insertable: bool,
}

/// Finds the component types that have no hooks, using a world of its own that never holds an
/// entity, since hooks can only be inspected by trying to register them.
///
/// Hooks declared on a type are found. Hooks that plugins register at runtime do not exist in the
/// remote world either, so they are not looked for.
struct HookProbe {
    world: World,
    hook_free: HashMap<ComponentId, bool>,
}

impl HookProbe {
    fn new() -> Self {
        Self {
            world: World::new(),
            hook_free: HashMap::default(),
        }
    }

    /// Whether the type of `reflect_component` and all its required components have no hooks.
    fn is_hook_free(&mut self, reflect_component: &ReflectComponent) -> bool {
        let id = reflect_component.register_component(&mut self.world);
        let required: Vec<ComponentId> = self
            .world
            .components()
            .get_info(id)
            .map(|info| info.required_components().iter_ids().collect())
            .unwrap_or_default();
        core::iter::once(id)
            .chain(required)
            .all(|id| self.has_no_hooks(id))
    }

    /// Whether the component `id` has no hooks. Each id is only probed once, since probing
    /// registers placeholder hooks.
    fn has_no_hooks(&mut self, id: ComponentId) -> bool {
        if let Some(hook_free) = self.hook_free.get(&id) {
            return *hook_free;
        }
        let hook_free = self
            .world
            .register_component_hooks_by_id(id)
            .and_then(|hooks| hooks.try_on_add(no_hook))
            .and_then(|hooks| hooks.try_on_insert(no_hook))
            .and_then(|hooks| hooks.try_on_discard(no_hook))
            .and_then(|hooks| hooks.try_on_remove(no_hook))
            .and_then(|hooks| hooks.try_on_despawn(no_hook))
            .is_some();
        self.hook_free.insert(id, hook_free);
        hook_free
    }
}

fn no_hook(_: DeferredWorld, _: HookContext) {}

/// Polled data that is no longer needed, to drop off the main thread.
#[derive(Default)]
pub(crate) struct Garbage {
    unchanged: Vec<Vec<PolledComponent>>,
    written: Vec<(String, Value)>,
}

impl Garbage {
    pub(crate) fn is_empty(&self) -> bool {
        self.unchanged.is_empty() && self.written.is_empty()
    }
}

impl FromWorld for RemoteWorld {
    fn from_world(world: &mut World) -> Self {
        Self::new(
            world
                .get_resource::<AppTypeRegistry>()
                .cloned()
                .unwrap_or_default(),
        )
    }
}

/// Why a remote entity could not be spawned in the [`RemoteWorld`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SpawnRemoteError {
    /// The index is taken by an entity of the remote world itself, such as a resource.
    Taken,
    /// The index has a later generation than the remote entity, so the remote app restarted.
    Behind,
    /// The generation is more than [`MAX_GENERATION_STEPS`] ahead. Later calls catch up further.
    Ahead,
}

impl RemoteWorld {
    /// An empty remote world sharing the types of `registry`.
    pub(crate) fn new(registry: AppTypeRegistry) -> Self {
        let mut world = World::new();
        world.insert_resource(registry);
        world.insert_resource(FallbackErrorHandler(warn));
        world.register_component::<ChildOf>();
        world.register_component::<Children>();
        world.register_component::<RemoteComponents>();
        world.register_component::<UnregisteredComponents>();
        Self {
            world,
            reserved: 0,
            types: HashMap::default(),
            probe: HookProbe::new(),
            garbage: Garbage::default(),
        }
    }

    /// Registers the component types at `type_paths` that are registered locally for reflection,
    /// and finds whether they have hooks.
    ///
    /// Only types with no hooks are inserted. [`ChildOf`] is the one exception, since its hooks
    /// only build [`Children`], which is never inserted from JSON. Types that have hooks are kept
    /// aside instead.
    ///
    /// A relationship and its target only know they form a relationship once both are
    /// registered, so the types of a whole request are registered before any is written.
    pub(crate) fn register<'a>(&mut self, type_paths: impl IntoIterator<Item = &'a str>) {
        let registry = self.world.resource::<AppTypeRegistry>().clone();
        let registry = registry.read();
        for type_path in type_paths {
            let hash = path_hash(type_path);
            if self.types.contains_key(&hash) {
                continue;
            }
            if let Some(reflect_component) = registry
                .get_with_type_path(type_path)
                .and_then(|registration| registration.data::<ReflectComponent>())
            {
                let id = reflect_component.register_component(&mut self.world);
                let insertable = Some(id) == self.world.component_id::<ChildOf>()
                    || Some(id) == self.world.component_id::<Children>()
                    || self.probe.is_hook_free(reflect_component);
                self.types.insert(
                    hash,
                    Registered {
                        id,
                        reflect_component: reflect_component.clone(),
                        insertable,
                    },
                );
            }
        }
    }

    /// Takes the polled data [`RemoteWorld::write`] no longer needs, so that the caller can drop
    /// it off the main thread.
    pub(crate) fn take_garbage(&mut self) -> Garbage {
        core::mem::take(&mut self.garbage)
    }

    /// The mirrored world.
    pub fn world(&self) -> &World {
        &self.world
    }

    /// Replaces the world with an empty one.
    pub(crate) fn reset(&mut self, registry: AppTypeRegistry) {
        *self = Self::new(registry);
    }

    /// Whether `remote` is mirrored.
    pub(crate) fn contains(&self, remote: Entity) -> bool {
        self.world.get::<RemoteComponents>(remote).is_some()
    }

    /// The mirrored entities.
    pub(crate) fn entities(&mut self) -> Vec<Entity> {
        self.world
            .query_filtered::<Entity, With<RemoteComponents>>()
            .iter(&self.world)
            .collect()
    }

    /// Spawns `remote` at its remote id. Returns whether it was not mirrored yet.
    pub(crate) fn spawn(&mut self, remote: Entity) -> Result<bool, SpawnRemoteError> {
        if self.contains(remote) {
            return Ok(false);
        }
        spawn_at_remote_id(&mut self.world, &mut self.reserved, remote)?;
        self.world
            .entity_mut(remote)
            .insert(RemoteComponents::default());
        Ok(true)
    }

    /// Despawns `remote` without freeing its index, detaching its relationship sources first so
    /// that they are not despawned with it.
    pub(crate) fn despawn(&mut self, remote: Entity) {
        let Ok(entity) = self.world.get_entity(remote) else {
            return;
        };
        let targets: Vec<ComponentId> = entity
            .archetype()
            .components()
            .iter()
            .copied()
            .filter(|id| is_relationship_target(&self.world, *id))
            .collect();
        let mut entity = self.world.entity_mut(remote);
        entity.remove_by_ids(&targets);
        entity.despawn_no_free();
    }

    /// Writes the components a request returned for `remote`.
    ///
    /// The types of `components` must be registered first with [`RemoteWorld::register`], or they
    /// are kept aside as unregistered.
    ///
    /// Only components whose JSON changed, or that a hook removed, are written. Components the
    /// request covers that it no longer returned are removed if they were written before.
    /// `unserialized` lists the types the request reported present, which count as unserialized
    /// unless `components` holds them. Components kept aside for relating to an entity that was not
    /// mirrored, or removed by a hook, are retried if `retry` is set.
    pub(crate) fn write(
        &mut self,
        remote: Entity,
        components: Vec<PolledComponent>,
        unserialized: &[String],
        coverage: Coverage<'_>,
        retry: bool,
    ) -> Written {
        let mut written = Written::default();
        let world = &mut self.world;
        let Ok(entity) = world.get_entity(remote) else {
            return written;
        };
        let Some(record) = entity.get::<RemoteComponents>() else {
            return written;
        };

        let removed: Vec<u64> = record
            .path_hashes()
            .filter(|path| {
                coverage.reads(*path)
                    && !components
                        .iter()
                        .any(|component| component.path_hash == *path)
            })
            .collect();
        let unserialized: Vec<String> = unserialized
            .iter()
            .filter(|type_path| {
                !components
                    .iter()
                    .any(|component| component.type_path == **type_path)
            })
            .cloned()
            .collect();
        let unserialized = record.next_unserialized(&unserialized, coverage);
        let state = |component: &PolledComponent| {
            record.state(component, |id| entity.contains_id(id), retry)
        };
        if removed.is_empty()
            && unserialized.is_none()
            && components
                .iter()
                .all(|component| matches!(state(component), State::Current))
        {
            self.garbage.unchanged.push(components);
            return written;
        }
        let mut pending: Vec<(PolledComponent, bool)> = Vec::new();
        let mut unchanged = Vec::new();
        for component in components {
            match state(&component) {
                State::Current => unchanged.push(component),
                State::Stale => pending.push((component, true)),
                State::New => pending.push((component, false)),
            }
        }
        self.garbage.unchanged.push(unchanged);

        written.changed = !removed.is_empty() || !pending.is_empty() || unserialized.is_some();
        written.tree = !removed.is_empty() || unserialized.is_some();
        for path in removed {
            remove_written(world, remote, path);
        }
        if let Some(unserialized) = unserialized {
            let registry = world.resource::<AppTypeRegistry>().clone();
            let registry = registry.read();
            let unserialized = unserialized
                .into_iter()
                .map(|type_path| (register_type_path(world, &registry, &type_path), type_path))
                .collect();
            if let Some(mut record) = world.get_mut::<RemoteComponents>(remote) {
                record.unserialized = unserialized;
            }
        }

        for (component, known) in pending {
            if world.get_entity(remote).is_err() {
                break;
            }
            let registered = self.types.get(&component.path_hash).cloned();
            let id = write_component(
                world,
                remote,
                registered,
                component,
                &mut self.garbage.written,
            );
            written.tree |= !known || id.is_none_or(|id| is_tree_type(world, id));
        }
        written
    }

    /// Sorts the [`Children`] of every mirrored entity into the order the remote app reported.
    ///
    /// The mirror builds [`Children`] from the [`ChildOf`] it inserts, so their order follows
    /// insertion. Children the remote app did not report are kept at the end.
    pub(crate) fn order_children(&mut self) {
        let mut query = self.world.query::<(&RemoteComponents, &mut Children)>();
        for (record, mut children) in query.iter_mut(&mut self.world) {
            if record.children.is_empty() {
                continue;
            }
            let order: EntityHashMap<usize> = record
                .children
                .iter()
                .enumerate()
                .map(|(position, child)| (*child, position))
                .collect();
            let position = |child: &Entity| order.get(child).copied().unwrap_or(usize::MAX);
            if !children.is_sorted_by_key(position) {
                children.sort_by_cached_key(position);
            }
        }
    }
}

/// What [`RemoteWorld::write`] changed.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Written {
    /// Whether any component was written or removed.
    pub(crate) changed: bool,
    /// Whether a change can affect the entity tree: a name, a relationship, or the unserialized
    /// types used for labels.
    pub(crate) tree: bool,
}

/// Spawns an empty entity at exactly `remote`, an id the remote app allocated.
///
/// This is the only place that works around the entity allocator of the remote world:
/// - Every index up to `remote.index()` is allocated and leaked first, so that the allocator never
///   hands one out later, which would make that spawn panic. `reserved` counts them.
/// - An index starts at the first generation, so it is stepped up to `remote.generation()` by
///   spawning and despawning it without freeing, which bumps the generation by one each time. At
///   most [`MAX_GENERATION_STEPS`] steps are taken per call.
fn spawn_at_remote_id(
    world: &mut World,
    reserved: &mut u32,
    remote: Entity,
) -> Result<(), SpawnRemoteError> {
    let index = remote.index_u32();
    while *reserved <= index {
        for entity in world.entity_allocator().alloc_many(index + 1 - *reserved) {
            *reserved = (*reserved).max(entity.index_u32() + 1);
        }
    }
    for _ in 0..MAX_GENERATION_STEPS {
        match world.entities().check_can_spawn_at(remote) {
            Ok(()) => {
                world
                    .spawn_empty_at(remote)
                    .map_err(|_| SpawnRemoteError::Taken)?;
                return Ok(());
            }
            Err(SpawnError::AlreadySpawned) => return Err(SpawnRemoteError::Taken),
            Err(SpawnError::Invalid(error)) => {
                if error.current_generation.cmp_approx(&remote.generation()) != Ordering::Less {
                    return Err(SpawnRemoteError::Behind);
                }
                let current =
                    Entity::from_index_and_generation(remote.index(), error.current_generation);
                world
                    .spawn_empty_at(current)
                    .map_err(|_| SpawnRemoteError::Taken)?
                    .despawn_no_free();
            }
        }
    }
    warn_once!(
        "the remote inspector is still catching up with the generation of {remote}, which can \
         take several polls"
    );
    Err(SpawnRemoteError::Ahead)
}

/// The components the remote app reported for one mirrored entity.
///
/// Only the components recorded here are shown by the details panel. Others in the entity's
/// archetype were added locally, as required components or by hooks.
#[derive(Component, Debug, Default)]
pub struct RemoteComponents {
    serialized: Vec<Serialized>,
    unserialized: Vec<(Option<ComponentId>, String)>,
    aside: Vec<AsideComponent>,
    /// The order of [`Children`] the remote app reported, empty if it reported none.
    children: Vec<Entity>,
}

#[derive(Debug)]
struct Serialized {
    id: ComponentId,
    path_hash: u64,
    /// The hash of the JSON last written.
    json_hash: u64,
}

/// A component the remote app serialized that is not inserted into the remote world.
#[derive(Debug)]
pub struct AsideComponent {
    /// The id of the component type in the remote world, if it is registered locally.
    pub id: Option<ComponentId>,
    /// The full type path of the component.
    pub type_path: String,
    /// The value as the remote app serialized it.
    pub json: Value,
    /// The deserialized value, if there is one.
    pub value: Option<Box<dyn Reflect>>,
    /// Why the component is kept aside.
    pub reason: AsideReason,
    path_hash: u64,
    json_hash: u64,
}

/// Why a component is kept aside instead of being inserted.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AsideReason {
    /// The type is not registered locally as a reflected component.
    Unregistered,
    /// The JSON does not match the local definition of the type.
    Failed(String),
    /// The type or one of its required components has component hooks.
    Hooked,
    /// The component relates to an entity that is not mirrored.
    MissingTarget,
    /// A hook removed the component right after it was inserted.
    RemovedByHook,
}

impl RemoteComponents {
    /// Whether the value of `id` in the remote world came from the remote app.
    pub fn is_serialized(&self, id: ComponentId) -> bool {
        self.serialized.iter().any(|entry| entry.id == id)
    }

    /// The ids of the components the remote app serialized.
    pub fn serialized(&self) -> impl Iterator<Item = ComponentId> + '_ {
        self.serialized.iter().map(|entry| entry.id)
    }

    /// The ids of every component the remote app reported that is registered locally, whether it
    /// was written, kept aside or not serialized.
    pub fn reported(&self) -> impl Iterator<Item = ComponentId> + '_ {
        self.serialized()
            .chain(self.unserialized.iter().filter_map(|(id, _)| *id))
            .chain(self.aside.iter().filter_map(|entry| entry.id))
    }

    /// The components the remote app holds without serializing them, as their id in the remote
    /// world if they are registered locally, and their full type path.
    pub fn unserialized(&self) -> &[(Option<ComponentId>, String)] {
        &self.unserialized
    }

    /// The components that are kept aside.
    pub fn aside(&self) -> &[AsideComponent] {
        &self.aside
    }

    fn path_hashes(&self) -> impl Iterator<Item = u64> + '_ {
        self.serialized
            .iter()
            .map(|entry| entry.path_hash)
            .chain(self.aside.iter().map(|entry| entry.path_hash))
    }

    /// Whether `component` is recorded with the same JSON and still present.
    fn state(
        &self,
        component: &PolledComponent,
        contains: impl Fn(ComponentId) -> bool,
        retry: bool,
    ) -> State {
        if let Some(entry) = self
            .serialized
            .iter()
            .find(|entry| entry.path_hash == component.path_hash)
        {
            return match entry.json_hash == component.json_hash && contains(entry.id) {
                true => State::Current,
                false => State::Stale,
            };
        }
        match self
            .aside
            .iter()
            .find(|entry| entry.path_hash == component.path_hash)
        {
            None => State::New,
            Some(entry)
                if entry.json_hash == component.json_hash
                    && !(retry
                        && matches!(
                            entry.reason,
                            AsideReason::MissingTarget | AsideReason::RemovedByHook
                        )) =>
            {
                State::Current
            }
            Some(_) => State::Stale,
        }
    }

    /// The unserialized types after a request reporting `reported`, or `None` if they did not
    /// change.
    fn next_unserialized(
        &self,
        reported: &[String],
        coverage: Coverage<'_>,
    ) -> Option<Vec<String>> {
        if matches!(coverage, Coverage::Paths(_))
            || (reported.is_empty() && self.unserialized.is_empty())
        {
            return None;
        }
        let mut next: Vec<String> = self
            .unserialized
            .iter()
            .map(|(_, type_path)| type_path)
            .filter(|type_path| !coverage.reports_unserialized(type_path))
            .chain(reported)
            .cloned()
            .collect();
        next.sort_unstable();
        next.dedup();
        let unchanged = next.len() == self.unserialized.len()
            && next
                .iter()
                .zip(&self.unserialized)
                .all(|(next, (_, current))| next == current);
        (!unchanged).then_some(next)
    }
}

/// Whether a polled component is recorded unchanged, recorded with another value, or new.
enum State {
    Current,
    Stale,
    New,
}

/// Registers the component type at `type_path` in `world` if it is registered locally for
/// reflection, returning its id.
fn register_type_path(
    world: &mut World,
    registry: &TypeRegistry,
    type_path: &str,
) -> Option<ComponentId> {
    Some(
        registry
            .get_with_type_path(type_path)?
            .data::<ReflectComponent>()?
            .register_component(world),
    )
}

/// What a request covers, deciding which recorded components it can remove.
#[derive(Debug, Clone, Copy)]
pub(crate) enum Coverage<'a> {
    /// A tree poll, reading only the types with these sorted [`path_hash`]es and reporting no
    /// unserialized ones.
    Paths(&'a [u64]),
    /// A full poll, reading every serializable type and asking for these unserialized ones.
    Full(&'a [String]),
    /// A fetch of one entity, reporting every component.
    Entity,
}

impl Coverage<'_> {
    fn reads(&self, path_hash: u64) -> bool {
        match self {
            Coverage::Paths(paths) => paths.binary_search(&path_hash).is_ok(),
            Coverage::Full(_) | Coverage::Entity => true,
        }
    }

    fn reports_unserialized(&self, type_path: &str) -> bool {
        match self {
            Coverage::Paths(_) => false,
            Coverage::Full(has) => has.iter().any(|path| path == type_path),
            Coverage::Entity => true,
        }
    }
}

/// A component value a request returned, deserialized off the main thread.
#[derive(Debug)]
pub(crate) struct PolledComponent {
    pub(crate) type_path: String,
    pub(crate) json: Value,
    pub(crate) value: Decoded,
    path_hash: u64,
    json_hash: u64,
}

/// The local value of a [`PolledComponent`].
#[derive(Debug)]
pub(crate) enum Decoded {
    Value(Box<dyn Reflect>),
    Unregistered,
    Failed(String),
}

impl PolledComponent {
    /// Deserializes `json` as the component type at `type_path`.
    pub(crate) fn decode(registry: &TypeRegistry, type_path: String, json: Value) -> Self {
        let value = match registry
            .get_with_type_path(&type_path)
            .filter(|registration| registration.data::<ReflectComponent>().is_some())
        {
            None => Decoded::Unregistered,
            Some(registration) => {
                match TypedReflectDeserializer::new(registration, registry).deserialize(&json) {
                    Err(error) => Decoded::Failed(error.to_string()),
                    Ok(value) => match registration
                        .data::<ReflectFromReflect>()
                        .and_then(|from_reflect| from_reflect.from_reflect(value.as_ref()))
                    {
                        Some(value) => Decoded::Value(value),
                        None => Decoded::Failed("no concrete value".to_string()),
                    },
                }
            }
        };
        Self {
            path_hash: path_hash(&type_path),
            json_hash: json_hash(&json),
            type_path,
            json,
            value,
        }
    }
}

/// A hash of a component type path, for cheap comparisons.
pub(crate) fn path_hash(type_path: &str) -> u64 {
    FixedHasher.hash_one(type_path)
}

/// A hash of a JSON value, for cheap comparisons.
fn json_hash(json: &Value) -> u64 {
    fn feed(json: &Value, hasher: &mut impl Hasher) {
        match json {
            Value::Null => hasher.write_u8(0),
            Value::Bool(value) => hasher.write_u8(1 + u8::from(*value)),
            Value::Number(number) => {
                hasher.write_u8(3);
                match (number.as_u64(), number.as_i64(), number.as_f64()) {
                    (Some(value), _, _) => hasher.write_u64(value),
                    (_, Some(value), _) => hasher.write_i64(value),
                    (_, _, value) => hasher.write_u64(value.unwrap_or_default().to_bits()),
                }
            }
            Value::String(value) => {
                hasher.write_u8(4);
                hasher.write(value.as_bytes());
                hasher.write_u8(0xff);
            }
            Value::Array(values) => {
                hasher.write_u8(5);
                hasher.write_usize(values.len());
                values.iter().for_each(|value| feed(value, hasher));
            }
            Value::Object(map) => {
                hasher.write_u8(6);
                hasher.write_usize(map.len());
                for (key, value) in map {
                    hasher.write(key.as_bytes());
                    hasher.write_u8(0xff);
                    feed(value, hasher);
                }
            }
        }
    }
    let mut hasher = FixedHasher.build_hasher();
    feed(json, &mut hasher);
    hasher.finish()
}

fn is_relationship_target(world: &World, id: ComponentId) -> bool {
    matches!(
        world
            .components()
            .get_info(id)
            .and_then(|info| info.relationship_accessor()),
        Some(RelationshipAccessor::RelationshipTarget { .. })
    )
}

fn is_relationship(world: &World, id: ComponentId) -> bool {
    matches!(
        world
            .components()
            .get_info(id)
            .and_then(|info| info.relationship_accessor()),
        Some(RelationshipAccessor::Relationship { .. })
    )
}

/// The entities held directly by the fields of `value`.
fn entities_in(value: &dyn PartialReflect) -> impl Iterator<Item = Entity> + '_ {
    let fields: Box<dyn Iterator<Item = &dyn PartialReflect>> = match value.reflect_ref() {
        ReflectRef::Struct(value) => Box::new(value.iter_fields().map(|(_, field)| field)),
        ReflectRef::TupleStruct(value) => Box::new(value.iter_fields()),
        _ => Box::new(core::iter::empty()),
    };
    fields.filter_map(|field| field.try_downcast_ref::<Entity>().copied())
}

/// Whether a new value of the component `id` can change the entity tree: a name or a
/// relationship.
fn is_tree_type(world: &World, id: ComponentId) -> bool {
    Some(id) == world.component_id::<Name>()
        || world
            .components()
            .get_info(id)
            .is_some_and(|info| info.relationship_accessor().is_some())
}

/// Removes the record of the type with `path_hash` from `remote`, and the component if it was
/// inserted.
fn remove_written(world: &mut World, remote: Entity, path_hash: u64) {
    let children = world.component_id::<Children>();
    let Some(mut record) = world.get_mut::<RemoteComponents>(remote) else {
        return;
    };
    record.aside.retain(|entry| entry.path_hash != path_hash);
    let Some(position) = record
        .serialized
        .iter()
        .position(|entry| entry.path_hash == path_hash)
    else {
        return;
    };
    let id = record.serialized.swap_remove(position).id;
    if Some(id) == children {
        record.children.clear();
    }
    if !is_relationship_target(world, id) {
        world.entity_mut(remote).remove_by_id(id);
    }
}

/// Writes one component into `remote`, returning its id if it was written into the world rather
/// than kept aside.
///
/// A present mutable component is replaced with [`Reflect::set`], which runs no hooks and
/// shrinks collections. Other components are inserted. Relationship targets such as `Children`
/// are never written, since the hooks of their relationships build them. The order of `Children`
/// is recorded for [`RemoteWorld::order_children`].
fn write_component(
    world: &mut World,
    remote: Entity,
    registered: Option<Registered>,
    component: PolledComponent,
    written: &mut Vec<(String, Value)>,
) -> Option<ComponentId> {
    let PolledComponent {
        type_path,
        json,
        value,
        path_hash,
        json_hash,
    } = component;
    let children = match &value {
        Decoded::Value(value) => value
            .downcast_ref::<Children>()
            .map(|children| children.to_vec()),
        _ => None,
    };
    let outcome = match (value, &registered) {
        (Decoded::Unregistered, _) | (_, None) => Err((AsideReason::Unregistered, None)),
        (Decoded::Failed(error), _) => Err((AsideReason::Failed(error), None)),
        (Decoded::Value(value), Some(registered)) if !registered.insertable => {
            Err((AsideReason::Hooked, Some(value)))
        }
        (Decoded::Value(value), Some(registered)) => insert_value(
            world,
            remote,
            &registered.reflect_component,
            registered.id,
            value,
        ),
    };
    let id = registered.map(|registered| registered.id);

    if outcome.is_err() {
        remove_written(world, remote, path_hash);
    }
    let mut record = world.get_mut::<RemoteComponents>(remote)?;
    record.aside.retain(|entry| entry.path_hash != path_hash);
    match outcome {
        Ok(()) => {
            let id = id?;
            if let Some(children) = children {
                record.children = children;
            }
            match record
                .serialized
                .iter_mut()
                .find(|entry| entry.path_hash == path_hash)
            {
                Some(entry) => entry.json_hash = json_hash,
                None => record.serialized.push(Serialized {
                    id,
                    path_hash,
                    json_hash,
                }),
            }
            written.push((type_path, json));
            Some(id)
        }
        Err((reason, value)) => {
            record.aside.push(AsideComponent {
                id,
                type_path,
                json,
                value,
                reason,
                path_hash,
                json_hash,
            });
            None
        }
    }
}

type Aside = (AsideReason, Option<Box<dyn Reflect>>);

fn insert_value(
    world: &mut World,
    remote: Entity,
    reflect_component: &ReflectComponent,
    id: ComponentId,
    value: Box<dyn Reflect>,
) -> Result<(), Aside> {
    if is_relationship_target(world, id) {
        return Ok(());
    }
    if is_relationship(world, id)
        && entities_in(value.as_partial_reflect())
            .any(|target| world.get::<RemoteComponents>(target).is_none())
    {
        return Err((AsideReason::MissingTarget, Some(value)));
    }
    let mutable = world
        .components()
        .get_info(id)
        .is_some_and(ComponentInfo::mutable);
    let mut entity = world.entity_mut(remote);
    if mutable && entity.contains_id(id) {
        let Some(mut current) = reflect_component.reflect_mut(&mut entity) else {
            return Err((AsideReason::Failed("not readable".to_string()), Some(value)));
        };
        return current.set(value).map_err(|value| {
            (
                AsideReason::Failed("the value has another type".to_string()),
                Some(value),
            )
        });
    }
    let registry = entity.resource::<AppTypeRegistry>().clone();
    reflect_component.insert(&mut entity, value.as_partial_reflect(), &registry.read());
    if world
        .get_entity(remote)
        .is_ok_and(|entity| entity.contains_id(id))
    {
        Ok(())
    } else {
        Err((AsideReason::RemovedByHook, Some(value)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy_ecs::hierarchy::{ChildOf, Children};
    use bevy_picking::pointer::PointerId;
    use bevy_reflect::{prelude::ReflectDefault, TypePath, TypeRegistryArc};
    use serde_json::json;

    #[derive(Component, Reflect, Default, Debug, PartialEq)]
    #[reflect(Component, Default)]
    struct Health {
        current: f32,
        names: Vec<String>,
    }

    #[derive(Component, Reflect, Default, Debug, PartialEq)]
    #[component(immutable)]
    #[reflect(Component, Default)]
    struct Team(u8);

    #[derive(Component, Reflect, Default)]
    #[reflect(Component, Default)]
    #[require(Shadow)]
    struct Visible(bool);

    #[derive(Component, Reflect, Default)]
    #[reflect(Component, Default)]
    struct Shadow(u8);

    #[derive(Component, Reflect, Default)]
    #[component(on_add = spawn_one)]
    #[reflect(Component, Default)]
    struct Spawner(u8);

    fn spawn_one(mut world: DeferredWorld, _: HookContext) {
        world.commands().spawn_empty();
    }

    #[derive(Component, Reflect, Default)]
    #[reflect(Component, Default)]
    #[require(Spawner)]
    struct NeedsSpawner(u8);

    fn registry() -> AppTypeRegistry {
        let registry = AppTypeRegistry(TypeRegistryArc::default());
        {
            let mut registry = registry.write();
            registry.register::<Name>();
            registry.register::<ChildOf>();
            registry.register::<Children>();
            registry.register::<Health>();
            registry.register::<Team>();
            registry.register::<Visible>();
            registry.register::<Shadow>();
            registry.register::<Spawner>();
            registry.register::<NeedsSpawner>();
            registry.register::<PointerId>();
        }
        registry
    }

    fn remote(index: u32) -> Entity {
        Entity::from_raw_u32(index).unwrap()
    }

    fn at_generation(index: u32, generation: u32) -> Entity {
        let entity = remote(index);
        Entity::from_index_and_generation(
            entity.index(),
            entity.generation().after_versions(generation),
        )
    }

    fn polled(remote_world: &RemoteWorld, components: Value) -> Vec<PolledComponent> {
        let registry = remote_world.world.resource::<AppTypeRegistry>().read();
        let Value::Object(components) = components else {
            panic!("components are a map");
        };
        components
            .into_iter()
            .map(|(type_path, json)| PolledComponent::decode(&registry, type_path, json))
            .collect()
    }

    fn write(remote_world: &mut RemoteWorld, entity: Entity, components: Value) -> Written {
        let components = polled(remote_world, components);
        remote_world.register(
            components
                .iter()
                .map(|component| component.type_path.as_str()),
        );
        remote_world.write(entity, components, &[], Coverage::Entity, false)
    }

    fn spawned(entity: Entity) -> RemoteWorld {
        let mut remote_world = RemoteWorld::new(registry());
        remote_world.spawn(entity).unwrap();
        remote_world
    }

    fn record(remote_world: &RemoteWorld, entity: Entity) -> &RemoteComponents {
        remote_world.world.get::<RemoteComponents>(entity).unwrap()
    }

    #[test]
    fn spawn_at_rejects_a_later_generation_in_a_fresh_world() {
        let entity = at_generation(5, 3);
        let mut world = World::new();
        assert!(matches!(
            world.spawn_at(entity, ()),
            Err(SpawnError::Invalid(_))
        ));

        let mut remote_world = RemoteWorld::new(registry());
        assert_eq!(remote_world.spawn(entity), Ok(true));
        assert!(remote_world.world.get_entity(entity).is_ok());
        assert_eq!(remote_world.spawn(entity), Ok(false));
    }

    #[test]
    fn hand_placed_ids_are_never_allocated_again() {
        let mut remote_world = RemoteWorld::new(registry());
        let placed = [remote(20), at_generation(40, 2)];
        for entity in placed {
            remote_world.spawn(entity).unwrap();
        }
        remote_world.despawn(placed[0]);
        for _ in 0..100 {
            let entity = remote_world.world.spawn_empty().id();
            assert!(placed.iter().all(|placed| placed.index() != entity.index()));
        }
        assert!(remote_world.world.get_entity(placed[1]).is_ok());
    }

    #[test]
    fn a_respawn_at_the_same_index_catches_up() {
        let mut remote_world = spawned(remote(4));
        remote_world.despawn(remote(4));
        assert_eq!(remote_world.spawn(at_generation(4, 2)), Ok(true));
        assert!(remote_world.world.get_entity(remote(4)).is_err());
    }

    #[test]
    fn an_earlier_generation_means_the_remote_app_restarted() {
        let mut remote_world = spawned(at_generation(4, 2));
        assert_eq!(
            remote_world.spawn(at_generation(4, 1)),
            Err(SpawnRemoteError::Behind)
        );
        remote_world.despawn(at_generation(4, 2));
        assert_eq!(remote_world.spawn(remote(4)), Err(SpawnRemoteError::Behind));
    }

    #[test]
    fn ids_taken_by_the_remote_world_itself_are_skipped() {
        let mut remote_world = RemoteWorld::new(registry());
        assert_eq!(remote_world.spawn(remote(0)), Err(SpawnRemoteError::Taken));
        assert!(!remote_world.contains(remote(0)));
    }

    #[test]
    fn distant_generations_are_caught_up_over_several_calls() {
        let mut remote_world = RemoteWorld::new(registry());
        let entity = at_generation(3, MAX_GENERATION_STEPS + 10);
        assert_eq!(remote_world.spawn(entity), Err(SpawnRemoteError::Ahead));
        assert_eq!(remote_world.spawn(entity), Ok(true));
    }

    #[test]
    #[should_panic]
    fn pointer_id_panics_without_its_resource() {
        World::new().spawn(PointerId::Mouse);
    }

    #[test]
    fn pointer_id_is_kept_aside_for_its_hooks() {
        let entity = remote(3);
        let mut remote_world = spawned(entity);
        write(
            &mut remote_world,
            entity,
            json!({ PointerId::type_path(): "Mouse" }),
        );
        assert!(!remote_world.world.entity(entity).contains::<PointerId>());
        let aside = &record(&remote_world, entity).aside()[0];
        assert_eq!(aside.reason, AsideReason::Hooked);
        assert!(aside.value.is_some());
        assert!(aside.id.is_some());
    }

    fn entity_count(remote_world: &mut RemoteWorld) -> usize {
        remote_world
            .world
            .query::<Entity>()
            .iter(&remote_world.world)
            .count()
    }

    #[test]
    fn a_component_whose_hook_spawns_is_kept_aside() {
        let entity = remote(3);
        let mut remote_world = spawned(entity);
        let before = entity_count(&mut remote_world);
        write(
            &mut remote_world,
            entity,
            json!({ Spawner::type_path(): 1 }),
        );
        assert!(!remote_world.world.entity(entity).contains::<Spawner>());
        assert_eq!(
            record(&remote_world, entity).aside()[0].reason,
            AsideReason::Hooked
        );
        assert_eq!(entity_count(&mut remote_world), before);
        assert_eq!(remote_world.spawn(remote(4)), Ok(true));
    }

    #[test]
    fn a_required_component_with_hooks_keeps_its_requirer_aside() {
        let entity = remote(3);
        let mut remote_world = spawned(entity);
        write(
            &mut remote_world,
            entity,
            json!({ NeedsSpawner::type_path(): 1 }),
        );
        let world = remote_world.world.entity(entity);
        assert!(!world.contains::<NeedsSpawner>() && !world.contains::<Spawner>());
        assert_eq!(
            record(&remote_world, entity).aside()[0].reason,
            AsideReason::Hooked
        );
    }

    #[test]
    fn hook_free_components_are_inserted_with_their_values() {
        let entity = remote(3);
        let mut remote_world = spawned(entity);
        write(
            &mut remote_world,
            entity,
            json!({ Health::type_path(): { "current": 2.5, "names": ["a"] } }),
        );
        assert_eq!(
            remote_world.world.get::<Health>(entity),
            Some(&Health {
                current: 2.5,
                names: alloc::vec!["a".into()],
            })
        );
        assert!(record(&remote_world, entity).aside().is_empty());
    }

    #[test]
    fn each_type_is_probed_once() {
        let mut remote_world = RemoteWorld::new(registry());
        let reflect_component = |type_id| {
            remote_world
                .world
                .resource::<AppTypeRegistry>()
                .read()
                .get(type_id)
                .and_then(|registration| registration.data::<ReflectComponent>())
                .unwrap()
                .clone()
        };
        let health = reflect_component(core::any::TypeId::of::<Health>());
        let visible = reflect_component(core::any::TypeId::of::<Visible>());
        assert!(remote_world.probe.is_hook_free(&health));
        assert!(remote_world.probe.is_hook_free(&health));
        assert_eq!(remote_world.probe.hook_free.len(), 1);
        assert!(remote_world.probe.is_hook_free(&visible));
        assert!(remote_world.probe.is_hook_free(&visible));
        assert_eq!(remote_world.probe.hook_free.len(), 3);

        remote_world.register([Health::type_path(), Health::type_path()]);
        assert_eq!(remote_world.probe.hook_free.len(), 3);
    }

    #[test]
    fn inserting_children_from_json_detaches_the_children() {
        let registry = registry();
        let mut world = World::new();
        let parent = world.spawn_empty().id();
        let child = world.spawn(ChildOf(parent)).id();
        let registry = registry.read();
        let registration = registry.get(core::any::TypeId::of::<Children>()).unwrap();
        let value = TypedReflectDeserializer::new(registration, &registry)
            .deserialize(&json!([child]))
            .unwrap();
        registration.data::<ReflectComponent>().unwrap().insert(
            &mut world.entity_mut(parent),
            value.as_ref(),
            &registry,
        );
        assert!(world.get::<ChildOf>(child).is_none());
    }

    #[test]
    fn children_are_built_from_child_of_and_never_written() {
        let parent = remote(10);
        let child = remote(11);
        let mut remote_world = spawned(parent);
        remote_world.spawn(child).unwrap();
        write(
            &mut remote_world,
            parent,
            json!({ Children::type_path(): [child] }),
        );
        write(
            &mut remote_world,
            child,
            json!({ ChildOf::type_path(): parent }),
        );
        write(
            &mut remote_world,
            parent,
            json!({ Children::type_path(): [child, remote(12)] }),
        );

        let world = &remote_world.world;
        assert_eq!(
            world.get::<ChildOf>(child).map(ChildOf::parent),
            Some(parent)
        );
        assert_eq!(
            world
                .get::<Children>(parent)
                .map(|children| children.to_vec()),
            Some(alloc::vec![child])
        );
        let children = world.component_id::<Children>().unwrap();
        assert!(record(&remote_world, parent).is_serialized(children));
    }

    #[test]
    fn apply_keeps_a_stale_vec_tail() {
        let mut world = World::new();
        let entity = world
            .spawn(Health {
                current: 1.0,
                names: alloc::vec!["a".into(), "b".into()],
            })
            .id();
        let reflect_component = registry()
            .read()
            .get(core::any::TypeId::of::<Health>())
            .and_then(|registration| registration.data::<ReflectComponent>())
            .unwrap()
            .clone();
        reflect_component.apply(
            world.entity_mut(entity),
            &Health {
                current: 2.0,
                names: alloc::vec!["c".into()],
            },
        );
        assert_eq!(world.get::<Health>(entity).unwrap().names, ["c", "b"]);
    }

    #[test]
    fn a_shrinking_vec_is_replaced() {
        let entity = remote(3);
        let mut remote_world = spawned(entity);
        let health =
            |names: Value| json!({ Health::type_path(): { "current": 1.0, "names": names } });
        write(&mut remote_world, entity, health(json!(["a", "b"])));
        write(&mut remote_world, entity, health(json!(["c"])));
        assert_eq!(
            remote_world.world.get::<Health>(entity).unwrap().names,
            ["c"]
        );
    }

    #[test]
    fn immutable_components_are_replaced() {
        let entity = remote(3);
        let mut remote_world = spawned(entity);
        write(&mut remote_world, entity, json!({ Team::type_path(): 1 }));
        write(&mut remote_world, entity, json!({ Team::type_path(): 2 }));
        assert_eq!(remote_world.world.get::<Team>(entity), Some(&Team(2)));
    }

    #[test]
    fn unchanged_json_is_not_written_again() {
        let entity = remote(3);
        let mut remote_world = spawned(entity);
        let first = write(&mut remote_world, entity, json!({ Shadow::type_path(): 1 }));
        let again = write(&mut remote_world, entity, json!({ Shadow::type_path(): 1 }));
        assert!(first.changed && first.tree);
        assert_eq!(again, Written::default());

        let changed = write(&mut remote_world, entity, json!({ Shadow::type_path(): 2 }));
        assert!(changed.changed && !changed.tree);
        assert_eq!(remote_world.world.get::<Shadow>(entity).unwrap().0, 2);
    }

    #[test]
    fn required_components_are_not_recorded() {
        let entity = remote(3);
        let mut remote_world = spawned(entity);
        write(
            &mut remote_world,
            entity,
            json!({ Visible::type_path(): true }),
        );
        let world = &remote_world.world;
        assert!(world.entity(entity).contains::<Shadow>());
        let record = record(&remote_world, entity);
        assert!(record.is_serialized(world.component_id::<Visible>().unwrap()));
        assert!(!record.is_serialized(world.component_id::<Shadow>().unwrap()));
    }

    #[test]
    fn vanished_components_are_removed_within_the_coverage() {
        let entity = remote(3);
        let mut remote_world = spawned(entity);
        write(
            &mut remote_world,
            entity,
            json!({ Health::type_path(): { "current": 1.0, "names": [] }, Team::type_path(): 1, "demo::Unknown": 1 }),
        );
        let read = [path_hash(Name::type_path())];
        remote_world.write(entity, Vec::new(), &[], Coverage::Paths(&read), false);
        assert!(remote_world.world.entity(entity).contains::<Health>());

        let components = polled(&remote_world, json!({ Team::type_path(): 1 }));
        let written = remote_world.write(entity, components, &[], Coverage::Full(&[]), false);
        assert!(written.changed && written.tree);
        let world = &remote_world.world;
        assert!(!world.entity(entity).contains::<Health>());
        assert!(world.entity(entity).contains::<Team>());
        assert!(record(&remote_world, entity).aside().is_empty());
    }

    #[test]
    fn a_child_of_a_missing_parent_is_kept_aside_until_it_appears() {
        let parent = remote(20);
        let child = remote(21);
        let mut remote_world = spawned(child);
        let child_of = json!({ ChildOf::type_path(): parent });
        write(&mut remote_world, child, child_of.clone());
        assert!(remote_world.world.get::<ChildOf>(child).is_none());
        assert_eq!(
            record(&remote_world, child).aside()[0].reason,
            AsideReason::MissingTarget
        );
        assert_eq!(
            write(&mut remote_world, child, child_of.clone()),
            Written::default()
        );

        remote_world.spawn(parent).unwrap();
        let components = polled(&remote_world, child_of);
        remote_world.write(child, components, &[], Coverage::Entity, true);
        assert_eq!(
            remote_world
                .world
                .get::<ChildOf>(child)
                .map(ChildOf::parent),
            Some(parent)
        );
        assert!(record(&remote_world, child).aside().is_empty());
    }

    #[test]
    fn unregistered_and_mismatched_types_are_kept_aside() {
        let entity = remote(3);
        let mut remote_world = spawned(entity);
        write(
            &mut remote_world,
            entity,
            json!({ "demo::Unknown": 1, Team::type_path(): "nope" }),
        );
        let aside = record(&remote_world, entity).aside();
        assert_eq!(aside.len(), 2);
        assert!(aside
            .iter()
            .any(|entry| entry.reason == AsideReason::Unregistered && entry.id.is_none()));
        assert!(aside
            .iter()
            .any(|entry| matches!(entry.reason, AsideReason::Failed(_)) && entry.id.is_some()));
    }

    #[test]
    fn unserialized_types_follow_the_coverage() {
        let entity = remote(3);
        let mut remote_world = spawned(entity);
        let has = [Visible::type_path().to_string()];
        remote_world.write(entity, Vec::new(), &has, Coverage::Full(&has), false);
        let types = |remote_world: &RemoteWorld| -> Vec<String> {
            record(remote_world, entity)
                .unserialized()
                .iter()
                .map(|(_, type_path)| type_path.clone())
                .collect()
        };
        assert_eq!(types(&remote_world), has);

        remote_world.write(entity, Vec::new(), &[], Coverage::Paths(&[]), false);
        assert_eq!(types(&remote_world), has);

        let listed = ["demo::Handled".to_string()];
        remote_world.write(entity, Vec::new(), &listed, Coverage::Entity, false);
        assert_eq!(types(&remote_world), listed);

        remote_world.write(entity, Vec::new(), &[], Coverage::Full(&has), false);
        assert_eq!(
            types(&remote_world),
            listed,
            "a full poll only covers `has`"
        );
    }
}
