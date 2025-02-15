//! Types for declaring and storing [`Component`]s.

use crate::{
    archetype::ArchetypeFlags,
    bundle::BundleInfo,
    change_detection::{MaybeLocation, MAX_CHANGE_AGE},
    entity::{ComponentCloneCtx, Entity},
    query::DebugCheckedUnwrap,
    resource::Resource,
    storage::{SparseSetIndex, SparseSets, Table, TableRow},
    system::{Commands, Local, SystemParam},
    world::{DeferredWorld, FromWorld, World},
};
#[cfg(feature = "bevy_reflect")]
use alloc::boxed::Box;
use alloc::{borrow::Cow, format, vec::Vec};
pub use bevy_ecs_macros::Component;
use bevy_platform_support::collections::{HashMap, HashSet};
use bevy_platform_support::sync::Arc;
use bevy_ptr::{OwningPtr, UnsafeCellDeref};
#[cfg(feature = "bevy_reflect")]
use bevy_reflect::Reflect;
use bevy_utils::{
    staging::{MaybeStaged, StagedChanges, StagedRef, Stager},
    TypeIdMap,
};
use core::{
    alloc::Layout,
    any::{Any, TypeId},
    cell::UnsafeCell,
    fmt::Debug,
    marker::PhantomData,
    mem::needs_drop,
    ops::Deref,
    panic::Location,
};
use disqualified::ShortName;
use smallvec::SmallVec;
use thiserror::Error;

pub use bevy_ecs_macros::require;

/// A data type that can be used to store data for an [entity].
///
/// `Component` is a [derivable trait]: this means that a data type can implement it by applying a `#[derive(Component)]` attribute to it.
/// However, components must always satisfy the `Send + Sync + 'static` trait bounds.
///
/// [entity]: crate::entity
/// [derivable trait]: https://doc.rust-lang.org/book/appendix-03-derivable-traits.html
///
/// # Examples
///
/// Components can take many forms: they are usually structs, but can also be of every other kind of data type, like enums or zero sized types.
/// The following examples show how components are laid out in code.
///
/// ```
/// # use bevy_ecs::component::Component;
/// # struct Color;
/// #
/// // A component can contain data...
/// #[derive(Component)]
/// struct LicensePlate(String);
///
/// // ... but it can also be a zero-sized marker.
/// #[derive(Component)]
/// struct Car;
///
/// // Components can also be structs with named fields...
/// #[derive(Component)]
/// struct VehiclePerformance {
///     acceleration: f32,
///     top_speed: f32,
///     handling: f32,
/// }
///
/// // ... or enums.
/// #[derive(Component)]
/// enum WheelCount {
///     Two,
///     Three,
///     Four,
/// }
/// ```
///
/// # Component and data access
///
/// Components can be marked as immutable by adding the `#[component(immutable)]`
/// attribute when using the derive macro.
/// See the documentation for [`ComponentMutability`] for more details around this
/// feature.
///
/// See the [`entity`] module level documentation to learn how to add or remove components from an entity.
///
/// See the documentation for [`Query`] to learn how to access component data from a system.
///
/// [`entity`]: crate::entity#usage
/// [`Query`]: crate::system::Query
/// [`ComponentMutability`]: crate::component::ComponentMutability
///
/// # Choosing a storage type
///
/// Components can be stored in the world using different strategies with their own performance implications.
/// By default, components are added to the [`Table`] storage, which is optimized for query iteration.
///
/// Alternatively, components can be added to the [`SparseSet`] storage, which is optimized for component insertion and removal.
/// This is achieved by adding an additional `#[component(storage = "SparseSet")]` attribute to the derive one:
///
/// ```
/// # use bevy_ecs::component::Component;
/// #
/// #[derive(Component)]
/// #[component(storage = "SparseSet")]
/// struct ComponentA;
/// ```
///
/// [`Table`]: crate::storage::Table
/// [`SparseSet`]: crate::storage::SparseSet
///
/// # Required Components
///
/// Components can specify Required Components. If some [`Component`] `A` requires [`Component`] `B`,  then when `A` is inserted,
/// `B` will _also_ be initialized and inserted (if it was not manually specified).
///
/// The [`Default`] constructor will be used to initialize the component, by default:
///
/// ```
/// # use bevy_ecs::prelude::*;
/// #[derive(Component)]
/// #[require(B)]
/// struct A;
///
/// #[derive(Component, Default, PartialEq, Eq, Debug)]
/// struct B(usize);
///
/// # let mut world = World::default();
/// // This will implicitly also insert B with the Default constructor
/// let id = world.spawn(A).id();
/// assert_eq!(&B(0), world.entity(id).get::<B>().unwrap());
///
/// // This will _not_ implicitly insert B, because it was already provided
/// world.spawn((A, B(11)));
/// ```
///
/// Components can have more than one required component:
///
/// ```
/// # use bevy_ecs::prelude::*;
/// #[derive(Component)]
/// #[require(B, C)]
/// struct A;
///
/// #[derive(Component, Default, PartialEq, Eq, Debug)]
/// #[require(C)]
/// struct B(usize);
///
/// #[derive(Component, Default, PartialEq, Eq, Debug)]
/// struct C(u32);
///
/// # let mut world = World::default();
/// // This will implicitly also insert B and C with their Default constructors
/// let id = world.spawn(A).id();
/// assert_eq!(&B(0), world.entity(id).get::<B>().unwrap());
/// assert_eq!(&C(0), world.entity(id).get::<C>().unwrap());
/// ```
///
/// You can also define a custom constructor function or closure:
///
/// ```
/// # use bevy_ecs::prelude::*;
/// #[derive(Component)]
/// #[require(C(init_c))]
/// struct A;
///
/// #[derive(Component, PartialEq, Eq, Debug)]
/// #[require(C(|| C(20)))]
/// struct B;
///
/// #[derive(Component, PartialEq, Eq, Debug)]
/// struct C(usize);
///
/// fn init_c() -> C {
///     C(10)
/// }
///
/// # let mut world = World::default();
/// // This will implicitly also insert C with the `|| C(20)` constructor closure
/// let id = world.spawn(B).id();
/// assert_eq!(&C(20), world.entity(id).get::<C>().unwrap());
/// ```
///
/// Required components are _recursive_. This means, if a Required Component has required components,
/// those components will _also_ be inserted if they are missing:
///
/// ```
/// # use bevy_ecs::prelude::*;
/// #[derive(Component)]
/// #[require(B)]
/// struct A;
///
/// #[derive(Component, Default, PartialEq, Eq, Debug)]
/// #[require(C)]
/// struct B(usize);
///
/// #[derive(Component, Default, PartialEq, Eq, Debug)]
/// struct C(u32);
///
/// # let mut world = World::default();
/// // This will implicitly also insert B and C with their Default constructors
/// let id = world.spawn(A).id();
/// assert_eq!(&B(0), world.entity(id).get::<B>().unwrap());
/// assert_eq!(&C(0), world.entity(id).get::<C>().unwrap());
/// ```
///
/// Note that cycles in the "component require tree" will result in stack overflows when attempting to
/// insert a component.
///
/// This "multiple inheritance" pattern does mean that it is possible to have duplicate requires for a given type
/// at different levels of the inheritance tree:
///
/// ```
/// # use bevy_ecs::prelude::*;
/// #[derive(Component)]
/// struct X(usize);
///
/// #[derive(Component, Default)]
/// #[require(X(|| X(1)))]
/// struct Y;
///
/// #[derive(Component)]
/// #[require(
///     Y,
///     X(|| X(2)),
/// )]
/// struct Z;
///
/// # let mut world = World::default();
/// // In this case, the x2 constructor is used for X
/// let id = world.spawn(Z).id();
/// assert_eq!(2, world.entity(id).get::<X>().unwrap().0);
/// ```
///
/// In general, this shouldn't happen often, but when it does the algorithm for choosing the constructor from the tree is simple and predictable:
/// 1. A constructor from a direct `#[require()]`, if one exists, is selected with priority.
/// 2. Otherwise, perform a Depth First Search on the tree of requirements and select the first one found.
///
/// From a user perspective, just think about this as the following:
/// 1. Specifying a required component constructor for Foo directly on a spawned component Bar will result in that constructor being used (and overriding existing constructors lower in the inheritance tree). This is the classic "inheritance override" behavior people expect.
/// 2. For cases where "multiple inheritance" results in constructor clashes, Components should be listed in "importance order". List a component earlier in the requirement list to initialize its inheritance tree earlier.
///
/// ## Registering required components at runtime
///
/// In most cases, required components should be registered using the `require` attribute as shown above.
/// However, in some cases, it may be useful to register required components at runtime.
///
/// This can be done through [`World::register_required_components`] or  [`World::register_required_components_with`]
/// for the [`Default`] and custom constructors respectively:
///
/// ```
/// # use bevy_ecs::prelude::*;
/// #[derive(Component)]
/// struct A;
///
/// #[derive(Component, Default, PartialEq, Eq, Debug)]
/// struct B(usize);
///
/// #[derive(Component, PartialEq, Eq, Debug)]
/// struct C(u32);
///
/// # let mut world = World::default();
/// // Register B as required by A and C as required by B.
/// world.register_required_components::<A, B>();
/// world.register_required_components_with::<B, C>(|| C(2));
///
/// // This will implicitly also insert B with its Default constructor
/// // and C with the custom constructor defined by B.
/// let id = world.spawn(A).id();
/// assert_eq!(&B(0), world.entity(id).get::<B>().unwrap());
/// assert_eq!(&C(2), world.entity(id).get::<C>().unwrap());
/// ```
///
/// Similar rules as before apply to duplicate requires fer a given type at different levels
/// of the inheritance tree. `A` requiring `C` directly would take precedence over indirectly
/// requiring it through `A` requiring `B` and `B` requiring `C`.
///
/// Unlike with the `require` attribute, directly requiring the same component multiple times
/// for the same component will result in a panic. This is done to prevent conflicting constructors
/// and confusing ordering dependencies.
///
/// Note that requirements must currently be registered before the requiring component is inserted
/// into the world for the first time. Registering requirements after this will lead to a panic.
///
/// # Adding component's hooks
///
/// See [`ComponentHooks`] for a detailed explanation of component's hooks.
///
/// Alternatively to the example shown in [`ComponentHooks`]' documentation, hooks can be configured using following attributes:
/// - `#[component(on_add = on_add_function)]`
/// - `#[component(on_insert = on_insert_function)]`
/// - `#[component(on_replace = on_replace_function)]`
/// - `#[component(on_remove = on_remove_function)]`
///
/// ```
/// # use bevy_ecs::component::{Component, HookContext};
/// # use bevy_ecs::world::DeferredWorld;
/// # use bevy_ecs::entity::Entity;
/// # use bevy_ecs::component::ComponentId;
/// # use core::panic::Location;
/// #
/// #[derive(Component)]
/// #[component(on_add = my_on_add_hook)]
/// #[component(on_insert = my_on_insert_hook)]
/// // Another possible way of configuring hooks:
/// // #[component(on_add = my_on_add_hook, on_insert = my_on_insert_hook)]
/// //
/// // We don't have a replace or remove hook, so we can leave them out:
/// // #[component(on_replace = my_on_replace_hook, on_remove = my_on_remove_hook)]
/// struct ComponentA;
///
/// fn my_on_add_hook(world: DeferredWorld, context: HookContext) {
///     // ...
/// }
///
/// // You can also destructure items directly in the signature
/// fn my_on_insert_hook(world: DeferredWorld, HookContext { caller, .. }: HookContext) {
///     // ...
/// }
/// ```
///
/// # Implementing the trait for foreign types
///
/// As a consequence of the [orphan rule], it is not possible to separate into two different crates the implementation of `Component` from the definition of a type.
/// This means that it is not possible to directly have a type defined in a third party library as a component.
/// This important limitation can be easily worked around using the [newtype pattern]:
/// this makes it possible to locally define and implement `Component` for a tuple struct that wraps the foreign type.
/// The following example gives a demonstration of this pattern.
///
/// ```
/// // `Component` is defined in the `bevy_ecs` crate.
/// use bevy_ecs::component::Component;
///
/// // `Duration` is defined in the `std` crate.
/// use std::time::Duration;
///
/// // It is not possible to implement `Component` for `Duration` from this position, as they are
/// // both foreign items, defined in an external crate. However, nothing prevents to define a new
/// // `Cooldown` type that wraps `Duration`. As `Cooldown` is defined in a local crate, it is
/// // possible to implement `Component` for it.
/// #[derive(Component)]
/// struct Cooldown(Duration);
/// ```
///
/// [orphan rule]: https://doc.rust-lang.org/book/ch10-02-traits.html#implementing-a-trait-on-a-type
/// [newtype pattern]: https://doc.rust-lang.org/book/ch19-03-advanced-traits.html#using-the-newtype-pattern-to-implement-external-traits-on-external-types
///
/// # `!Sync` Components
/// A `!Sync` type cannot implement `Component`. However, it is possible to wrap a `Send` but not `Sync`
/// type in [`SyncCell`] or the currently unstable [`Exclusive`] to make it `Sync`. This forces only
/// having mutable access (`&mut T` only, never `&T`), but makes it safe to reference across multiple
/// threads.
///
/// This will fail to compile since `RefCell` is `!Sync`.
/// ```compile_fail
/// # use std::cell::RefCell;
/// # use bevy_ecs::component::Component;
/// #[derive(Component)]
/// struct NotSync {
///    counter: RefCell<usize>,
/// }
/// ```
///
/// This will compile since the `RefCell` is wrapped with `SyncCell`.
/// ```
/// # use std::cell::RefCell;
/// # use bevy_ecs::component::Component;
/// use bevy_utils::synccell::SyncCell;
///
/// // This will compile.
/// #[derive(Component)]
/// struct ActuallySync {
///    counter: SyncCell<RefCell<usize>>,
/// }
/// ```
///
/// [`SyncCell`]: bevy_utils::synccell::SyncCell
/// [`Exclusive`]: https://doc.rust-lang.org/nightly/std/sync/struct.Exclusive.html
#[diagnostic::on_unimplemented(
    message = "`{Self}` is not a `Component`",
    label = "invalid `Component`",
    note = "consider annotating `{Self}` with `#[derive(Component)]`"
)]
pub trait Component: Send + Sync + 'static {
    /// A constant indicating the storage type used for this component.
    const STORAGE_TYPE: StorageType;

    /// A marker type to assist Bevy with determining if this component is
    /// mutable, or immutable. Mutable components will have [`Component<Mutability = Mutable>`],
    /// while immutable components will instead have [`Component<Mutability = Immutable>`].
    ///
    /// * For a component to be mutable, this type must be [`Mutable`].
    /// * For a component to be immutable, this type must be [`Immutable`].
    type Mutability: ComponentMutability;

    /// Called when registering this component, allowing mutable access to its [`ComponentHooks`].
    #[deprecated(
        since = "0.16.0",
        note = "Use the individual hook methods instead (e.g., `Component::on_add`, etc.)"
    )]
    fn register_component_hooks(hooks: &mut ComponentHooks) {
        hooks.update_from_component::<Self>();
    }

    /// Gets the `on_add` [`ComponentHook`] for this [`Component`] if one is defined.
    fn on_add() -> Option<ComponentHook> {
        None
    }

    /// Gets the `on_insert` [`ComponentHook`] for this [`Component`] if one is defined.
    fn on_insert() -> Option<ComponentHook> {
        None
    }

    /// Gets the `on_replace` [`ComponentHook`] for this [`Component`] if one is defined.
    fn on_replace() -> Option<ComponentHook> {
        None
    }

    /// Gets the `on_remove` [`ComponentHook`] for this [`Component`] if one is defined.
    fn on_remove() -> Option<ComponentHook> {
        None
    }

    /// Gets the `on_despawn` [`ComponentHook`] for this [`Component`] if one is defined.
    fn on_despawn() -> Option<ComponentHook> {
        None
    }

    /// Registers required components.
    fn register_required_components(
        _component_id: ComponentId,
        _components: &mut impl ComponentsWriter,
        _required_components: &mut RequiredComponents,
        _inheritance_depth: u16,
        _recursion_check_stack: &mut Vec<ComponentId>,
    ) {
    }

    /// Called when registering this component, allowing to override clone function (or disable cloning altogether) for this component.
    ///
    /// See [Handlers section of `EntityClonerBuilder`](crate::entity::EntityClonerBuilder#handlers) to understand how this affects handler priority.
    #[inline]
    fn clone_behavior() -> ComponentCloneBehavior {
        ComponentCloneBehavior::Default
    }

    /// Visits entities stored on the component.
    #[inline]
    fn visit_entities(_this: &Self, _f: impl FnMut(Entity)) {}

    /// Returns pointers to every entity stored on the component. This will be used to remap entity references when this entity
    /// is cloned.
    #[inline]
    fn visit_entities_mut(_this: &mut Self, _f: impl FnMut(&mut Entity)) {}
}

