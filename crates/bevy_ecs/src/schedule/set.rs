use std::any::TypeId;
use std::fmt::Debug;
use std::hash::{Hash, Hasher};
use std::marker::PhantomData;

pub use crate::label::DynEq;
pub use bevy_ecs_macros::{ScheduleLabel, SystemSet};

use crate::{
    define_label,
    intern::Interned,
    system::{
        ExclusiveFunctionSystem, ExclusiveSystemParamFunction, FunctionSystem,
        IsExclusiveFunctionSystem, IsFunctionSystem, SystemParamFunction,
    },
};

define_label!(
    /// A strongly-typed class of labels used to identify a [`Schedule`](crate::schedule::Schedule).
    ScheduleLabel,
    SCHEDULE_LABEL_INTERNER
);

define_label!(
    /// Types that identify logical groups of systems.
    SystemSet,
    SYSTEM_SET_INTERNER,
    extra_methods: {
        /// Returns `Some` if this system set is a [`SystemTypeSet`].
        fn system_type(&self) -> Option<TypeId> {
            None
        }

        /// Returns `true` if this system set is an [`AnonymousSet`].
        fn is_anonymous(&self) -> bool {
            false
        }
    },
    extra_methods_impl: {
        fn system_type(&self) -> Option<TypeId> {
            (**self).system_type()
        }

        fn is_anonymous(&self) -> bool {
            (**self).is_anonymous()
        }
    }
);

/// A shorthand for `Interned<dyn SystemSet>`.
pub type InternedSystemSet = Interned<dyn SystemSet>;
/// A shorthand for `Interned<dyn ScheduleLabel>`.
pub type InternedScheduleLabel = Interned<dyn ScheduleLabel>;

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

    fn as_dyn_eq(&self) -> &dyn DynEq {
        self
    }

    fn dyn_hash(&self, mut state: &mut dyn Hasher) {
        TypeId::of::<Self>().hash(&mut state);
        self.hash(&mut state);
    }
}

/// A [`SystemSet`] implicitly created when using
/// [`Schedule::add_systems`](super::Schedule::add_systems) or
/// [`Schedule::configure_sets`](super::Schedule::configure_sets).
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

    fn as_dyn_eq(&self) -> &dyn DynEq {
        self
    }

    fn dyn_hash(&self, mut state: &mut dyn Hasher) {
        TypeId::of::<Self>().hash(&mut state);
        self.hash(&mut state);
    }
}

/// Types that can be converted into a [`SystemSet`].
#[diagnostic::on_unimplemented(
    message = "`{Self}` is not a system set",
    label = "invalid system set"
)]
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
    Marker: 'static,
    F: SystemParamFunction<Marker>,
{
    type Set = SystemTypeSet<FunctionSystem<Marker, F>>;

    #[inline]
    fn into_system_set(self) -> Self::Set {
        SystemTypeSet::<FunctionSystem<Marker, F>>::new()
    }
}

