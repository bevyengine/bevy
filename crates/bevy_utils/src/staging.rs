//! Provides an abstracted system for staging modifications to data structures that rarely change.
//! See [`StageOnWrite`] as a starting point.
//!
//! Here's an example of this utility in action for registering players. This is a bit contrived, but it should communicate the idea.
//!
//! ```
//! use core::mem::take;
//! use std::sync::RwLockReadGuard;
//! use core::ops::{Deref, DerefMut};
//!
//! use crate as bevy_utils;
//! use bevy_platform_support::collections::HashMap;
//! use bevy_platform_support::prelude::String;
//! use bevy_utils::staging::{
//!     MaybeStaged, StagableWrites, StageOnWrite, StagedChanges, StagedRef, StagedRefLocked,
//!     Stager, StagerLocked,
//! };
//!
//! /// Stores some arbitrary player data.
//! #[derive(Debug, Clone)]
//! pub struct PlayerData {
//!     name: String,
//!     secs_in_game: u32,
//!     id: PlayerId,
//! }
//!
//! /// A unique id per player
//! #[derive(Hash, Debug, Clone, Copy, PartialEq, Eq)]
//! pub struct PlayerId(u32);
//!
//! /// The standard collection of players
//! #[derive(Default, Debug)]
//! pub struct Players(HashMap<PlayerId, PlayerData>);
//!
//! /// When a change is made to a player
//! #[derive(Default, Debug)]
//! pub struct StagedPlayerChanges {
//!     replacements: HashMap<PlayerId, PlayerData>,
//!     additional_time_played: HashMap<PlayerId, u32>,
//! }
//!
//! impl StagedChanges for StagedPlayerChanges {
//!     type Cold = Players;
//!
//!     fn apply_staged(&mut self, storage: &mut Self::Cold) {
//!         for replaced in self.replacements.drain() {
//!             storage.0.insert(replaced.0, replaced.1);
//!         }
//!         for (id, new_time) in self.additional_time_played.iter_mut() {
//!             if let Some(player) = storage.0.get_mut(id) {
//!                 player.secs_in_game += take(new_time);
//!             }
//!         }
//!     }
//!
//!     fn any_staged(&self) -> bool {
//!         !self.replacements.is_empty() || !self.additional_time_played.is_empty()
//!     }
//! }
//!
//! /// Allows read only access to player data.
//! trait PlayerAccess {
//!     fn get_name(&self, id: PlayerId) -> Option<impl Deref<Target = str>>;
//!     fn get_secs_in_game(&self, id: PlayerId) -> Option<u32>;
//! }
//!
//! impl PlayerAccess for Players {
//!     fn get_name(&self, id: PlayerId) -> Option<impl Deref<Target = str>> {
//!         self.0.get(&id).map(|player| player.name.as_str())
//!     }
//!
//!     fn get_secs_in_game(&self, id: PlayerId) -> Option<u32> {
//!         self.0.get(&id).map(|player| player.secs_in_game)
//!     }
//! }
//!
//! impl PlayerAccess for StagedRef<'_, StagedPlayerChanges> {
//!     fn get_name(&self, id: PlayerId) -> Option<impl Deref<Target = str>> {
//!         if let Some(staged) = self.staged.replacements.get(&id) {
//!             Some(MaybeStaged::Staged(staged.name.as_str()))
//!         } else {
//!             self.cold.get_name(id).map(MaybeStaged::Cold)
//!         }
//!     }
//!
//!     fn get_secs_in_game(&self, id: PlayerId) -> Option<u32> {
//!         let base = if let Some(staged) = self.staged.replacements.get(&id) {
//!             Some(staged.secs_in_game)
//!         } else {
//!             self.cold.0.get(&id).map(|player| player.secs_in_game)
//!         }?;
//!         let additional = self
//!             .staged
//!             .additional_time_played
//!             .get(&id)
//!             .copied()
//!             .unwrap_or_default();
//!         Some(base + additional)
//!     }
//! }
//!
//! /// Allows mutable access to player data.
//! trait PlayerAccessMut {
//!     fn get_name_mut(&mut self, id: PlayerId) -> Option<impl DerefMut<Target = str>>;
//!     fn add_secs_in_game(&mut self, id: PlayerId, secs: u32);
//!     fn add(&mut self, name: String) -> PlayerId;
//! }
//!
//! impl PlayerAccessMut for Players {
//!     fn get_name_mut(&mut self, id: PlayerId) -> Option<impl DerefMut<Target = str>> {
//!         self.0.get_mut(&id).map(|player| player.name.as_mut_str())
//!     }
//!
//!     fn add_secs_in_game(&mut self, id: PlayerId, secs: u32) {
//!         if let Some(player) = self.0.get_mut(&id) {
//!             player.secs_in_game += secs;
//!         }
//!     }
//!
//!     fn add(&mut self, name: String) -> PlayerId {
//!         let id = PlayerId(self.0.len() as u32);
//!         self.0.insert(
//!             id,
//!             PlayerData {
//!                 name,
//!                 secs_in_game: 0,
//!                 id,
//!             },
//!         );
//!         id
//!     }
//! }
//!
//! impl PlayerAccessMut for Stager<'_, StagedPlayerChanges> {
//!     fn get_name_mut(&mut self, id: PlayerId) -> Option<impl DerefMut<Target = str>> {
//!         if !self.cold.0.contains_key(&id) && !self.staged.replacements.contains_key(&id) {
//!             return None;
//!         }
//!
//!         let player = self
//!             .staged
//!             .replacements
//!             .entry(id)
//!             .or_insert_with(|| self.cold.0.get(&id).cloned().unwrap());
//!         Some(player.name.as_mut_str())
//!     }
//!
//!     fn add_secs_in_game(&mut self, id: PlayerId, secs: u32) {
//!         *self.staged.additional_time_played.entry(id).or_default() += secs;
//!     }
//!
//!     fn add(&mut self, name: String) -> PlayerId {
//!         let id = PlayerId((self.cold.0.len() + self.staged.replacements.len()) as u32);
//!         self.staged.replacements.insert(
//!             id,
//!             PlayerData {
//!                 name,
//!                 secs_in_game: 0,
//!                 id,
//!             },
//!         );
//!         id
//!     }
//! }
//!
//! struct LockedNameStagedRef<'a> {
//!     staged: RwLockReadGuard<'a, StagedPlayerChanges>,
//!     // must be valid
//!     id: PlayerId,
//! }
//!
//! struct LockedNameColdRef<'a, T: StagableWrites<Staging = StagedPlayerChanges> + 'a> {
//!     cold: T::ColdStorage<'a>,
//!     // must be valid
//!     id: PlayerId,
//! }
//!
//! impl Deref for LockedNameStagedRef<'_> {
//!     type Target = str;
//!
//!     fn deref(&self) -> &Self::Target {
//!         self.staged
//!             .replacements
//!             .get(&self.id)
//!             .unwrap()
//!             .name
//!             .as_str()
//!     }
//! }
//!
//! impl<'a, T: StagableWrites<Staging = StagedPlayerChanges> + 'a> Deref for LockedNameColdRef<'a, T> {
//!     type Target = str;
//!
//!     fn deref(&self) -> &Self::Target {
//!         self.cold.deref().0.get(&self.id).unwrap().name.as_str()
//!     }
//! }
//!
//! #[derive(Debug, Default)]
//! pub struct PlayerRegistry {
//!     players: StageOnWrite<StagedPlayerChanges>,
//! }
//!
//! impl PlayerRegistry {
//!     /// Runs relatively rarely
//!     pub fn player_joined(&self, name: String) -> PlayerId {
//!         self.players.stage_scope_locked(|stager| stager.add(name))
//!     }
//!
//!     /// Runs very often
//!     pub fn get_name<'a>(&'a self, id: PlayerId) -> Option<impl Deref<Target = str> + 'a> {
//!         {
//!             let this = self.players.read_lock();
//!             if this.staged.replacements.contains_key(&id) {
//!                 Some(MaybeStaged::Staged(LockedNameStagedRef {
//!                     staged: this.get_staged_guard(),
//!                     id,
//!                 }))
//!             } else if this.cold.0.contains_key(&id) {
//!                 Some(MaybeStaged::Cold(LockedNameColdRef::<
//!                     StageOnWrite<StagedPlayerChanges>,
//!                 > {
//!                     cold: this.get_cold_guard(),
//!                     id,
//!                 }))
//!             } else {
//!                 None
//!             }
//!         }
//!     }
//!
//!     /// Cleans up internal data to make reading faster.
//!     pub fn clean(&mut self) {
//!         self.players.apply_staged_for_full();
//!     }
//!
//!     /// Allows reading in bulk without extra locking.
//!     pub fn bulk_read(&self) -> StagedRefLocked<'_, StageOnWrite<StagedPlayerChanges>> {
//!         self.players.read_lock()
//!     }
//!
//!     /// Allows writing in bulk without extra locking.
//!     pub fn bulk_write(&self) -> StagerLocked<'_, StageOnWrite<StagedPlayerChanges>> {
//!         self.players.stage_lock()
//!     }
//! }
//! ```