mod private {
    pub trait Seal {}
}

/// The mutability option for a [`Component`]. This can either be:
/// * [`Mutable`]
/// * [`Immutable`]
///
/// This is controlled through either [`Component::Mutability`] or `#[component(immutable)]`
/// when using the derive macro.
///
/// Immutable components are guaranteed to never have an exclusive reference,
/// `&mut ...`, created while inserted onto an entity.
/// In all other ways, they are identical to mutable components.
/// This restriction allows hooks to observe all changes made to an immutable
/// component, effectively turning the `OnInsert` and `OnReplace` hooks into a
/// `OnMutate` hook.
/// This is not practical for mutable components, as the runtime cost of invoking
/// a hook for every exclusive reference created would be far too high.
///
/// # Examples
///
/// ```rust
/// # use bevy_ecs::component::Component;
/// #
/// #[derive(Component)]
/// #[component(immutable)]
/// struct ImmutableFoo;
/// ```
pub trait ComponentMutability: private::Seal + 'static {
    /// Boolean to indicate if this mutability setting implies a mutable or immutable
    /// component.
    const MUTABLE: bool;
}

/// Parameter indicating a [`Component`] is immutable.
///
/// See [`ComponentMutability`] for details.
pub struct Immutable;

impl private::Seal for Immutable {}
impl ComponentMutability for Immutable {
    const MUTABLE: bool = false;
}

/// Parameter indicating a [`Component`] is mutable.
///
/// See [`ComponentMutability`] for details.
pub struct Mutable;

impl private::Seal for Mutable {}
impl ComponentMutability for Mutable {
    const MUTABLE: bool = true;
}

/// The storage used for a specific component type.
///
/// # Examples
/// The [`StorageType`] for a component is configured via the derive attribute
///
/// ```
/// # use bevy_ecs::{prelude::*, component::*};
/// #[derive(Component)]
/// #[component(storage = "SparseSet")]
/// struct A;
/// ```
#[derive(Debug, Copy, Clone, Default, Eq, PartialEq)]
pub enum StorageType {
    /// Provides fast and cache-friendly iteration, but slower addition and removal of components.
    /// This is the default storage type.
    #[default]
    Table,
    /// Provides fast addition and removal of components, but slower iteration.
    SparseSet,
}

/// The type used for [`Component`] lifecycle hooks such as `on_add`, `on_insert` or `on_remove`.
pub type ComponentHook = for<'w> fn(DeferredWorld<'w>, HookContext);

/// Context provided to a [`ComponentHook`].
#[derive(Clone, Copy, Debug)]
pub struct HookContext {
    /// The [`Entity`] this hook was invoked for.
    pub entity: Entity,
    /// The [`ComponentId`] this hook was invoked for.
    pub component_id: ComponentId,
    /// The caller location is `Some` if the `track_caller` feature is enabled.
    pub caller: MaybeLocation,
}

/// [`World`]-mutating functions that run as part of lifecycle events of a [`Component`].
///
/// Hooks are functions that run when a component is added, overwritten, or removed from an entity.
/// These are intended to be used for structural side effects that need to happen when a component is added or removed,
/// and are not intended for general-purpose logic.
///
/// For example, you might use a hook to update a cached index when a component is added,
/// to clean up resources when a component is removed,
/// or to keep hierarchical data structures across entities in sync.
///
/// This information is stored in the [`ComponentInfo`] of the associated component.
///
/// There is two ways of configuring hooks for a component:
/// 1. Defining the [`Component::register_component_hooks`] method (see [`Component`])
/// 2. Using the [`World::register_component_hooks`] method
///
/// # Example 2
///
/// ```
/// use bevy_ecs::prelude::*;
/// use bevy_platform_support::collections::HashSet;
///
/// #[derive(Component)]
/// struct MyTrackedComponent;
///
/// #[derive(Resource, Default)]
/// struct TrackedEntities(HashSet<Entity>);
///
/// let mut world = World::new();
/// world.init_resource::<TrackedEntities>();
///
/// // No entities with `MyTrackedComponent` have been added yet, so we can safely add component hooks
/// let mut tracked_component_query = world.query::<&MyTrackedComponent>();
/// assert!(tracked_component_query.iter(&world).next().is_none());
///
/// world.register_component_hooks::<MyTrackedComponent>().on_add(|mut world, context| {
///    let mut tracked_entities = world.resource_mut::<TrackedEntities>();
///   tracked_entities.0.insert(context.entity);
/// });
///
/// world.register_component_hooks::<MyTrackedComponent>().on_remove(|mut world, context| {
///   let mut tracked_entities = world.resource_mut::<TrackedEntities>();
///   tracked_entities.0.remove(&context.entity);
/// });
///
/// let entity = world.spawn(MyTrackedComponent).id();
/// let tracked_entities = world.resource::<TrackedEntities>();
/// assert!(tracked_entities.0.contains(&entity));
///
/// world.despawn(entity);
/// let tracked_entities = world.resource::<TrackedEntities>();
/// assert!(!tracked_entities.0.contains(&entity));
/// ```
#[derive(Debug, Clone, Default)]
pub struct ComponentHooks {
    pub(crate) on_add: Option<ComponentHook>,
    pub(crate) on_insert: Option<ComponentHook>,
    pub(crate) on_replace: Option<ComponentHook>,
    pub(crate) on_remove: Option<ComponentHook>,
    pub(crate) on_despawn: Option<ComponentHook>,
}

impl ComponentHooks {
    pub(crate) fn update_from_component<C: Component + ?Sized>(&mut self) -> &mut Self {
        if let Some(hook) = C::on_add() {
            self.on_add(hook);
        }
        if let Some(hook) = C::on_insert() {
            self.on_insert(hook);
        }
        if let Some(hook) = C::on_replace() {
            self.on_replace(hook);
        }
        if let Some(hook) = C::on_remove() {
            self.on_remove(hook);
        }
        if let Some(hook) = C::on_despawn() {
            self.on_despawn(hook);
        }

        self
    }

    /// Register a [`ComponentHook`] that will be run when this component is added to an entity.
    /// An `on_add` hook will always run before `on_insert` hooks. Spawning an entity counts as
    /// adding all of its components.
    ///
    /// # Panics
    ///
    /// Will panic if the component already has an `on_add` hook
    pub fn on_add(&mut self, hook: ComponentHook) -> &mut Self {
        self.try_on_add(hook)
            .expect("Component already has an on_add hook")
    }

    /// Register a [`ComponentHook`] that will be run when this component is added (with `.insert`)
    /// or replaced.
    ///
    /// An `on_insert` hook always runs after any `on_add` hooks (if the entity didn't already have the component).
    ///
    /// # Warning
    ///
    /// The hook won't run if the component is already present and is only mutated, such as in a system via a query.
    /// As a result, this is *not* an appropriate mechanism for reliably updating indexes and other caches.
    ///
    /// # Panics
    ///
    /// Will panic if the component already has an `on_insert` hook
    pub fn on_insert(&mut self, hook: ComponentHook) -> &mut Self {
        self.try_on_insert(hook)
            .expect("Component already has an on_insert hook")
    }

    /// Register a [`ComponentHook`] that will be run when this component is about to be dropped,
    /// such as being replaced (with `.insert`) or removed.
    ///
    /// If this component is inserted onto an entity that already has it, this hook will run before the value is replaced,
    /// allowing access to the previous data just before it is dropped.
    /// This hook does *not* run if the entity did not already have this component.
    ///
    /// An `on_replace` hook always runs before any `on_remove` hooks (if the component is being removed from the entity).
    ///
    /// # Warning
    ///
    /// The hook won't run if the component is already present and is only mutated, such as in a system via a query.
    /// As a result, this is *not* an appropriate mechanism for reliably updating indexes and other caches.
    ///
    /// # Panics
    ///
    /// Will panic if the component already has an `on_replace` hook
    pub fn on_replace(&mut self, hook: ComponentHook) -> &mut Self {
        self.try_on_replace(hook)
            .expect("Component already has an on_replace hook")
    }

    /// Register a [`ComponentHook`] that will be run when this component is removed from an entity.
    /// Despawning an entity counts as removing all of its components.
    ///
    /// # Panics
    ///
    /// Will panic if the component already has an `on_remove` hook
    pub fn on_remove(&mut self, hook: ComponentHook) -> &mut Self {
        self.try_on_remove(hook)
            .expect("Component already has an on_remove hook")
    }

