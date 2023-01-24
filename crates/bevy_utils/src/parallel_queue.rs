use core::{
    cell::Cell,
    ops::{Deref, DerefMut, Drop},
};
use thread_local::ThreadLocal;

/// A cohesive set of thread-local values of a given type.
///
/// Mutable references can be fetched if `T: Default` via [`Parallel::get`].
#[derive(Default)]
pub struct Parallel<T: Send> {
    locals: ThreadLocal<Cell<T>>,
}

impl<T: Send> Parallel<T> {
    /// Gets a mutable iterator over all of the per-thread queues.
    pub fn iter_mut(&mut self) -> impl Iterator<Item = &'_ mut T> {
        self.locals.iter_mut().map(|cell| cell.get_mut())
    }

    /// Clears all of the stored thread local values.
    pub fn clear(&mut self) {
        self.locals.clear()
    }
}

impl<T: Default + Send> Parallel<T> {
    /// Takes the thread-local value and replaces it with the default.
    #[inline]
    pub fn get(&self) -> ParRef<'_, T> {
        let cell = self.locals.get_or_default();
        let value = cell.take();
        ParRef { cell, value }
    }
}

impl<T, I> Parallel<I>
where
    I: IntoIterator<Item = T> + Default + Send + 'static,
{
    /// Collect all enqueued items from all threads and them into one
    pub fn drain<B>(&mut self) -> B
    where
        B: FromIterator<T>,
    {
        self.locals
            .iter_mut()
            .flat_map(|item| item.take().into_iter())
            .collect()
    }
}

impl<T: Send> Parallel<Vec<T>> {
    /// Collect all enqueued items from all threads and them into one
    pub fn drain_into(&mut self, out: &mut Vec<T>) {
        out.clear();
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

/// A retrieved thread-local reference to a value in [`Parallel`].
pub struct ParRef<'a, T: Default> {
    cell: &'a Cell<T>,
    value: T,
}

impl<'a, T: Default> Deref for ParRef<'a, T> {
    type Target = T;
    fn deref(&self) -> &T {
        &self.value
    }
}

impl<'a, T: Default> DerefMut for ParRef<'a, T> {
    fn deref_mut(&mut self) -> &mut T {
        &mut self.value
    }
}

impl<'a, T: Default> Drop for ParRef<'a, T> {
    fn drop(&mut self) {
        self.cell.set(core::mem::take(&mut self.value));
    }
}
