//! Types for declaring and storing [`Component`]s.

use crate::{
    archetype::ArchetypeFlags,
    bundle::BundleInfo,
    change_detection::{MaybeLocation, MAX_CHANGE_AGE},
    entity::{ComponentCloneCtx, Entity, EntityMapper, SourceComponent},
    query::DebugCheckedUnwrap,
    relationship::RelationshipHookMode,
    resource::Resource,
    storage::{SparseSetIndex, SparseSets, Table, TableRow},
    system::{Local, SystemParam},
    world::{DeferredWorld, FromWorld, World},
};
use alloc::boxed::Box;
use alloc::{borrow::Cow, format, vec::Vec};
pub use bevy_ecs_macros::Component;
use bevy_platform::sync::Arc;
use bevy_platform::{
    collections::{HashMap, HashSet},
    sync::PoisonError,
};
use bevy_ptr::{OwningPtr, UnsafeCellDeref};
#[cfg(feature = "bevy_reflect")]
use bevy_reflect::Reflect;
use bevy_utils::TypeIdMap;
use core::{
    alloc::Layout,
    any::{Any, TypeId},
    cell::UnsafeCell,
    fmt::Debug,
    marker::PhantomData,
    mem::needs_drop,
    ops::{Deref, DerefMut},
};
use disqualified::ShortName;
use smallvec::SmallVec;
use thiserror::Error;

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
/// You can define inline component values that take the following forms:
/// ```
/// # use bevy_ecs::prelude::*;
/// #[derive(Component)]
/// #[require(
///     B(1), // tuple structs
///     C { // named-field structs
///         x: 1,
///         ..Default::default()
///     },
///     D::One, // enum variants
///     E::ONE, // associated consts
///     F::new(1) // constructors
/// )]
/// struct A;
///
/// #[derive(Component, PartialEq, Eq, Debug)]
/// struct B(u8);
///
/// #[derive(Component, PartialEq, Eq, Debug, Default)]
/// struct C {
///     x: u8,
///     y: u8,
/// }
///
/// #[derive(Component, PartialEq, Eq, Debug)]
/// enum D {
///    Zero,
///    One,
/// }
///
/// #[derive(Component, PartialEq, Eq, Debug)]
/// struct E(u8);
///
/// impl E {
///     pub const ONE: Self = Self(1);
/// }
///
/// #[derive(Component, PartialEq, Eq, Debug)]
/// struct F(u8);
///
/// impl F {
///     fn new(value: u8) -> Self {
///         Self(value)
///     }
/// }
///
/// # let mut world = World::default();
/// let id = world.spawn(A).id();
/// assert_eq!(&B(1), world.entity(id).get::<B>().unwrap());
/// assert_eq!(&C { x: 1, y: 0 }, world.entity(id).get::<C>().unwrap());
/// assert_eq!(&D::One, world.entity(id).get::<D>().unwrap());
/// assert_eq!(&E(1), world.entity(id).get::<E>().unwrap());
/// assert_eq!(&F(1), world.entity(id).get::<F>().unwrap());
/// ````
///
///
/// You can also define arbitrary expressions by using `=`
///
/// ```
/// # use bevy_ecs::prelude::*;
/// #[derive(Component)]
/// #[require(C = init_c())]
/// struct A;
///
/// #[derive(Component, PartialEq, Eq, Debug)]
/// #[require(C = C(20))]
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
/// // This will implicitly also insert C with the init_c() constructor
/// let id = world.spawn(A).id();
/// assert_eq!(&C(10), world.entity(id).get::<C>().unwrap());
///
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
/// #[require(X(1))]
/// struct Y;
///
/// #[derive(Component)]
/// #[require(
///     Y,
///     X(2),
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
/// # Relationships between Entities
///
/// Sometimes it is useful to define relationships between entities.  A common example is the
/// parent / child relationship. Since Components are how data is stored for Entities, one might
/// naturally think to create a Component which has a field of type [`Entity`].
///
/// To facilitate this pattern, Bevy provides the [`Relationship`](`crate::relationship::Relationship`)
/// trait. You can derive the [`Relationship`](`crate::relationship::Relationship`) and
/// [`RelationshipTarget`](`crate::relationship::RelationshipTarget`) traits in addition to the
/// Component trait in order to implement data driven relationships between entities, see the trait
/// docs for more details.
///
/// In addition, Bevy provides canonical implementations of the parent / child relationship via the
/// [`ChildOf`](crate::hierarchy::ChildOf) [`Relationship`](crate::relationship::Relationship) and
/// the [`Children`](crate::hierarchy::Children)
/// [`RelationshipTarget`](crate::relationship::RelationshipTarget).
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
/// This also supports function calls that yield closures
///
/// ```
/// # use bevy_ecs::component::{Component, HookContext};
/// # use bevy_ecs::world::DeferredWorld;
/// #
/// #[derive(Component)]
/// #[component(on_add = my_msg_hook("hello"))]
/// #[component(on_despawn = my_msg_hook("yoink"))]
/// struct ComponentA;
///
/// // a hook closure generating function
/// fn my_msg_hook(message: &'static str) -> impl Fn(DeferredWorld, HookContext) {
///     move |_world, _ctx| {
///         println!("{message}");
///     }
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
        _components: &mut ComponentsRegistrator,
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

    /// Maps the entities on this component using the given [`EntityMapper`]. This is used to remap entities in contexts like scenes and entity cloning.
    /// When deriving [`Component`], this is populated by annotating fields containing entities with `#[entities]`
    ///
    /// ```
    /// # use bevy_ecs::{component::Component, entity::Entity};
    /// #[derive(Component)]
    /// struct Inventory {
    ///     #[entities]
    ///     items: Vec<Entity>
    /// }
    /// ```
    ///
    /// Fields with `#[entities]` must implement [`MapEntities`](crate::entity::MapEntities).
    #[inline]
    fn map_entities<E: EntityMapper>(_this: &mut Self, _mapper: &mut E) {}
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
    /// Configures how relationship hooks will run
    pub relationship_hook_mode: RelationshipHookMode,
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
/// use bevy_platform::collections::HashSet;
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
    reflect(Debug, Hash, PartialEq, Clone)
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
pub type ComponentCloneFn = fn(&SourceComponent, &mut ComponentCloneCtx);

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
            ComponentCloneBehavior::Custom(custom) => *custom,
        }
    }
}

