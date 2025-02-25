#![expect(
    unsafe_code,
    reason = "This module marks some items as unsafe to alert users to deadlock potential."
)]
//! Provides an abstracted system for staging modifications to data structures that rarely change.
//! See [`StageOnWrite`] as a starting point.
//!
//! # Rationale
//!
//! Lets say you want to have a collection of items that is read from often but rarely written to.
//! This comes up a lot in registries, like components, bundles, reflection, assets, etc.
//! Lets say you also want to share this collection between threads freely.
//! There are lots of ways to do this, so what does this module do differently?
//!
//! The standard solution is to use a [`RwLock`] with the collection inside.
//! Then you can pass around the lock between threads freely, even putting it in an `Arc` if desired.
//! However, writing to the collection blocks all reads from it.
//! In performance critical code, this can shut down processes that are reading data from the collection.
//! Worse, if the lifetime of the data being read is long, it can block the writing thread for a significant time.
//! In some cases, this is even prone to deadlocking.
//! If any of these are concerns, [`RwLock`] is not enough.
//!
//! There are plenty of third party crates that offer relevant functionality.
//! For example, [left_right](https://docs.rs/left-right/latest/left_right/) is similar to this module, but it only supports one writer at a time, allows readers to desync from the writer, and lacks `no_std` support.
//! Other crates exist but come with similar downsides, taking double memory, not having `no_std` support, etc.
//!
//! So this module is its own solution to the problem.
//! Use this if and only if:
//! - The collection you are storing has very few writes.
//! - The collection has many concurrent reads.
//! - The collection needs to be able to be written to from anywhere.
//! - The collection needs to be updated everywhere immediately when written to.
//! - The collection can't let writes interupt/block reading other data.
//!
//! # How it works
//!
//! The general approach here is called "Stage on Write", similar to "Copy on Write" from std.
//!
//! Data that has not been changed in a while lives in a compact, read-optimized data structure.
//! We call this read-optimized, old (not changed recently) data "cold" data.
//! When a change is made, instead of locking the cold data and applying it directly, we lock a temporary storage data structure and queue the change.
//! These queued changes we call "staged" data, and its type implements [`StagedChanges`].
//! Then, at user defined points, we drain these staged changes into the cold data.
//! This requires locking both data structures, but since these points can be much less frequent than the already rare writes, this doesn't matter.
//! In principal, the staged data never needs to be drained into the cold data, but the data in staged data will not benefit from the faster read access of cold data.
//! The traits [`StagableWrites`] and company represent types that coordinate this behavior.
//!
//! A few other types help with this.
//! If a lock is held or there is mutable access to the underlying stage on write type, [`Stager`] and [`StagedRef`] can be obtained.
//! Use [`StagedRef`] when you need to access data with standard references.
//! Since the there might be a lock involved, this should be used only when needed.
//! Use [`Stager`] when you need to write data, since this gives read access to cold data, and write access to staged data.
//! However, these types are typically obtained by locking, and returning a reference from them requires all of its locks be kept.
//! (Both cold *and* staged are locked even if the reference only points to one of them).
//! As a result, there are better types to use for reading.
//!
//! [`StagedRefLocked`] fills the same role as [`StagedRef`], but gives underlying access to the locks.
//! So, if the requested data lives in cold data, you can return the lock guard for cold, and drop the lock guard for staged, freeing it up for writes.
//! The [`MaybeStaged`] enum helps with this, and this will be even easier once [RwLockGuard mapping](https://doc.rust-lang.org/std/sync/struct.MappedRwLockReadGuard.html) is stabilized.
//! [`StagerLocked`] offers effectively the same powers as [`Stager`], but also gives access to the underlying locks.
//! So, if you want to return a mutable reference to something in staged data, you can use this to do that while allowing the cold lock to be dropped.
//! This is less useful, but still has its place.
//!
//! In general:
//! - Implement general purpose reads on [`StagedRef`],
//! - high performance reads on [`StagedRefLocked`],
//! - writes on [`Stager`],
//! - and niche/advanced uses on [`StagerLocked`].
//!
//! In addition, this module offers two implementations of the stage on write concept: [`StageOnWrite`] and [`AtomicStageOnWrite`].
//! [`StageOnWrite`] is the simpler implementation, storing cold data directly and staged data in an [`RwLock`] to synchronize writes.
//! Because it stores cold data directly, the only way to clean the data (drain staged data into cold) is to have mutable access to it.
//! This means it can't be put in an `Arc` or similar and still be able to be cleaned.
//! [`AtomicStageOnWrite`] comes to the resque here. It stores cold data in another [`RwLock`], allowing the data to be cleaned with immutable access.
//! Although blocking methods for this exist, [`AtomicStageOnWrite`] also offers non-blocking methods for cleaning.
//! Hence, in normal use, [`AtomicStageOnWrite`] will almost never block to read from cold data.
//! Additionally, because it can see when cold is being read or not, it can apply staged changes as needed without needing specific calls from the user.
//!
//! Finally, [`StagableWrites`] offers some utilities to prevent deadlocking.
//! It is implemented by [`RefStageOnWrite`] and `ArcStageOnWrite` for utility.
//! The idea behind this type is that the most common way to accidentally deadlock is to maintain an immutable borrow while making a muttable borrow via synchronization.
//! This trait makes mutable borrows requie mutable access to self, preventing this kind of deadlock.
//! Because these types only wrap cloneable references to to the [`StagableWritesCore`] type, this can still be shared between thread safely.
//! It also means that this does not fully prevent deadlock, especially if the same thread is maintaining a lock guard on one copy of the stage on write structure while another copy is being used.
//! Still this protection is better than none.
//!
//! # Example
//!
//! Here's an example of this utility in action for registering players.
//! This is a bit contrived, but it should communicate the idea.
//!
//! ```
//! use core::mem::take;
//! use std::sync::RwLockReadGuard;
//! use core::ops::{Deref, DerefMut};
//!
//! use bevy_platform_support::collections::HashMap;
//! use bevy_platform_support::prelude::String;
//! use self::bevy_utils::staging::*;
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
//! struct LockedNameColdRef<'a, T: StagableWritesCore<Staging = StagedPlayerChanges> + 'a> {
//!     cold: T::ColdRef<'a>,
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
//! impl<'a, T: StagableWritesCore<Staging = StagedPlayerChanges> + 'a> Deref for LockedNameColdRef<'a, T> {
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
//!         self.bulk_write().as_stager().add(name)
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
//!         // SAFETY: unsafe is used to take responsibility for deadlocks.
//!         unsafe { self.players.stage_lock_unsafe() }
//!     }
//! }
//! ```
//!

