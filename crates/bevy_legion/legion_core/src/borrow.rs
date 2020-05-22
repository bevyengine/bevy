//! Atomic runtime borrow checking module.
//! These types implement something akin to `RefCell`, but are atomically handled allowing them to
//! cross thread boundaries.
use std::cell::UnsafeCell;
use std::hash::{Hash, Hasher};
use std::ops::Deref;
use std::ops::DerefMut;
use std::sync::atomic::AtomicIsize;

#[cfg(not(debug_assertions))]
use std::marker::PhantomData;

/// A `RefCell` implementation which is thread safe. This type performs all the standard runtime
/// borrow checking which would be familiar from using `RefCell`.
///
/// `UnsafeCell` is used in this type, but borrow checking is performed using atomic values,
/// garunteeing safe access across threads.
///
/// # Safety
/// Runtime borrow checking is only conducted in builds with `debug_assertions` enabled. Release
/// builds assume proper resource access and will cause undefined behavior with improper use.
pub struct AtomicRefCell<T> {
    value: UnsafeCell<T>,
    borrow_state: AtomicIsize,
}

impl<T: Default> Default for AtomicRefCell<T> {
    fn default() -> Self { Self::new(T::default()) }
}

impl<T: std::fmt::Debug> std::fmt::Debug for AtomicRefCell<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "({:?}) {:?}", self.borrow_state, self.value)
    }
}

impl<T> AtomicRefCell<T> {
    pub fn new(value: T) -> Self {
        AtomicRefCell {
            value: UnsafeCell::from(value),
            borrow_state: AtomicIsize::from(0),
        }
    }

    /// Retrieve an immutable `Ref` wrapped reference of `&T`.
    ///
    /// # Panics
    ///
    /// This method panics if this value is already mutably borrowed.
    ///
    /// # Safety
    /// Runtime borrow checking is only conducted in builds with `debug_assertions` enabled. Release
    /// builds assume proper resource access and will cause undefined behavior with improper use.
    #[inline(always)]
    pub fn get(&self) -> Ref<T> { self.try_get().unwrap() }

    /// Unwrap the value from the RefCell and kill it, returning the value.
    pub fn into_inner(self) -> T { self.value.into_inner() }

    /// Retrieve an immutable `Ref` wrapped reference of `&T`. This is the safe version of `get`
    /// providing an error result on failure.
    ///
    /// # Returns
    ///
    /// `Some(T)` if the value can be retrieved.
    /// `Err` if the value is already mutably borrowed.
    #[cfg(debug_assertions)]
    pub fn try_get(&self) -> Result<Ref<T>, String> {
        loop {
            let read = self.borrow_state.load(std::sync::atomic::Ordering::SeqCst);
            if read < 0 {
                return Err(format!(
                    "resource already borrowed as mutable: {}",
                    std::any::type_name::<T>()
                ));
            }

            if self.borrow_state.compare_and_swap(
                read,
                read + 1,
                std::sync::atomic::Ordering::SeqCst,
            ) == read
            {
                break;
            }
        }

        Ok(Ref::new(Shared::new(&self.borrow_state), unsafe {
            &*self.value.get()
        }))
    }

