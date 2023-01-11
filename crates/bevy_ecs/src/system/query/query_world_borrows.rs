//! Conceptually the world borrows for `Query` are:
//!
//! `&'lock mut HashMap<ArchetypeComponentId, ReadOrWriteGuard<'world, Column>>`
//!
//! or for a `(Q, F): ReadOnlyWorldQuery`:
//!
//! `&'lock HashMap<ArchetypeComponentId, ReadOrWriteGuard<'world, Column>>`
//!
//! Lifetime names are a bit confusing here because the `'world` lifetime on `Query` actually corresponds to
//! the `'lock` lifetime I used here and there is no lifetime on `Query` representing the `'world` in the above types.
//! For the rest of these comments I'll use `qworld` to refer to the `'world` lifetime on `Query` and `'world` to refer
//! to the one in the type above.
//!
//! ---
//!
//! Perhaps counter to intuition `Query` does not 'own' the locks for its component access, this is required in order
//! to allow [`Query::get_inner`] and [`Query::iter_inner`] to exist (these methods allow producing borrows that outlive
//! the `Query`)
//!
//! A notable thing about this is that for non-readonly `Query`'s the world borrows are an `&'lock mut ...` this
//! helps to explain why the lifetime annotations on `Query` methods do not return borrows tied to `'qworld` but instead `'_`
//! (see [#4243](https://github.com/bevyengine/bevy/pull/4243)). In our methods we either have `&Self` or `&mut Self` which
//! also means that inside of `Query::get (_mut?)` we have:
//!
//! `&'a (mut?) &'lock mut HashMap<ArchetypeComponentId, ReadOrWriteGuard<'world, Column>>`
//!
//! and borrow check would naturally prevent us from creating an `&'a (mut?) Column` out of this, which is what the unsound fn sig:
//!
//! `fn get_mut(&mut self, e: Entity) -> QueryItem<'qworld, Q>`
//!
//! would require doing.
//!
//! ---
//!
//! The [`QueryLockBorrows`] abstraction exists to help prevent this kind of impl footgun by never giving out a
//! `&'qworld World` without taking ownership of `QueryLockBorrows` (conceptually similar to having a `&'a mut &'b mut T`
//! and consuming the `&'a mut ..` to get a `&'a mut T`)
//!
//! This abstraction also exists to make all ways of gaining access to the `&World` that `Query` requires, unsafe. Since the
//! `&World` does not actually mean you can access the entire world immutably, a large amount of safe APIs on `World` would
//! be unsound to call which is a footgun for anyone implementing `Query`.
//!
//! In the future this type may be extended to _actually_ store a borrow of a hashmap when `debug_assertions` are enabled
//! so that we can check that all usage of the query is consistent with what access we told the scheduler we required.

use super::*;
use std::marker::PhantomData;

/// Modelling `Query`'s borrow on `World` as `&'qworld World` is not "correct" for many reasons.
/// This struct intends to hide the `&'qworld World` and expose an API that is more similar to what
/// the `&'qworld World` conceptually represents.
///
/// This struct only implements [`Copy`] and [`Clone`] when `QF: ReadOnlyWorldQuery` to mimic the fact that
/// `ReadOnlyWorldQuery` borrows should act like `&'lock HashMap<..., ...>` wheras mutable world queries act
/// like `&'lock mut HashMap<..., ...>` (see module level docs).
pub struct QueryLockBorrows<'lock, QF: WorldQuery> {
    world: &'lock World,
    // invariance because its probably possible to have two worldquery's where one is the subtype of another
    // but each have different mutabilities. i.e. `dyn for<'a> Trait<'a>: WorldQuery` and `dyn Trait<'static>: WorldQuery`.
    // probably not unsound since `Q` and `F` are already invariant on `Query` but this seems like a footgun and I don't care
    // to try and reason about if this could cause trouble when covariant.
    // we dont use `*mut QF` because it would make this type not `Sync` and we dont use `&'static mut QF` because it
    // would implicitly require `QF: 'static`.
    _p: PhantomData<fn(QF) -> QF>,
}

impl<'lock, QF: WorldQuery> QueryLockBorrows<'lock, QF> {
    // FIXME: this should take some kind of `InteriorMutableWorld`, see #5956.
    /// # Safety
    ///
    /// It must be valid to access data specified by the `QF` `WorldQuery` from
    /// `world` for as long as the `'lock` lifetime is live.
    pub unsafe fn new(world: &'lock World) -> Self {
        Self {
            world,
            _p: PhantomData,
        }
    }