    /// Register a [`ComponentHook`] that will be run for each component on an entity when it is despawned.
    ///
    /// # Panics
    ///
    /// Will panic if the component already has an `on_despawn` hook
    pub fn on_despawn(&mut self, hook: ComponentHook) -> &mut Self {
        self.try_on_despawn(hook)
            .expect("Component already has an on_despawn hook")
    }

    /// Attempt to register a [`ComponentHook`] that will be run when this component is added to an entity.
    ///
    /// This is a fallible version of [`Self::on_add`].
    ///
    /// Returns `None` if the component already has an `on_add` hook.
    pub fn try_on_add(&mut self, hook: ComponentHook) -> Option<&mut Self> {
        if self.on_add.is_some() {
            return None;
        }
        self.on_add = Some(hook);
        Some(self)
    }

    /// Attempt to register a [`ComponentHook`] that will be run when this component is added (with `.insert`)
    ///
    /// This is a fallible version of [`Self::on_insert`].
    ///
    /// Returns `None` if the component already has an `on_insert` hook.
    pub fn try_on_insert(&mut self, hook: ComponentHook) -> Option<&mut Self> {
        if self.on_insert.is_some() {
            return None;
        }
        self.on_insert = Some(hook);
        Some(self)
    }

    /// Attempt to register a [`ComponentHook`] that will be run when this component is replaced (with `.insert`) or removed
    ///
    /// This is a fallible version of [`Self::on_replace`].
    ///
    /// Returns `None` if the component already has an `on_replace` hook.
    pub fn try_on_replace(&mut self, hook: ComponentHook) -> Option<&mut Self> {
        if self.on_replace.is_some() {
            return None;
        }
        self.on_replace = Some(hook);
        Some(self)
    }

    /// Attempt to register a [`ComponentHook`] that will be run when this component is removed from an entity.
    ///
    /// This is a fallible version of [`Self::on_remove`].
    ///
    /// Returns `None` if the component already has an `on_remove` hook.
    pub fn try_on_remove(&mut self, hook: ComponentHook) -> Option<&mut Self> {
        if self.on_remove.is_some() {
            return None;
        }
        self.on_remove = Some(hook);
        Some(self)
    }

    /// Attempt to register a [`ComponentHook`] that will be run for each component on an entity when it is despawned.
    ///
    /// This is a fallible version of [`Self::on_despawn`].
    ///
    /// Returns `None` if the component already has an `on_despawn` hook.
    pub fn try_on_despawn(&mut self, hook: ComponentHook) -> Option<&mut Self> {
        if self.on_despawn.is_some() {
            return None;
        }
        self.on_despawn = Some(hook);
        Some(self)
    }
}

/// Stores metadata for a type of component or resource stored in a specific [`World`].
#[derive(Debug, Clone)]
pub struct ComponentInfo {
    id: ComponentId,
    descriptor: ComponentDescriptor,
    hooks: ComponentHooks,
    required_components: RequiredComponents,
    required_by: HashSet<ComponentId>,
}

impl ComponentInfo {
    /// Returns a value uniquely identifying the current component.
    #[inline]
    pub fn id(&self) -> ComponentId {
        self.id
    }

    /// Returns the name of the current component.
    #[inline]
    pub fn name(&self) -> &str {
        &self.descriptor.name
    }

    /// Returns `true` if the current component is mutable.
    #[inline]
    pub fn mutable(&self) -> bool {
        self.descriptor.mutable
    }

    /// Returns [`ComponentCloneBehavior`] of the current component.
    #[inline]
    pub fn clone_behavior(&self) -> &ComponentCloneBehavior {
        &self.descriptor.clone_behavior
    }

    /// Returns the [`TypeId`] of the underlying component type.
    /// Returns `None` if the component does not correspond to a Rust type.
    #[inline]
    pub fn type_id(&self) -> Option<TypeId> {
        self.descriptor.type_id
    }

    /// Returns the layout used to store values of this component in memory.
    #[inline]
    pub fn layout(&self) -> Layout {
        self.descriptor.layout
    }

    #[inline]
    /// Get the function which should be called to clean up values of
    /// the underlying component type. This maps to the
    /// [`Drop`] implementation for 'normal' Rust components
    ///
    /// Returns `None` if values of the underlying component type don't
    /// need to be dropped, e.g. as reported by [`needs_drop`].
    pub fn drop(&self) -> Option<unsafe fn(OwningPtr<'_>)> {
        self.descriptor.drop
    }

    /// Returns a value indicating the storage strategy for the current component.
    #[inline]
    pub fn storage_type(&self) -> StorageType {
        self.descriptor.storage_type
    }

    /// Returns `true` if the underlying component type can be freely shared between threads.
    /// If this returns `false`, then extra care must be taken to ensure that components
    /// are not accessed from the wrong thread.
    #[inline]
    pub fn is_send_and_sync(&self) -> bool {
        self.descriptor.is_send_and_sync
    }

    /// Create a new [`ComponentInfo`].
    pub(crate) fn new(id: ComponentId, descriptor: ComponentDescriptor) -> Self {
        ComponentInfo {
            id,
            descriptor,
            hooks: Default::default(),
            required_components: Default::default(),
            required_by: Default::default(),
        }
    }

    /// Update the given flags to include any [`ComponentHook`] registered to self
    #[inline]
    pub(crate) fn update_archetype_flags(&self, flags: &mut ArchetypeFlags) {
        if self.hooks().on_add.is_some() {
            flags.insert(ArchetypeFlags::ON_ADD_HOOK);
        }
        if self.hooks().on_insert.is_some() {
            flags.insert(ArchetypeFlags::ON_INSERT_HOOK);
        }
        if self.hooks().on_replace.is_some() {
            flags.insert(ArchetypeFlags::ON_REPLACE_HOOK);
        }
        if self.hooks().on_remove.is_some() {
            flags.insert(ArchetypeFlags::ON_REMOVE_HOOK);
        }
        if self.hooks().on_despawn.is_some() {
            flags.insert(ArchetypeFlags::ON_DESPAWN_HOOK);
        }
    }

    /// Provides a reference to the collection of hooks associated with this [`Component`]
    pub fn hooks(&self) -> &ComponentHooks {
        &self.hooks
    }

    /// Retrieves the [`RequiredComponents`] collection, which contains all required components (and their constructors)
    /// needed by this component. This includes _recursive_ required components.
    pub fn required_components(&self) -> &RequiredComponents {
        &self.required_components
    }
}

/// A value which uniquely identifies the type of a [`Component`] or [`Resource`] within a
/// [`World`].
///
/// Each time a new `Component` type is registered within a `World` using
/// e.g. [`World::register_component`] or [`World::register_component_with_descriptor`]
/// or a Resource with e.g. [`World::init_resource`],
/// a corresponding `ComponentId` is created to track it.
///
/// While the distinction between `ComponentId` and [`TypeId`] may seem superficial, breaking them
/// into two separate but related concepts allows components to exist outside of Rust's type system.
/// Each Rust type registered as a `Component` will have a corresponding `ComponentId`, but additional
/// `ComponentId`s may exist in a `World` to track components which cannot be
/// represented as Rust types for scripting or other advanced use-cases.
///
/// A `ComponentId` is tightly coupled to its parent `World`. Attempting to use a `ComponentId` from
/// one `World` to access the metadata of a `Component` in a different `World` is undefined behavior
/// and must not be attempted.
///
/// Given a type `T` which implements [`Component`], the `ComponentId` for `T` can be retrieved
/// from a `World` using [`World::component_id()`] or via [`Components::component_id()`]. Access
/// to the `ComponentId` for a [`Resource`] is available via [`Components::resource_id()`].
#[derive(Debug, Copy, Clone, Hash, Ord, PartialOrd, Eq, PartialEq)]
#[cfg_attr(
    feature = "bevy_reflect",
    derive(Reflect),
    reflect(Debug, Hash, PartialEq)
)]
pub struct ComponentId(usize);

impl ComponentId {
    /// Creates a new [`ComponentId`].
    ///
    /// The `index` is a unique value associated with each type of component in a given world.
    /// Usually, this value is taken from a counter incremented for each type of component registered with the world.
    #[inline]
    pub const fn new(index: usize) -> ComponentId {
        ComponentId(index)
    }

    /// Returns the index of the current component.
    #[inline]
    pub fn index(self) -> usize {
        self.0
    }
}

impl SparseSetIndex for ComponentId {
    #[inline]
    fn sparse_set_index(&self) -> usize {
        self.index()
    }

    #[inline]
    fn get_sparse_set_index(value: usize) -> Self {
        Self(value)
    }
}

/// A value describing a component or resource, which may or may not correspond to a Rust type.
#[derive(Clone)]
pub struct ComponentDescriptor {
    name: Cow<'static, str>,
    // SAFETY: This must remain private. It must match the statically known StorageType of the
    // associated rust component type if one exists.
    storage_type: StorageType,
    // SAFETY: This must remain private. It must only be set to "true" if this component is
    // actually Send + Sync
    is_send_and_sync: bool,
    type_id: Option<TypeId>,
    layout: Layout,
    // SAFETY: this function must be safe to call with pointers pointing to items of the type
    // this descriptor describes.
    // None if the underlying type doesn't need to be dropped
    drop: Option<for<'a> unsafe fn(OwningPtr<'a>)>,
    mutable: bool,
    clone_behavior: ComponentCloneBehavior,
}

// We need to ignore the `drop` field in our `Debug` impl
impl Debug for ComponentDescriptor {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("ComponentDescriptor")
            .field("name", &self.name)
            .field("storage_type", &self.storage_type)
            .field("is_send_and_sync", &self.is_send_and_sync)
            .field("type_id", &self.type_id)
            .field("layout", &self.layout)
            .field("mutable", &self.mutable)
            .field("clone_behavior", &self.clone_behavior)
            .finish()
    }
}

impl ComponentDescriptor {
    /// # Safety
    ///
    /// `x` must point to a valid value of type `T`.
    unsafe fn drop_ptr<T>(x: OwningPtr<'_>) {
        // SAFETY: Contract is required to be upheld by the caller.
        unsafe {
            x.drop_as::<T>();
        }
    }

    /// Create a new `ComponentDescriptor` for the type `T`.
    pub fn new<T: Component>() -> Self {
        Self {
            name: Cow::Borrowed(core::any::type_name::<T>()),
            storage_type: T::STORAGE_TYPE,
            is_send_and_sync: true,
            type_id: Some(TypeId::of::<T>()),
            layout: Layout::new::<T>(),
            drop: needs_drop::<T>().then_some(Self::drop_ptr::<T> as _),
            mutable: T::Mutability::MUTABLE,
            clone_behavior: T::clone_behavior(),
        }
    }

    /// Create a new `ComponentDescriptor`.
    ///
    /// # Safety
    /// - the `drop` fn must be usable on a pointer with a value of the layout `layout`
    /// - the component type must be safe to access from any thread (Send + Sync in rust terms)
    pub unsafe fn new_with_layout(
        name: impl Into<Cow<'static, str>>,
        storage_type: StorageType,
        layout: Layout,
        drop: Option<for<'a> unsafe fn(OwningPtr<'a>)>,
        mutable: bool,
        clone_behavior: ComponentCloneBehavior,
    ) -> Self {
        Self {
            name: name.into(),
            storage_type,
            is_send_and_sync: true,
            type_id: None,
            layout,
            drop,
            mutable,
            clone_behavior,
        }
    }

    /// Create a new `ComponentDescriptor` for a resource.
    ///
    /// The [`StorageType`] for resources is always [`StorageType::Table`].
    pub fn new_resource<T: Resource>() -> Self {
        Self {
            name: Cow::Borrowed(core::any::type_name::<T>()),
            // PERF: `SparseStorage` may actually be a more
            // reasonable choice as `storage_type` for resources.
            storage_type: StorageType::Table,
            is_send_and_sync: true,
            type_id: Some(TypeId::of::<T>()),
            layout: Layout::new::<T>(),
            drop: needs_drop::<T>().then_some(Self::drop_ptr::<T> as _),
            mutable: true,
            clone_behavior: ComponentCloneBehavior::Default,
        }
    }

    fn new_non_send<T: Any>(storage_type: StorageType) -> Self {
        Self {
            name: Cow::Borrowed(core::any::type_name::<T>()),
            storage_type,
            is_send_and_sync: false,
            type_id: Some(TypeId::of::<T>()),
            layout: Layout::new::<T>(),
            drop: needs_drop::<T>().then_some(Self::drop_ptr::<T> as _),
            mutable: true,
            clone_behavior: ComponentCloneBehavior::Default,
        }
    }

    /// Returns a value indicating the storage strategy for the current component.
    #[inline]
    pub fn storage_type(&self) -> StorageType {
        self.storage_type
    }

    /// Returns the [`TypeId`] of the underlying component type.
    /// Returns `None` if the component does not correspond to a Rust type.
    #[inline]
    pub fn type_id(&self) -> Option<TypeId> {
        self.type_id
    }

    /// Returns the name of the current component.
    #[inline]
    pub fn name(&self) -> &str {
        self.name.as_ref()
    }

    /// Returns whether this component is mutable.
    #[inline]
    pub fn mutable(&self) -> bool {
        self.mutable
    }
}