    /// Retrieve an immutable `Ref` wrapped reference of `&T`. This is the safe version of `get`
    /// providing an error result on failure.
    ///
    /// # Returns
    ///
    /// `Some(T)` if the value can be retrieved.
    /// `Err` if the value is already mutably borrowed.
    ///
    /// # Safety
    ///
    /// This release version of this function does not perform runtime borrow checking and will
    /// cause undefined behavior if borrow rules are violated. This means they should be enforced
    /// on the use of this type.
    #[cfg(not(debug_assertions))]
    #[inline(always)]
    pub fn try_get(&self) -> Result<Ref<T>, &'static str> {
        Ok(Ref::new(Shared::new(&self.borrow_state), unsafe {
            &*self.value.get()
        }))
    }

    /// Retrieve an mutable `RefMut` wrapped reference of `&mut T`.
    ///
    /// # Panics
    ///
    /// This method panics if this value is already mutably borrowed.
    ///
    /// # Safety
    /// Runtime borrow checking is only conducted in builds with `debug_assertions` enabled. Release
    /// builds assume proper resource access and will cause undefined behavior with improper use.
    #[inline(always)]
    pub fn get_mut(&self) -> RefMut<T> { self.try_get_mut().unwrap() }

    /// Retrieve a mutable `RefMut` wrapped reference of `&mut T`. This is the safe version of
    /// `get_mut` providing an error result on failure.
    ///
    /// # Returns
    ///
    /// `Some(T)` if the value can be retrieved.
    /// `Err` if the value is already mutably borrowed.
    ///
    /// # Safety
    ///
    /// This release version of this function does not perform runtime borrow checking and will
    /// cause undefined behavior if borrow rules are violated. This means they should be enforced
    /// on the use of this type.
    #[cfg(debug_assertions)]
    pub fn try_get_mut(&self) -> Result<RefMut<T>, String> {
        let borrowed =
            self.borrow_state
                .compare_and_swap(0, -1, std::sync::atomic::Ordering::SeqCst);
        match borrowed {
            0 => Ok(RefMut::new(Exclusive::new(&self.borrow_state), unsafe {
                &mut *self.value.get()
            })),
            x if x < 0 => Err(format!(
                "resource already borrowed as mutable: {}",
                std::any::type_name::<T>()
            )),
            _ => Err(format!(
                "resource already borrowed as immutable: {}",
                std::any::type_name::<T>()
            )),
        }
    }

    /// Retrieve a mutable `RefMut` wrapped reference of `&mut T`. This is the safe version of
    /// `get_mut` providing an error result on failure.
    ///
    /// # Returns
    ///
    /// `Some(T)` if the value can be retrieved.
    /// `Err` if the value is already mutably borrowed.
    ///
    /// # Safety
    ///
    /// This release version of this function does not perform runtime borrow checking and will
    /// cause undefined behavior if borrow rules are violated. This means they should be enforced
    /// on the use of this type.
    #[cfg(not(debug_assertions))]
    #[inline(always)]
    pub fn try_get_mut(&self) -> Result<RefMut<T>, &'static str> {
        Ok(RefMut::new(Exclusive::new(&self.borrow_state), unsafe {
            &mut *self.value.get()
        }))
    }
}

unsafe impl<T: Send> Send for AtomicRefCell<T> {}

unsafe impl<T: Sync> Sync for AtomicRefCell<T> {}

/// Type used for allowing unsafe cloning of internal types
pub trait UnsafeClone {
    /// Clone this type unsafely
    ///
    /// # Safety
    /// Types implementing this trait perform clones under an unsafe context.
    unsafe fn clone(&self) -> Self;
}

impl<A: UnsafeClone, B: UnsafeClone> UnsafeClone for (A, B) {
    unsafe fn clone(&self) -> Self { (self.0.clone(), self.1.clone()) }
}

#[derive(Debug)]
pub struct Shared<'a> {
    #[cfg(debug_assertions)]
    state: &'a AtomicIsize,
    #[cfg(not(debug_assertions))]
    state: PhantomData<&'a ()>,
}

