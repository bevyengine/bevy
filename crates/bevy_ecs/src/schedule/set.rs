use std::any::TypeId;
use std::fmt::Debug;
use std::hash::{Hash, Hasher};
use std::marker::PhantomData;
use std::sync::atomic::{AtomicUsize, Ordering};

pub use bevy_ecs_macros::{ScheduleLabel, SystemSet};
use bevy_utils::define_boxed_label;
use bevy_utils::label::DynHash;

use crate::system::{
    ExclusiveSystemParamFunction, IsExclusiveFunctionSystem, IsFunctionSystem, SystemParamFunction,
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