/// Function type that can be used to clone an entity.
pub type ComponentCloneFn = fn(&mut Commands, &mut ComponentCloneCtx);

/// The clone behavior to use when cloning a [`Component`].
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub enum ComponentCloneBehavior {
    /// Uses the default behavior (which is passed to [`ComponentCloneBehavior::resolve`])
    #[default]
    Default,
    /// Do not clone this component.
    Ignore,
    /// Uses a custom [`ComponentCloneFn`].
    Custom(ComponentCloneFn),
    /// Uses a [`ComponentCloneFn`] that produces an empty version of the given relationship target.
    // TODO: this exists so that the current scene spawning code can know when to skip these components.
    // When we move to actually cloning entities in scene spawning code, this should be removed in favor of Custom, as the
    // distinction will no longer be necessary.
    RelationshipTarget(ComponentCloneFn),
}

impl ComponentCloneBehavior {
    /// Set clone handler based on `Clone` trait.
    ///
    /// If set as a handler for a component that is not the same as the one used to create this handler, it will panic.
    pub fn clone<C: Component + Clone>() -> Self {
        Self::Custom(component_clone_via_clone::<C>)
    }

    /// Set clone handler based on `Reflect` trait.
    #[cfg(feature = "bevy_reflect")]
    pub fn reflect_handler() -> Self {
        Self(Some(component_clone_via_reflect))
    }

    /// Set a custom handler for the component.
    pub fn custom_handler(handler: ComponentCloneFn) -> Self {
        Self(Some(handler))
    }

    /// Get [`ComponentCloneFn`] representing this handler or `None` if set to default handler.
    pub fn get_handler(&self) -> Option<ComponentCloneFn> {
        self.0
    }
}

/// A registry of component clone handlers. Allows to set global default and per-component clone function for all components in the world.
#[derive(Debug)]
pub struct ComponentCloneHandlers {
    handlers: Vec<Option<ComponentCloneFn>>,
    default_handler: ComponentCloneFn,
}

/// A trait that allows reading a collection of [`ComponentCloneHandler`]s
pub trait ComponentCloneHandlersReader {
    /// Returns the currently registered default clone handler.
    fn get_default_clone_handler(&self) -> ComponentCloneFn;

    /// Returns the [`ComponentCloneHandler`] for this [`ComponentId`].
    /// If no [`ComponentCloneFn`] is specified for this id, the inner value will be [`None`], and the default handler may be used.
    ///
    /// This will return an incorrect result if `id` did not come from the same world as `self`.
    ///
    /// See also [`get_clone_handler`](ComponentsReader::get_clone_handler).
    fn get_special_clone_handler(&self, id: ComponentId) -> ComponentCloneHandler;

    /// Checks if the specified component has a registered [`ComponentCloneFn`]. If not, the component will use the default global handler.
    ///
    /// This will return an incorrect result if `id` did not come from the same world as `self`.
    #[inline]
    fn is_clone_handler_registered(&self, id: ComponentId) -> bool {
        self.get_special_clone_handler(id).0.is_some()
    }

    /// Gets a handler to clone a component. This can be one of the following:
    /// - Custom clone function for this specific component.
    /// - Default global handler.
    /// - A [`component_clone_ignore`] (no cloning).
    ///
    /// This will return an incorrect result if `id` did not come from the same world as `self`.
    #[inline]
    fn get_clone_handler(&self, id: ComponentId) -> ComponentCloneFn {
        match self.get_special_clone_handler(id).0 {
            Some(handler) => handler,
            None => self.get_default_clone_handler(),
        }
    }
}

/// A trait that allows writing a collection of [`ComponentCloneHandler`]s
pub trait ComponentCloneHandlersWriter: ComponentCloneHandlersReader {
    /// Sets the [`ComponentCloneHandler`] for the [`ComponentId`].
    /// If the inner [`ComponentCloneFn`] is [`None`], the component will use the default handler.
    fn set_clone_handler(&mut self, id: ComponentId, handler: ComponentCloneHandler);

    /// Sets the default [`ComponentCloneFn`] for this collection.
    fn set_default_clone_handler(&mut self, handler: ComponentCloneFn);
}

impl ComponentCloneHandlersReader for ComponentCloneHandlers {
    #[inline]
    fn get_default_clone_handler(&self) -> ComponentCloneFn {
        self.default_handler
    }

    #[inline]
    fn get_special_clone_handler(&self, id: ComponentId) -> ComponentCloneHandler {
        self.handlers
            .get(id.0)
            .copied()
            .map(ComponentCloneHandler)
            .unwrap_or(ComponentCloneHandler(None))
    }
}

impl ComponentCloneHandlersWriter for ComponentCloneHandlers {
    fn set_clone_handler(&mut self, id: ComponentId, handler: ComponentCloneHandler) {
        if id.0 >= self.handlers.len() {
            self.handlers.resize(id.0 + 1, None);
        }
        self.handlers[id.0] = handler.0;
    }

    fn set_default_clone_handler(&mut self, handler: ComponentCloneFn) {
        self.default_handler = handler;
    }
}

impl Default for ComponentCloneHandlers {
    fn default() -> Self {
        Self {
            handlers: Default::default(),
            #[cfg(feature = "bevy_reflect")]
            default_handler: component_clone_via_reflect,
            #[cfg(not(feature = "bevy_reflect"))]
            default_handler: component_clone_ignore,
    pub fn reflect() -> Self {
        Self::Custom(component_clone_via_reflect)
    }

    /// Returns the "global default"
    pub fn global_default_fn() -> ComponentCloneFn {
        #[cfg(feature = "bevy_reflect")]
        return component_clone_via_reflect;
        #[cfg(not(feature = "bevy_reflect"))]
        return component_clone_ignore;
    }

    /// Resolves the [`ComponentCloneBehavior`] to a [`ComponentCloneFn`]. If [`ComponentCloneBehavior::Default`] is
    /// specified, the given `default` function will be used.
    pub fn resolve(&self, default: ComponentCloneFn) -> ComponentCloneFn {
        match self {
            ComponentCloneBehavior::Default => default,
            ComponentCloneBehavior::Ignore => component_clone_ignore,
            ComponentCloneBehavior::Custom(custom)
            | ComponentCloneBehavior::RelationshipTarget(custom) => *custom,
        }
    }
}

/// Stores metadata associated with each kind of [`Component`] in a given [`World`].
#[derive(Debug, Default)]
pub struct Components {
    components: Vec<ComponentInfo>,
    indices: TypeIdMap<ComponentId>,
    resource_indices: TypeIdMap<ComponentId>,
}

/// Stores metadata associated with each kind of [`Component`] in a given [`World`].
#[derive(Debug, Default)]
pub struct StagedComponents {
    components: Vec<ComponentInfo>,
    indices: TypeIdMap<ComponentId>,
    resource_indices: TypeIdMap<ComponentId>,
    component_clone_handlers: HashMap<ComponentId, ComponentCloneHandler>,
}

impl StagedChanges for StagedComponents {
    type Cold = Components;

    fn apply_staged(&mut self, storage: &mut Self::Cold) {
        storage.components.append(&mut self.components);
        storage.indices.extend(self.indices.drain());
        storage
            .resource_indices
            .extend(self.resource_indices.drain());
        for (id, handler) in self.component_clone_handlers.drain() {
            storage
                .component_clone_handlers
                .set_clone_handler(id, handler);
        }
    }

    fn any_staged(&self) -> bool {
        !self.components.is_empty()
    }
}

/// The [`Deref`] trait ties the lifetime of the returned reference to the lifetime of the value itself.
/// However, when the returned reference has nothing to do with the value containing it, this behavior is undesirable.
/// This trait gets around that by putting the lifetime in the trait, allowing a number of niche conviniencies.
pub trait DerefByLifetime<'a>: Deref {
    /// Corresponds to [`Deref::deref`]
    fn deref_lifetime(&self) -> &'a Self::Target;
}

impl<'a, T> DerefByLifetime<'a> for &'a T {
    #[inline]
    fn deref_lifetime(&self) -> &'a Self::Target {
        self
    }
}

impl<'a, C: DerefByLifetime<'a>, S: DerefByLifetime<'a, Target = C::Target>> DerefByLifetime<'a>
    for MaybeStaged<C, S>
{
    #[inline]
    fn deref_lifetime(&self) -> &'a Self::Target {
        match self {
            MaybeStaged::Cold(c) => c.deref_lifetime(),
            MaybeStaged::Staged(s) => s.deref_lifetime(),
        }
    }
}

/// Allows [`ComponentInfo::name`] to be retrieved with a lifetime from anything that implements [`DerefByLifetime`] for [`ComponentInfo`].
pub struct ComponentNameFromRef<'a, T: DerefByLifetime<'a, Target = ComponentInfo>>(
    pub T,
    pub PhantomData<&'a ()>,
);

impl<'a, T: DerefByLifetime<'a, Target = ComponentInfo>> Deref for ComponentNameFromRef<'a, T> {
    type Target = str;

    #[inline]
    fn deref(&self) -> &'a Self::Target {
        self.0.deref_lifetime().name()
    }
}

impl<'a, T: DerefByLifetime<'a, Target = ComponentInfo>> DerefByLifetime<'a>
    for ComponentNameFromRef<'a, T>
{
    #[inline]
    fn deref_lifetime(&self) -> &'a Self::Target {
        self.0.deref_lifetime().name()
    }
}

/// Reports how "registered" a component is in a particular collection.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ComponentRegistrationStatus {
    /// The component is not registered at all.
    Unregistered,
    /// The component is fully registered in staged changes.
    /// Reading it will:
    /// - block other components from being registered.
    /// - never block other components from being read.
    /// - may lock to be read.
    Staged,
    /// The component is fully registered in cold storage.
    /// Reading it will:
    /// - never block other components from being registered.
    /// - never block other components from being read.
    /// - will *almost never lock to be read.
    Cold,
}

/// A trait that allows the user to read into a collection of registered [`Component`]s.
pub trait ComponentsReader: ComponentCloneHandlersReader {
    /// Returns the number of components registered with this instance.
    fn len(&self) -> usize;

    /// Returns `true` if there are no components registered with this instance. Otherwise, this returns `false`.
    #[inline]
    fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Gets the metadata associated with the given component.
    ///
    /// This will return an incorrect result if `id` did not come from the same world as `self`. It may return `None` or a garbage value.
    #[inline]
    fn get_info(&self, id: ComponentId) -> Option<impl DerefByLifetime<Target = ComponentInfo>> {
        if !self.is_id_valid(id) {
            return None;
        }

        // SAFETY: We know the id is valid
        let info = unsafe { self.get_info_unchecked(id) };

        Some(info)
    }

    /// Returns the name associated with the given component.
    ///
    /// This will return an incorrect result if `id` did not come from the same world as `self`. It may return `None` or a garbage value.
    #[inline]
    fn get_name(&self, id: ComponentId) -> Option<impl DerefByLifetime<Target = str>> {
        self.get_info(id)
            .map(|info| ComponentNameFromRef(info, PhantomData))
    }

    /// Gets the metadata associated with the given component.
    /// # Safety
    ///
    /// `id` must be a valid [`ComponentId`]
    unsafe fn get_info_unchecked(
        &self,
        id: ComponentId,
    ) -> impl DerefByLifetime<Target = ComponentInfo>;

    /// Returns true only if this `id` is valid on this collection of [`Component`]s
    fn is_id_valid(&self, id: ComponentId) -> bool;

    /// Returns true if this `id` is staged on this collection of [`Component`]s.
    /// If [`is_id_valid`](ComponentsReader::is_id_valid) is not true, this value is meanngless.
    /// See also [`get`]
    fn is_id_staged(&self, id: ComponentId) -> bool;

    /// Gets the [`ComponentRegistrationStatus`] for a [`ComponentId`].
    /// See [`ComponentRegistrationStatus`] and its variants for more
    #[inline]
    fn get_registration_status(&self, id: ComponentId) -> ComponentRegistrationStatus {
        if !self.is_id_valid(id) {
            ComponentRegistrationStatus::Unregistered
        } else if self.is_id_staged(id) {
            ComponentRegistrationStatus::Staged
        } else {
            ComponentRegistrationStatus::Cold
        }
    }

    /// Type-erased equivalent of [`Components::component_id()`].
    fn get_id(&self, type_id: TypeId) -> Option<ComponentId>;