use bevy_platform_support::sync::{PoisonError, RwLock, RwLockReadGuard, RwLockWriteGuard};
use core::ops::{Deref, DerefMut};

#[cfg(feature = "alloc")]
use bevy_platform_support::sync::Arc;

/// Signifies that this type represents staged changes to [`Cold`](Self::Cold).
pub trait StagedChanges: Default {
    /// The more compact data structure that these changes compact into.
    type Cold;

    /// This applies these changes to the passed [`Cold`](Self::Cold).
    /// When this is done, there should be no more staged changes, and [`any_staged`](Self::any_staged) should be `false`.
    fn apply_staged(&mut self, storage: &mut Self::Cold);

    /// Returns `true` if and only if there are staged changes that could be applied.
    fn any_staged(&self) -> bool;
}

/// This trait defines relevant types for [`StagableWrites`].
/// See [`this github issue`](https://github.com/rust-lang/rust/issues/87479) for why this needs to be separate.
pub trait StagableWritesTypes {
    /// This is the type that will store staged changes.
    type Staging: StagedChanges;
    /// This is the type that will store [`Cold`](StagedChanges::Cold) for [`Staging`](Self::Staging).
    /// This is left generalized so that it can be put in a lock or otherwise if necessary.
    type ColdRef<'a>: Deref<Target = <Self::Staging as StagedChanges>::Cold>
    where
        <Self::Staging as StagedChanges>::Cold: 'a;
}

