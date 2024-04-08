use core::cell::Cell;
use std::{
    mem,
    ops::{Deref, DerefMut},
    ptr::drop_in_place,
};
use thread_local::ThreadLocal;

/// A cohesive set of thread-local values of a given type.
///
/// Mutable references can be fetched if `T: Default` via [`Parallel::scope`].
#[derive(Default)]
pub struct Parallel<T: Send> {
    locals: ThreadLocal<Cell<T>>,
}

/// A scope guard of a `Parallel`, when this struct is dropped ,the value will writeback to its `Parallel`
pub struct ParallelGuard<'a, T: Send + Default> {
    value: T,
    parallel: &'a Parallel<T>,
}
impl<'a, T: Send + Default> Drop for ParallelGuard<'a, T> {
    fn drop(&mut self) {
        let cell = self.parallel.locals.get().unwrap();
        let mut value = T::default();
        std::mem::swap(&mut value, &mut self.value);
        cell.set(value);
        // SAFETY:
        // value is longer needed
        unsafe {
            drop_in_place(&mut self.value);
        }
    }
}
impl<'a, T: Send + Default> Deref for ParallelGuard<'a, T> {
    type Target = T;
    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.value
    }
}
impl<'a, T: Send + Default> DerefMut for ParallelGuard<'a, T> {
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.value
    }
}
impl<T: Send> Parallel<T> {
    /// Gets a mutable iterator over all of the per-thread queues.
    pub fn iter_mut(&mut self) -> impl Iterator<Item = &'_ mut T> {
        self.locals.iter_mut().map(|cell| cell.get_mut())
    }

    /// Clears all of the stored thread local values.
    pub fn clear(&mut self) {
        self.locals.clear();
    }
}

impl<T: Default + Send> Parallel<T> {
    /// Retrieves the thread-local value for the current thread and runs `f` on it.
    ///
    /// If there is no thread-local value, it will be initialized to it's default.
    pub fn scope<R>(&self, f: impl FnOnce(&mut T) -> R) -> R {
        let cell = self.locals.get_or_default();
        let mut value = cell.take();
        let ret = f(&mut value);
        cell.set(value);
        ret
    }

    /// Get the guard of Parallel
    ///
    /// If there is no thread-local value, it will be initialized to it's default.
    pub fn guard<'a>(&'a self) -> ParallelGuard<'a, T> {
        let cell = self.locals.get_or_default();
        let value = cell.take();
        ParallelGuard {
            value,
            parallel: self,
        }
    }
}

impl<T, I> Parallel<I>
where
    I: IntoIterator<Item = T> + Default + Send + 'static,
{
    /// Drains all enqueued items from all threads and returns an iterator over them.
    ///
    /// Unlike [`Vec::drain`], this will piecemeal remove chunks of the data stored.
    /// If iteration is terminated part way, the rest of the enqueued items in the same
    /// chunk will be dropped, and the rest of the undrained elements will remain.
    ///
    /// The ordering is not guaranteed.
    pub fn drain<B>(&mut self) -> impl Iterator<Item = T> + '_
    where
        B: FromIterator<T>,
    {
        self.locals.iter_mut().flat_map(|item| item.take())
    }
}

impl<T: Send> Parallel<Vec<T>> {
    /// Collect all enqueued items from all threads and appends them to the end of a
    /// single Vec.
    ///
    /// The ordering is not guaranteed.
    pub fn drain_into(&mut self, out: &mut Vec<T>) {
        let size = self
            .locals
            .iter_mut()
            .map(|queue| queue.get_mut().len())
            .sum();
        out.reserve(size);
        for queue in self.locals.iter_mut() {
            out.append(queue.get_mut());
        }
    }
}