use bevy_platform_support::sync::{
    PoisonError, RwLock, RwLockReadGuard, RwLockWriteGuard, TryLockError,
};
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
pub trait StagableWritesTypes: Sized {
    /// This is the type that will store staged changes.
    type Staging: StagedChanges;
    /// This is the type that will reference [`Cold`](StagedChanges::Cold) for [`Staging`](Self::Staging).
    /// This is left generalized so that it can be put in a lock or otherwise if necessary.
    type ColdRef<'a>: Deref<Target = <Self::Staging as StagedChanges>::Cold>
    where
        <Self::Staging as StagedChanges>::Cold: 'a;
    /// This is the type that will mutably reference [`Cold`](StagedChanges::Cold) for [`Staging`](Self::Staging).
    /// This is left generalized so that it can be put in a lock or otherwise if necessary.
    type ColdMut<'a>: Deref<Target = <Self::Staging as StagedChanges>::Cold>
    where
        <Self::Staging as StagedChanges>::Cold: 'a;
}

/// This trait generallizes the stage on write concept.
pub trait StagableWritesCore: StagableWritesTypes {
    /// Allows raw access to reading cold storage, which may still have unapplied staged changes that make this out of date.
    /// Use this to return data attached to a lock guard when one such guard is already in existence.
    ///
    /// This must never deadlock if there is already a read for this value on this thread.
    fn raw_read_cold(&self) -> Self::ColdRef<'_>;

