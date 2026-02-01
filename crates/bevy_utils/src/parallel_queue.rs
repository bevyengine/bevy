use alloc::vec::Vec;
use core::{cell::RefCell, ops::DerefMut};
use thread_local::ThreadLocal;

/// A cohesive set of thread-local values of a given type.
///
/// Mutable references can be fetched if `T: Default` via [`Parallel::scope`].
pub struct Parallel<T: Send> {
    locals: ThreadLocal<RefCell<T>>,
}

impl<T: Send> Parallel<T> {
    /// Gets a mutable iterator over all of the per-thread queues.
    pub fn iter_mut(&mut self) -> impl Iterator<Item = &'_ mut T> {
        self.locals.iter_mut().map(RefCell::get_mut)
    }

    /// Clears all of the stored thread local values.
    pub fn clear(&mut self) {
        self.locals.clear();
    }

    /// Retrieves the thread-local value for the current thread and runs `f` on it.
    ///
    /// If there is no thread-local value, it will be initialized to the result
    /// of `create`.
    pub fn scope_or<R>(&self, create: impl FnOnce() -> T, f: impl FnOnce(&mut T) -> R) -> R {
        f(&mut self.borrow_local_mut_or(create))
    }

    /// Mutably borrows the thread-local value.
    ///
    /// If there is no thread-local value, it will be initialized to the result
    /// of `create`.
    pub fn borrow_local_mut_or(
        &self,
        create: impl FnOnce() -> T,
    ) -> impl DerefMut<Target = T> + '_ {
        self.locals.get_or(|| RefCell::new(create())).borrow_mut()
    }
}

impl<T: Default + Send> Parallel<T> {
    /// Retrieves the thread-local value for the current thread and runs `f` on it.
    ///
    /// If there is no thread-local value, it will be initialized to its default.
    pub fn scope<R>(&self, f: impl FnOnce(&mut T) -> R) -> R {
        self.scope_or(Default::default, f)
    }

    /// Mutably borrows the thread-local value.
    ///
    /// If there is no thread-local value, it will be initialized to its default.
    pub fn borrow_local_mut(&self) -> impl DerefMut<Target = T> + '_ {
        self.borrow_local_mut_or(Default::default)
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
    pub fn drain(&mut self) -> impl Iterator<Item = T> + '_ {
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

// `Default` is manually implemented to avoid the `T: Default` bound.
impl<T: Send> Default for Parallel<T> {
    fn default() -> Self {
        Self {
            locals: ThreadLocal::default(),
        }
    }
}
