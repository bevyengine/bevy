use std::any::TypeId;
use std::borrow::Borrow;
use std::fmt::Debug;
use std::hash::{Hash, Hasher};
use std::marker::PhantomData;

pub use bevy_ecs_macros::{ScheduleLabel, SystemSet};
use bevy_utils::define_interned_label;
use bevy_utils::intern::{Interned, Leak, OptimizedInterner, StaticRef};
use bevy_utils::label::DynHash;

use crate::system::{
    ExclusiveSystemParamFunction, IsExclusiveFunctionSystem, IsFunctionSystem, SystemParamFunction,
};

define_interned_label!(ScheduleLabel, SCHEDULE_LABEL_INTERNER);

static SYSTEM_SET_INTERNER: OptimizedInterner<dyn SystemSet> = OptimizedInterner::new();
/// A shorthand for `Interned<dyn SystemSet>`.
pub type InternedSystemSet = Interned<dyn SystemSet>;
/// A shorthand for `Interned<dyn ScheduleLabel>`.
pub type InternedScheduleLabel = Interned<dyn ScheduleLabel>;

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

    /// Returns a static reference to a value equal to `self`, if possible.
    /// This method is used to optimize [interning](bevy_utils::intern).
    ///
    /// # Invariant
    ///
    /// The following invariants must be hold:
    ///
    /// `ptr_eq(a.dyn_static_ref(), b.dyn_static_ref())` if `a.dyn_eq(b)`
    /// `ptr_neq(a.dyn_static_ref(), b.dyn_static_ref())` if `!a.dyn_eq(b)`
    ///
    /// where `ptr_eq` and `ptr_neq` are defined as :
    /// ```
    /// fn ptr_eq<T>(x: Option<&'static T>, y: Option<&'static T>) -> bool {
    ///     match (x, y) {
    ///         (Some(x), Some(y)) => std::ptr::eq(x, y),
    ///         (None, None) => true,
    ///         _ => false,
    ///     }
    /// }
    ///
    /// fn ptr_neq<T>(x: Option<&'static T>, y: Option<&'static T>) -> bool {
    ///     match (x, y) {
    ///         (Some(x), Some(y)) => !std::ptr::eq(x, y),
    ///         (None, None) => true,
    ///         _ => false,
    ///     }
    /// }
    /// ```
    ///
    /// # Provided implementation
    ///
    /// The provided implementation always returns `None`.
    fn dyn_static_ref(&self) -> Option<&'static dyn SystemSet> {
        None
    }
}

impl From<&dyn SystemSet> for Interned<dyn SystemSet> {
    fn from(value: &dyn SystemSet) -> Interned<dyn SystemSet> {
        struct LeakHelper<'a>(&'a dyn SystemSet);

        impl<'a> Borrow<dyn SystemSet> for LeakHelper<'a> {
            fn borrow(&self) -> &dyn SystemSet {
                self.0
            }
        }

        impl<'a> Leak<dyn SystemSet> for LeakHelper<'a> {
            fn leak(self) -> &'static dyn SystemSet {
                Box::leak(self.0.dyn_clone())
            }
        }

        SYSTEM_SET_INTERNER.intern(LeakHelper(value))
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

impl StaticRef for dyn SystemSet {
    fn static_ref(&self) -> Option<&'static dyn SystemSet> {
        self.dyn_static_ref()
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
            .field(&std::any::type_name::<T>())
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

    fn dyn_static_ref(&self) -> Option<&'static dyn SystemSet> {
        Some(&Self(PhantomData))
    }
}

/// A [`SystemSet`] implicitly created when using
/// [`Schedule::add_systems`](super::Schedule::add_systems).
#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq)]
pub struct AnonymousSet(usize);

impl AnonymousSet {
    pub(crate) fn new(id: usize) -> Self {
        Self(id)
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
    fn test_interned_label() {
        use crate::{self as bevy_ecs, world::World};

        #[derive(Resource)]
        struct Flag(bool);

        #[derive(ScheduleLabel, Debug, Default, Clone, Copy, PartialEq, Eq, Hash)]
        struct A;

        let mut world = World::new();

        let mut schedule = Schedule::new();
        schedule.add_systems(|mut flag: ResMut<Flag>| flag.0 = true);
        world.add_schedule(schedule, A);

        let interned = InternedScheduleLabel::from(&A as &dyn ScheduleLabel);

        world.insert_resource(Flag(false));
        world.run_schedule(interned);
        assert!(world.resource::<Flag>().0);

        world.insert_resource(Flag(false));
        world.run_schedule(interned);
        assert!(world.resource::<Flag>().0);
    }
}