    /// Allows raw access to reading staged changes, which may be missing context of cold storage.
    /// Use this to return data attached to a lock guard when one such guard is already in existence.
    ///
    /// This must never deadlock if there is already a read for this value on this thread.
    fn raw_read_staged(&self) -> RwLockReadGuard<'_, Self::Staging>;

    /// Same as [`raw_read_cold`](StagableWritesCore::raw_read_cold), but never blocks.
    fn raw_read_cold_non_blocking(&self) -> Option<Self::ColdRef<'_>>;

    /// Same as [`raw_read_staged`](StagableWritesCore::raw_read_staged), but never blocks.
    fn raw_read_staged_non_blocking(&self) -> Option<RwLockReadGuard<'_, Self::Staging>>;

    /// Allows raw access to reading staged changes, which may be missing context of cold storage.
    fn raw_write_staged(&self) -> RwLockWriteGuard<'_, Self::Staging>;

    /// Same as [`raw_write_staged`](StagableWritesCore::raw_write_staged), but never blocks.
    fn raw_write_staged_non_blocking(&self) -> Option<RwLockWriteGuard<'_, Self::Staging>>;

    /// Allows raw access to both staged and cold data.
    fn raw_write_both_mut(
        &mut self,
    ) -> (
        &mut Self::Staging,
        &mut <Self::Staging as StagedChanges>::Cold,
    );

    /// Gets the cold data mutably without locking.
    #[inline]
    fn raw_write_cold_mut(&mut self) -> &mut <Self::Staging as StagedChanges>::Cold {
        self.raw_write_both_mut().1
    }

    /// Same as [`raw_write_staged`](StagableWritesCore::raw_write_staged), but never locks.
    #[inline]
    fn raw_write_staged_mut(&mut self) -> &mut Self::Staging {
        self.raw_write_both_mut().0
    }

    /// Gets the inner cold data if there are no staged changes.
    /// If [`any_staged`](Self::any_staged) is known to be false, this can be safely unwrapped.
    #[inline]
    fn full(&mut self) -> Option<&mut <Self::Staging as StagedChanges>::Cold> {
        if self.raw_write_staged_mut().any_staged() {
            None
        } else {
            Some(self.raw_write_cold_mut())
        }
    }

    /// Applies any staged changes before returning the full value with all changes applied.
    /// Immediately after this, [`any_staged`](Self::any_staged) will be false.
    #[inline]
    fn apply_staged_for_full(&mut self) -> &mut <Self::Staging as StagedChanges>::Cold {
        let (staged, cold) = self.raw_write_both_mut();
        if staged.any_staged() {
            staged.apply_staged(cold);
        }
        cold
    }

    /// Returns true if and only if there are staged changes that could be applied.
    #[inline]
    fn any_staged(&mut self) -> bool {
        self.raw_write_staged_mut().any_staged()
    }

    /// Same as [`any_staged`](StagableWritesCore::any_staged), but locks and works without mutible access.
    #[inline]
    fn any_staged_ref(&self) -> bool {
        self.raw_read_staged().any_staged()
    }