impl<'a> Shared<'a> {
    #[cfg(debug_assertions)]
    fn new(state: &'a AtomicIsize) -> Self { Self { state } }
    #[cfg(not(debug_assertions))]
    #[inline(always)]
    fn new(_: &'a AtomicIsize) -> Self { Self { state: PhantomData } }
}

#[cfg(debug_assertions)]
impl<'a> Drop for Shared<'a> {
    fn drop(&mut self) { self.state.fetch_sub(1, std::sync::atomic::Ordering::SeqCst); }
}

impl<'a> Clone for Shared<'a> {
    #[inline(always)]
    fn clone(&self) -> Self {
        #[cfg(debug_assertions)]
        self.state.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        Shared { state: self.state }
    }
}

impl<'a> UnsafeClone for Shared<'a> {
    unsafe fn clone(&self) -> Self { Clone::clone(&self) }
}

#[derive(Debug)]
pub struct Exclusive<'a> {
    #[cfg(debug_assertions)]
    state: &'a AtomicIsize,
    #[cfg(not(debug_assertions))]
    state: PhantomData<&'a ()>,
}

impl<'a> Exclusive<'a> {
    #[cfg(debug_assertions)]
    fn new(state: &'a AtomicIsize) -> Self { Self { state } }
    #[cfg(not(debug_assertions))]
    #[inline(always)]
    fn new(_: &'a AtomicIsize) -> Self { Self { state: PhantomData } }
}

#[cfg(debug_assertions)]
impl<'a> Drop for Exclusive<'a> {
    fn drop(&mut self) { self.state.fetch_add(1, std::sync::atomic::Ordering::SeqCst); }
}

impl<'a> UnsafeClone for Exclusive<'a> {
    #[inline(always)]
    unsafe fn clone(&self) -> Self {
        #[cfg(debug_assertions)]
        self.state.fetch_sub(1, std::sync::atomic::Ordering::SeqCst);
        Exclusive { state: self.state }
    }
}

#[derive(Debug)]
pub struct Ref<'a, T: 'a> {
    #[allow(dead_code)]
    // held for drop impl
    borrow: Shared<'a>,
    value: &'a T,
}

impl<'a, T: 'a> Clone for Ref<'a, T> {
    #[inline(always)]
    fn clone(&self) -> Self { Ref::new(Clone::clone(&self.borrow), self.value) }
}

impl<'a, T: 'a> Ref<'a, T> {
    #[inline(always)]
    pub fn new(borrow: Shared<'a>, value: &'a T) -> Self { Self { borrow, value } }

    #[inline(always)]
    pub fn map_into<K: 'a, F: FnMut(&'a T) -> K>(self, mut f: F) -> RefMap<'a, K> {
        RefMap::new(self.borrow, f(&self.value))
    }

    #[inline(always)]
    pub fn map<K: 'a, F: FnMut(&T) -> &K>(&self, mut f: F) -> Ref<'a, K> {
        Ref::new(Clone::clone(&self.borrow), f(&self.value))
    }

    /// Deconstructs this mapped borrow to its underlying borrow state and value.
    ///
    /// # Safety
    ///
    /// Ensure that you still follow all safety guidelines of this mapped ref.
    #[inline(always)]
    pub unsafe fn deconstruct(self) -> (Shared<'a>, &'a T) { (self.borrow, self.value) }
}

impl<'a, T: 'a> Deref for Ref<'a, T> {
    type Target = T;

    #[inline(always)]
    fn deref(&self) -> &Self::Target { self.value }
}

impl<'a, T: 'a> AsRef<T> for Ref<'a, T> {
    #[inline(always)]
    fn as_ref(&self) -> &T { self.value }
}

impl<'a, T: 'a> std::borrow::Borrow<T> for Ref<'a, T> {
    #[inline(always)]
    fn borrow(&self) -> &T { self.value }
}

impl<'a, T> PartialEq for Ref<'a, T>
where
    T: 'a + PartialEq,
{
    fn eq(&self, other: &Self) -> bool { self.value == other.value }
}
impl<'a, T> Eq for Ref<'a, T> where T: 'a + Eq {}

impl<'a, T> PartialOrd for Ref<'a, T>
where
    T: 'a + PartialOrd,
{
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        self.value.partial_cmp(&other.value)
    }
}
impl<'a, T> Ord for Ref<'a, T>
where
    T: 'a + Ord,
{
    fn cmp(&self, other: &Self) -> std::cmp::Ordering { self.value.cmp(&other.value) }
}

impl<'a, T> Hash for Ref<'a, T>
where
    T: 'a + Hash,
{
    fn hash<H: Hasher>(&self, state: &mut H) { self.value.hash(state); }
}