/// This trait generallizes the stage on write concept.
pub trait StagableWrites: StagableWritesTypes {
    /// Allows raw access to reading cold storage, which may still have unapplied staged changes that make this out of date.
    /// Use this to return data attached to a lock guard when one such guard is already in existence.
    ///
    /// This must never deadlock if there is already a [`Self::ColdStorage`] for this value on this thread.
    fn raw_read_cold(&self) -> Self::ColdRef<'_>;

    /// Allows raw access to reading staged changes, which may be missing context of cold storage.
    /// Use this to return data attached to a lock guard when one such guard is already in existence.
    ///
    /// This must never deadlock if there is already a read for this value on this thread.
    fn raw_read_staged(&self) -> RwLockReadGuard<'_, Self::Staging>;
}

/// A struct that allows staging changes while reading from cold storage.
/// Generally, staging changes should be implemented on this type.
#[derive(Debug)]
pub struct Stager<'a, T: StagedChanges> {
    /// The storage that is read optimized.
    pub cold: &'a T::Cold,
    /// The staged changes.
    pub staged: &'a mut T,
}

/// A struct that allows accessing changes while reading from cold storage.
/// Generally, reading data should be implemented on this type.
#[derive(Copy, Debug)]
pub struct StagedRef<'a, T: StagedChanges> {
    /// The storage that is read optimized.
    pub cold: &'a T::Cold,
    /// The staged changes.
    pub staged: &'a T,
}