/// A queued component registration.
struct QueuedRegistration {
    registrator: Box<dyn FnOnce(&mut ComponentsRegistrator, ComponentId, ComponentDescriptor)>,
    id: ComponentId,
    descriptor: ComponentDescriptor,
}

impl QueuedRegistration {
    /// Creates the [`QueuedRegistration`].
    ///
    /// # Safety
    ///
    /// [`ComponentId`] must be unique.
    unsafe fn new(
        id: ComponentId,
        descriptor: ComponentDescriptor,
        func: impl FnOnce(&mut ComponentsRegistrator, ComponentId, ComponentDescriptor) + 'static,
    ) -> Self {
        Self {
            registrator: Box::new(func),
            id,
            descriptor,
        }
    }

    /// Performs the registration, returning the now valid [`ComponentId`].
    fn register(self, registrator: &mut ComponentsRegistrator) -> ComponentId {
        (self.registrator)(registrator, self.id, self.descriptor);
        self.id
    }
}

/// Allows queuing components to be registered.
#[derive(Default)]
pub struct QueuedComponents {
    components: TypeIdMap<QueuedRegistration>,
    resources: TypeIdMap<QueuedRegistration>,
    dynamic_registrations: Vec<QueuedRegistration>,
}

impl Debug for QueuedComponents {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let components = self
            .components
            .iter()
            .map(|(type_id, queued)| (type_id, queued.id))
            .collect::<Vec<_>>();
        let resources = self
            .resources
            .iter()
            .map(|(type_id, queued)| (type_id, queued.id))
            .collect::<Vec<_>>();
        let dynamic_registrations = self
            .dynamic_registrations
            .iter()
            .map(|queued| queued.id)
            .collect::<Vec<_>>();
        write!(f, "components: {components:?}, resources: {resources:?}, dynamic_registrations: {dynamic_registrations:?}")
    }
}

/// Generates [`ComponentId`]s.
#[derive(Debug, Default)]
pub struct ComponentIds {
    next: bevy_platform::sync::atomic::AtomicUsize,
}

impl ComponentIds {
    /// Peeks the next [`ComponentId`] to be generated without generating it.
    pub fn peek(&self) -> ComponentId {
        ComponentId(
            self.next
                .load(bevy_platform::sync::atomic::Ordering::Relaxed),
        )
    }

    /// Generates and returns the next [`ComponentId`].
    pub fn next(&self) -> ComponentId {
        ComponentId(
            self.next
                .fetch_add(1, bevy_platform::sync::atomic::Ordering::Relaxed),
        )
    }

    /// Peeks the next [`ComponentId`] to be generated without generating it.
    pub fn peek_mut(&mut self) -> ComponentId {
        ComponentId(*self.next.get_mut())
    }

    /// Generates and returns the next [`ComponentId`].
    pub fn next_mut(&mut self) -> ComponentId {
        let id = self.next.get_mut();
        let result = ComponentId(*id);
        *id += 1;
        result
    }

    /// Returns the number of [`ComponentId`]s generated.
    pub fn len(&self) -> usize {
        self.peek().0
    }

    /// Returns true if and only if no ids have been generated.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

/// A type that enables queuing registration in [`Components`].
///
/// # Note
///
/// These queued registrations return [`ComponentId`]s.
/// These ids are not yet valid, but they will become valid
/// when either [`ComponentsRegistrator::apply_queued_registrations`] is called or the same registration is made directly.
/// In either case, the returned [`ComponentId`]s will be correct, but they are not correct yet.
///
/// Generally, that means these [`ComponentId`]s can be safely used for read-only purposes.
/// Modifying the contents of the world through these [`ComponentId`]s directly without waiting for them to be fully registered
/// and without then confirming that they have been fully registered is not supported.
/// Hence, extra care is needed with these [`ComponentId`]s to ensure all safety rules are followed.
///
/// As a rule of thumb, if you have mutable access to [`ComponentsRegistrator`], prefer to use that instead.
/// Use this only if you need to know the id of a component but do not need to modify the contents of the world based on that id.
#[derive(Clone, Copy)]
pub struct ComponentsQueuedRegistrator<'w> {
    components: &'w Components,
    ids: &'w ComponentIds,
}

impl Deref for ComponentsQueuedRegistrator<'_> {
    type Target = Components;

    fn deref(&self) -> &Self::Target {
        self.components
    }
}

impl<'w> ComponentsQueuedRegistrator<'w> {
    /// Constructs a new [`ComponentsQueuedRegistrator`].
    ///
    /// # Safety
    ///
    /// The [`Components`] and [`ComponentIds`] must match.
    /// For example, they must be from the same world.
    pub unsafe fn new(components: &'w Components, ids: &'w ComponentIds) -> Self {
        Self { components, ids }
    }

    /// Queues this function to run as a component registrator.
    ///
    /// # Safety
    ///
    /// The [`TypeId`] must not already be registered or queued as a component.
    unsafe fn force_register_arbitrary_component(
        &self,
        type_id: TypeId,
        descriptor: ComponentDescriptor,
        func: impl FnOnce(&mut ComponentsRegistrator, ComponentId, ComponentDescriptor) + 'static,
    ) -> ComponentId {
        let id = self.ids.next();
        self.components
            .queued
            .write()
            .unwrap_or_else(PoisonError::into_inner)
            .components
            .insert(
                type_id,
                // SAFETY: The id was just generated.
                unsafe { QueuedRegistration::new(id, descriptor, func) },
            );
        id
    }

    /// Queues this function to run as a resource registrator.
    ///
    /// # Safety
    ///
    /// The [`TypeId`] must not already be registered or queued as a resource.
    unsafe fn force_register_arbitrary_resource(
        &self,
        type_id: TypeId,
        descriptor: ComponentDescriptor,
        func: impl FnOnce(&mut ComponentsRegistrator, ComponentId, ComponentDescriptor) + 'static,
    ) -> ComponentId {
        let id = self.ids.next();
        self.components
            .queued
            .write()
            .unwrap_or_else(PoisonError::into_inner)
            .resources
            .insert(
                type_id,
                // SAFETY: The id was just generated.
                unsafe { QueuedRegistration::new(id, descriptor, func) },
            );
        id
    }