    /// Returns the [`ComponentId`] of the given [`Component`] type `T`.
    ///
    /// The returned `ComponentId` is specific to the `Components` instance
    /// it was retrieved from and should not be used with another `Components`
    /// instance.
    ///
    /// Returns [`None`] if the `Component` type has not
    /// yet been initialized using [`Components::register_component()`].
    ///
    /// ```
    /// use bevy_ecs::prelude::*;
    ///
    /// let mut world = World::new();
    ///
    /// #[derive(Component)]
    /// struct ComponentA;
    ///
    /// let component_a_id = world.register_component::<ComponentA>();
    ///
    /// assert_eq!(component_a_id, world.components().component_id::<ComponentA>().unwrap())
    /// ```
    ///
    /// # See also
    ///
    /// * [`Components::get_id()`]
    /// * [`Components::resource_id()`]
    /// * [`World::component_id()`]
    #[inline]
    fn component_id<T: Component>(&self) -> Option<ComponentId> {
        self.get_id(TypeId::of::<T>())
    }

    /// Type-erased equivalent of [`Components::resource_id()`].
    fn get_resource_id(&self, type_id: TypeId) -> Option<ComponentId>;

    /// Returns the [`ComponentId`] of the given [`Resource`] type `T`.
    ///
    /// The returned `ComponentId` is specific to the `Components` instance
    /// it was retrieved from and should not be used with another `Components`
    /// instance.
    ///
    /// Returns [`None`] if the `Resource` type has not
    /// yet been initialized using [`Components::register_resource()`].
    ///
    /// ```
    /// use bevy_ecs::prelude::*;
    ///
    /// let mut world = World::new();
    ///
    /// #[derive(Resource, Default)]
    /// struct ResourceA;
    ///
    /// let resource_a_id = world.init_resource::<ResourceA>();
    ///
    /// assert_eq!(resource_a_id, world.components().resource_id::<ResourceA>().unwrap())
    /// ```
    ///
    /// # See also
    ///
    /// * [`Components::component_id()`]
    /// * [`Components::get_resource_id()`]
    #[inline]
    fn resource_id<T: Resource>(&self) -> Option<ComponentId> {
        self.get_resource_id(TypeId::of::<T>())
    }
}

/// A trait that allows the user to read into a collection of registered Components.
pub trait ComponentsWriter: ComponentsReader + ComponentCloneHandlersWriter {
    /// Registers a [`Component`] of type `T` with this instance.
    /// If a component of this type has already been registered, this will return
    /// the ID of the pre-existing component.
    ///
    /// # See also
    ///
    /// * [`Components::component_id()`]
    /// * [`Components::register_component_with_descriptor()`]
    fn register_component<T: Component>(&mut self, storages: &mut Storages) -> ComponentId;

    /// Registers a component described by `descriptor`.
    ///
    /// # Note
    ///
    /// If this method is called multiple times with identical descriptors, a distinct [`ComponentId`]
    /// will be created for each one.
    ///
    /// # See also
    ///
    /// * [`Components::component_id()`]
    /// * [`Components::register_component()`]
    fn register_component_with_descriptor(
        &mut self,
        storages: &mut Storages,
        descriptor: ComponentDescriptor,
    ) -> ComponentId;

    // NOTE: This should maybe be private, but it is currently public so that `bevy_ecs_macros` can use it.
    //       We can't directly move this there either, because this uses `Components::get_required_by_mut`,
    //       which is private, and could be equally risky to expose to users.
    /// Registers the given component `R` and [required components] inherited from it as required by `T`,
    /// and adds `T` to their lists of requirees.
    ///
    /// The given `inheritance_depth` determines how many levels of inheritance deep the requirement is.
    /// A direct requirement has a depth of `0`, and each level of inheritance increases the depth by `1`.
    /// Lower depths are more specific requirements, and can override existing less specific registrations.
    ///
    /// The `recursion_check_stack` allows checking whether this component tried to register itself as its
    /// own (indirect) required component.
    ///
    /// This method does *not* register any components as required by components that require `T`.
    ///
    /// Only use this method if you know what you are doing. In most cases, you should instead use [`World::register_required_components`],
    /// or the equivalent method in `bevy_app::App`.
    ///
    /// [required component]: Component#required-components
    #[doc(hidden)]
    fn register_required_components_manual<T: Component, R: Component>(
        &mut self,
        storages: &mut Storages,
        required_components: &mut RequiredComponents,
        constructor: fn() -> R,
        inheritance_depth: u16,
        recursion_check_stack: &mut Vec<ComponentId>,
    );

    /// Registers a [`Resource`] of type `T` with this instance.
    /// If a resource of this type has already been registered, this will return
    /// the ID of the pre-existing resource.
    ///
    /// # See also
    ///
    /// * [`Components::resource_id()`]
    /// * [`Components::register_resource_with_descriptor()`]
    fn register_resource<T: Resource>(&mut self) -> ComponentId;

    /// Registers a [`Resource`] described by `descriptor`.
    ///
    /// # Note
    ///
    /// If this method is called multiple times with identical descriptors, a distinct [`ComponentId`]
    /// will be created for each one.
    ///
    /// # See also
    ///
    /// * [`Components::resource_id()`]
    /// * [`Components::register_resource()`]
    fn register_resource_with_descriptor(&mut self, descriptor: ComponentDescriptor)
        -> ComponentId;

    /// Registers a [non-send resource](crate::system::NonSend) of type `T` with this instance.
    /// If a resource of this type has already been registered, this will return
    /// the ID of the pre-existing resource.
    fn register_non_send<T: Any>(&mut self) -> ComponentId;
}

/// This trait provides easily misused read only access to [`Component`] collections intended for use only within this crate.
pub(crate) trait ComponentsInternalReader: ComponentsReader {
    /// Gets the [`RequiredComponentsStagedMut`] for a component if it exists.
    fn get_required_components(&self, id: ComponentId) -> Option<RequiredComponentsStagedRef>;

    /// Gets the [`RequiredByStagedMut`] for a component if it exists.
    fn get_required_by(&self, id: ComponentId) -> Option<RequiredByStagedRef>;
}

/// This trait provides easily misused write access to [`Component`] collections intended for use only within this crate.
#[expect(
    private_bounds,
    reason = "
        This trait is internal, so there is not real cost for the private bounds.
        This allows the more complex parts of this trait to be implemented automatically.
        Further, since any implementation of `ComponentsInternalWriter` would likely need to be in this file anyway, we aren't really giving anything up.
    "
)]
pub(crate) trait ComponentsInternalWriter:
    ComponentsInternalReader + ComponentsPrivateWriter
{
    /// Gets the [`ComponentHooks`] for a component if it exists.
    fn get_hooks_mut(&mut self, id: ComponentId) -> Option<&mut ComponentHooks>;

    /// Gets the [`RequiredComponentsStagedMut`] for a component if it exists.
    fn get_required_components_mut(
        &mut self,
        id: ComponentId,
    ) -> Option<RequiredComponentsStagedMut>;

    /// Gets the [`RequiredByStagedMut`] for a component if it exists.
    fn get_required_by_mut(&mut self, id: ComponentId) -> Option<RequiredByStagedMut>;

    /// Registers a [`ComponentDescriptor`] returning a unique [`ComponentId`].
    /// If this is called multiple times with the same arguments, it will produce different results.
    fn register_descriptor(&mut self, descriptor: ComponentDescriptor) -> ComponentId;

    /// Registers the given component `R` and [required components] inherited from it as required by `T`.
    ///
    /// When `T` is added to an entity, `R` will also be added if it was not already provided.
    /// The given `constructor` will be used for the creation of `R`.
    ///
    /// [required components]: Component#required-components
    ///
    /// # Safety
    ///
    /// The given component IDs `required` and `requiree` must be valid.
    ///
    /// # Errors
    ///
    /// Returns a [`RequiredComponentsError`] if the `required` component is already a directly required component for the `requiree`.
    ///
    /// Indirect requirements through other components are allowed. In those cases, the more specific
    /// registration will be used.
    unsafe fn register_required_components<R: Component>(
        &mut self,
        requiree: ComponentId,
        required: ComponentId,
        constructor: fn() -> R,
    ) -> Result<(), RequiredComponentsError> {
        // SAFETY: The caller ensures that the `requiree` is valid.
        let required_components = unsafe {
            self.get_required_components_mut(requiree)
                .debug_checked_unwrap()
        };

        // Cannot directly require the same component twice.
        if required_components
            .get(required)
            .is_some_and(|c| c.inheritance_depth == 0)
        {
            return Err(RequiredComponentsError::DuplicateRegistration(
                requiree, required,
            ));
        }

        // Register the required component for the requiree.
        // This is a direct requirement with a depth of `0`.
        required_components
            .working
            .register_by_id(required, constructor, 0);

        // Add the requiree to the list of components that require the required component.
        // SAFETY: The component is in the list of required components, so it must exist already.
        let required_by = unsafe { self.get_required_by_mut(required).debug_checked_unwrap() };
        required_by.working.insert(requiree);

        // SAFETY: The caller ensures that the `requiree` and `required` components are valid.
        let inherited_requirements =
            unsafe { self.register_inherited_required_components(requiree, required) };

        // Propagate the new required components up the chain to all components that require the requiree.
        if let Some(required_by) = self.get_required_by(requiree).map(|required_by| {
            required_by
                .iter_ids()
                .collect::<SmallVec<[ComponentId; 8]>>()
        }) {
            // `required` is now required by anything that `requiree` was required by.
            self.get_required_by_mut(required)
                .unwrap()
                .working
                .extend(required_by.iter().copied());
            for &required_by_id in required_by.iter() {
                // SAFETY: The component is in the list of required components, so it must exist already.
                let required_components = unsafe {
                    self.get_required_components_mut(required_by_id)
                        .debug_checked_unwrap()
                };

                // Register the original required component in the "parent" of the requiree.
                // The inheritance depth is 1 deeper than the `requiree` wrt `required_by_id`.
                let depth = required_components.get(requiree).expect("requiree is required by required_by_id, so its required_components must include requiree").inheritance_depth;
                required_components
                    .working
                    .register_by_id(required, constructor, depth + 1);

                for (component_id, component) in inherited_requirements.iter() {
                    // Register the required component.
                    // The inheritance depth of inherited components is whatever the requiree's
                    // depth is relative to `required_by_id`, plus the inheritance depth of the
                    // inherited component relative to the requiree, plus 1 to account for the
                    // requiree in between.
                    // SAFETY: Component ID and constructor match the ones on the original requiree.
                    //         The original requiree is responsible for making sure the registration is safe.
                    unsafe {
                        required_components.working.register_dynamic(
                            *component_id,
                            component.constructor.clone(),
                            component.inheritance_depth + depth + 1,
                        );
                    };
                }
            }
        }

        Ok(())
    }

    /// Registers the given component `R` and [required components] inherited from it as required by `T`,
    /// and adds `T` to their lists of requirees.
    ///
    /// The given `inheritance_depth` determines how many levels of inheritance deep the requirement is.
    /// A direct requirement has a depth of `0`, and each level of inheritance increases the depth by `1`.
    /// Lower depths are more specific requirements, and can override existing less specific registrations.
    ///
    /// This method does *not* register any components as required by components that require `T`.
    ///
    /// [required component]: Component#required-components
    ///
    /// # Safety
    ///
    /// The given component IDs `required` and `requiree` must be valid.
    unsafe fn register_required_components_manual_unchecked<R: Component>(
        &mut self,
        requiree: ComponentId,
        required: ComponentId,
        required_components: &mut RequiredComponents,
        constructor: fn() -> R,
        inheritance_depth: u16,
    ) {
        // Components cannot require themselves.
        if required == requiree {
            return;
        }

        // Register the required component `R` for the requiree.
        required_components.register_by_id(required, constructor, inheritance_depth);

        // Add the requiree to the list of components that require `R`.
        // SAFETY: The caller ensures that the component ID is valid.
        //         Assuming it is valid, the component is in the list of required components, so it must exist already.
        let required_by = unsafe { self.get_required_by_mut(required).debug_checked_unwrap() };
        required_by.working.insert(requiree);

        // Register the inherited required components for the requiree.
        let required: Vec<(ComponentId, RequiredComponent)> = self
            .get_info(required)
            .unwrap()
            .required_components()
            .0
            .iter()
            .map(|(id, component)| (*id, component.clone()))
            .collect();

        for (id, component) in required {
            // Register the inherited required components for the requiree.
            // The inheritance depth is increased by `1` since this is a component required by the original required component.
            required_components.register_dynamic(
                id,
                component.constructor.clone(),
                component.inheritance_depth + 1,
            );
            self.get_required_by_mut(id)
                .unwrap()
                .working
                .insert(requiree);
        }
    }
}