#[derive(Debug)]
pub struct RefMut<'a, T: 'a> {
    #[allow(dead_code)]
    // held for drop impl
    borrow: Exclusive<'a>,
    value: &'a mut T,
}

impl<'a, T: 'a> RefMut<'a, T> {
    #[inline(always)]
    pub fn new(borrow: Exclusive<'a>, value: &'a mut T) -> Self { Self { borrow, value } }

    #[inline(always)]
    pub fn map_into<K: 'a, F: FnMut(&mut T) -> K>(mut self, mut f: F) -> RefMapMut<'a, K> {
        RefMapMut::new(self.borrow, f(&mut self.value))
    }

    /// Deconstructs this mapped borrow to its underlying borrow state and value.
    ///
    /// # Safety
    ///
    /// Ensure that you still follow all safety guidelines of this mapped ref.
    #[inline(always)]
    pub unsafe fn deconstruct(self) -> (Exclusive<'a>, &'a mut T) { (self.borrow, self.value) }

    #[inline(always)]
    pub fn split<First, Rest, F: Fn(&'a mut T) -> (&'a mut First, &'a mut Rest)>(
        self,
        f: F,
    ) -> (RefMut<'a, First>, RefMut<'a, Rest>) {
        let (first, rest) = f(self.value);
        (
            RefMut::new(unsafe { self.borrow.clone() }, first),
            RefMut::new(self.borrow, rest),
        )
    }
}

impl<'a, T: 'a> Deref for RefMut<'a, T> {
    type Target = T;

    #[inline(always)]
    fn deref(&self) -> &Self::Target { self.value }
}

impl<'a, T: 'a> DerefMut for RefMut<'a, T> {
    #[inline(always)]
    fn deref_mut(&mut self) -> &mut Self::Target { self.value }
}

impl<'a, T: 'a> AsRef<T> for RefMut<'a, T> {
    #[inline(always)]
    fn as_ref(&self) -> &T { self.value }
}

impl<'a, T: 'a> AsMut<T> for RefMut<'a, T> {
    #[inline(always)]
    fn as_mut(&mut self) -> &mut T { self.value }
}

impl<'a, T: 'a> std::borrow::Borrow<T> for RefMut<'a, T> {
    #[inline(always)]
    fn borrow(&self) -> &T { self.value }
}

impl<'a, T> PartialEq for RefMut<'a, T>
where
    T: 'a + PartialEq,
{
    fn eq(&self, other: &Self) -> bool { self.value == other.value }
}
impl<'a, T> Eq for RefMut<'a, T> where T: 'a + Eq {}

impl<'a, T> PartialOrd for RefMut<'a, T>
where
    T: 'a + PartialOrd,
{
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        self.value.partial_cmp(&other.value)
    }
}
impl<'a, T> Ord for RefMut<'a, T>
where
    T: 'a + Ord,
{
    fn cmp(&self, other: &Self) -> std::cmp::Ordering { self.value.cmp(&other.value) }
}

impl<'a, T> Hash for RefMut<'a, T>
where
    T: 'a + Hash,
{
    fn hash<H: Hasher>(&self, state: &mut H) { self.value.hash(state); }
}

#[derive(Debug)]
pub struct RefMap<'a, T: 'a> {
    #[allow(dead_code)]
    // held for drop impl
    borrow: Shared<'a>,
    value: T,
}

impl<'a, T: 'a> RefMap<'a, T> {
    #[inline(always)]
    pub fn new(borrow: Shared<'a>, value: T) -> Self { Self { borrow, value } }

    #[inline(always)]
    pub fn map_into<K: 'a, F: FnMut(&mut T) -> K>(mut self, mut f: F) -> RefMap<'a, K> {
        RefMap::new(self.borrow, f(&mut self.value))
    }

    /// Deconstructs this mapped borrow to its underlying borrow state and value.
    ///
    /// # Safety
    ///
    /// Ensure that you still follow all safety guidelines of this  mapped ref.
    #[inline(always)]
    pub unsafe fn deconstruct(self) -> (Shared<'a>, T) { (self.borrow, self.value) }
}

impl<'a, T: 'a> Deref for RefMap<'a, T> {
    type Target = T;

    #[inline(always)]
    fn deref(&self) -> &Self::Target { &self.value }
}

