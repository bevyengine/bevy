use std::any::TypeId;
use std::fmt::Debug;
use std::hash::{Hash, Hasher};
use std::marker::PhantomData;
use std::sync::atomic::{AtomicUsize, Ordering};

pub use bevy_ecs_macros::{ScheduleLabel, SystemSet};
use bevy_utils::define_boxed_label;
use bevy_utils::label::DynHash;

use crate::component::ComponentDescriptor;
use crate::prelude::Component;
use crate::system::{
    ExclusiveSystemParamFunction, IsExclusiveFunctionSystem, IsFunctionSystem, Resource,
    SystemParamFunction,
};

define_boxed_label!(ScheduleLabel);

/// A shorthand for `Box<dyn SystemSet>`.
pub type BoxedSystemSet = Box<dyn SystemSet>;
/// A shorthand for `Box<dyn ScheduleLabel>`.
pub type BoxedScheduleLabel = Box<dyn ScheduleLabel>;

/// Types that identify logical groups of systems.
pub trait SystemSet: DynHash + Debug + Send + Sync + 'static {
    /// Returns `Some` if this system set is a [`SystemTypeSet`].
    fn system_type(&self) -> Option<TypeId> {
        None
    }

    /// Returns `true` if this system set is an [`AnonymousSet`].
    fn is_anonymous(&self) -> bool {
        false
    }
    /// Creates a boxed clone of the label corresponding to this system set.
    fn dyn_clone(&self) -> Box<dyn SystemSet>;

    /// Returns `true` if this system set is automatically populated based on
    /// systems' world accesses.
    /// The component tracked by this set is specified in either [`Self::reads_component`]
    /// or [`Self::writes_component`].
    fn is_access_set(&self) -> bool {
        self.reads_component().is_some() || self.writes_component().is_some()
    }

    /// If this returns `Some`, then that means any systems that have read-only access
    /// to the returned component will automatically be added to this set.
    /// This also means that outside configuration of this system set is disallowed.
    fn reads_component(&self) -> Option<ComponentDescriptor> {
        None
    }

    /// If this returns `Some`, then that means any systems that have mutable access
    /// to the returned component will automatically be added to this set.
    /// This also means that outside configuration of this system set is disallowed.
    fn writes_component(&self) -> Option<ComponentDescriptor> {
        None
    }
}

impl PartialEq for dyn SystemSet {
    fn eq(&self, other: &Self) -> bool {
        self.dyn_eq(other.as_dyn_eq())
    }
}

impl Eq for dyn SystemSet {}

impl Hash for dyn SystemSet {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.dyn_hash(state);
    }
}

impl Clone for Box<dyn SystemSet> {
    fn clone(&self) -> Self {
        self.dyn_clone()
    }
}

/// A [`SystemSet`] grouping instances of the same function.
///
/// This kind of set is automatically populated and thus has some special rules:
/// - You cannot manually add members.
/// - You cannot configure them.
/// - You cannot order something relative to one if it has more than one member.
pub struct SystemTypeSet<T: 'static>(PhantomData<fn() -> T>);

impl<T: 'static> SystemTypeSet<T> {
    pub(crate) fn new() -> Self {
        Self(PhantomData)
    }
}

impl<T> Debug for SystemTypeSet<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("SystemTypeSet")
            .field(&format_args!("fn {}()", &std::any::type_name::<T>()))
            .finish()
    }
}

impl<T> Hash for SystemTypeSet<T> {
    fn hash<H: Hasher>(&self, _state: &mut H) {
        // all systems of a given type are the same
    }
}
impl<T> Clone for SystemTypeSet<T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T> Copy for SystemTypeSet<T> {}

impl<T> PartialEq for SystemTypeSet<T> {
    #[inline]
    fn eq(&self, _other: &Self) -> bool {
        // all systems of a given type are the same
        true
    }
}

impl<T> Eq for SystemTypeSet<T> {}

impl<T> SystemSet for SystemTypeSet<T> {
    fn system_type(&self) -> Option<TypeId> {
        Some(TypeId::of::<T>())
    }

    fn dyn_clone(&self) -> Box<dyn SystemSet> {
        Box::new(*self)
    }
}

/// A [`SystemSet`] implicitly created when using
/// [`Schedule::add_systems`](super::Schedule::add_systems) or
/// [`Schedule::configure_sets`](super::Schedule::configure_sets).
#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq)]
pub struct AnonymousSet(usize);

static NEXT_ANONYMOUS_SET_ID: AtomicUsize = AtomicUsize::new(0);

impl AnonymousSet {
    pub(crate) fn new() -> Self {
        Self(NEXT_ANONYMOUS_SET_ID.fetch_add(1, Ordering::Relaxed))
    }
}

impl SystemSet for AnonymousSet {
    fn is_anonymous(&self) -> bool {
        true
    }

    fn dyn_clone(&self) -> Box<dyn SystemSet> {
        Box::new(*self)
    }
}

