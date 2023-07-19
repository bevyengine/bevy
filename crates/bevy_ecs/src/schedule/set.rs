use std::any::TypeId;
use std::fmt::Debug;
use std::hash::{Hash, Hasher};
use std::marker::PhantomData;
use std::sync::atomic::{AtomicUsize, Ordering};

pub use bevy_ecs_macros::{ScheduleLabel, SystemSet};
use bevy_utils::label::DynHash;

use crate::system::{
    ExclusiveSystemParamFunction, IsExclusiveFunctionSystem, IsFunctionSystem, SystemParamFunction,
};

/// Identifies a logical group of systems.
pub trait SystemSet: DynHash + Debug + Send + Sync + 'static {
    /// Returns the [`TypeId`] of the system if the system set is a [`SystemTypeSet`].
    ///
    /// A [`SystemTypeSet`] has special properties:
    /// - You cannot manually add systems or sets to it.
    /// - You cannot configure it.
    /// - You cannot order relative to it if it contains more than one instance.
    ///
    /// These sets are automatically populated, so these constraints exist to prevent unintentional ambiguity.
    fn system_type(&self) -> Option<TypeId> {
        None
    }

    /// Returns `true` if the system set is an [`AnonymousSet`].
    fn is_anonymous(&self) -> bool {
        false
    }

    /// Returns the unique, type-elided identifier for the system set.
    fn as_label(&self) -> SystemSetId {
        SystemSetId::of(self)
    }

    /// Returns the type-elided version of the system set.
    fn as_untyped(&self) -> SystemSetUntyped {
        SystemSetUntyped::of(self)
    }
}

/// A lightweight and printable identifier for a [`SystemSet`].
#[derive(Clone, Copy, Eq)]
pub struct SystemSetId(&'static str);

impl SystemSetId {
    /// Returns the [`SystemSetId`] of the [`SystemSet`].
    pub fn of<S: SystemSet + ?Sized>(set: &S) -> SystemSetId {
        let str = bevy_utils::label::intern_debug_string(set);
        SystemSetId(str)
    }
}

impl PartialEq for SystemSetId {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self.0, other.0)
    }
}

impl Hash for SystemSetId {
    fn hash<H: Hasher>(&self, state: &mut H) {
        std::ptr::hash(self.0, state);
    }
}

impl Debug for SystemSetId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

/// The different variants of system sets.
#[derive(Clone, Copy, Eq, PartialEq, Hash)]
enum SystemSetKind {
    Anonymous,
    Named,
    SystemType(TypeId),
}

/// Type-elided struct whose methods return the same values as its original [`SystemSet`].
#[derive(Clone, Copy, Eq, PartialEq, Hash)]
pub struct SystemSetUntyped {
    id: SystemSetId,
    kind: SystemSetKind,
}

impl SystemSetUntyped {
    /// Converts a [`SystemSet`] into the equivalent [`SystemSetUntyped`].
    pub(crate) fn of<S: SystemSet + ?Sized>(set: &S) -> SystemSetUntyped {
        assert!(!(set.is_anonymous() && set.system_type().is_some()));
        let kind = if let Some(type_id) = set.system_type() {
            SystemSetKind::SystemType(type_id)
        } else if set.is_anonymous() {
            SystemSetKind::Anonymous
        } else {
            SystemSetKind::Named
        };

        SystemSetUntyped {
            id: SystemSetId::of(set),
            kind,
        }
    }

    /// Returns the [`TypeId`] of the system if the system set is a [`SystemTypeSet`].
    ///
    /// A [`SystemTypeSet`] has special properties:
    /// - You cannot manually add systems or sets to it.
    /// - You cannot configure it.
    /// - You cannot order relative to it if it contains more than one instance.
    ///
    /// These sets are automatically populated, so these constraints exist to prevent unintentional ambiguity.
    pub fn system_type(&self) -> Option<TypeId> {
        if let SystemSetKind::SystemType(type_id) = self.kind {
            Some(type_id)
        } else {
            None
        }
    }

    /// Returns `true` if this system set is an [`AnonymousSet`].
    pub fn is_anonymous(&self) -> bool {
        matches!(self.kind, SystemSetKind::Anonymous)
    }

    /// Returns the [`SystemSetId`] of the set.
    pub fn id(&self) -> SystemSetId {
        self.id
    }
}

impl Debug for SystemSetUntyped {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.id.fmt(f)
    }
}

/// A [`SystemSet`] grouping instances of the same function.
///
/// These sets have special properties:
/// - You cannot manually add systems or sets to them.
/// - You cannot configure them.
/// - You cannot order relative to one if it has more than one instance.
///
/// These sets are automatically populated, so these constraints exist to prevent unintentional ambiguity.
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
        Self(PhantomData)
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

    fn is_anonymous(&self) -> bool {
        false
    }
}

/// A [`SystemSet`] implicitly created when using
/// [`Schedule::add_systems`](super::Schedule::add_systems).
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