impl<'a, T: 'a> AsRef<T> for RefMap<'a, T> {
    #[inline(always)]
    fn as_ref(&self) -> &T { &self.value }
}

impl<'a, T: 'a> std::borrow::Borrow<T> for RefMap<'a, T> {
    #[inline(always)]
    fn borrow(&self) -> &T { &self.value }
}

#[derive(Debug)]
pub struct RefMapMut<'a, T: 'a> {
    #[allow(dead_code)]
    // held for drop impl
    borrow: Exclusive<'a>,
    value: T,
}

impl<'a, T: 'a> RefMapMut<'a, T> {
    #[inline(always)]
    pub fn new(borrow: Exclusive<'a>, value: T) -> Self { Self { borrow, value } }

    #[inline(always)]
    pub fn map_into<K: 'a, F: FnMut(&mut T) -> K>(mut self, mut f: F) -> RefMapMut<'a, K> {
        RefMapMut {
            value: f(&mut self.value),
            borrow: self.borrow,
        }
    }

    /// Deconstructs this mapped borrow to its underlying borrow state and value.
    ///
    /// # Safety
    ///
    /// Ensure that you still follow all safety guidelines of this mutable mapped ref.
    #[inline(always)]
    pub unsafe fn deconstruct(self) -> (Exclusive<'a>, T) { (self.borrow, self.value) }
}

impl<'a, T: 'a> Deref for RefMapMut<'a, T> {
    type Target = T;

    #[inline(always)]
    fn deref(&self) -> &Self::Target { &self.value }
}

impl<'a, T: 'a> DerefMut for RefMapMut<'a, T> {
    #[inline(always)]
    fn deref_mut(&mut self) -> &mut Self::Target { &mut self.value }
}

impl<'a, T: 'a> AsRef<T> for RefMapMut<'a, T> {
    #[inline(always)]
    fn as_ref(&self) -> &T { &self.value }
}

impl<'a, T: 'a> AsMut<T> for RefMapMut<'a, T> {
    #[inline(always)]
    fn as_mut(&mut self) -> &mut T { &mut self.value }
}

impl<'a, T: 'a> std::borrow::Borrow<T> for RefMapMut<'a, T> {
    #[inline(always)]
    fn borrow(&self) -> &T { &self.value }
}

#[derive(Debug)]
pub struct RefIter<'a, T: 'a, I: Iterator<Item = &'a T>> {
    #[allow(dead_code)]
    // held for drop impl
    borrow: Shared<'a>,
    iter: I,
}

impl<'a, T: 'a, I: Iterator<Item = &'a T>> RefIter<'a, T, I> {
    #[inline(always)]
    pub fn new(borrow: Shared<'a>, iter: I) -> Self { Self { borrow, iter } }
}

impl<'a, T: 'a, I: Iterator<Item = &'a T>> Iterator for RefIter<'a, T, I> {
    type Item = Ref<'a, T>;

    #[inline(always)]
    fn next(&mut self) -> Option<Self::Item> {
        if let Some(item) = self.iter.next() {
            Some(Ref::new(Clone::clone(&self.borrow), item))
        } else {
            None
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) { self.iter.size_hint() }
}

impl<'a, T: 'a, I: Iterator<Item = &'a T> + ExactSizeIterator> ExactSizeIterator
    for RefIter<'a, T, I>
{
}

#[derive(Debug)]
enum TryIter<State, T> {
    Found { borrow: State, iter: T },
    Missing(usize),
}

#[derive(Debug)]
pub struct TryRefIter<'a, T: 'a, I: Iterator<Item = &'a T>> {
    inner: TryIter<Shared<'a>, I>,
}

impl<'a, T: 'a, I: Iterator<Item = &'a T>> TryRefIter<'a, T, I> {
    #[inline(always)]
    pub(crate) fn found(borrow: Shared<'a>, iter: I) -> Self {
        Self {
            inner: TryIter::Found { borrow, iter },
        }
    }

