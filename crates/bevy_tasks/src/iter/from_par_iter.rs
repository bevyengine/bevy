use crate::iter::{FromParallelIterator, IntoParallelIterator, ParallelExtend};

/// Creates an empty default collection and extends it.
fn collect_extended<C, I>(par_iter: I) -> C
where
    I: IntoParallelIterator,
    C: ParallelExtend<I::Item> + Default,
{
    let mut collection = C::default();
    collection.par_extend(par_iter);
    collection
}

/// Collects items from a parallel iterator into a vector.
impl<T> FromParallelIterator<T> for Vec<T>
where
    T: Send,
{
    fn from_par_iter<I>(par_iter: I) -> Self
    where
        I: IntoParallelIterator<Item = T>,
    {
        collect_extended(par_iter)
    }
}