/// A locked version of [`Stager`].
/// Use this to hold a lock guard while using [`StagerLocked::as_stager`] or similar.
#[derive(Debug)]
pub struct StagerLocked<'a, T: StagableWrites> {
    inner: &'a T,
    /// The storage that is read optimized.
    pub cold: T::ColdRef<'a>,
    /// The staged changes.
    pub staged: RwLockWriteGuard<'a, T::Staging>,
}

/// A locked version of [`StagedRef`].
/// Use this to hold a lock guard while using [`StagerLocked::as_staged_ref`].
#[derive(Debug)]
pub struct StagedRefLocked<'a, T: StagableWrites> {
    inner: &'a T,
    /// The storage that is read optimized.
    pub cold: T::ColdRef<'a>,
    /// The staged changes.
    pub staged: RwLockReadGuard<'a, T::Staging>,
}

/// A general purpose enum for representing data that may or may not need to be staged.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MaybeStaged<C, S> {
    /// There is no staging necessary.
    Cold(C),
    /// There is staging necessary.
    Staged(S),
}

/// A struct that allows read-optimized operations while still allowing mutation.
/// When mutations are made, they are staged.
/// Then, at user-defined times, they are applied to the read-optimized storage.
/// This allows mutations through [`RwLock`]s without needing to constantly lock old or cold data.
///
/// This is not designed for atomic use (ie. in an `Arc`).
#[cfg_attr(feature = "alloc", doc = "See [`AtomicStageOnWrite`] for that.")]
#[derive(Default, Debug)]
pub struct StageOnWrite<T: StagedChanges> {
    /// Cold data is read optimized.
    cold: T::Cold,
    /// Staged data stores recent modifications to cold. It's [`RwLock`] coordinates mutations.
    staged: RwLock<T>,
}

#[cfg(feature = "alloc")]
#[derive(Default, Debug)]
struct AtomicStageOnWriteInner<T: StagedChanges> {
    /// Cold data is read optimized.
    /// This lives behind a [`RwLock`], but it is only written to for applying changes in a non-blocking way.
    /// This will only block if a thread tries to read from it while it is having changes applied, but that is extremely rare.
    cold: RwLock<T::Cold>,
    /// Staged data stores recent modifications to cold.
    /// It's [`RwLock`] coordinates mutations.
    staged: RwLock<T>,
}

/// A version of [`StageOnWrite`] designed for atomic use.
/// See [`StageOnWrite`] for details.
///
/// This type includes a baked in [`Arc`], so it can be shared across threads.
///
/// Many of it's methods take `&mut self` to ensure access is exclusive, preventing possible deadlocks.
/// This doesn not guarantee there are no deadlocks when working with multiple clones of this on the same thread.
/// Here's an example:
///
/// ```compile_fail
/// use ...
/// let mut stage_on_write = AtomicStageOnWrite::<MyStagingType>::default();
/// let reading = stage_on_write.read_lock();
/// stage_on_write.apply_staged_non_blocking();
/// ```
///
/// Remember to use [`apply_staged_non_blocking`](Self::apply_staged_non_blocking) or similar methods periodically as a best practice.
#[cfg(feature = "alloc")]
#[derive(Clone)]
pub struct AtomicStageOnWrite<T: StagedChanges>(Arc<AtomicStageOnWriteInner<T>>);

impl<T: StagedChanges> StagableWritesTypes for StageOnWrite<T> {
    type Staging = T;

    type ColdRef<'a>
        = &'a T::Cold
    where
        T::Cold: 'a;
}

impl<T: StagedChanges> StagableWritesTypes for AtomicStageOnWrite<T> {
    type Staging = T;