    /// Queues this function to run as a dynamic registrator.
    fn force_register_arbitrary_dynamic(
        &self,
        descriptor: ComponentDescriptor,
        func: impl FnOnce(&mut ComponentsRegistrator, ComponentId, ComponentDescriptor) + 'static,
    ) -> ComponentId {
        let id = self.ids.next();
        self.components
            .queued
            .write()
            .unwrap_or_else(PoisonError::into_inner)
            .dynamic_registrations
            .push(
                // SAFETY: The id was just generated.
                unsafe { QueuedRegistration::new(id, descriptor, func) },
            );
        id
    }

    /// This is a queued version of [`ComponentsRegistrator::register_component`].
    /// This will reserve an id and queue the registration.
    /// These registrations will be carried out at the next opportunity.
    ///
    /// If this has already been registered or queued, this returns the previous [`ComponentId`].
    ///
    /// # Note
    ///
    /// Technically speaking, the returned [`ComponentId`] is not valid, but it will become valid later.
    /// See type level docs for details.
    #[inline]
    pub fn queue_register_component<T: Component>(&self) -> ComponentId {
        self.component_id::<T>().unwrap_or_else(|| {
            // SAFETY: We just checked that this type was not in the queue.
            unsafe {
                self.force_register_arbitrary_component(
                    TypeId::of::<T>(),
                    ComponentDescriptor::new::<T>(),
                    |registrator, id, _descriptor| {
                        // SAFETY: We just checked that this is not currently registered or queued, and if it was registered since, this would have been dropped from the queue.
                        #[expect(unused_unsafe, reason = "More precise to specify.")]
                        unsafe {
                            registrator.register_component_unchecked::<T>(&mut Vec::new(), id);
                        }
                    },
                )
            }
        })
    }

    /// This is a queued version of [`ComponentsRegistrator::register_component_with_descriptor`].
    /// This will reserve an id and queue the registration.
    /// These registrations will be carried out at the next opportunity.
    ///
    /// # Note
    ///
    /// Technically speaking, the returned [`ComponentId`] is not valid, but it will become valid later.
    /// See type level docs for details.
    #[inline]
    pub fn queue_register_component_with_descriptor(
        &self,
        descriptor: ComponentDescriptor,
    ) -> ComponentId {
        self.force_register_arbitrary_dynamic(descriptor, |registrator, id, descriptor| {
            // SAFETY: Id uniqueness handled by caller.
            unsafe {
                registrator.register_component_inner(id, descriptor);
            }
        })
    }

    /// This is a queued version of [`ComponentsRegistrator::register_resource`].
    /// This will reserve an id and queue the registration.
    /// These registrations will be carried out at the next opportunity.
    ///
    /// If this has already been registered or queued, this returns the previous [`ComponentId`].
    ///
    /// # Note
    ///
    /// Technically speaking, the returned [`ComponentId`] is not valid, but it will become valid later.
    /// See type level docs for details.
    #[inline]
    pub fn queue_register_resource<T: Resource>(&self) -> ComponentId {
        let type_id = TypeId::of::<T>();
        self.get_resource_id(type_id).unwrap_or_else(|| {
            // SAFETY: We just checked that this type was not in the queue.
            unsafe {
                self.force_register_arbitrary_resource(
                    type_id,
                    ComponentDescriptor::new_resource::<T>(),
                    move |registrator, id, descriptor| {
                        // SAFETY: We just checked that this is not currently registered or queued, and if it was registered since, this would have been dropped from the queue.
                        // SAFETY: Id uniqueness handled by caller, and the type_id matches descriptor.
                        #[expect(unused_unsafe, reason = "More precise to specify.")]
                        unsafe {
                            registrator.register_resource_unchecked(type_id, id, descriptor);
                        }
                    },
                )
            }
        })
    }

    /// This is a queued version of [`ComponentsRegistrator::register_non_send`].
    /// This will reserve an id and queue the registration.
    /// These registrations will be carried out at the next opportunity.
    ///
    /// If this has already been registered or queued, this returns the previous [`ComponentId`].
    ///
    /// # Note
    ///
    /// Technically speaking, the returned [`ComponentId`] is not valid, but it will become valid later.
    /// See type level docs for details.
    #[inline]
    pub fn queue_register_non_send<T: Any>(&self) -> ComponentId {
        let type_id = TypeId::of::<T>();
        self.get_resource_id(type_id).unwrap_or_else(|| {
            // SAFETY: We just checked that this type was not in the queue.
            unsafe {
                self.force_register_arbitrary_resource(
                    type_id,
                    ComponentDescriptor::new_non_send::<T>(StorageType::default()),
                    move |registrator, id, descriptor| {
                        // SAFETY: We just checked that this is not currently registered or queued, and if it was registered since, this would have been dropped from the queue.
                        // SAFETY: Id uniqueness handled by caller, and the type_id matches descriptor.
                        #[expect(unused_unsafe, reason = "More precise to specify.")]
                        unsafe {
                            registrator.register_resource_unchecked(type_id, id, descriptor);
                        }
                    },
                )
            }
        })
    }

    /// This is a queued version of [`ComponentsRegistrator::register_resource_with_descriptor`].
    /// This will reserve an id and queue the registration.
    /// These registrations will be carried out at the next opportunity.
    ///
    /// # Note
    ///
    /// Technically speaking, the returned [`ComponentId`] is not valid, but it will become valid later.
    /// See type level docs for details.
    #[inline]
    pub fn queue_register_resource_with_descriptor(
        &self,
        descriptor: ComponentDescriptor,
    ) -> ComponentId {
        self.force_register_arbitrary_dynamic(descriptor, |registrator, id, descriptor| {
            // SAFETY: Id uniqueness handled by caller.
            unsafe {
                registrator.register_component_inner(id, descriptor);
            }
        })
    }
}

/// A [`Components`] wrapper that enables additional features, like registration.
pub struct ComponentsRegistrator<'w> {
    components: &'w mut Components,
    ids: &'w mut ComponentIds,
}

impl Deref for ComponentsRegistrator<'_> {
    type Target = Components;

    fn deref(&self) -> &Self::Target {
        self.components
    }
}

impl DerefMut for ComponentsRegistrator<'_> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.components
    }
}