    /// Constructs a [`Stager`] that will stage changes.
    #[inline]
    fn stage(&mut self) -> Stager<'_, Self::Staging> {
        let writes = self.raw_write_both_mut();
        Stager {
            cold: writes.1,
            staged: writes.0,
        }
    }

    /// Constructs a [`StagerLocked`], locking internally.
    ///
    /// # Safety
    ///
    /// There must not be any other lock guards on this thread for this value. Otherwise it deadlocks.
    #[inline]
    unsafe fn stage_lock_unsafe(&self) -> StagerLocked<'_, Self> {
        StagerLocked {
            inner: RefStageOnWrite(self),
            cold: self.raw_read_cold(),
            staged: self.raw_write_staged(),
        }
    }

    /// Constructs a [`Stager`] that will stage changes.
    #[inline]
    fn read(&mut self) -> StagedRef<'_, Self::Staging> {
        let writes = self.raw_write_both_mut();
        StagedRef {
            cold: writes.1,
            staged: writes.0,
        }
    }

    /// Constructs a [`StagedRefLocked`], locking internally.
    #[inline]
    fn read_lock(&self) -> StagedRefLocked<'_, Self> {
        StagedRefLocked {
            inner: RefStageOnWrite(self),
            cold: self.raw_read_cold(),
            staged: self.raw_read_staged(),
        }
    }

    /// Runs different logic depending on if additional changes are already staged.
    /// This can be faster than greedily applying staged changes if there are already staged changes.
    #[inline]
    fn maybe_stage<C, S>(
        &mut self,
        for_full: impl FnOnce(&mut <Self::ColdMut<'_> as Deref>::Target) -> C,
        for_staged: impl FnOnce(&mut Stager<Self::Staging>) -> S,
    ) -> MaybeStaged<C, S> {
        let (staged, cold) = self.raw_write_both_mut();
        if staged.any_staged() {
            MaybeStaged::Staged(for_staged(&mut Stager { cold, staged }))
        } else {
            MaybeStaged::Cold(for_full(cold))
        }
    }

    /// Easily run a [`StagedRef`] function.
    #[inline]
    fn read_scope_locked<R>(
        &self,
        f: impl FnOnce(&StagedRef<<Self as StagableWritesTypes>::Staging>) -> R,
    ) -> R {
        f(&self.read_lock().as_staged_ref())
    }
}

/// This trait provides some conviniencies around [`StagableWritesCore`].
///
/// For example, mutable references are used to enforce safety for some functions.
pub trait StagableWrites {
    /// This is the inner [`StagableWritesCore`] type responsible for the bulk of the implementation.
    type Core: StagableWritesCore;

    /// Gets the inner core.
    fn get_core(&self) -> &Self::Core;

    /// Exactly the same as [`StagableWritesCore::stage_lock_unsafe`]
    #[inline]
    fn stage_lock(&mut self) -> StagerLocked<'_, Self::Core> {
        // Safety: Because we have exclusive, mutable access, this is safe.
        unsafe { self.get_core().stage_lock_unsafe() }
    }

    /// Easily run a stager function to stage changes.
    #[inline]
    fn stage_scope_locked<R>(
        &mut self,
        f: impl FnOnce(&mut Stager<<Self::Core as StagableWritesTypes>::Staging>) -> R,
    ) -> R {
        f(&mut self.stage_lock().as_stager())
    }
}

impl<T: StagableWritesCore> StagableWrites for T {
    type Core = T;

    fn get_core(&self) -> &Self::Core {
        self
    }
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
#[derive(Debug)]
pub struct StagedRef<'a, T: StagedChanges> {
    /// The storage that is read optimized.
    pub cold: &'a T::Cold,
    /// The staged changes.
    pub staged: &'a T,
}

/// A locked version of [`Stager`].
/// Use this to hold a lock guard while using [`StagerLocked::as_stager`] or similar.
#[derive(Debug)]
pub struct StagerLocked<'a, T: StagableWritesCore> {
    inner: RefStageOnWrite<'a, T>,
    /// The storage that is read optimized.
    pub cold: T::ColdRef<'a>,
    /// The staged changes.
    pub staged: RwLockWriteGuard<'a, T::Staging>,
}

/// A locked version of [`StagedRef`].
/// Use this to hold a lock guard while using [`StagerLocked::as_staged_ref`].
#[derive(Debug)]
pub struct StagedRefLocked<'a, T: StagableWritesCore> {
    inner: RefStageOnWrite<'a, T>,
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

/// A version of [`StageOnWrite`] designed for atomic use.
/// It functions fully without needing `&mut self`.
/// See [`StageOnWrite`] for details.
#[derive(Default, Debug)]
pub struct AtomicStageOnWrite<T: StagedChanges> {
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
/// Remember to use [`apply_staged_non_blocking`](AtomicStageOnWrite::apply_staged_non_blocking) or similar methods periodically as a best practice.
#[cfg(feature = "alloc")]
#[derive(Clone)]
pub struct ArcStageOnWrite<T: StagedChanges>(pub Arc<AtomicStageOnWrite<T>>);

/// Although it is is often enough to pass around references to a [`StagableWritesCore`], it is sometimes desierable to encapsulate that reference here.
/// That enables utilities like [`StagableWrites`]
#[derive(Debug)]
pub struct RefStageOnWrite<'a, T: StagableWritesCore>(pub &'a T);