    type ColdRef<'a>
        = RwLockReadGuard<'a, T::Cold>
    where
        T::Cold: 'a;
}

impl<T: StagedChanges> StagableWrites for StageOnWrite<T> {
    fn raw_read_cold(&self) -> Self::ColdRef<'_> {
        &self.cold
    }

    fn raw_read_staged(&self) -> RwLockReadGuard<'_, Self::Staging> {
        self.staged.read().unwrap_or_else(PoisonError::into_inner)
    }
}

impl<T: StagedChanges> StagableWrites for AtomicStageOnWrite<T> {
    fn raw_read_cold(&self) -> Self::ColdRef<'_> {
        self.0.cold.read().unwrap_or_else(PoisonError::into_inner)
    }

    fn raw_read_staged(&self) -> RwLockReadGuard<'_, Self::Staging> {
        self.0.staged.read().unwrap_or_else(PoisonError::into_inner)
    }
}

impl<T: StagedChanges> StageOnWrite<T> {
    /// Constructs a new [`StageOnWrite`] with the given value and no staged changes.
    pub fn new(value: T::Cold) -> Self {
        Self {
            cold: value,
            staged: RwLock::default(),
        }
    }

    /// Gets the inner cold data if there are no staged changes.
    /// If [`any_staged`](Self::any_staged) is known to be false, this can be safely unwrapped.
    #[inline]
    pub fn full(&mut self) -> Option<&mut T::Cold> {
        if self
            .staged
            .get_mut()
            .unwrap_or_else(PoisonError::into_inner)
            .any_staged()
        {
            None
        } else {
            Some(&mut self.cold)
        }
    }

    /// Applies any staged changes before returning the full value with all changes applied.
    /// Immediately after this, [`any_staged`](Self::any_staged) will be false.
    #[inline]
    pub fn apply_staged_for_full(&mut self) -> &mut T::Cold {
        let staged = self
            .staged
            .get_mut()
            .unwrap_or_else(PoisonError::into_inner);
        if staged.any_staged() {
            staged.apply_staged(&mut self.cold);
        }
        &mut self.cold
    }

    /// Returns true if and only if there are staged changes that could be applied.
    /// If you only have a immutable reference, consider using [`read_scope_locked`](Self::read_scope_locked) with [`StagedChanges::any_staged`].
    #[inline]
    pub fn any_staged(&mut self) -> bool {
        self.staged
            .get_mut()
            .unwrap_or_else(PoisonError::into_inner)
            .any_staged()
    }