impl<'w> ComponentsRegistrator<'w> {
    /// Constructs a new [`ComponentsRegistrator`].
    ///
    /// # Safety
    ///
    /// The [`Components`] and [`ComponentIds`] must match.
    /// For example, they must be from the same world.
    pub unsafe fn new(components: &'w mut Components, ids: &'w mut ComponentIds) -> Self {
        Self { components, ids }
    }

    /// Converts this [`ComponentsRegistrator`] into a [`ComponentsQueuedRegistrator`].
    /// This is intended for use to pass this value to a function that requires [`ComponentsQueuedRegistrator`].
    /// It is generally not a good idea to queue a registration when you can instead register directly on this type.
    pub fn as_queued(&self) -> ComponentsQueuedRegistrator<'_> {
        // SAFETY: ensured by the caller that created self.
        unsafe { ComponentsQueuedRegistrator::new(self.components, self.ids) }
    }

    /// Applies every queued registration.
    /// This ensures that every valid [`ComponentId`] is registered,
    /// enabling retrieving [`ComponentInfo`], etc.
    pub fn apply_queued_registrations(&mut self) {
        if !self.any_queued_mut() {
            return;
        }

        // Note:
        //
        // This is not just draining the queue. We need to empty the queue without removing the information from `Components`.
        // If we drained directly, we could break invariance.
        //
        // For example, say `ComponentA` and `ComponentB` are queued, and `ComponentA` requires `ComponentB`.
        // If we drain directly, and `ComponentA` was the first to be registered, then, when `ComponentA`
        // registers `ComponentB` in `Component::register_required_components`,
        // `Components` will not know that `ComponentB` was queued
        // (since it will have been drained from the queue.)
        // If that happened, `Components` would assign a new `ComponentId` to `ComponentB`
        // which would be *different* than the id it was assigned in the queue.
        // Then, when the drain iterator gets to `ComponentB`,
        // it would be unsafely registering `ComponentB`, which is already registered.
        //
        // As a result, we need to pop from each queue one by one instead of draining.

        // components
        while let Some(registrator) = {
            let queued = self
                .components
                .queued
                .get_mut()
                .unwrap_or_else(PoisonError::into_inner);
            queued.components.keys().next().copied().map(|type_id| {
                // SAFETY: the id just came from a valid iterator.
                unsafe { queued.components.remove(&type_id).debug_checked_unwrap() }
            })
        } {
            registrator.register(self);
        }

        // resources
        while let Some(registrator) = {
            let queued = self
                .components
                .queued
                .get_mut()
                .unwrap_or_else(PoisonError::into_inner);
            queued.resources.keys().next().copied().map(|type_id| {
                // SAFETY: the id just came from a valid iterator.
                unsafe { queued.resources.remove(&type_id).debug_checked_unwrap() }
            })
        } {
            registrator.register(self);
        }

        // dynamic
        let queued = &mut self
            .components
            .queued
            .get_mut()
            .unwrap_or_else(PoisonError::into_inner);
        if !queued.dynamic_registrations.is_empty() {
            for registrator in core::mem::take(&mut queued.dynamic_registrations) {
                registrator.register(self);
            }
        }
    }

    /// Registers a [`Component`] of type `T` with this instance.
    /// If a component of this type has already been registered, this will return
    /// the ID of the pre-existing component.
    ///
    /// # See also
    ///
    /// * [`Components::component_id()`]
    /// * [`ComponentsRegistrator::register_component_with_descriptor()`]
    #[inline]
    pub fn register_component<T: Component>(&mut self) -> ComponentId {
        self.register_component_checked::<T>(&mut Vec::new())
    }

    /// Same as [`Self::register_component_unchecked`] but keeps a checks for safety.
    #[inline]
    fn register_component_checked<T: Component>(
        &mut self,
        recursion_check_stack: &mut Vec<ComponentId>,
    ) -> ComponentId {
        let type_id = TypeId::of::<T>();
        if let Some(id) = self.indices.get(&type_id) {
            return *id;
        }

        if let Some(registrator) = self
            .components
            .queued
            .get_mut()
            .unwrap_or_else(PoisonError::into_inner)
            .components
            .remove(&type_id)
        {
            // If we are trying to register something that has already been queued, we respect the queue.
            // Just like if we are trying to register something that already is, we respect the first registration.
            return registrator.register(self);
        }

        let id = self.ids.next_mut();
        // SAFETY: The component is not currently registered, and the id is fresh.
        unsafe {
            self.register_component_unchecked::<T>(recursion_check_stack, id);
        }
        id
    }

    /// # Safety
    ///
    /// Neither this component, nor its id may be registered or queued. This must be a new registration.
    #[inline]
    unsafe fn register_component_unchecked<T: Component>(
        &mut self,
        recursion_check_stack: &mut Vec<ComponentId>,
        id: ComponentId,
    ) {
        // SAFETY: ensured by caller.
        unsafe {
            self.register_component_inner(id, ComponentDescriptor::new::<T>());
        }
        let type_id = TypeId::of::<T>();
        let prev = self.indices.insert(type_id, id);
        debug_assert!(prev.is_none());

        let mut required_components = RequiredComponents::default();
        T::register_required_components(
            id,
            self,
            &mut required_components,
            0,
            recursion_check_stack,
        );
        // SAFETY: we just inserted it in `register_component_inner`
        let info = unsafe {
            &mut self
                .components
                .components
                .get_mut(id.0)
                .debug_checked_unwrap()
                .as_mut()
                .debug_checked_unwrap()
        };

        #[expect(
            deprecated,
            reason = "need to use this method until it is removed to ensure user defined components register hooks correctly"
        )]
        // TODO: Replace with `info.hooks.update_from_component::<T>();` once `Component::register_component_hooks` is removed
        T::register_component_hooks(&mut info.hooks);

        info.required_components = required_components;
    }

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
    /// * [`ComponentsRegistrator::register_component()`]
    #[inline]
    pub fn register_component_with_descriptor(
        &mut self,
        descriptor: ComponentDescriptor,
    ) -> ComponentId {
        let id = self.ids.next_mut();
        // SAFETY: The id is fresh.
        unsafe {
            self.register_component_inner(id, descriptor);
        }
        id
    }

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
    pub fn register_required_components_manual<T: Component, R: Component>(
        &mut self,
        required_components: &mut RequiredComponents,
        constructor: fn() -> R,
        inheritance_depth: u16,
        recursion_check_stack: &mut Vec<ComponentId>,
    ) {
        let requiree = self.register_component_checked::<T>(recursion_check_stack);
        let required = self.register_component_checked::<R>(recursion_check_stack);

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

    /// Registers a [`Resource`] of type `T` with this instance.
    /// If a resource of this type has already been registered, this will return
    /// the ID of the pre-existing resource.
    ///
    /// # See also
    ///
    /// * [`Components::resource_id()`]
    /// * [`ComponentsRegistrator::register_resource_with_descriptor()`]
    #[inline]
    pub fn register_resource<T: Resource>(&mut self) -> ComponentId {
        // SAFETY: The [`ComponentDescriptor`] matches the [`TypeId`]
        unsafe {
            self.register_resource_with(TypeId::of::<T>(), || {
                ComponentDescriptor::new_resource::<T>()
            })
        }
    }

    /// Registers a [non-send resource](crate::system::NonSend) of type `T` with this instance.
    /// If a resource of this type has already been registered, this will return
    /// the ID of the pre-existing resource.
    #[inline]
    pub fn register_non_send<T: Any>(&mut self) -> ComponentId {
        // SAFETY: The [`ComponentDescriptor`] matches the [`TypeId`]
        unsafe {
            self.register_resource_with(TypeId::of::<T>(), || {
                ComponentDescriptor::new_non_send::<T>(StorageType::default())
            })
        }
    }

    /// Same as [`Components::register_resource_unchecked`] but handles safety.
    ///
    /// # Safety
    ///
    /// The [`ComponentDescriptor`] must match the [`TypeId`].
    #[inline]
    unsafe fn register_resource_with(
        &mut self,
        type_id: TypeId,
        descriptor: impl FnOnce() -> ComponentDescriptor,
    ) -> ComponentId {
        if let Some(id) = self.resource_indices.get(&type_id) {
            return *id;
        }

        if let Some(registrator) = self
            .components
            .queued
            .get_mut()
            .unwrap_or_else(PoisonError::into_inner)
            .resources
            .remove(&type_id)
        {
            // If we are trying to register something that has already been queued, we respect the queue.
            // Just like if we are trying to register something that already is, we respect the first registration.
            return registrator.register(self);
        }

        let id = self.ids.next_mut();
        // SAFETY: The resource is not currently registered, the id is fresh, and the [`ComponentDescriptor`] matches the [`TypeId`]
        unsafe {
            self.register_resource_unchecked(type_id, id, descriptor());
        }
        id
    }

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
    /// * [`ComponentsRegistrator::register_resource()`]
    #[inline]
    pub fn register_resource_with_descriptor(
        &mut self,
        descriptor: ComponentDescriptor,
    ) -> ComponentId {
        let id = self.ids.next_mut();
        // SAFETY: The id is fresh.
        unsafe {
            self.register_component_inner(id, descriptor);
        }
        id
    }
}