impl<T: StagedChanges> StagableWritesTypes for StageOnWrite<T> {
    type Staging = T;

    type ColdRef<'a>
        = &'a T::Cold
    where
        T::Cold: 'a;

    type ColdMut<'a>
        = &'a mut T::Cold
    where
        <Self::Staging as StagedChanges>::Cold: 'a;
}

impl<T: StagedChanges> StagableWritesTypes for AtomicStageOnWrite<T> {
    type Staging = T;

    type ColdRef<'a>
        = RwLockReadGuard<'a, T::Cold>
    where
        T::Cold: 'a;

    type ColdMut<'a>
        = RwLockWriteGuard<'a, T::Cold>
    where
        T::Cold: 'a;
}

impl<T: StagedChanges> StagableWritesCore for StageOnWrite<T> {
    #[inline]
    fn raw_read_cold(&self) -> Self::ColdRef<'_> {
        &self.cold
    }

    #[inline]
    fn raw_read_staged(&self) -> RwLockReadGuard<'_, Self::Staging> {
        self.staged.read().unwrap_or_else(PoisonError::into_inner)
    }

    #[inline]
    fn raw_read_cold_non_blocking(&self) -> Option<Self::ColdRef<'_>> {
        Some(&self.cold)
    }

    #[inline]
    fn raw_read_staged_non_blocking(&self) -> Option<RwLockReadGuard<'_, Self::Staging>> {
        match self.staged.try_read() {
            Ok(read) => Some(read),
            Err(TryLockError::Poisoned(poison)) => Some(poison.into_inner()),
            Err(TryLockError::WouldBlock) => None,
        }
    }

    #[inline]
    fn raw_write_staged(&self) -> RwLockWriteGuard<'_, Self::Staging> {
        self.staged.write().unwrap_or_else(PoisonError::into_inner)
    }

    #[inline]
    fn raw_write_staged_non_blocking(&self) -> Option<RwLockWriteGuard<'_, Self::Staging>> {
        match self.staged.try_write() {
            Ok(read) => Some(read),
            Err(TryLockError::Poisoned(poison)) => Some(poison.into_inner()),
            Err(TryLockError::WouldBlock) => None,
        }
    }

    #[inline]
    fn raw_write_both_mut(
        &mut self,
    ) -> (
        &mut Self::Staging,
        &mut <Self::Staging as StagedChanges>::Cold,
    ) {
        (
            self.staged
                .get_mut()
                .unwrap_or_else(PoisonError::into_inner),
            &mut self.cold,
        )
    }
}

impl<T: StagedChanges> StagableWritesCore for AtomicStageOnWrite<T> {
    #[inline]
    fn raw_read_cold(&self) -> Self::ColdRef<'_> {
        self.cold.read().unwrap_or_else(PoisonError::into_inner)
    }

    #[inline]
    fn raw_read_staged(&self) -> RwLockReadGuard<'_, Self::Staging> {
        self.staged.read().unwrap_or_else(PoisonError::into_inner)
    }

    #[inline]
    fn raw_read_cold_non_blocking(&self) -> Option<Self::ColdRef<'_>> {
        match self.cold.try_read() {
            Ok(read) => Some(read),
            Err(TryLockError::Poisoned(poison)) => Some(poison.into_inner()),
            Err(TryLockError::WouldBlock) => None,
        }
    }

    #[inline]
    fn raw_read_staged_non_blocking(&self) -> Option<RwLockReadGuard<'_, Self::Staging>> {
        match self.staged.try_read() {
            Ok(read) => Some(read),
            Err(TryLockError::Poisoned(poison)) => Some(poison.into_inner()),
            Err(TryLockError::WouldBlock) => None,
        }
    }

    #[inline]
    fn raw_write_staged(&self) -> RwLockWriteGuard<'_, Self::Staging> {
        self.staged.write().unwrap_or_else(PoisonError::into_inner)
    }

    #[inline]
    fn raw_write_staged_non_blocking(&self) -> Option<RwLockWriteGuard<'_, Self::Staging>> {
        match self.staged.try_write() {
            Ok(read) => Some(read),
            Err(TryLockError::Poisoned(poison)) => Some(poison.into_inner()),
            Err(TryLockError::WouldBlock) => None,
        }
    }

    #[inline]
    fn raw_write_both_mut(
        &mut self,
    ) -> (
        &mut Self::Staging,
        &mut <Self::Staging as StagedChanges>::Cold,
    ) {
        (
            self.staged
                .get_mut()
                .unwrap_or_else(PoisonError::into_inner),
            self.cold.get_mut().unwrap_or_else(PoisonError::into_inner),
        )
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
}