/// This trait provides low level access to [`Component`] collections intended for use only within this module.
trait ComponentsPrivateWriter {
    /// Registers the components inherited from `required` for the given `requiree`,
    /// returning the requirements in a list.
    ///
    /// # Safety
    ///
    /// The given component IDs `requiree` and `required` must be valid.
    unsafe fn register_inherited_required_components(
        &mut self,
        requiree: ComponentId,
        required: ComponentId,
    ) -> Vec<(ComponentId, RequiredComponent)>;

    /// # Safety
    ///
    /// The [`ComponentDescriptor`] must match the [`TypeId`]
    unsafe fn get_or_register_resource_with(
        &mut self,
        type_id: TypeId,
        func: impl FnOnce() -> ComponentDescriptor,
    ) -> ComponentId;

    fn register_component_internal<T: Component>(
        &mut self,
        storages: &mut Storages,
        recursion_check_stack: &mut Vec<ComponentId>,
    ) -> ComponentId;
}

impl<C: ComponentsReader + ComponentsInternalWriter + ComponentCloneHandlersWriter> ComponentsWriter
    for C
{
    fn register_component<T: Component>(&mut self, storages: &mut Storages) -> ComponentId {
        self.register_component_internal::<T>(storages, &mut Vec::new())
    }

    fn register_component_with_descriptor(
        &mut self,
        storages: &mut Storages,
        descriptor: ComponentDescriptor,
    ) -> ComponentId {
        self.register_descriptor(descriptor)
    }

    #[inline]
    fn register_required_components_manual<T: Component, R: Component>(
        &mut self,
        storages: &mut Storages,
        required_components: &mut RequiredComponents,
        constructor: fn() -> R,
        inheritance_depth: u16,
        recursion_check_stack: &mut Vec<ComponentId>,
    ) {
        let requiree = self.register_component_internal::<T>(storages, recursion_check_stack);
        let required = self.register_component_internal::<R>(storages, recursion_check_stack);

        // SAFETY: We just created the components.
        unsafe {
            self.register_required_components_manual_unchecked::<R>(
                requiree,
                required,
                required_components,
                constructor,
                inheritance_depth,
            );
        }
    }

    fn register_resource<T: Resource>(&mut self) -> ComponentId {
        // SAFETY: The [`ComponentDescriptor`] matches the [`TypeId`]
        unsafe {
            self.get_or_register_resource_with(TypeId::of::<T>(), || {
                ComponentDescriptor::new_resource::<T>()
            })
        }
    }

    #[inline]
    fn register_resource_with_descriptor(
        &mut self,
        descriptor: ComponentDescriptor,
    ) -> ComponentId {
        self.register_descriptor(descriptor)
    }

    fn register_non_send<T: Any>(&mut self) -> ComponentId {
        // SAFETY: The [`ComponentDescriptor`] matches the [`TypeId`]
        unsafe {
            self.get_or_register_resource_with(TypeId::of::<T>(), || {
                ComponentDescriptor::new_non_send::<T>(StorageType::default())
            })
        }
    }
}

impl ComponentCloneHandlersReader for StagedRef<'_, StagedComponents> {
    #[inline]
    fn get_default_clone_handler(&self) -> ComponentCloneFn {
        self.cold.get_default_clone_handler()
    }

    #[inline]
    fn get_special_clone_handler(&self, id: ComponentId) -> ComponentCloneHandler {
        self.staged
            .component_clone_handlers
            .get(&id)
            .cloned()
            .unwrap_or_else(|| self.cold.get_special_clone_handler(id))
    }
}

impl<'a> ComponentsReader for StagedRef<'a, StagedComponents> {
    #[inline]
    fn len(&self) -> usize {
        self.cold.len() + self.staged.components.len()
    }

    #[inline]
    unsafe fn get_info_unchecked(
        &self,
        id: ComponentId,
    ) -> impl DerefByLifetime<Target = ComponentInfo> {
        debug_assert!(id.index() < self.len());
        if self.is_id_staged(id) {
            // SAFETY: The caller ensures `id` is valid.
            MaybeStaged::Staged(unsafe {
                self.staged.components.get_unchecked(id.0 - self.cold.len())
            })
        } else {
            // SAFETY: The caller ensures `id` is valid.
            MaybeStaged::Cold(unsafe { self.cold.get_info_unchecked(id) })
        }
    }

    #[inline]
    fn is_id_valid(&self, id: ComponentId) -> bool {
        self.len() > id.0
    }

    #[inline]
    fn get_id(&self, type_id: TypeId) -> Option<ComponentId> {
        self.staged
            .indices
            .get(&type_id)
            .copied()
            .or_else(|| self.cold.get_id(type_id))
    }

    #[inline]
    fn get_resource_id(&self, type_id: TypeId) -> Option<ComponentId> {
        self.staged
            .resource_indices
            .get(&type_id)
            .copied()
            .or_else(|| self.cold.get_resource_id(type_id))
    }

    #[inline]
    fn is_id_staged(&self, id: ComponentId) -> bool {
        self.cold.len() > id.0
    }
}

impl ComponentCloneHandlersReader for Stager<'_, StagedComponents> {
    #[inline]
    fn get_default_clone_handler(&self) -> ComponentCloneFn {
        self.as_staged_ref().get_default_clone_handler()
    }

    #[inline]
    fn get_special_clone_handler(&self, id: ComponentId) -> ComponentCloneHandler {
        self.as_staged_ref().get_special_clone_handler(id)
    }
}

impl<'a> ComponentsReader for Stager<'a, StagedComponents> {
    #[inline]
    fn len(&self) -> usize {
        self.as_staged_ref().len()
    }

    #[inline]
    unsafe fn get_info_unchecked(
        &self,
        id: ComponentId,
    ) -> impl DerefByLifetime<Target = ComponentInfo> {
        debug_assert!(id.index() < self.len());
        if self.is_id_staged(id) {
            // SAFETY: The caller ensures `id` is valid.
            MaybeStaged::Staged(unsafe {
                self.staged.components.get_unchecked(id.0 - self.cold.len())
            })
        } else {
            // SAFETY: The caller ensures `id` is valid.
            MaybeStaged::Cold(unsafe { self.cold.get_info_unchecked(id) })
        }
    }

    #[inline]
    fn is_id_valid(&self, id: ComponentId) -> bool {
        self.as_staged_ref().is_id_valid(id)
    }

    #[inline]
    fn get_id(&self, type_id: TypeId) -> Option<ComponentId> {
        self.as_staged_ref().get_id(type_id)
    }

    #[inline]
    fn get_resource_id(&self, type_id: TypeId) -> Option<ComponentId> {
        self.as_staged_ref().get_resource_id(type_id)
    }

    #[inline]
    fn is_id_staged(&self, id: ComponentId) -> bool {
        self.as_staged_ref().is_id_staged(id)
    }
}

impl ComponentCloneHandlersReader for Components {
    #[inline]
    fn get_default_clone_handler(&self) -> ComponentCloneFn {
        self.component_clone_handlers.get_default_clone_handler()
    }

    #[inline]
    fn get_special_clone_handler(&self, id: ComponentId) -> ComponentCloneHandler {
        self.component_clone_handlers.get_special_clone_handler(id)
    }
}

impl ComponentsReader for Components {
    #[inline]
    fn len(&self) -> usize {
        self.components.len()
    }

    #[inline]
    unsafe fn get_info_unchecked(
        &self,
        id: ComponentId,
    ) -> impl DerefByLifetime<Target = ComponentInfo> {
        debug_assert!(id.index() < self.components.len());
        // SAFETY: The caller ensures `id` is valid.
        unsafe { self.components.get_unchecked(id.0) }
    }

    #[inline]
    fn is_id_valid(&self, id: ComponentId) -> bool {
        self.components.len() > id.0
    }

    #[inline]
    fn get_id(&self, type_id: TypeId) -> Option<ComponentId> {
        self.indices.get(&type_id).copied()
    }

    #[inline]
    fn get_resource_id(&self, type_id: TypeId) -> Option<ComponentId> {
        self.resource_indices.get(&type_id).copied()
    }

    #[inline]
    fn is_id_staged(&self, _id: ComponentId) -> bool {
        false // this is cold storage, so nothing is staged.
    }
}

impl ComponentCloneHandlersWriter for Components {
    #[inline]
    fn set_clone_handler(&mut self, id: ComponentId, handler: ComponentCloneHandler) {
        self.component_clone_handlers.set_clone_handler(id, handler);
    }

    #[inline]
    fn set_default_clone_handler(&mut self, handler: ComponentCloneFn) {
        self.component_clone_handlers
            .set_default_clone_handler(handler);
    }
}

impl ComponentsPrivateWriter for Components {
    unsafe fn register_inherited_required_components(
        &mut self,
        requiree: ComponentId,
        required: ComponentId,
    ) -> Vec<(ComponentId, RequiredComponent)> {
        // Get required components inherited from the `required` component.
        let inherited_requirements: Vec<(ComponentId, RequiredComponent)> = {
            // SAFETY: The caller ensures that the `required` component is valid.
            let required_component_info = unsafe { self.get_info(required).debug_checked_unwrap() };
            required_component_info
                .required_components()
                .0
                .iter()
                .map(|(component_id, required_component)| {
                    (
                        *component_id,
                        RequiredComponent {
                            constructor: required_component.constructor.clone(),
                            // Add `1` to the inheritance depth since this will be registered
                            // for the component that requires `required`.
                            inheritance_depth: required_component.inheritance_depth + 1,
                        },
                    )
                })
                .collect()
        };

        // Register the new required components.
        for (component_id, component) in inherited_requirements.iter().cloned() {
            // SAFETY: The caller ensures that the `requiree` is valid.
            let required_components = unsafe {
                self.get_required_components_mut(requiree)
                    .debug_checked_unwrap()
            };

            // Register the required component for the requiree.
            // SAFETY: Component ID and constructor match the ones on the original requiree.
            unsafe {
                required_components.working.register_dynamic(
                    component_id,
                    component.constructor,
                    component.inheritance_depth,
                );
            };

            // Add the requiree to the list of components that require the required component.
            // SAFETY: The caller ensures that the required components are valid.
            let required_by = unsafe {
                self.get_required_by_mut(component_id)
                    .debug_checked_unwrap()
            };
            required_by.working.insert(requiree);
        }

        inherited_requirements
    }

    unsafe fn get_or_register_resource_with(
        &mut self,
        type_id: TypeId,
        func: impl FnOnce() -> ComponentDescriptor,
    ) -> ComponentId {
        if let Some(id) = self.resource_indices.get(&type_id) {
            *id
        } else {
            let id = self.register_descriptor(func());
            self.resource_indices.insert(type_id, id);
            id
        }
    }

    fn register_component_internal<T: Component>(
        &mut self,
        storages: &mut Storages,
        recursion_check_stack: &mut Vec<ComponentId>,
    ) -> ComponentId {
        let type_id = TypeId::of::<T>();
        if let Some(id) = self.indices.get(&type_id) {
            return *id;
        }
        let id = self.register_descriptor(ComponentDescriptor::new::<T>());
        self.indices.insert(type_id, id);
        let mut required_components = RequiredComponents::default();
        T::register_required_components(
            id,
            self,
            storages,
            &mut required_components,
            0,
            recursion_check_stack,
        );
        let info = &mut self.components[id.index()];
        T::register_component_hooks(&mut info.hooks);
        info.required_components = required_components;
        let clone_handler = T::get_component_clone_handler();
        self.component_clone_handlers
            .set_clone_handler(id, clone_handler);

        id
    }
}

impl ComponentsInternalReader for Components {
    #[inline]
    fn get_required_components(&self, id: ComponentId) -> Option<RequiredComponentsStagedRef> {
        self.components
            .get(id.0)
            .map(|info| RequiredComponentsStagedRef {
                working: &info.required_components,
                cold: None,
            })
    }

    #[inline]
    fn get_required_by(&self, id: ComponentId) -> Option<RequiredByStagedRef> {
        self.components.get(id.0).map(|info| RequiredByStagedRef {
            working: &info.required_by,
            cold: None,
        })
    }
}

impl ComponentsInternalWriter for Components {
    #[inline]
    fn get_hooks_mut(&mut self, id: ComponentId) -> Option<&mut ComponentHooks> {
        self.components.get_mut(id.0).map(|info| &mut info.hooks)
    }

    #[inline]
    fn get_required_components_mut(
        &mut self,
        id: ComponentId,
    ) -> Option<RequiredComponentsStagedMut> {
        self.components
            .get_mut(id.0)
            .map(|info| RequiredComponentsStagedMut {
                working: &mut info.required_components,
                cold: None,
            })
    }

