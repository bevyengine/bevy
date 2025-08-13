use alloc::boxed::Box;
use bevy_utils::prelude::DebugName;
use core::{
    any::TypeId,
    fmt::Debug,
    hash::{Hash, Hasher},
    marker::PhantomData,
};

pub use crate::label::DynEq;
pub use bevy_ecs_macros::{ScheduleLabel, SystemSet};

use crate::{
    define_label,
    intern::Interned,
    system::{
        ExclusiveFunctionSystem, ExclusiveSystemParamFunction, FunctionSystem, IntoResult,
        IsExclusiveFunctionSystem, IsFunctionSystem, SystemParamFunction,
    },
};

define_label!(
    /// A strongly-typed class of labels used to identify a [`Schedule`].
    ///
    /// Each schedule in a [`World`] has a unique schedule label value, and
    /// schedules can be automatically created from labels via [`Schedules::add_systems()`].
    ///
    /// # Defining new schedule labels
    ///
    /// By default, you should use Bevy's premade schedule labels which implement this trait.
    /// If you are using [`bevy_ecs`] directly or if you need to run a group of systems outside
    /// the existing schedules, you may define your own schedule labels by using
    /// `#[derive(ScheduleLabel)]`.
    ///
    /// ```
    /// use bevy_ecs::prelude::*;
    /// use bevy_ecs::schedule::ScheduleLabel;
    ///
    /// // Declare a new schedule label.
    /// #[derive(ScheduleLabel, Clone, Debug, PartialEq, Eq, Hash, Default)]
    /// struct Update;
    ///
    /// let mut world = World::new();
    ///
    /// // Add a system to the schedule with that label (creating it automatically).
    /// fn a_system_function() {}
    /// world.get_resource_or_init::<Schedules>().add_systems(Update, a_system_function);
    ///
    /// // Run the schedule, and therefore run the system.
    /// world.run_schedule(Update);
    /// ```
    ///
    /// [`Schedule`]: crate::schedule::Schedule
    /// [`Schedules::add_systems()`]: crate::schedule::Schedules::add_systems
    /// [`World`]: crate::world::World
    #[diagnostic::on_unimplemented(
        note = "consider annotating `{Self}` with `#[derive(ScheduleLabel)]`"
    )]
    ScheduleLabel,
    SCHEDULE_LABEL_INTERNER
);