impl<T: StagedChanges> AtomicStageOnWrite<T> {
    /// Constructs a new [`AtomicStageOnWrite`] with the given value and no staged changes.
    pub fn new(value: T::Cold) -> Self {
        Self {
            cold: RwLock::new(value),
            staged: RwLock::default(),
        }
    }

    /// Gets the inner cold data if there are no staged changes and nobody is reading from the cold data.
    #[inline]
    pub fn full_non_blocking(&self) -> Option<RwLockWriteGuard<'_, T::Cold>> {
        let staged = self.staged.try_read().ok()?;
        if staged.any_staged() {
            None
        } else {
            self.cold.try_write().ok()
        }
    }

    /// Applies any staged changes before returning the full value with all changes applied.
    #[inline]
    pub fn apply_staged_for_full_non_blocking(&self) -> Option<RwLockWriteGuard<'_, T::Cold>> {
        let mut cold = self.cold.try_write().ok()?;
        match self.staged.try_write() {
            Ok(mut staged) => {
                if staged.any_staged() {
                    staged.apply_staged(&mut cold);
                }
                Some(cold)
            }
            Err(_) => {
                let staged = self.staged.read().unwrap_or_else(PoisonError::into_inner);
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
    pub fn apply_staged_non_blocking(&self) -> bool {
        let Ok(mut staged) = self.staged.try_write() else {
            return false;
        };
        if staged.any_staged() {
            let Ok(mut cold) = self.cold.try_write() else {
                return false;
            };
            staged.apply_staged(&mut cold);
            true
        } else {
            false
        }
    }

    /// Runs different logic depending on if additional changes are already staged and if using cold directly would block.
    /// This *can* be faster than greedily applying staged changes if there are no staged changes and no reads from cold.
    ///
    /// # Safety
    ///
    /// There must not be any other lock guards for this value on this thread. Otherwise, this will deadlock.
    pub unsafe fn maybe_stage_unsafe<C, S>(
        &self,
        for_full: impl FnOnce(&mut T::Cold) -> C,
        for_staged: impl FnOnce(&mut Stager<T>) -> S,
    ) -> MaybeStaged<C, S> {
        let mut staged = self.staged.write().unwrap_or_else(PoisonError::into_inner);
        if staged.any_staged() {
            let cold = self.cold.read().unwrap_or_else(PoisonError::into_inner);
            MaybeStaged::Staged(for_staged(&mut Stager {
                cold: &cold,
                staged: &mut staged,
            }))
        } else if let Ok(mut cold) = self.cold.try_write() {
            MaybeStaged::Cold(for_full(&mut cold))
        } else {
            let cold = self.cold.read().unwrap_or_else(PoisonError::into_inner);
            MaybeStaged::Staged(for_staged(&mut Stager {
                cold: &cold,
                staged: &mut staged,
            }))
        }
    }
}

#[cfg(feature = "alloc")]
impl<T: StagedChanges> ArcStageOnWrite<T> {
    /// Constructs a new [`ArcStageOnWrite`] with the given value and no staged changes.
    pub fn new(value: T::Cold) -> Self {
        Self(Arc::new(AtomicStageOnWrite::new(value)))
    }

    /// Exactly the same as [`AtomicStageOnWrite::maybe_stage`], but uses `&mut` to maintain safety.
    pub fn maybe_stage<C, S>(
        &mut self,
        for_full: impl FnOnce(&mut T::Cold) -> C,
        for_staged: impl FnOnce(&mut Stager<T>) -> S,
    ) -> MaybeStaged<C, S> {
        // Safety: Safe since we have an exclusive reference to self.
        unsafe { self.maybe_stage_unsafe(for_full, for_staged) }
    }

    /// Easily run a stager function to stage changes.
    /// Then, tries to apply those changes if doing so wouldn't lock.
    ///
    /// # Deadlocks
    ///
    /// This can still deadlock if this [`Arc`] has been cloned around on the same thread and is still being locked on.
    /// But that is very unlikely.
    #[inline]
    pub fn stage_scope_locked_eager<R>(&mut self, f: impl FnOnce(&mut Stager<T>) -> R) -> R {
        // Safety: Since this has mutible access to self, we can be reasonably sure this is safe.
        // The only way this isn't safe is if the arc has been cloned on the same thread instead of passed by ref.
        // But that is documented above
        let mut lock = unsafe { self.stage_lock_unsafe() };
        let result = f(&mut lock.as_stager());
        self.apply_staged_non_blocking();
        result
    }
}

#[cfg(feature = "alloc")]
impl<T: StagedChanges> Deref for ArcStageOnWrite<T> {
    type Target = Arc<AtomicStageOnWrite<T>>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T: StagableWritesCore> Deref for RefStageOnWrite<'_, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        self.0
    }
}