macro_rules! access_set_enum {
    ( $(#[$($tt:tt)*])* $name:ident) => {
        $(#[$($tt)*])*
        #[non_exhaustive] // Discourages people from matching on `_Marker`.
        pub enum $name<T> {
            /// Any systems with read-only access to `T` will be added to this set.
            Reads,
            /// Any systems with mutable access to `T` will be added to this set.
            Writes,
            /// Any systems with access to `T` will be added to this set.
            ReadsOrWrites,
            #[doc(hidden)]
            _Marker(PhantomData<T>, std::convert::Infallible),
        }

        impl<T> Clone for $name<T> {
            fn clone(&self) -> Self {
                *self
            }
        }

        impl<T> Copy for $name<T> {}

        impl<T> PartialEq for $name<T> {
            fn eq(&self, other: &Self) -> bool {
                use $name::*;
                match (self, other) {
                    (Reads, Reads) | (Writes, Writes) | (ReadsOrWrites, ReadsOrWrites) => true,
                    _ => false,
                }
            }
        }

        impl<T> Eq for $name<T> {}

        impl<T> Hash for $name<T> {
            fn hash<H: Hasher>(&self, state: &mut H) {
                match self {
                    $name::Reads => 0,
                    $name::Writes => 1,
                    $name::ReadsOrWrites => 2,
                    _ => 3,
                }.hash(state);
            }
        }
    };
}

access_set_enum!(
    /// [`SystemSet`]s that are automatically populated with systems that access the component `T`.
    ComponentAccessSet
);

impl<T: 'static> std::fmt::Debug for ComponentAccessSet<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Reads => write!(f, "ReadsComponent")?,
            Self::Writes => write!(f, "WritesComponent")?,
            Self::ReadsOrWrites => write!(f, "ReadsOrWritesComponent")?,
            _ => {}
        }
        write!(f, "<{}>", std::any::type_name::<T>())
    }
}

impl<T: Component> SystemSet for ComponentAccessSet<T> {
    fn dyn_clone(&self) -> Box<dyn SystemSet> {
        Box::new(*self)
    }

    fn reads_component(&self) -> Option<ComponentDescriptor> {
        match self {
            Self::Reads | Self::ReadsOrWrites => Some(ComponentDescriptor::new::<T>()),
            _ => None,
        }
    }

    fn writes_component(&self) -> Option<ComponentDescriptor> {
        match self {
            Self::Writes | Self::ReadsOrWrites => Some(ComponentDescriptor::new::<T>()),
            _ => None,
        }
    }
}

access_set_enum!(
    /// [`SystemSet`]s that are automatically populated with systems that access the resource `T`.
    ResourceAccessSet
);

impl<T: 'static> std::fmt::Debug for ResourceAccessSet<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Reads => write!(f, "ReadsResource")?,
            Self::Writes => write!(f, "WritesResource")?,
            Self::ReadsOrWrites => write!(f, "ReadsOrWritesResource")?,
            _ => {}
        }
        write!(f, "<{}>", std::any::type_name::<T>())
    }
}

impl<T: Resource> SystemSet for ResourceAccessSet<T> {
    fn dyn_clone(&self) -> Box<dyn SystemSet> {
        Box::new(*self)
    }

    fn reads_component(&self) -> Option<ComponentDescriptor> {
        match self {
            Self::Reads | Self::ReadsOrWrites => Some(ComponentDescriptor::new_resource::<T>()),
            _ => None,
        }
    }

    fn writes_component(&self) -> Option<ComponentDescriptor> {
        match self {
            Self::Writes | Self::ReadsOrWrites => Some(ComponentDescriptor::new_resource::<T>()),
            _ => None,
        }
    }
}

/// Types that can be converted into a [`SystemSet`].
pub trait IntoSystemSet<Marker>: Sized {
    /// The type of [`SystemSet`] this instance converts into.
    type Set: SystemSet;

    /// Converts this instance to its associated [`SystemSet`] type.
    fn into_system_set(self) -> Self::Set;
}

// systems sets
impl<S: SystemSet> IntoSystemSet<()> for S {
    type Set = Self;

    #[inline]
    fn into_system_set(self) -> Self::Set {
        self
    }
}

// systems
impl<Marker, F> IntoSystemSet<(IsFunctionSystem, Marker)> for F
where
    F: SystemParamFunction<Marker>,
{
    type Set = SystemTypeSet<Self>;

    #[inline]
    fn into_system_set(self) -> Self::Set {
        SystemTypeSet::new()
    }
}

// exclusive systems
impl<Marker, F> IntoSystemSet<(IsExclusiveFunctionSystem, Marker)> for F
where
    F: ExclusiveSystemParamFunction<Marker>,
{
    type Set = SystemTypeSet<Self>;

    #[inline]
    fn into_system_set(self) -> Self::Set {
        SystemTypeSet::new()
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        schedule::{tests::ResMut, Schedule},
        system::Resource,
    };

    use super::*;

    #[test]
    fn test_boxed_label() {
        use crate::{self as bevy_ecs, world::World};

        #[derive(Resource)]
        struct Flag(bool);

        #[derive(ScheduleLabel, Debug, Default, Clone, Copy, PartialEq, Eq, Hash)]
        struct A;

        let mut world = World::new();

        let mut schedule = Schedule::new(A);
        schedule.add_systems(|mut flag: ResMut<Flag>| flag.0 = true);
        world.add_schedule(schedule);

        let boxed: Box<dyn ScheduleLabel> = Box::new(A);

        world.insert_resource(Flag(false));
        world.run_schedule(&boxed);
        assert!(world.resource::<Flag>().0);

        world.insert_resource(Flag(false));
        world.run_schedule(boxed);
        assert!(world.resource::<Flag>().0);
    }
}