/// Stores metadata associated with each kind of [`Component`] in a given [`World`].
#[derive(Debug, Default)]
pub struct Components {
    components: Vec<Option<ComponentInfo>>,
    indices: TypeIdMap<ComponentId>,
    resource_indices: TypeIdMap<ComponentId>,
    // This is kept internal and local to verify that no deadlocks can occor.
    queued: bevy_platform::sync::RwLock<QueuedComponents>,
}

impl Components {
    /// This registers any descriptor, component or resource.
    ///
    /// # Safety
    ///
    /// The id must have never been registered before. This must be a fresh registration.
    #[inline]
    unsafe fn register_component_inner(
        &mut self,
        id: ComponentId,
        descriptor: ComponentDescriptor,
    ) {
        let info = ComponentInfo::new(id, descriptor);
        let least_len = id.0 + 1;
        if self.components.len() < least_len {
            self.components.resize_with(least_len, || None);
        }
        // SAFETY: We just extended the vec to make this index valid.
        let slot = unsafe { self.components.get_mut(id.0).debug_checked_unwrap() };
        // Caller ensures id is unique
        debug_assert!(slot.is_none());
        *slot = Some(info);
    }

    /// Returns the number of components registered or queued with this instance.
    #[inline]
    pub fn len(&self) -> usize {
        self.num_queued() + self.num_registered()
    }

    /// Returns `true` if there are no components registered or queued with this instance. Otherwise, this returns `false`.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Returns the number of components registered with this instance.
    #[inline]
    pub fn num_queued(&self) -> usize {
        let queued = self.queued.read().unwrap_or_else(PoisonError::into_inner);
        queued.components.len() + queued.dynamic_registrations.len() + queued.resources.len()
    }

    /// Returns `true` if there are any components registered with this instance. Otherwise, this returns `false`.
    #[inline]
    pub fn any_queued(&self) -> bool {
        self.num_queued() > 0
    }

    /// A faster version of [`Self::num_queued`].
    #[inline]
    pub fn num_queued_mut(&mut self) -> usize {
        let queued = self
            .queued
            .get_mut()
            .unwrap_or_else(PoisonError::into_inner);
        queued.components.len() + queued.dynamic_registrations.len() + queued.resources.len()
    }

    /// A faster version of [`Self::any_queued`].
    #[inline]
    pub fn any_queued_mut(&mut self) -> bool {
        self.num_queued_mut() > 0
    }

    /// Returns the number of components registered with this instance.
    #[inline]
    pub fn num_registered(&self) -> usize {
        self.components.len()
    }

    /// Returns `true` if there are any components registered with this instance. Otherwise, this returns `false`.
    #[inline]
    pub fn any_registered(&self) -> bool {
        self.num_registered() > 0
    }

    /// Gets the metadata associated with the given component, if it is registered.
    /// This will return `None` if the id is not regiserted or is queued.
    ///
    /// This will return an incorrect result if `id` did not come from the same world as `self`. It may return `None` or a garbage value.
    #[inline]
    pub fn get_info(&self, id: ComponentId) -> Option<&ComponentInfo> {
        self.components.get(id.0).and_then(|info| info.as_ref())
    }