impl<T: StagableWritesCore> StagableWrites for RefStageOnWrite<'_, T> {
    type Core = T;

    fn get_core(&self) -> &Self::Core {
        self.0
    }
}

#[cfg(feature = "alloc")]
impl<T: StagedChanges> StagableWrites for ArcStageOnWrite<T> {
    type Core = AtomicStageOnWrite<T>;

    fn get_core(&self) -> &Self::Core {
        &self.0
    }
}

impl<'a, T: StagedChanges> Stager<'a, T> {
    /// Allows a user to view this as a [`Stager`].
    #[inline]
    pub fn as_staged_ref(&self) -> StagedRef<'_, T> {
        StagedRef {
            cold: self.cold,
            staged: self.staged,
        }
    }
}

impl<'a, T: StagableWritesCore> StagerLocked<'a, T> {
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
    pub fn release(self) -> RefStageOnWrite<'a, T> {
        self.inner
    }
}

impl<'a, T: StagableWritesCore> StagedRefLocked<'a, T> {
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
    pub fn release(self) -> RefStageOnWrite<'a, T> {
        self.inner
    }

    /// Allows returning a reference to the locked staged data without releasing its lock.
    #[inline]
    pub fn get_staged_guard(&self) -> RwLockReadGuard<'a, T::Staging> {
        self.inner.0.raw_read_staged()
    }

    /// Allows returning a reference to the cold data without releasing its lock (it it has one).
    #[inline]
    pub fn get_cold_guard(&self) -> T::ColdRef<'a> {
        self.inner.0.raw_read_cold()
    }
}

impl<'a, T: StagedChanges> Clone for StagedRef<'a, T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<'a, T: StagedChanges> Copy for StagedRef<'a, T> {}

impl<'a, T: StagableWritesCore> Copy for RefStageOnWrite<'a, T> {}

impl<'a, T: StagableWritesCore> Clone for RefStageOnWrite<'a, T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<'a, T: StagableWritesCore> Clone for StagedRefLocked<'a, T> {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner,
            staged: self.get_staged_guard(),
            cold: self.get_cold_guard(),
        }
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

#[cfg(feature = "alloc")]
impl<T: StagedChanges> Default for ArcStageOnWrite<T>
where
    T::Cold: Default,
{
    fn default() -> Self {
        Self::new(T::Cold::default())
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