    /// Constructs a [`Stager`] that will stage changes.
    #[inline]
    pub fn stage(&mut self) -> Stager<'_, T> {
        Stager {
            cold: &mut self.cold,
            staged: self
                .staged
                .get_mut()
                .unwrap_or_else(PoisonError::into_inner),
        }
    }

    /// Constructs a [`StagerLocked`], locking internally.
    ///
    /// # Deadlocks
    ///
    /// This deadlocks if there are any other lock guards on this thread for this value.
    #[inline]
    pub fn stage_lock(&self) -> StagerLocked<'_, Self> {
        StagerLocked {
            inner: self,
            cold: &self.cold,
            staged: self.staged.write().unwrap_or_else(PoisonError::into_inner),
        }
    }

    /// Constructs a [`Stager`] that will stage changes.
    #[inline]
    pub fn read(&mut self) -> StagedRef<'_, T> {
        StagedRef {
            cold: &self.cold,
            staged: self
                .staged
                .get_mut()
                .unwrap_or_else(PoisonError::into_inner),
        }
    }

    /// Constructs a [`StagedRefLocked`], locking internally.
    ///
    /// # Deadlocks
    ///
    /// This deadlocks if there are any write lock guards on this thread for this value.
    #[inline]
    pub fn read_lock(&self) -> StagedRefLocked<'_, Self> {
        StagedRefLocked {
            inner: self,
            cold: &self.cold,
            staged: self.staged.read().unwrap_or_else(PoisonError::into_inner),
        }
    }

    /// Runs different logic depending on if additional changes are already staged.
    /// This can be faster than greedily applying staged changes if there are already staged changes.
    pub fn maybe_stage<C, S>(
        &mut self,
        for_full: impl FnOnce(&mut T::Cold) -> C,
        for_staged: impl FnOnce(&mut Stager<T>) -> S,
    ) -> MaybeStaged<C, S> {
        let staged = self
            .staged
            .get_mut()
            .unwrap_or_else(PoisonError::into_inner);
        let cold = &mut self.cold;
        if staged.any_staged() {
            MaybeStaged::Staged(for_staged(&mut Stager { cold, staged }))
        } else {
            MaybeStaged::Cold(for_full(cold))
        }
    }

    /// Easily run a stager function to stage changes.
    ///
    /// # Deadlocks
    ///
    /// This deadlocks if there are any other lock guards on this thread for this value.
    #[inline]
    pub fn stage_scope_locked<R>(&self, f: impl FnOnce(&mut Stager<T>) -> R) -> R {
        f(&mut self.stage_lock().as_stager())
    }

    /// Easily run a [`StagedRef`] function.
    ///
    /// # Deadlocks
    ///
    /// This deadlocks if there are any write lock guards on this thread for this value.
    #[inline]
    pub fn read_scope_locked<R>(&self, f: impl FnOnce(&StagedRef<T>) -> R) -> R {
        f(&self.read_lock().as_staged_ref())
    }
}

#[cfg(feature = "alloc")]
impl<T: StagedChanges> AtomicStageOnWrite<T> {
    /// Constructs a new [`AtomicStageOnWrite`] with the given value and no staged changes.
    pub fn new(value: T::Cold) -> Self {
        Self(Arc::new(AtomicStageOnWriteInner {
            cold: RwLock::new(value),
            staged: RwLock::default(),
        }))
    }

