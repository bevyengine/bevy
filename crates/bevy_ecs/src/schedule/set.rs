use std::any::TypeId;
use std::fmt::Debug;
use std::hash::{Hash, Hasher};
use std::marker::PhantomData;

pub use bevy_ecs_macros::{ScheduleLabel, SystemSet};
use bevy_utils::define_label;
use bevy_utils::intern::{Interned, Interner, Leak};
use bevy_utils::label::DynEq;

use crate::system::{
    ExclusiveSystemParamFunction, IsExclusiveFunctionSystem, IsFunctionSystem, SystemParamFunction,
};

define_label!(
    /// A strongly-typed class of labels used to identify an [`Schedule`].
    ScheduleLabel,
    SCHEDULE_LABEL_INTERNER
);

static SYSTEM_SET_INTERNER: Interner<dyn SystemSet> = Interner::new();
/// A shorthand for `Interned<dyn SystemSet>`.
pub type InternedSystemSet = Interned<dyn SystemSet>;
/// A shorthand for `Interned<dyn ScheduleLabel>`.
pub type InternedScheduleLabel = Interned<dyn ScheduleLabel>;

/// Types that identify logical groups of systems.
pub trait SystemSet: Debug + Send + Sync + 'static {
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

    /// Casts this value to a form where it can be compared with other type-erased values.
    fn as_dyn_eq(&self) -> &dyn DynEq;

    /// Feeds this value into the given [`Hasher`].
    fn dyn_hash(&self, state: &mut dyn Hasher);

    /// Returns a static reference to a value equal to `self`, if possible.
    /// This method is used to optimize [interning](bevy_utils::intern).
    ///
    /// # Invariant
    ///
    /// The following invariants most hold:
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
    /// The provided implementation always returns `None`.
    fn dyn_static_ref(&self) -> Option<&'static dyn SystemSet> {
        None
    }

    /// Returns an [`InternedSystemSet`] corresponding to `self`.
    fn intern(&self) -> InternedSystemSet
    where
        Self: Sized,
    {
        SYSTEM_SET_INTERNER.intern(self)
    }
}

impl SystemSet for InternedSystemSet {
    fn system_type(&self) -> Option<TypeId> {
        (**self).system_type()
    }

    fn is_anonymous(&self) -> bool {
        (**self).is_anonymous()
    }

    fn dyn_clone(&self) -> Box<dyn SystemSet> {
        (**self).dyn_clone()
    }

    fn as_dyn_eq(&self) -> &dyn DynEq {
        (**self).as_dyn_eq()
    }

    fn dyn_hash(&self, state: &mut dyn Hasher) {
        (**self).dyn_hash(state);
    }

    fn dyn_static_ref(&self) -> Option<&'static dyn SystemSet> {
        Some(self.0)
    }

    fn intern(&self) -> Self {
        *self
    }
}

impl PartialEq for dyn SystemSet {
    fn eq(&self, other: &Self) -> bool {
        self.as_dyn_eq().dyn_eq(other.as_dyn_eq())
    }
}

impl Eq for dyn SystemSet {}

impl Hash for dyn SystemSet {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.dyn_hash(state);
    }
}

impl Leak for dyn SystemSet {
    fn leak(&self) -> &'static Self {
        Box::leak(self.dyn_clone())
    }

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

    fn as_dyn_eq(&self) -> &dyn DynEq {
        self
    }

    fn dyn_hash(&self, mut state: &mut dyn Hasher) {
        std::any::TypeId::of::<Self>().hash(&mut state);
        self.hash(&mut state);
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

    fn as_dyn_eq(&self) -> &dyn DynEq {
        self
    }

    fn dyn_hash(&self, mut state: &mut dyn Hasher) {
        std::any::TypeId::of::<Self>().hash(&mut state);
        self.hash(&mut state);
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
    fn test_schedule_label() {
        use crate::{self as bevy_ecs, world::World};

        #[derive(Resource)]
        struct Flag(bool);

        #[derive(ScheduleLabel, Debug, Default, Clone, Copy, PartialEq, Eq, Hash)]
        struct A;

        let mut world = World::new();

        let mut schedule = Schedule::new();
        schedule.add_systems(|mut flag: ResMut<Flag>| flag.0 = true);
        world.add_schedule(schedule, A);

        let interned = A.intern();

        world.insert_resource(Flag(false));
        world.run_schedule(interned);
        assert!(world.resource::<Flag>().0);

        world.insert_resource(Flag(false));
        world.run_schedule(interned);
        assert!(world.resource::<Flag>().0);
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
            EmptyTuple(),
            EmptyStruct {},
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

        assert!(UnitLabel.dyn_static_ref().is_some());
        assert!(EmptyTupleLabel().dyn_static_ref().is_some());
        assert!(EmptyStructLabel {}.dyn_static_ref().is_some());
        assert!(EnumLabel::Unit.dyn_static_ref().is_some());
        assert!(EnumLabel::EmptyTuple().dyn_static_ref().is_some());
        assert!(EnumLabel::EmptyStruct {}.dyn_static_ref().is_some());
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
            EmptyTuple(),
            EmptyStruct {},
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

        assert!(UnitSet.dyn_static_ref().is_some());
        assert!(EmptyTupleSet().dyn_static_ref().is_some());
        assert!(EmptyStructSet {}.dyn_static_ref().is_some());
        assert!(EnumSet::Unit.dyn_static_ref().is_some());
        assert!(EnumSet::EmptyTuple().dyn_static_ref().is_some());
        assert!(EnumSet::EmptyStruct {}.dyn_static_ref().is_some());
    }
}
