//! Types for declaring and storing [`Component`]s.

mod clone;
mod queued_registration;
mod required;
mod tick;

pub use clone::*;
pub use queued_registration::*;
pub use required::*;
pub use tick::*;

use crate::{
    archetype::ArchetypeFlags,
    entity::EntityMapper,
    lifecycle::{ComponentHook, ComponentHooks},
    query::DebugCheckedUnwrap,
    resource::Resource,
    storage::SparseSetIndex,
    system::{Local, SystemParam},
    world::{FromWorld, World},
};
use alloc::{borrow::Cow, vec::Vec};
pub use bevy_ecs_macros::Component;
use bevy_platform::{collections::HashSet, sync::PoisonError};
use bevy_ptr::OwningPtr;
#[cfg(feature = "bevy_reflect")]
use bevy_reflect::Reflect;
use bevy_utils::{prelude::DebugName, TypeIdMap};
use core::{
    alloc::Layout,
    any::{Any, TypeId},
    fmt::Debug,
    marker::PhantomData,
    mem::needs_drop,
    ops::{Deref, DerefMut},
};

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
/// # use bevy_ecs::component::Component;
/// # use bevy_ecs::lifecycle::HookContext;
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
/// # use bevy_ecs::component::Component;
/// # use bevy_ecs::lifecycle::HookContext;
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
///
/// ```
/// # Setting the clone behavior
///
/// You can specify how the [`Component`] is cloned when deriving it.
///
/// Your options are the functions and variants of [`ComponentCloneBehavior`]
/// See [Handlers section of `EntityClonerBuilder`](crate::entity::EntityClonerBuilder#handlers) to understand how this affects handler priority.
/// ```
/// # use bevy_ecs::prelude::*;
///
/// #[derive(Component)]
/// #[component(clone_behavior = Ignore)]
/// struct MyComponent;
///
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
/// use bevy_platform::cell::SyncCell;
///
/// // This will compile.
/// #[derive(Component)]
/// struct ActuallySync {
///    counter: SyncCell<RefCell<usize>>,
/// }
/// ```
///
/// [`SyncCell`]: bevy_platform::cell::SyncCell
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
    ///
    /// Bevy provides various implementations of [`MapEntities`](crate::entity::MapEntities), so that arbitrary combinations like these are supported with `#[entities]`:
    ///
    /// ```rust
    /// # use bevy_ecs::{component::Component, entity::Entity};
    /// #[derive(Component)]
    /// struct Inventory {
    ///     #[entities]
    ///     items: Vec<Option<Entity>>
    /// }
    /// ```
    ///
    /// You might need more specialized logic. A likely cause of this is your component contains collections of entities that
    /// don't implement [`MapEntities`](crate::entity::MapEntities). In that case, you can annotate your component with
    /// `#[component(map_entities)]`. Using this attribute, you must implement `MapEntities` for the
    /// component itself, and this method will simply call that implementation.
    ///
    /// ```
    /// # use bevy_ecs::{component::Component, entity::{Entity, MapEntities, EntityMapper}};
    /// # use std::collections::HashMap;
    /// #[derive(Component)]
    /// #[component(map_entities)]
    /// struct Inventory {
    ///     items: HashMap<Entity, usize>
    /// }
    ///
    /// impl MapEntities for Inventory {
    ///   fn map_entities<M: EntityMapper>(&mut self, entity_mapper: &mut M) {
    ///      self.items = self.items
    ///          .drain()
    ///          .map(|(id, count)|(entity_mapper.get_mapped(id), count))
    ///          .collect();
    ///   }
    /// }
    /// # let a = Entity::from_bits(0x1_0000_0001);
    /// # let b = Entity::from_bits(0x1_0000_0002);
    /// # let mut inv = Inventory { items: Default::default() };
    /// # inv.items.insert(a, 10);
    /// # <Inventory as Component>::map_entities(&mut inv, &mut (a,b));
    /// # assert_eq!(inv.items.get(&b), Some(&10));
    /// ````
    ///
    /// Alternatively, you can specify the path to a function with `#[component(map_entities = function_path)]`, similar to component hooks.
    /// In this case, the inputs of the function should mirror the inputs to this method, with the second parameter being generic.
    ///
    /// ```
    /// # use bevy_ecs::{component::Component, entity::{Entity, MapEntities, EntityMapper}};
    /// # use std::collections::HashMap;
    /// #[derive(Component)]
    /// #[component(map_entities = map_the_map)]
    /// // Also works: map_the_map::<M> or map_the_map::<_>
    /// struct Inventory {
    ///     items: HashMap<Entity, usize>
    /// }
    ///
    /// fn map_the_map<M: EntityMapper>(inv: &mut Inventory, entity_mapper: &mut M) {
    ///    inv.items = inv.items
    ///        .drain()
    ///        .map(|(id, count)|(entity_mapper.get_mapped(id), count))
    ///        .collect();
    /// }
    /// # let a = Entity::from_bits(0x1_0000_0001);
    /// # let b = Entity::from_bits(0x1_0000_0002);
    /// # let mut inv = Inventory { items: Default::default() };
    /// # inv.items.insert(a, 10);
    /// # <Inventory as Component>::map_entities(&mut inv, &mut (a,b));
    /// # assert_eq!(inv.items.get(&b), Some(&10));
    /// ````
    ///
    /// You can use the turbofish (`::<A,B,C>`) to specify parameters when a function is generic, using either M or _ for the type of the mapper parameter.
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
/// component, effectively turning the `Insert` and `Replace` hooks into a
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
    pub fn name(&self) -> DebugName {
        self.descriptor.name.clone()
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
    name: DebugName,
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
            name: DebugName::type_name::<T>(),
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
            name: name.into().into(),
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
            name: DebugName::type_name::<T>(),
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
            name: DebugName::type_name::<T>(),
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
    pub fn name(&self) -> DebugName {
        self.name.clone()
    }

    /// Returns whether this component is mutable.
    #[inline]
    pub fn mutable(&self) -> bool {
        self.mutable
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

        info.hooks.update_from_component::<T>();

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
    /// This will return `None` if the id is not registered or is queued.
    ///
    /// This will return an incorrect result if `id` did not come from the same world as `self`. It may return `None` or a garbage value.
    #[inline]
    pub fn get_info(&self, id: ComponentId) -> Option<&ComponentInfo> {
        self.components.get(id.0).and_then(|info| info.as_ref())
    }

    /// Gets the [`ComponentDescriptor`] of the component with this [`ComponentId`] if it is present.
    /// This will return `None` only if the id is neither registered nor queued to be registered.
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
    /// This will return `None` only if the id is neither registered nor queued to be registered.
    ///
    /// This will return an incorrect result if `id` did not come from the same world as `self`. It may return `None` or a garbage value.
    #[inline]
    pub fn get_name<'a>(&'a self, id: ComponentId) -> Option<DebugName> {
        self.components
            .get(id.0)
            .and_then(|info| info.as_ref().map(|info| info.descriptor.name()))
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
        self.get_valid_id(TypeId::of::<T>())
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
        self.get_valid_resource_id(TypeId::of::<T>())
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