    /// Gets the inner cold data if there are no staged changes.
    /// If [`any_staged`](Self::any_staged) is known to be false, this can be safely unwrapped.
    ///
    /// Note that this **Blocks**, so generally prefer [`full_non_blocking`](Self::full_non_blocking).
    #[inline]
    pub fn full_locked(&mut self) -> Option<RwLockWriteGuard<'_, T::Cold>> {
        if self
            .0
            .staged
            .read()
            .unwrap_or_else(PoisonError::into_inner)
            .any_staged()
        {
            None
        } else {
            Some(self.0.cold.write().unwrap_or_else(PoisonError::into_inner))
        }
    }

    /// Applies any staged changes before returning the full value with all changes applied.
    /// Immediately after this, [`any_staged`](Self::any_staged) will be false.
    ///
    /// Note that this **Blocks**, so generally prefer [`apply_staged_for_full_non_blocking`](Self::apply_staged_for_full_non_blocking).
    #[inline]
    pub fn apply_staged_for_full_locked(&mut self) -> RwLockWriteGuard<'_, T::Cold> {
        let mut staged = self
            .0
            .staged
            .write()
            .unwrap_or_else(PoisonError::into_inner);
        let mut cold = self.0.cold.write().unwrap_or_else(PoisonError::into_inner);
        if staged.any_staged() {
            staged.apply_staged(&mut cold);
        }
        cold
    }

    /// Gets the inner cold data if there are no staged changes and nobody is reading from the cold data.
    #[inline]
    pub fn full_non_blocking(&mut self) -> Option<RwLockWriteGuard<'_, T::Cold>> {
        let staged = self.0.staged.try_read().ok()?;
        if staged.any_staged() {
            None
        } else {
            self.0.cold.try_write().ok()
        }
    }

    /// Applies any staged changes before returning the full value with all changes applied.
    #[inline]
    pub fn apply_staged_for_full_non_blocking(&mut self) -> Option<RwLockWriteGuard<'_, T::Cold>> {
        let mut cold = self.0.cold.try_write().ok()?;
        match self.0.staged.try_write() {
            Ok(mut staged) => {
                if staged.any_staged() {
                    staged.apply_staged(&mut cold);
                }
                Some(cold)
            }
            Err(_) => {
                let staged = self.0.staged.read().unwrap_or_else(PoisonError::into_inner);
                if staged.any_staged() {
                    None
                } else {
                    Some(cold)
                }
            }
        }
    }

    /// If possible applies any staged changes.
    /// Returns true if it can guarantee there are no more staged changes.
    #[inline]
    pub fn apply_staged_non_blocking(&mut self) -> bool {
        let Ok(mut staged) = self.0.staged.try_write() else {
            return false;
        };
        if staged.any_staged() {
            let Ok(mut cold) = self.0.cold.try_write() else {
                return false;
            };
            staged.apply_staged(&mut cold);
            true
        } else {
            false
        }
    }

    /// Returns true if and only if there are staged changes that could be applied.
    #[inline]
    pub fn any_staged(&self) -> bool {
        self.0
            .staged
            .read()
            .unwrap_or_else(PoisonError::into_inner)
            .any_staged()
    }

    /// Constructs a [`StagerLocked`], locking internally.
    #[inline]
    pub fn stage_lock(&mut self) -> StagerLocked<'_, Self> {
        StagerLocked {
            inner: self,
            cold: self.0.cold.read().unwrap_or_else(PoisonError::into_inner),
            staged: self
                .0
                .staged
                .write()
                .unwrap_or_else(PoisonError::into_inner),
        }
    }

    /// Constructs a [`StagedRefLocked`], locking internally.
    #[inline]
    pub fn read_lock(&self) -> StagedRefLocked<'_, Self> {
        StagedRefLocked {
            inner: self,
            cold: self.0.cold.read().unwrap_or_else(PoisonError::into_inner),
            staged: self.0.staged.read().unwrap_or_else(PoisonError::into_inner),
        }
    }

    /// Runs different logic depending on if additional changes are already staged and if using cold directly would block.
    /// This *can* be faster than greedily applying staged changes if there are no staged changes and no reads from cold.
    pub fn maybe_stage<C, S>(
        &mut self,
        for_full: impl FnOnce(&mut T::Cold) -> C,
        for_staged: impl FnOnce(&mut Stager<T>) -> S,
    ) -> MaybeStaged<C, S> {
        let mut staged = self
            .0
            .staged
            .write()
            .unwrap_or_else(PoisonError::into_inner);
        if staged.any_staged() {
            let cold = self.0.cold.read().unwrap_or_else(PoisonError::into_inner);
            MaybeStaged::Staged(for_staged(&mut Stager {
                cold: &cold,
                staged: &mut staged,
            }))
        } else if let Ok(mut cold) = self.0.cold.try_write() {
            MaybeStaged::Cold(for_full(&mut cold))
        } else {
            let cold = self.0.cold.read().unwrap_or_else(PoisonError::into_inner);
            MaybeStaged::Staged(for_staged(&mut Stager {
                cold: &cold,
                staged: &mut staged,
            }))
        }
    }

    /// Easily run a stager function to stage changes.
    #[inline]
    pub fn stage_scope_locked<R>(&mut self, f: impl FnOnce(&mut Stager<T>) -> R) -> R {
        f(&mut self.stage_lock().as_stager())
    }

    /// Easily run a stager function to stage changes.
    /// Then, tries to apply those changes if doing so wouldn't lock.
    #[inline]
    pub fn stage_scope_locked_eager<R>(&mut self, f: impl FnOnce(&mut Stager<T>) -> R) -> R {
        let result = self.stage_scope_locked(f);
        self.apply_staged_non_blocking();
        result
    }

    /// Easily run a [`StagedRef`] function.
    #[inline]
    pub fn read_scope_locked<R>(&self, f: impl FnOnce(&StagedRef<T>) -> R) -> R {
        f(&self.read_lock().as_staged_ref())
    }
}