    /// Gets the [`ComponentDescriptor`] of the component with this [`ComponentId`] if it is present.
    /// This will return `None` only if the id is neither regisered nor queued to be registered.
    ///
    /// Currently, the [`Cow`] will be [`Cow::Owned`] if and only if the component is queued. It will be [`Cow::Borrowed`] otherwise.
    ///
    /// This will return an incorrect result if `id` did not come from the same world as `self`. It may return `None` or a garbage value.
    #[inline]
    pub fn get_descriptor<'a>(&'a self, id: ComponentId) -> Option<Cow<'a, ComponentDescriptor>> {
        self.components
            .get(id.0)
            .and_then(|info| info.as_ref().map(|info| Cow::Borrowed(&info.descriptor)))
            .or_else(|| {
                let queued = self.queued.read().unwrap_or_else(PoisonError::into_inner);
                // first check components, then resources, then dynamic
                queued
                    .components
                    .values()
                    .chain(queued.resources.values())
                    .chain(queued.dynamic_registrations.iter())
                    .find(|queued| queued.id == id)
                    .map(|queued| Cow::Owned(queued.descriptor.clone()))
            })
    }

    /// Gets the name of the component with this [`ComponentId`] if it is present.
    /// This will return `None` only if the id is neither regisered nor queued to be registered.
    ///
    /// This will return an incorrect result if `id` did not come from the same world as `self`. It may return `None` or a garbage value.
    #[inline]
    pub fn get_name<'a>(&'a self, id: ComponentId) -> Option<Cow<'a, str>> {
        self.components
            .get(id.0)
            .and_then(|info| {
                info.as_ref()
                    .map(|info| Cow::Borrowed(info.descriptor.name()))
            })
            .or_else(|| {
                let queued = self.queued.read().unwrap_or_else(PoisonError::into_inner);
                // first check components, then resources, then dynamic
                queued
                    .components
                    .values()
                    .chain(queued.resources.values())
                    .chain(queued.dynamic_registrations.iter())
                    .find(|queued| queued.id == id)
                    .map(|queued| queued.descriptor.name.clone())
            })
    }

    /// Gets the metadata associated with the given component.
    /// # Safety
    ///
    /// `id` must be a valid and fully registered [`ComponentId`].
    #[inline]
    pub unsafe fn get_info_unchecked(&self, id: ComponentId) -> &ComponentInfo {
        // SAFETY: The caller ensures `id` is valid.
        unsafe {
            self.components
                .get(id.0)
                .debug_checked_unwrap()
                .as_ref()
                .debug_checked_unwrap()
        }
    }

    #[inline]
    pub(crate) fn get_hooks_mut(&mut self, id: ComponentId) -> Option<&mut ComponentHooks> {
        self.components
            .get_mut(id.0)
            .and_then(|info| info.as_mut().map(|info| &mut info.hooks))
    }

    #[inline]
    pub(crate) fn get_required_components_mut(
        &mut self,
        id: ComponentId,
    ) -> Option<&mut RequiredComponents> {
        self.components
            .get_mut(id.0)
            .and_then(|info| info.as_mut().map(|info| &mut info.required_components))
    }

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
    pub(crate) unsafe fn register_required_components<R: Component>(
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
            .0
            .get(&required)
            .is_some_and(|c| c.inheritance_depth == 0)
        {
            return Err(RequiredComponentsError::DuplicateRegistration(
                requiree, required,
            ));
        }

        // Register the required component for the requiree.
        // This is a direct requirement with a depth of `0`.
        required_components.register_by_id(required, constructor, 0);

        // Add the requiree to the list of components that require the required component.
        // SAFETY: The component is in the list of required components, so it must exist already.
        let required_by = unsafe { self.get_required_by_mut(required).debug_checked_unwrap() };
        required_by.insert(requiree);

        let mut required_components_tmp = RequiredComponents::default();
        // SAFETY: The caller ensures that the `requiree` and `required` components are valid.
        let inherited_requirements = unsafe {
            self.register_inherited_required_components(
                requiree,
                required,
                &mut required_components_tmp,
            )
        };

        // SAFETY: The caller ensures that the `requiree` is valid.
        let required_components = unsafe {
            self.get_required_components_mut(requiree)
                .debug_checked_unwrap()
        };
        required_components.0.extend(required_components_tmp.0);

        // Propagate the new required components up the chain to all components that require the requiree.
        if let Some(required_by) = self
            .get_required_by(requiree)
            .map(|set| set.iter().copied().collect::<SmallVec<[ComponentId; 8]>>())
        {
            // `required` is now required by anything that `requiree` was required by.
            self.get_required_by_mut(required)
                .unwrap()
                .extend(required_by.iter().copied());
            for &required_by_id in required_by.iter() {
                // SAFETY: The component is in the list of required components, so it must exist already.
                let required_components = unsafe {
                    self.get_required_components_mut(required_by_id)
                        .debug_checked_unwrap()
                };

                // Register the original required component in the "parent" of the requiree.
                // The inheritance depth is 1 deeper than the `requiree` wrt `required_by_id`.
                let depth = required_components.0.get(&requiree).expect("requiree is required by required_by_id, so its required_components must include requiree").inheritance_depth;
                required_components.register_by_id(required, constructor, depth + 1);

                for (component_id, component) in inherited_requirements.iter() {
                    // Register the required component.
                    // The inheritance depth of inherited components is whatever the requiree's
                    // depth is relative to `required_by_id`, plus the inheritance depth of the
                    // inherited component relative to the requiree, plus 1 to account for the
                    // requiree in between.
                    // SAFETY: Component ID and constructor match the ones on the original requiree.
                    //         The original requiree is responsible for making sure the registration is safe.
                    unsafe {
                        required_components.register_dynamic_with(
                            *component_id,
                            component.inheritance_depth + depth + 1,
                            || component.constructor.clone(),
                        );
                    };
                }
            }
        }

        Ok(())
    }

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
        required_components: &mut RequiredComponents,
    ) -> Vec<(ComponentId, RequiredComponent)> {
        // Get required components inherited from the `required` component.
        // SAFETY: The caller ensures that the `required` component is valid.
        let required_component_info = unsafe { self.get_info(required).debug_checked_unwrap() };
        let inherited_requirements: Vec<(ComponentId, RequiredComponent)> = required_component_info
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
            .collect();

        // Register the new required components.
        for (component_id, component) in inherited_requirements.iter() {
            // Register the required component for the requiree.
            // SAFETY: Component ID and constructor match the ones on the original requiree.
            unsafe {
                required_components.register_dynamic_with(
                    *component_id,
                    component.inheritance_depth,
                    || component.constructor.clone(),
                );
            };

            // Add the requiree to the list of components that require the required component.
            // SAFETY: The caller ensures that the required components are valid.
            let required_by = unsafe {
                self.get_required_by_mut(*component_id)
                    .debug_checked_unwrap()
            };
            required_by.insert(requiree);
        }

        inherited_requirements
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
    pub(crate) unsafe fn register_required_components_manual_unchecked<R: Component>(
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
        required_by.insert(requiree);

        self.register_inherited_required_components(requiree, required, required_components);
    }

    #[inline]
    pub(crate) fn get_required_by(&self, id: ComponentId) -> Option<&HashSet<ComponentId>> {
        self.components
            .get(id.0)
            .and_then(|info| info.as_ref().map(|info| &info.required_by))
    }

    #[inline]
    pub(crate) fn get_required_by_mut(
        &mut self,
        id: ComponentId,
    ) -> Option<&mut HashSet<ComponentId>> {
        self.components
            .get_mut(id.0)
            .and_then(|info| info.as_mut().map(|info| &mut info.required_by))
    }

    /// Returns true if the [`ComponentId`] is fully registered and valid.
    /// Ids may be invalid if they are still queued to be registered.
    /// Those ids are still correct, but they are not usable in every context yet.
    #[inline]
    pub fn is_id_valid(&self, id: ComponentId) -> bool {
        self.components.get(id.0).is_some_and(Option::is_some)
    }

    /// Type-erased equivalent of [`Components::valid_component_id()`].
    #[inline]
    pub fn get_valid_id(&self, type_id: TypeId) -> Option<ComponentId> {
        self.indices.get(&type_id).copied()
    }

    /// Returns the [`ComponentId`] of the given [`Component`] type `T` if it is fully registered.
    /// If you want to include queued registration, see [`Components::component_id()`].
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
    /// assert_eq!(component_a_id, world.components().valid_component_id::<ComponentA>().unwrap())
    /// ```
    ///
    /// # See also
    ///
    /// * [`Components::get_valid_id()`]
    /// * [`Components::valid_resource_id()`]
    /// * [`World::component_id()`]
    #[inline]
    pub fn valid_component_id<T: Component>(&self) -> Option<ComponentId> {
        self.get_id(TypeId::of::<T>())
    }

    /// Type-erased equivalent of [`Components::valid_resource_id()`].
    #[inline]
    pub fn get_valid_resource_id(&self, type_id: TypeId) -> Option<ComponentId> {
        self.resource_indices.get(&type_id).copied()
    }

    /// Returns the [`ComponentId`] of the given [`Resource`] type `T` if it is fully registered.
    /// If you want to include queued registration, see [`Components::resource_id()`].
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
    /// assert_eq!(resource_a_id, world.components().valid_resource_id::<ResourceA>().unwrap())
    /// ```
    ///
    /// # See also
    ///
    /// * [`Components::valid_component_id()`]
    /// * [`Components::get_resource_id()`]
    #[inline]
    pub fn valid_resource_id<T: Resource>(&self) -> Option<ComponentId> {
        self.get_resource_id(TypeId::of::<T>())
    }

    /// Type-erased equivalent of [`Components::component_id()`].
    #[inline]
    pub fn get_id(&self, type_id: TypeId) -> Option<ComponentId> {
        self.indices.get(&type_id).copied().or_else(|| {
            self.queued
                .read()
                .unwrap_or_else(PoisonError::into_inner)
                .components
                .get(&type_id)
                .map(|queued| queued.id)
        })
    }

    /// Returns the [`ComponentId`] of the given [`Component`] type `T`.
    ///
    /// The returned `ComponentId` is specific to the `Components` instance
    /// it was retrieved from and should not be used with another `Components`
    /// instance.
    ///
    /// Returns [`None`] if the `Component` type has not
    /// yet been initialized using [`ComponentsRegistrator::register_component()`] or [`ComponentsQueuedRegistrator::queue_register_component()`].
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
    pub fn component_id<T: Component>(&self) -> Option<ComponentId> {
        self.get_id(TypeId::of::<T>())
    }

    /// Type-erased equivalent of [`Components::resource_id()`].
    #[inline]
    pub fn get_resource_id(&self, type_id: TypeId) -> Option<ComponentId> {
        self.resource_indices.get(&type_id).copied().or_else(|| {
            self.queued
                .read()
                .unwrap_or_else(PoisonError::into_inner)
                .resources
                .get(&type_id)
                .map(|queued| queued.id)
        })
    }

    /// Returns the [`ComponentId`] of the given [`Resource`] type `T`.
    ///
    /// The returned `ComponentId` is specific to the `Components` instance
    /// it was retrieved from and should not be used with another `Components`
    /// instance.
    ///
    /// Returns [`None`] if the `Resource` type has not
    /// yet been initialized using [`ComponentsRegistrator::register_resource()`] or [`ComponentsQueuedRegistrator::queue_register_resource()`].
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
    pub fn resource_id<T: Resource>(&self) -> Option<ComponentId> {
        self.get_resource_id(TypeId::of::<T>())
    }

    /// # Safety
    ///
    /// The [`ComponentDescriptor`] must match the [`TypeId`].
    /// The [`ComponentId`] must be unique.
    /// The [`TypeId`] and [`ComponentId`] must not be registered or queued.
    #[inline]
    unsafe fn register_resource_unchecked(
        &mut self,
        type_id: TypeId,
        component_id: ComponentId,
        descriptor: ComponentDescriptor,
    ) {
        // SAFETY: ensured by caller
        unsafe {
            self.register_component_inner(component_id, descriptor);
        }
        let prev = self.resource_indices.insert(type_id, component_id);
        debug_assert!(prev.is_none());
    }

    /// Gets an iterator over all components fully registered with this instance.
    pub fn iter_registered(&self) -> impl Iterator<Item = &ComponentInfo> + '_ {
        self.components.iter().filter_map(Option::as_ref)
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
    reflect(Debug, Hash, PartialEq, Clone)
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
#[cfg_attr(feature = "bevy_reflect", derive(Reflect), reflect(Debug, Clone))]
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
    /// This is normally done automatically via the [`DerefMut`] implementation
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
    pub unsafe fn register_dynamic_with(
        &mut self,
        component_id: ComponentId,
        inheritance_depth: u16,
        constructor: impl FnOnce() -> RequiredComponentConstructor,
    ) {
        let entry = self.0.entry(component_id);
        match entry {
            bevy_platform::collections::hash_map::Entry::Occupied(mut occupied) => {
                let current = occupied.get_mut();
                if current.inheritance_depth > inheritance_depth {
                    *current = RequiredComponent {
                        constructor: constructor(),
                        inheritance_depth,
                    }
                }
            }
            bevy_platform::collections::hash_map::Entry::Vacant(vacant) => {
                vacant.insert(RequiredComponent {
                    constructor: constructor(),
                    inheritance_depth,
                });
            }
        }
    }

    /// Registers a required component.
    ///
    /// If the component is already registered, it will be overwritten if the given inheritance depth
    /// is smaller than the depth of the existing registration. Otherwise, the new registration will be ignored.
    pub fn register<C: Component>(
        &mut self,
        components: &mut ComponentsRegistrator,
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
        let erased = || {
            RequiredComponentConstructor({
                // `portable-atomic-util` `Arc` is not able to coerce an unsized
                // type like `std::sync::Arc` can. Creating a `Box` first does the
                // coercion.
                //
                // This would be resolved by https://github.com/rust-lang/rust/issues/123430

                #[cfg(not(target_has_atomic = "ptr"))]
                use alloc::boxed::Box;

                type Constructor = dyn for<'a, 'b> Fn(
                    &'a mut Table,
                    &'b mut SparseSets,
                    Tick,
                    TableRow,
                    Entity,
                    MaybeLocation,
                );

                #[cfg(not(target_has_atomic = "ptr"))]
                type Intermediate<T> = Box<T>;

                #[cfg(target_has_atomic = "ptr")]
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
            })
        };

        // SAFETY:
        // `component_id` matches the type initialized by the `erased` constructor above.
        // `erased` initializes a component for `component_id` in such a way that
        // matches the storage type of the component. It only uses the given `table_row` or `Entity` to
        // initialize the storage corresponding to the given entity.
        unsafe { self.register_dynamic_with(component_id, inheritance_depth, erased) };
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

    /// Merges `required_components` into this collection. This only inserts a required component
    /// if it _did not already exist_ *or* if the required component is more specific than the existing one
    /// (in other words, if the inheritance depth is smaller).
    ///
    /// See [`register_dynamic_with`](Self::register_dynamic_with) for details.
    pub(crate) fn merge(&mut self, required_components: &RequiredComponents) {
        for (
            component_id,
            RequiredComponent {
                constructor,
                inheritance_depth,
            },
        ) in required_components.0.iter()
        {
            // SAFETY: This exact registration must have been done on `required_components`, so safety is ensured by that caller.
            unsafe {
                self.register_dynamic_with(*component_id, *inheritance_depth, || {
                    constructor.clone()
                });
            }
        }
    }
}

// NOTE: This should maybe be private, but it is currently public so that `bevy_ecs_macros` can use it.
// This exists as a standalone function instead of being inlined into the component derive macro so as
// to reduce the amount of generated code.
#[doc(hidden)]
pub fn enforce_no_required_components_recursion(
    components: &Components,
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
                    .map(|id| format!("{}", ShortName(&components.get_name(*id).unwrap())))
                    .collect::<Vec<_>>()
                    .join(" → "),
                if direct_recursion {
                    format!(
                        "Remove require({}).",
                        ShortName(&components.get_name(requiree).unwrap())
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
    source: &SourceComponent,
    ctx: &mut ComponentCloneCtx,
) {
    if let Some(component) = source.read::<C>() {
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
/// - Component can be cloned via [`PartialReflect::reflect_clone`] _or_ has one of the following registered: [`ReflectFromReflect`](bevy_reflect::ReflectFromReflect),
///   [`ReflectDefault`](bevy_reflect::std_traits::ReflectDefault), [`ReflectFromWorld`](crate::reflect::ReflectFromWorld)
///
/// If any of the conditions is not satisfied, the component will be skipped.
///
/// See [`EntityClonerBuilder`](crate::entity::EntityClonerBuilder) for details.
///
/// [`PartialReflect::reflect_clone`]: bevy_reflect::PartialReflect::reflect_clone
#[cfg(feature = "bevy_reflect")]
pub fn component_clone_via_reflect(source: &SourceComponent, ctx: &mut ComponentCloneCtx) {
    let Some(app_registry) = ctx.type_registry().cloned() else {
        return;
    };
    let registry = app_registry.read();
    let Some(source_component_reflect) = source.read_reflect(&registry) else {
        return;
    };
    let component_info = ctx.component_info();
    // checked in read_source_component_reflect
    let type_id = component_info.type_id().unwrap();

    // Try to clone using `reflect_clone`
    if let Ok(mut component) = source_component_reflect.reflect_clone() {
        if let Some(reflect_component) =
            registry.get_type_data::<crate::reflect::ReflectComponent>(type_id)
        {
            reflect_component.map_entities(&mut *component, ctx.entity_mapper());
        }
        drop(registry);

        ctx.write_target_component_reflect(component);
        return;
    }

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
                reflect_component.map_entities(&mut *component, ctx.entity_mapper());
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
        let source_component_cloned = source_component_reflect.to_dynamic();
        let component_layout = component_info.layout();
        let target = ctx.target();
        let component_id = ctx.component_id();
        drop(registry);
        ctx.queue_deferred(move |world: &mut World, mapper: &mut dyn EntityMapper| {
            let mut component = reflect_from_world.from_world(world);
            assert_eq!(type_id, (*component).type_id());
            component.apply(source_component_cloned.as_partial_reflect());
            if let Some(reflect_component) = app_registry
                .read()
                .get_type_data::<crate::reflect::ReflectComponent>(type_id)
            {
                reflect_component.map_entities(&mut *component, mapper);
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

                if component_layout.size() > 0 {
                    // Ensure we don't attempt to deallocate zero-sized components
                    alloc::alloc::dealloc(raw_component_ptr.as_ptr(), component_layout);
                }
            }
        });
    }
}

/// Noop implementation of component clone handler function.
///
/// See [`EntityClonerBuilder`](crate::entity::EntityClonerBuilder) for details.
pub fn component_clone_ignore(_source: &SourceComponent, _ctx: &mut ComponentCloneCtx) {}

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