    #[inline]
    fn get_required_by_mut(&mut self, id: ComponentId) -> Option<RequiredByStagedMut> {
        self.components
            .get_mut(id.0)
            .map(|info| RequiredByStagedMut {
                working: &mut info.required_by,
                cold: None,
            })
    }

    #[inline]
    fn register_descriptor(&mut self, descriptor: ComponentDescriptor) -> ComponentId {
        let component_id = ComponentId(self.components.len());
        self.components
            .push(ComponentInfo::new(component_id, descriptor));
        component_id
    }
}

impl Components {
    /// Gets an iterator over all components registered with this instance.
    pub fn iter(&self) -> impl Iterator<Item = &ComponentInfo> + '_ {
        self.components.iter()
    }
}

/// Allows modifying required components with potential staged changes.
pub(crate) struct RequiredComponentsStagedMut<'a> {
    working: &'a mut RequiredComponents,
    cold: Option<&'a RequiredComponents>,
}

/// Allows viewing required components with potential staged changes.
pub struct RequiredByStagedRef<'a> {
    working: &'a HashSet<ComponentId>,
    cold: Option<&'a HashSet<ComponentId>>,
}

/// Allows viewing required components with potential staged changes.
pub struct RequiredComponentsStagedRef<'a> {
    working: &'a RequiredComponents,
    cold: Option<&'a RequiredComponents>,
}

/// Allows modifying required components with potential staged changes.
pub(crate) struct RequiredByStagedMut<'a> {
    working: &'a mut HashSet<ComponentId>,
    #[expect(
        unused,
        reason = "Although we aren't using this now, it is useful to have."
    )]
    cold: Option<&'a HashSet<ComponentId>>,
}

impl<'a> RequiredComponentsStagedMut<'a> {
    /// Gets the [`RequiredComponent`] for this id if it exists.
    #[inline]
    pub fn get(&self, id: ComponentId) -> Option<&RequiredComponent> {
        self.working
            .0
            .get(&id)
            .or_else(|| self.cold.and_then(|cold| cold.0.get(&id)))
    }
}

impl<'a> RequiredComponentsStagedRef<'a> {
    /// Iterates the required components
    #[inline]
    pub fn iter(&self) -> impl Iterator<Item = &RequiredComponents> {
        core::iter::once(self.working).chain(self.cold)
    }

    /// See [`RequiredComponents::iter_ids`]
    #[inline]
    pub fn iter_ids(&self) -> impl Iterator<Item = ComponentId> + '_ {
        self.iter().flat_map(RequiredComponents::iter_ids)
    }

    /// Merges these required components into the `other`.
    #[inline]
    pub fn merge_into(&self, other: &mut RequiredComponents) {
        for c in self.iter() {
            other.merge(c);
        }
    }

    /// Gets the [`RequiredComponent`] for this id if it exists.
    #[inline]
    pub fn get(&self, id: ComponentId) -> Option<&RequiredComponent> {
        self.working
            .0
            .get(&id)
            .or_else(|| self.cold.and_then(|cold| cold.0.get(&id)))
    }
}

impl<'a> RequiredByStagedRef<'a> {
    /// Iterates the required components
    #[inline]
    pub fn iter(&self) -> impl Iterator<Item = &HashSet<ComponentId>> {
        [self.working].into_iter().chain(self.cold)
    }

    /// See [`RequiredComponents::iter_ids`]
    #[inline]
    pub fn iter_ids(&self) -> impl Iterator<Item = ComponentId> + '_ {
        self.iter().flat_map(|by| by.iter().copied())
    }
}

/// A value that tracks when a system ran relative to other systems.
/// This is used to power change detection.
///
/// *Note* that a system that hasn't been run yet has a `Tick` of 0.
#[derive(Copy, Clone, Default, Debug, Eq, Hash, PartialEq)]
#[cfg_attr(
    feature = "bevy_reflect",
    derive(Reflect),
    reflect(Debug, Hash, PartialEq)
)]
pub struct Tick {
    tick: u32,
}

impl Tick {
    /// The maximum relative age for a change tick.
    /// The value of this is equal to [`MAX_CHANGE_AGE`].
    ///
    /// Since change detection will not work for any ticks older than this,
    /// ticks are periodically scanned to ensure their relative values are below this.
    pub const MAX: Self = Self::new(MAX_CHANGE_AGE);

    /// Creates a new [`Tick`] wrapping the given value.
    #[inline]
    pub const fn new(tick: u32) -> Self {
        Self { tick }
    }

    /// Gets the value of this change tick.
    #[inline]
    pub const fn get(self) -> u32 {
        self.tick
    }

    /// Sets the value of this change tick.
    #[inline]
    pub fn set(&mut self, tick: u32) {
        self.tick = tick;
    }

    /// Returns `true` if this `Tick` occurred since the system's `last_run`.
    ///
    /// `this_run` is the current tick of the system, used as a reference to help deal with wraparound.
    #[inline]
    pub fn is_newer_than(self, last_run: Tick, this_run: Tick) -> bool {
        // This works even with wraparound because the world tick (`this_run`) is always "newer" than
        // `last_run` and `self.tick`, and we scan periodically to clamp `ComponentTicks` values
        // so they never get older than `u32::MAX` (the difference would overflow).
        //
        // The clamp here ensures determinism (since scans could differ between app runs).
        let ticks_since_insert = this_run.relative_to(self).tick.min(MAX_CHANGE_AGE);
        let ticks_since_system = this_run.relative_to(last_run).tick.min(MAX_CHANGE_AGE);

        ticks_since_system > ticks_since_insert
    }

    /// Returns a change tick representing the relationship between `self` and `other`.
    #[inline]
    pub(crate) fn relative_to(self, other: Self) -> Self {
        let tick = self.tick.wrapping_sub(other.tick);
        Self { tick }
    }

    /// Wraps this change tick's value if it exceeds [`Tick::MAX`].
    ///
    /// Returns `true` if wrapping was performed. Otherwise, returns `false`.
    #[inline]
    pub(crate) fn check_tick(&mut self, tick: Tick) -> bool {
        let age = tick.relative_to(*self);
        // This comparison assumes that `age` has not overflowed `u32::MAX` before, which will be true
        // so long as this check always runs before that can happen.
        if age.get() > Self::MAX.get() {
            *self = tick.relative_to(Self::MAX);
            true
        } else {
            false
        }
    }
}

/// Interior-mutable access to the [`Tick`]s for a single component or resource.
#[derive(Copy, Clone, Debug)]
pub struct TickCells<'a> {
    /// The tick indicating when the value was added to the world.
    pub added: &'a UnsafeCell<Tick>,
    /// The tick indicating the last time the value was modified.
    pub changed: &'a UnsafeCell<Tick>,
}

impl<'a> TickCells<'a> {
    /// # Safety
    /// All cells contained within must uphold the safety invariants of [`UnsafeCellDeref::read`].
    #[inline]
    pub(crate) unsafe fn read(&self) -> ComponentTicks {
        ComponentTicks {
            // SAFETY: The callers uphold the invariants for `read`.
            added: unsafe { self.added.read() },
            // SAFETY: The callers uphold the invariants for `read`.
            changed: unsafe { self.changed.read() },
        }
    }
}

/// Records when a component or resource was added and when it was last mutably dereferenced (or added).
#[derive(Copy, Clone, Debug)]
#[cfg_attr(feature = "bevy_reflect", derive(Reflect), reflect(Debug))]
pub struct ComponentTicks {
    /// Tick recording the time this component or resource was added.
    pub added: Tick,

    /// Tick recording the time this component or resource was most recently changed.
    pub changed: Tick,
}

impl ComponentTicks {
    /// Returns `true` if the component or resource was added after the system last ran
    /// (or the system is running for the first time).
    #[inline]
    pub fn is_added(&self, last_run: Tick, this_run: Tick) -> bool {
        self.added.is_newer_than(last_run, this_run)
    }

    /// Returns `true` if the component or resource was added or mutably dereferenced after the system last ran
    /// (or the system is running for the first time).
    #[inline]
    pub fn is_changed(&self, last_run: Tick, this_run: Tick) -> bool {
        self.changed.is_newer_than(last_run, this_run)
    }

    /// Creates a new instance with the same change tick for `added` and `changed`.
    pub fn new(change_tick: Tick) -> Self {
        Self {
            added: change_tick,
            changed: change_tick,
        }
    }

    /// Manually sets the change tick.
    ///
    /// This is normally done automatically via the [`DerefMut`](std::ops::DerefMut) implementation
    /// on [`Mut<T>`](crate::change_detection::Mut), [`ResMut<T>`](crate::change_detection::ResMut), etc.
    /// However, components and resources that make use of interior mutability might require manual updates.
    ///
    /// # Example
    /// ```no_run
    /// # use bevy_ecs::{world::World, component::ComponentTicks};
    /// let world: World = unimplemented!();
    /// let component_ticks: ComponentTicks = unimplemented!();
    ///
    /// component_ticks.set_changed(world.read_change_tick());
    /// ```
    #[inline]
    pub fn set_changed(&mut self, change_tick: Tick) {
        self.changed = change_tick;
    }
}

/// A [`SystemParam`] that provides access to the [`ComponentId`] for a specific component type.
///
/// # Example
/// ```
/// # use bevy_ecs::{system::Local, component::{Component, ComponentId, ComponentIdFor}};
/// #[derive(Component)]
/// struct Player;
/// fn my_system(component_id: ComponentIdFor<Player>) {
///     let component_id: ComponentId = component_id.get();
///     // ...
/// }
/// ```
#[derive(SystemParam)]
pub struct ComponentIdFor<'s, T: Component>(Local<'s, InitComponentId<T>>);

impl<T: Component> ComponentIdFor<'_, T> {
    /// Gets the [`ComponentId`] for the type `T`.
    #[inline]
    pub fn get(&self) -> ComponentId {
        **self
    }
}

impl<T: Component> Deref for ComponentIdFor<'_, T> {
    type Target = ComponentId;
    fn deref(&self) -> &Self::Target {
        &self.0.component_id
    }
}

impl<T: Component> From<ComponentIdFor<'_, T>> for ComponentId {
    #[inline]
    fn from(to_component_id: ComponentIdFor<T>) -> ComponentId {
        *to_component_id
    }
}

/// Initializes the [`ComponentId`] for a specific type when used with [`FromWorld`].
struct InitComponentId<T: Component> {
    component_id: ComponentId,
    marker: PhantomData<T>,
}

impl<T: Component> FromWorld for InitComponentId<T> {
    fn from_world(world: &mut World) -> Self {
        Self {
            component_id: world.register_component::<T>(),
            marker: PhantomData,
        }
    }
}

/// An error returned when the registration of a required component fails.
#[derive(Error, Debug)]
#[non_exhaustive]
pub enum RequiredComponentsError {
    /// The component is already a directly required component for the requiree.
    #[error("Component {0:?} already directly requires component {1:?}")]
    DuplicateRegistration(ComponentId, ComponentId),
    /// An archetype with the component that requires other components already exists
    #[error("An archetype with the component {0:?} that requires other components already exists")]
    ArchetypeExists(ComponentId),
}

/// A Required Component constructor. See [`Component`] for details.
#[derive(Clone)]
pub struct RequiredComponentConstructor(
    pub Arc<dyn Fn(&mut Table, &mut SparseSets, Tick, TableRow, Entity, MaybeLocation)>,
);

impl RequiredComponentConstructor {
    /// # Safety
    /// This is intended to only be called in the context of [`BundleInfo::write_components`] to initialized required components.
    /// Calling it _anywhere else_ should be considered unsafe.
    ///
    /// `table_row` and `entity` must correspond to a valid entity that currently needs a component initialized via the constructor stored
    /// on this [`RequiredComponentConstructor`]. The stored constructor must correspond to a component on `entity` that needs initialization.
    /// `table` and `sparse_sets` must correspond to storages on a world where `entity` needs this required component initialized.
    ///
    /// Again, don't call this anywhere but [`BundleInfo::write_components`].
    pub(crate) unsafe fn initialize(
        &self,
        table: &mut Table,
        sparse_sets: &mut SparseSets,
        change_tick: Tick,
        table_row: TableRow,
        entity: Entity,
        caller: MaybeLocation,
    ) {
        (self.0)(table, sparse_sets, change_tick, table_row, entity, caller);
    }
}

/// Metadata associated with a required component. See [`Component`] for details.
#[derive(Clone)]
pub struct RequiredComponent {
    /// The constructor used for the required component.
    pub constructor: RequiredComponentConstructor,

    /// The depth of the component requirement in the requirement hierarchy for this component.
    /// This is used for determining which constructor is used in cases where there are duplicate requires.
    ///
    /// For example, consider the inheritance tree `X -> Y -> Z`, where `->` indicates a requirement.
    /// `X -> Y` and `Y -> Z` are direct requirements with a depth of 0, while `Z` is only indirectly
    /// required for `X` with a depth of `1`.
    ///
    /// In cases where there are multiple conflicting requirements with the same depth, a higher priority
    /// will be given to components listed earlier in the `require` attribute, or to the latest added requirement
    /// if registered at runtime.
    pub inheritance_depth: u16,
}