impl<'a, T: StagableWrites> StagerLocked<'a, T> {
    /// Allows a user to view this as a [`Stager`].
    #[inline]
    pub fn as_stager(&mut self) -> Stager<'_, T::Staging> {
        Stager {
            cold: &self.cold,
            staged: &mut self.staged,
        }
    }

    /// Allows a user to view this as a [`StagedRef`].
    #[inline]
    pub fn as_staged_ref(&self) -> StagedRef<'_, T::Staging> {
        StagedRef {
            cold: &self.cold,
            staged: &self.staged,
        }
    }

    /// Releases the lock, returning the underlying [`StagableWrites`] structure.
    #[inline]
    pub fn release(self) -> &'a T {
        self.inner
    }
}

impl<'a, T: StagableWrites> StagedRefLocked<'a, T> {
    /// Allows a user to view this as a [`StagedRef`].
    #[inline]
    pub fn as_staged_ref(&self) -> StagedRef<'_, T::Staging> {
        StagedRef {
            cold: &self.cold,
            staged: &self.staged,
        }
    }

    /// Releases the lock, returning the underlying [`StagableWrites`] structure.
    #[inline]
    pub fn release(self) -> &'a T {
        self.inner
    }

    /// Allows returning a reference to the locked staged data without releasing its lock.
    #[inline]
    pub fn get_staged_guard(&self) -> RwLockReadGuard<'a, T::Staging> {
        self.inner.raw_read_staged()
    }

    /// Allows returning a reference to the cold data without releasing its lock (it it has one).
    #[inline]
    pub fn get_cold_guard(&self) -> T::ColdRef<'a> {
        self.inner.raw_read_cold()
    }
}

impl<'a, T: StagedChanges> Clone for StagedRef<'a, T> {
    fn clone(&self) -> Self {
        Self {
            staged: self.staged,
            cold: self.cold,
        }
    }
}

impl<'a, T: StagableWrites> Clone for StagedRefLocked<'a, T> {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner,
            staged: self.get_staged_guard(),
            cold: self.get_cold_guard(),
        }
    }
}

#[cfg(feature = "alloc")]
impl<T: StagedChanges> Default for AtomicStageOnWrite<T>
where
    T::Cold: Default,
{
    fn default() -> Self {
        Self::new(T::Cold::default())
    }
}

impl<C: Deref, S: Deref<Target = C::Target>> Deref for MaybeStaged<C, S> {
    type Target = C::Target;

    fn deref(&self) -> &Self::Target {
        match self {
            MaybeStaged::Cold(c) => c,
            MaybeStaged::Staged(s) => s,
        }
    }
}

impl<C: DerefMut, S: DerefMut<Target = C::Target>> DerefMut for MaybeStaged<C, S> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        match self {
            MaybeStaged::Cold(c) => c,
            MaybeStaged::Staged(s) => s,
        }
    }
}

#[cfg(test)]
mod tests {
    use bevy_platform_support::{collections::HashMap, prelude::Vec};

    use super::*;

    #[derive(Default)]
    struct StagedNumVec {
        added: Vec<u32>,
        changed: HashMap<usize, u32>,
    }

    impl StagedChanges for StagedNumVec {
        type Cold = Vec<u32>;

        fn apply_staged(&mut self, storage: &mut Self::Cold) {
            storage.append(&mut self.added);
            for (index, new) in self.changed.drain() {
                storage[index] = new;
            }
        }

        fn any_staged(&self) -> bool {
            !self.added.is_empty() || !self.changed.is_empty()
        }
    }

    #[test]
    fn test_simple_stage() {
        let mut data = StageOnWrite::<StagedNumVec>::default();
        data.stage_scope_locked(|stager| stager.staged.added.push(5));
        let full = data.apply_staged_for_full();
        assert_eq!(&full[..], &[5]);
    }
}