    #[inline(always)]
    pub(crate) fn missing(count: usize) -> Self {
        Self {
            inner: TryIter::Missing(count),
        }
    }
}

impl<'a, T: 'a, I: Iterator<Item = &'a T>> Iterator for TryRefIter<'a, T, I> {
    type Item = Option<Ref<'a, T>>;

    #[inline(always)]
    fn next(&mut self) -> Option<Self::Item> {
        Some(match self.inner {
            TryIter::Found {
                ref borrow,
                ref mut iter,
                ..
            } => Some(Ref::new(Clone::clone(borrow), iter.next()?)),
            TryIter::Missing(ref mut n) => {
                *n = n.checked_sub(1)?;
                None
            }
        })
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        match self.inner {
            TryIter::Found { ref iter, .. } => iter.size_hint(),
            TryIter::Missing(n) => (n, Some(n)),
        }
    }
}

impl<'a, T: 'a, I: Iterator<Item = &'a T> + ExactSizeIterator> ExactSizeIterator
    for TryRefIter<'a, T, I>
{
}

#[derive(Debug)]
pub struct RefIterMut<'a, T: 'a, I: Iterator<Item = &'a mut T>> {
    #[allow(dead_code)]
    // held for drop impl
    borrow: Exclusive<'a>,
    iter: I,
}

impl<'a, T: 'a, I: Iterator<Item = &'a mut T>> RefIterMut<'a, T, I> {
    #[inline(always)]
    pub fn new(borrow: Exclusive<'a>, iter: I) -> Self { Self { borrow, iter } }
}

impl<'a, T: 'a, I: Iterator<Item = &'a mut T>> Iterator for RefIterMut<'a, T, I> {
    type Item = RefMut<'a, T>;

    #[inline(always)]
    fn next(&mut self) -> Option<Self::Item> {
        if let Some(item) = self.iter.next() {
            Some(RefMut::new(unsafe { self.borrow.clone() }, item))
        } else {
            None
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) { self.iter.size_hint() }
}

impl<'a, T: 'a, I: Iterator<Item = &'a mut T> + ExactSizeIterator> ExactSizeIterator
    for RefIterMut<'a, T, I>
{
}

#[derive(Debug)]
pub struct TryRefIterMut<'a, T: 'a, I: Iterator<Item = &'a mut T>> {
    inner: TryIter<Exclusive<'a>, I>,
}

impl<'a, T: 'a, I: Iterator<Item = &'a mut T>> TryRefIterMut<'a, T, I> {
    #[inline(always)]
    pub(crate) fn found(borrow: Exclusive<'a>, iter: I) -> Self {
        Self {
            inner: TryIter::Found { borrow, iter },
        }
    }

    #[inline(always)]
    pub(crate) fn missing(count: usize) -> Self {
        Self {
            inner: TryIter::Missing(count),
        }
    }
}

impl<'a, T: 'a, I: Iterator<Item = &'a mut T>> Iterator for TryRefIterMut<'a, T, I> {
    type Item = Option<RefMut<'a, T>>;

    #[inline(always)]
    fn next(&mut self) -> Option<Self::Item> {
        Some(match self.inner {
            TryIter::Found {
                ref borrow,
                ref mut iter,
                ..
            } => Some(RefMut::new(unsafe { borrow.clone() }, iter.next()?)),
            TryIter::Missing(ref mut n) => {
                *n = n.checked_sub(1)?;
                None
            }
        })
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        match self.inner {
            TryIter::Found { ref iter, .. } => iter.size_hint(),
            TryIter::Missing(n) => (n, Some(n)),
        }
    }
}

impl<'a, T: 'a, I: Iterator<Item = &'a mut T> + ExactSizeIterator> ExactSizeIterator
    for TryRefIterMut<'a, T, I>
{
}