define_label!(
    /// System sets are tag-like labels that can be used to group systems together.
    ///
    /// This allows you to share configuration (like run conditions) across multiple systems,
    /// and order systems or system sets relative to conceptual groups of systems.
    /// To control the behavior of a system set as a whole, use [`Schedule::configure_sets`](crate::prelude::Schedule::configure_sets),
    /// or the method of the same name on `App`.
    ///
    /// Systems can belong to any number of system sets, reflecting multiple roles or facets that they might have.
    /// For example, you may want to annotate a system as "consumes input" and "applies forces",
    /// and ensure that your systems are ordered correctly for both of those sets.
    ///
    /// System sets can belong to any number of other system sets,
    /// allowing you to create nested hierarchies of system sets to group systems together.
    /// Configuration applied to system sets will flow down to their members (including other system sets),
    /// allowing you to set and modify the configuration in a single place.
    ///
    /// Systems sets are also useful for exposing a consistent public API for dependencies
    /// to hook into across versions of your crate,
    /// allowing them to add systems to a specific set, or order relative to that set,
    /// without leaking implementation details of the exact systems involved.
    ///
    /// ## Defining new system sets
    ///
    /// To create a new system set, use the `#[derive(SystemSet)]` macro.
    /// Unit structs are a good choice for one-off sets.
    ///
    /// ```rust
    /// # use bevy_ecs::prelude::*;
    ///
    /// #[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
    /// struct PhysicsSystems;
    /// ```
    ///
    /// When you want to define several related system sets,
    /// consider creating an enum system set.
    /// Each variant will be treated as a separate system set.
    ///
    /// ```rust
    /// # use bevy_ecs::prelude::*;
    ///
    /// #[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
    /// enum CombatSystems {
    ///    TargetSelection,
    ///    DamageCalculation,
    ///    Cleanup,
    /// }
    /// ```
    ///
    /// By convention, the listed order of the system set in the enum
    /// corresponds to the order in which the systems are run.
    /// Ordering must be explicitly added to ensure that this is the case,
    /// but following this convention will help avoid confusion.
    ///
    /// ### Adding systems to system sets
    ///
    /// To add systems to a system set, call [`in_set`](crate::prelude::IntoScheduleConfigs::in_set) on the system function
    /// while adding it to your app or schedule.
    ///
    /// Like usual, these methods can be chained with other configuration methods like [`before`](crate::prelude::IntoScheduleConfigs::before),
    /// or repeated to add systems to multiple sets.
    ///
    /// ```rust
    /// use bevy_ecs::prelude::*;
    ///
    /// #[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
    /// enum CombatSystems {
    ///    TargetSelection,
    ///    DamageCalculation,
    ///    Cleanup,
    /// }
    ///
    /// fn target_selection() {}
    ///
    /// fn enemy_damage_calculation() {}
    ///
    /// fn player_damage_calculation() {}
    ///
    /// let mut schedule = Schedule::default();
    /// // Configuring the sets to run in order.
    /// schedule.configure_sets((CombatSystems::TargetSelection, CombatSystems::DamageCalculation, CombatSystems::Cleanup).chain());
    ///
    /// // Adding a single system to a set.
    /// schedule.add_systems(target_selection.in_set(CombatSystems::TargetSelection));
    ///
    /// // Adding multiple systems to a set.
    /// schedule.add_systems((player_damage_calculation, enemy_damage_calculation).in_set(CombatSystems::DamageCalculation));
    /// ```
    #[diagnostic::on_unimplemented(
        note = "consider annotating `{Self}` with `#[derive(SystemSet)]`"
    )]
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
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_tuple("SystemTypeSet")
            .field(&format_args!("fn {}()", DebugName::type_name::<T>()))
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
///
/// # Usage notes
///
/// This trait should only be used as a bound for trait implementations or as an
/// argument to a function. If a system set needs to be returned from a function
/// or stored somewhere, use [`SystemSet`] instead of this trait.
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
    F::Out: IntoResult<()>,
    F: SystemParamFunction<Marker>,
{
    type Set = SystemTypeSet<FunctionSystem<Marker, (), F>>;

    #[inline]
    fn into_system_set(self) -> Self::Set {
        SystemTypeSet::<FunctionSystem<Marker, (), F>>::new()
    }
}

// exclusive systems
impl<Marker, F> IntoSystemSet<(IsExclusiveFunctionSystem, Marker)> for F
where
    Marker: 'static,
    F::Out: IntoResult<()>,
    F: ExclusiveSystemParamFunction<Marker>,
{
    type Set = SystemTypeSet<ExclusiveFunctionSystem<Marker, (), F>>;

    #[inline]
    fn into_system_set(self) -> Self::Set {
        SystemTypeSet::<ExclusiveFunctionSystem<Marker, (), F>>::new()
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        resource::Resource,
        schedule::{tests::ResMut, Schedule},
        system::{IntoSystem, System},
    };

    use super::*;

    #[test]
    fn test_schedule_label() {
        use crate::world::World;

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

    #[test]
    fn system_set_matches_default_system_set() {
        fn system() {}
        let set_from_into_system_set = IntoSystemSet::into_system_set(system).intern();
        let system = IntoSystem::into_system(system);
        let set_from_system = system.default_system_sets()[0];
        assert_eq!(set_from_into_system_set, set_from_system);
    }

    #[test]
    fn system_set_matches_default_system_set_exclusive() {
        fn system(_: &mut crate::world::World) {}
        let set_from_into_system_set = IntoSystemSet::into_system_set(system).intern();
        let system = IntoSystem::into_system(system);
        let set_from_system = system.default_system_sets()[0];
        assert_eq!(set_from_into_system_set, set_from_system);
    }
}
