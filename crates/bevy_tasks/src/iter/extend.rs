use super::{ParallelExtend, IntoParallelIterator, ParallelIterator};

/// Extends a vector with items from a parallel iterator.
impl<T> ParallelExtend<T> for Vec<T>
where
    T: Send,
{
    fn par_extend<I>(&mut self, par_iter: I)
    where
        I: IntoParallelIterator<Item = T>,
    {
        // See the vec_collect benchmarks in rayon-demo for different strategies.
        let par_iter = par_iter.into_par_iter();
        match par_iter.opt_len() {
            Some(len) => {
                // When Rust gets specialization, we can get here for indexed iterators
                // without relying on `opt_len`.  Until then, `special_extend()` fakes
                // an unindexed mode on the promise that `opt_len()` is accurate.
                crate::iter::collect::special_extend(par_iter, len, self);
            }
            None => {
                todo!();
                // This works like `extend`, but `Vec::append` is more efficient.
                // let list = par_iter.drive_unindexed(ListVecConsumer);
                // vec_append(self, list);
            }
        }
    }
}