// exclusive systems
impl<Marker, F> IntoSystemSet<(IsExclusiveFunctionSystem, Marker)> for F
where
    Marker: 'static,
    F: ExclusiveSystemParamFunction<Marker>,
{
    type Set = SystemTypeSet<ExclusiveFunctionSystem<Marker, F>>;

    #[inline]
    fn into_system_set(self) -> Self::Set {
        SystemTypeSet::<ExclusiveFunctionSystem<Marker, F>>::new()
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
    fn test_schedule_label() {
        use crate::{self as bevy_ecs, world::World};

        #[derive(Resource)]
        struct Flag(bool);

        #[derive(ScheduleLabel, Debug, Default, Clone, Copy, PartialEq, Eq, Hash)]
        struct A;

        #[derive(ScheduleLabel, Debug, Default, Clone, Copy, PartialEq, Eq, Hash)]
        struct B;

        let mut world = World::new();

        let mut schedule = Schedule::new(A);
        schedule.add_systems(|mut flag: ResMut<Flag>| flag.0 = true);
        world.add_schedule(schedule);

        let interned = A.intern();

        world.insert_resource(Flag(false));
        world.run_schedule(interned);
        assert!(world.resource::<Flag>().0);

        world.insert_resource(Flag(false));
        world.run_schedule(interned);
        assert!(world.resource::<Flag>().0);

        assert_ne!(A.intern(), B.intern());
    }

    #[test]
    fn test_derive_schedule_label() {
        use crate::{self as bevy_ecs};

        #[derive(ScheduleLabel, Debug, Default, Clone, Copy, PartialEq, Eq, Hash)]
        struct UnitLabel;

        #[derive(ScheduleLabel, Debug, Default, Clone, Copy, PartialEq, Eq, Hash)]
        struct TupleLabel(u32, u32);

        #[derive(ScheduleLabel, Debug, Default, Clone, Copy, PartialEq, Eq, Hash)]
        struct StructLabel {
            a: u32,
            b: u32,
        }

        #[derive(ScheduleLabel, Debug, Default, Clone, Copy, PartialEq, Eq, Hash)]
        struct EmptyTupleLabel();

        #[derive(ScheduleLabel, Debug, Default, Clone, Copy, PartialEq, Eq, Hash)]
        struct EmptyStructLabel {}

        #[derive(ScheduleLabel, Debug, Default, Clone, Copy, PartialEq, Eq, Hash)]
        enum EnumLabel {
            #[default]
            Unit,
            Tuple(u32, u32),
            Struct {
                a: u32,
                b: u32,
            },
        }

        #[derive(ScheduleLabel, Debug, Default, Clone, Copy, PartialEq, Eq, Hash)]
        struct GenericLabel<T>(PhantomData<T>);

        assert_eq!(UnitLabel.intern(), UnitLabel.intern());
        assert_eq!(EnumLabel::Unit.intern(), EnumLabel::Unit.intern());
        assert_ne!(UnitLabel.intern(), EnumLabel::Unit.intern());
        assert_ne!(UnitLabel.intern(), TupleLabel(0, 0).intern());
        assert_ne!(EnumLabel::Unit.intern(), EnumLabel::Tuple(0, 0).intern());

        assert_eq!(TupleLabel(0, 0).intern(), TupleLabel(0, 0).intern());
        assert_eq!(
            EnumLabel::Tuple(0, 0).intern(),
            EnumLabel::Tuple(0, 0).intern()
        );
        assert_ne!(TupleLabel(0, 0).intern(), TupleLabel(0, 1).intern());
        assert_ne!(
            EnumLabel::Tuple(0, 0).intern(),
            EnumLabel::Tuple(0, 1).intern()
        );
        assert_ne!(TupleLabel(0, 0).intern(), EnumLabel::Tuple(0, 0).intern());
        assert_ne!(
            TupleLabel(0, 0).intern(),
            StructLabel { a: 0, b: 0 }.intern()
        );
        assert_ne!(
            EnumLabel::Tuple(0, 0).intern(),
            EnumLabel::Struct { a: 0, b: 0 }.intern()
        );

        assert_eq!(
            StructLabel { a: 0, b: 0 }.intern(),
            StructLabel { a: 0, b: 0 }.intern()
        );
        assert_eq!(
            EnumLabel::Struct { a: 0, b: 0 }.intern(),
            EnumLabel::Struct { a: 0, b: 0 }.intern()
        );
        assert_ne!(
            StructLabel { a: 0, b: 0 }.intern(),
            StructLabel { a: 0, b: 1 }.intern()
        );
        assert_ne!(
            EnumLabel::Struct { a: 0, b: 0 }.intern(),
            EnumLabel::Struct { a: 0, b: 1 }.intern()
        );
        assert_ne!(
            StructLabel { a: 0, b: 0 }.intern(),
            EnumLabel::Struct { a: 0, b: 0 }.intern()
        );
        assert_ne!(
            StructLabel { a: 0, b: 0 }.intern(),
            EnumLabel::Struct { a: 0, b: 0 }.intern()
        );
        assert_ne!(StructLabel { a: 0, b: 0 }.intern(), UnitLabel.intern(),);
        assert_ne!(
            EnumLabel::Struct { a: 0, b: 0 }.intern(),
            EnumLabel::Unit.intern()
        );

        assert_eq!(
            GenericLabel::<u32>(PhantomData).intern(),
            GenericLabel::<u32>(PhantomData).intern()
        );
        assert_ne!(
            GenericLabel::<u32>(PhantomData).intern(),
            GenericLabel::<u64>(PhantomData).intern()
        );
    }

    #[test]
    fn test_derive_system_set() {
        use crate::{self as bevy_ecs};

        #[derive(SystemSet, Debug, Default, Clone, Copy, PartialEq, Eq, Hash)]
        struct UnitSet;

        #[derive(SystemSet, Debug, Default, Clone, Copy, PartialEq, Eq, Hash)]
        struct TupleSet(u32, u32);

        #[derive(SystemSet, Debug, Default, Clone, Copy, PartialEq, Eq, Hash)]
        struct StructSet {
            a: u32,
            b: u32,
        }

        #[derive(SystemSet, Debug, Default, Clone, Copy, PartialEq, Eq, Hash)]
        struct EmptyTupleSet();

        #[derive(SystemSet, Debug, Default, Clone, Copy, PartialEq, Eq, Hash)]
        struct EmptyStructSet {}

        #[derive(SystemSet, Debug, Default, Clone, Copy, PartialEq, Eq, Hash)]
        enum EnumSet {
            #[default]
            Unit,
            Tuple(u32, u32),
            Struct {
                a: u32,
                b: u32,
            },
        }

        #[derive(SystemSet, Debug, Default, Clone, Copy, PartialEq, Eq, Hash)]
        struct GenericSet<T>(PhantomData<T>);

        assert_eq!(UnitSet.intern(), UnitSet.intern());
        assert_eq!(EnumSet::Unit.intern(), EnumSet::Unit.intern());
        assert_ne!(UnitSet.intern(), EnumSet::Unit.intern());
        assert_ne!(UnitSet.intern(), TupleSet(0, 0).intern());
        assert_ne!(EnumSet::Unit.intern(), EnumSet::Tuple(0, 0).intern());

        assert_eq!(TupleSet(0, 0).intern(), TupleSet(0, 0).intern());
        assert_eq!(EnumSet::Tuple(0, 0).intern(), EnumSet::Tuple(0, 0).intern());
        assert_ne!(TupleSet(0, 0).intern(), TupleSet(0, 1).intern());
        assert_ne!(EnumSet::Tuple(0, 0).intern(), EnumSet::Tuple(0, 1).intern());
        assert_ne!(TupleSet(0, 0).intern(), EnumSet::Tuple(0, 0).intern());
        assert_ne!(TupleSet(0, 0).intern(), StructSet { a: 0, b: 0 }.intern());
        assert_ne!(
            EnumSet::Tuple(0, 0).intern(),
            EnumSet::Struct { a: 0, b: 0 }.intern()
        );

        assert_eq!(
            StructSet { a: 0, b: 0 }.intern(),
            StructSet { a: 0, b: 0 }.intern()
        );
        assert_eq!(
            EnumSet::Struct { a: 0, b: 0 }.intern(),
            EnumSet::Struct { a: 0, b: 0 }.intern()
        );
        assert_ne!(
            StructSet { a: 0, b: 0 }.intern(),
            StructSet { a: 0, b: 1 }.intern()
        );
        assert_ne!(
            EnumSet::Struct { a: 0, b: 0 }.intern(),
            EnumSet::Struct { a: 0, b: 1 }.intern()
        );
        assert_ne!(
            StructSet { a: 0, b: 0 }.intern(),
            EnumSet::Struct { a: 0, b: 0 }.intern()
        );
        assert_ne!(
            StructSet { a: 0, b: 0 }.intern(),
            EnumSet::Struct { a: 0, b: 0 }.intern()
        );
        assert_ne!(StructSet { a: 0, b: 0 }.intern(), UnitSet.intern(),);
        assert_ne!(
            EnumSet::Struct { a: 0, b: 0 }.intern(),
            EnumSet::Unit.intern()
        );

        assert_eq!(
            GenericSet::<u32>(PhantomData).intern(),
            GenericSet::<u32>(PhantomData).intern()
        );
        assert_ne!(
            GenericSet::<u32>(PhantomData).intern(),
            GenericSet::<u64>(PhantomData).intern()
        );
    }
}
