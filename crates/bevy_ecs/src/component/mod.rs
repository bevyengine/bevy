//! Types for declaring and storing [`Component`]s.

mod clone;
mod info;
mod register;
mod required;
mod tick;

pub use clone::*;
pub use info::*;
pub use register::*;
pub use required::*;
pub use tick::*;

use crate::{
    entity::EntityMapper,
    lifecycle::ComponentHook,
    system::{Local, SystemParam},
    world::{FromWorld, World},
};
use alloc::vec::Vec;
pub use bevy_ecs_macros::Component;
use core::{fmt::Debug, marker::PhantomData, ops::Deref};

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
/// [`ComponentHooks`]: crate::lifecycle::ComponentHooks
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