/// The collection of metadata for components that are required for a given component.
///
/// For more information, see the "Required Components" section of [`Component`].
#[derive(Default, Clone)]
pub struct RequiredComponents(pub(crate) HashMap<ComponentId, RequiredComponent>);

impl Debug for RequiredComponents {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_tuple("RequiredComponents")
            .field(&self.0.keys())
            .finish()
    }
}

impl RequiredComponents {
    /// Registers a required component.
    ///
    /// If the component is already registered, it will be overwritten if the given inheritance depth
    /// is smaller than the depth of the existing registration. Otherwise, the new registration will be ignored.
    ///
    /// # Safety
    ///
    /// `component_id` must match the type initialized by `constructor`.
    /// `constructor` _must_ initialize a component for `component_id` in such a way that
    /// matches the storage type of the component. It must only use the given `table_row` or `Entity` to
    /// initialize the storage for `component_id` corresponding to the given entity.
    pub unsafe fn register_dynamic(
        &mut self,
        component_id: ComponentId,
        constructor: RequiredComponentConstructor,
        inheritance_depth: u16,
    ) {
        self.0
            .entry(component_id)
            .and_modify(|component| {
                if component.inheritance_depth > inheritance_depth {
                    // New registration is more specific than existing requirement
                    component.constructor = constructor.clone();
                    component.inheritance_depth = inheritance_depth;
                }
            })
            .or_insert(RequiredComponent {
                constructor,
                inheritance_depth,
            });
    }

    /// Registers a required component.
    ///
    /// If the component is already registered, it will be overwritten if the given inheritance depth
    /// is smaller than the depth of the existing registration. Otherwise, the new registration will be ignored.
    pub fn register<C: Component>(
        &mut self,
        components: &mut Components,
        constructor: fn() -> C,
        inheritance_depth: u16,
    ) {
        let component_id = components.register_component::<C>();
        self.register_by_id(component_id, constructor, inheritance_depth);
    }

    /// Registers the [`Component`] with the given ID as required if it exists.
    ///
    /// If the component is already registered, it will be overwritten if the given inheritance depth
    /// is smaller than the depth of the existing registration. Otherwise, the new registration will be ignored.
    pub fn register_by_id<C: Component>(
        &mut self,
        component_id: ComponentId,
        constructor: fn() -> C,
        inheritance_depth: u16,
    ) {
        let erased: RequiredComponentConstructor = RequiredComponentConstructor({
            // `portable-atomic-util` `Arc` is not able to coerce an unsized
            // type like `std::sync::Arc` can. Creating a `Box` first does the
            // coercion.
            //
            // This would be resolved by https://github.com/rust-lang/rust/issues/123430

            #[cfg(feature = "portable-atomic")]
            use alloc::boxed::Box;

            type Constructor = dyn for<'a, 'b> Fn(
                &'a mut Table,
                &'b mut SparseSets,
                Tick,
                TableRow,
                Entity,
                MaybeLocation,
            );

            #[cfg(feature = "portable-atomic")]
            type Intermediate<T> = Box<T>;

            #[cfg(not(feature = "portable-atomic"))]
            type Intermediate<T> = Arc<T>;

            let boxed: Intermediate<Constructor> = Intermediate::new(
                move |table, sparse_sets, change_tick, table_row, entity, caller| {
                    OwningPtr::make(constructor(), |ptr| {
                        // SAFETY: This will only be called in the context of `BundleInfo::write_components`, which will
                        // pass in a valid table_row and entity requiring a C constructor
                        // C::STORAGE_TYPE is the storage type associated with `component_id` / `C`
                        // `ptr` points to valid `C` data, which matches the type associated with `component_id`
                        unsafe {
                            BundleInfo::initialize_required_component(
                                table,
                                sparse_sets,
                                change_tick,
                                table_row,
                                entity,
                                component_id,
                                C::STORAGE_TYPE,
                                ptr,
                                caller,
                            );
                        }
                    });
                },
            );

            Arc::from(boxed)
        });

        // SAFETY:
        // `component_id` matches the type initialized by the `erased` constructor above.
        // `erased` initializes a component for `component_id` in such a way that
        // matches the storage type of the component. It only uses the given `table_row` or `Entity` to
        // initialize the storage corresponding to the given entity.
        unsafe { self.register_dynamic(component_id, erased, inheritance_depth) };
    }

    /// Iterates the ids of all required components. This includes recursive required components.
    pub fn iter_ids(&self) -> impl Iterator<Item = ComponentId> + '_ {
        self.0.keys().copied()
    }

    /// Removes components that are explicitly provided in a given [`Bundle`]. These components should
    /// be logically treated as normal components, not "required components".
    ///
    /// [`Bundle`]: crate::bundle::Bundle
    pub(crate) fn remove_explicit_components(&mut self, components: &[ComponentId]) {
        for component in components {
            self.0.remove(component);
        }
    }

    // Merges `required_components` into this collection. This only inserts a required component
    // if it _did not already exist_.
    pub(crate) fn merge(&mut self, required_components: &RequiredComponents) {
        for (id, constructor) in &required_components.0 {
            self.0.entry(*id).or_insert_with(|| constructor.clone());
        }
    }
}

// NOTE: This should maybe be private, but it is currently public so that `bevy_ecs_macros` can use it.
// This exists as a standalone function instead of being inlined into the component derive macro so as
// to reduce the amount of generated code.
#[doc(hidden)]
pub fn enforce_no_required_components_recursion(
    components: &impl ComponentsReader,
    recursion_check_stack: &[ComponentId],
) {
    if let Some((&requiree, check)) = recursion_check_stack.split_last() {
        if let Some(direct_recursion) = check
            .iter()
            .position(|&id| id == requiree)
            .map(|index| index == check.len() - 1)
        {
            panic!(
                "Recursive required components detected: {}\nhelp: {}",
                recursion_check_stack
                    .iter()
                    .map(|id| format!("{}", ShortName(components.get_name(*id).unwrap().deref())))
                    .collect::<Vec<_>>()
                    .join(" → "),
                if direct_recursion {
                    format!(
                        "Remove require({}).",
                        ShortName(components.get_name(requiree).unwrap().deref())
                    )
                } else {
                    "If this is intentional, consider merging the components.".into()
                }
            );
        }
    }
}

/// Component [clone handler function](ComponentCloneFn) implemented using the [`Clone`] trait.
/// Can be [set](Component::clone_behavior) as clone handler for the specific component it is implemented for.
/// It will panic if set as handler for any other component.
///
pub fn component_clone_via_clone<C: Clone + Component>(
    _commands: &mut Commands,
    ctx: &mut ComponentCloneCtx,
) {
    if let Some(component) = ctx.read_source_component::<C>() {
        ctx.write_target_component(component.clone());
    }
}

/// Component [clone handler function](ComponentCloneFn) implemented using reflect.
/// Can be [set](Component::clone_behavior) as clone handler for any registered component,
/// but only reflected components will be cloned.
///
/// To clone a component using this handler, the following must be true:
/// - World has [`AppTypeRegistry`](crate::reflect::AppTypeRegistry)
/// - Component has [`TypeId`]
/// - Component is registered
/// - Component has [`ReflectFromPtr`](bevy_reflect::ReflectFromPtr) registered
/// - Component has one of the following registered: [`ReflectFromReflect`](bevy_reflect::ReflectFromReflect),
///   [`ReflectDefault`](bevy_reflect::std_traits::ReflectDefault), [`ReflectFromWorld`](crate::reflect::ReflectFromWorld)
///
/// If any of the conditions is not satisfied, the component will be skipped.
///
/// See [`EntityClonerBuilder`](crate::entity::EntityClonerBuilder) for details.
#[cfg(feature = "bevy_reflect")]
pub fn component_clone_via_reflect(commands: &mut Commands, ctx: &mut ComponentCloneCtx) {
    let Some(app_registry) = ctx.type_registry().cloned() else {
        return;
    };
    let Some(source_component_reflect) = ctx.read_source_component_reflect() else {
        return;
    };
    let component_info = ctx.component_info();
    // checked in read_source_component_reflect
    let type_id = component_info.type_id().unwrap();
    let registry = app_registry.read();

    // Try to clone using ReflectFromReflect
    if let Some(reflect_from_reflect) =
        registry.get_type_data::<bevy_reflect::ReflectFromReflect>(type_id)
    {
        if let Some(mut component) =
            reflect_from_reflect.from_reflect(source_component_reflect.as_partial_reflect())
        {
            if let Some(reflect_component) =
                registry.get_type_data::<crate::reflect::ReflectComponent>(type_id)
            {
                reflect_component.visit_entities_mut(&mut *component, &mut |entity| {
                    *entity = ctx.entity_mapper().get_mapped(*entity);
                });
            }
            drop(registry);

            ctx.write_target_component_reflect(component);
            return;
        }
    }
    // Else, try to clone using ReflectDefault
    if let Some(reflect_default) =
        registry.get_type_data::<bevy_reflect::std_traits::ReflectDefault>(type_id)
    {
        let mut component = reflect_default.default();
        component.apply(source_component_reflect.as_partial_reflect());
        drop(registry);
        ctx.write_target_component_reflect(component);
        return;
    }
    // Otherwise, try to clone using ReflectFromWorld
    if let Some(reflect_from_world) =
        registry.get_type_data::<crate::reflect::ReflectFromWorld>(type_id)
    {
        let reflect_from_world = reflect_from_world.clone();
        let mut mapped_entities = Vec::new();
        if let Some(reflect_component) =
            registry.get_type_data::<crate::reflect::ReflectComponent>(type_id)
        {
            reflect_component.visit_entities(source_component_reflect, &mut |entity| {
                mapped_entities.push(entity);
            });
        }
        let source_component_cloned = source_component_reflect.clone_value();
        let component_layout = component_info.layout();
        let target = ctx.target();
        let component_id = ctx.component_id();
        for entity in mapped_entities.iter_mut() {
            *entity = ctx.entity_mapper().get_mapped(*entity);
        }
        drop(registry);
        commands.queue(move |world: &mut World| {
            let mut component = reflect_from_world.from_world(world);
            assert_eq!(type_id, (*component).type_id());
            component.apply(source_component_cloned.as_partial_reflect());
            if let Some(reflect_component) = app_registry
                .read()
                .get_type_data::<crate::reflect::ReflectComponent>(type_id)
            {
                let mut i = 0;
                reflect_component.visit_entities_mut(&mut *component, &mut |entity| {
                    *entity = mapped_entities[i];
                    i += 1;
                });
            }
            // SAFETY:
            // - component_id is from the same world as target entity
            // - component is a valid value represented by component_id
            unsafe {
                let raw_component_ptr =
                    core::ptr::NonNull::new_unchecked(Box::into_raw(component).cast::<u8>());
                world
                    .entity_mut(target)
                    .insert_by_id(component_id, OwningPtr::new(raw_component_ptr));
                alloc::alloc::dealloc(raw_component_ptr.as_ptr(), component_layout);
            }
        });
    }
}

/// Noop implementation of component clone handler function.
///
/// See [`EntityClonerBuilder`](crate::entity::EntityClonerBuilder) for details.
pub fn component_clone_ignore(_commands: &mut Commands, _ctx: &mut ComponentCloneCtx) {}

/// Wrapper for components clone specialization using autoderef.
#[doc(hidden)]
pub struct DefaultCloneBehaviorSpecialization<T>(PhantomData<T>);

impl<T> Default for DefaultCloneBehaviorSpecialization<T> {
    fn default() -> Self {
        Self(PhantomData)
    }
}

/// Base trait for components clone specialization using autoderef.
#[doc(hidden)]
pub trait DefaultCloneBehaviorBase {
    fn default_clone_behavior(&self) -> ComponentCloneBehavior;
}
impl<C> DefaultCloneBehaviorBase for DefaultCloneBehaviorSpecialization<C> {
    fn default_clone_behavior(&self) -> ComponentCloneBehavior {
        ComponentCloneBehavior::Default
    }
}

/// Specialized trait for components clone specialization using autoderef.
#[doc(hidden)]
pub trait DefaultCloneBehaviorViaClone {
    fn default_clone_behavior(&self) -> ComponentCloneBehavior;
}
impl<C: Clone + Component> DefaultCloneBehaviorViaClone for &DefaultCloneBehaviorSpecialization<C> {
    fn default_clone_behavior(&self) -> ComponentCloneBehavior {
        ComponentCloneBehavior::clone::<C>()
    }
}