    /// See module level docs for why this takes `self` // TODO
    ///
    /// # Safety
    /// - The `World` must not be accessed in a way that the `Query`'s access does not give
    /// it permission to. You should be careful when working with this `&World` as many safe functions
    /// on `World` will be unsound to call.
    pub unsafe fn into_world_ref(self) -> &'lock World {
        self.world
    }

    /// If the returned `World` is going to be accessed mutably consider using
    /// [`QueryLockBorrows::world_mut`] instead.
    ///
    /// See module level docs for why this does not return `&'world World`
    ///
    /// # Safety
    /// - The `World` must not be accessed in a way that the `Query`'s access does not give
    /// it permission to. You should be careful when working with this `&World` as many safe functions
    /// on `World` will be unsound to call.
    pub unsafe fn world_ref(&self) -> &World {
        self.world
    }

    /// This is the same as [`QueryLockBorrows::world_mut`] except that it ties the `&World`
    /// to a mutable borrow of self, in theory allowing the borrow checker to catch more mistakes.
    /// `world_mut` should be used whenever mutable access of world is required if possible, otherwise use `world` instead.
    ///
    /// See module level docs for why this does not return `&'world World`
    ///
    /// # Safety
    /// - The `World` must not be accessed in a way that the `Query`'s access does not give
    /// it permission to. You should be careful when working with this `&World` as many safe functions
    /// on `World` will be unsound to call.
    pub unsafe fn world_mut(&mut self) -> &World {
        self.world
    }

    // FIXME ideally we remove this method its super sketchy
    /// Returns the underlying `&'world World` without the lifetime tied to the borrow of self that this method makes.
    /// You should almost NEVER use this method and instead opt to use `world_ref` `world_mut` or `into_world_ref`.
    ///
    /// # Safety
    /// - The `World` must not be accessed in a way that the `Query`'s access does not give
    /// it permission to. You should be careful when working with this `&World` as many safe functions
    /// on `World` will be unsound to call.
    /// - As the returned lifetime is not bound to `&self` you should avoid calling any methods on this struct
    /// until you can be sure that the returned borrow (and any copies of it) are dead.
    pub unsafe fn world_ref_unbounded(&self) -> &'lock World {
        self.world
    }

    /// This API mimics reborrowing `&'_ mut &'lock mut HashMap<..., ...>`
    /// as `&'_ mut HashMap<..., ...>`. If `QF: ReadOnlyWorldQuery` holds then you should
    /// just copy/clone `QueryLockBorrows` out from underneath the reference.
    ///
    /// See also [`QueryLockBorrows::reborrow`] for a version that returns readonly QF.
    pub fn reborrow_mut(&mut self) -> QueryLockBorrows<'_, QF> {
        QueryLockBorrows {
            world: self.world,
            _p: PhantomData,
        }
    }

    /// This API mimics reborrowing `&'_ &'lock mut HashMap<..., ...>`
    /// as `&'_ HashMap<..., ...>`. If `QF: ReadOnlyWorldQuery` holds then you should
    /// just copy/clone `QueryLockBorrows` out from underneath the reference.
    ///
    /// See also [`QueryLockBorrows::reborrow_mut`] for a version that returns QF.
    pub fn reborrow(&self) -> QueryLockBorrows<'_, QF::ReadOnly> {
        QueryLockBorrows {
            world: self.world,
            _p: PhantomData,
        }
    }

    pub fn to_readonly(self) -> QueryLockBorrows<'lock, QF::ReadOnly> {
        QueryLockBorrows {
            world: self.world,
            _p: PhantomData,
        }
    }
}

/// See module level docs and [`QueryLockBorrows`] docs for why this is only implemented for
/// `QF: ReadOnlyWorldQuery` instead of all `QF`.
impl<QF: ReadOnlyWorldQuery> Copy for QueryLockBorrows<'_, QF> {}
/// See module level docs and [`QueryLockBorrows`] docs for why this is only implemented for
/// `QF: ReadOnlyWorldQuery` instead of all `QF`.
impl<QF: ReadOnlyWorldQuery> Clone for QueryLockBorrows<'_, QF> {
    fn clone(&self) -> Self {
        Self {
            world: self.world,
            _p: PhantomData,
        }
    }
}
