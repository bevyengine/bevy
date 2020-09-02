use super::TaskPool;

pub trait ParallelSlice<T: Sync>: AsRef<[T]> {
    fn par_chunk_map<F, R>(&self, task_pool: &TaskPool, chunk_size: usize, f: F) -> Vec<R>
    where
        F: Fn(&[T]) -> R + Send + Sync,
        R: Send + 'static,
    {
        let slice = self.as_ref();
        let f = &f;
        task_pool.scope(|scope| {
            for chunk in slice.chunks(chunk_size) {
                scope.spawn(async move { f(chunk) });
            }
        })
    }

    fn par_splat_map<F, R>(&self, task_pool: &TaskPool, max_tasks: Option<usize>, f: F) -> Vec<R>
    where
        F: Fn(&[T]) -> R + Send + Sync,
        R: Send + 'static,
    {
        let slice = self.as_ref();
        let chunk_size = std::cmp::max(
            1,
            std::cmp::max(
                slice.len() / task_pool.thread_num(),
                slice.len() / max_tasks.unwrap_or(usize::MAX),
            ),
        );

        slice.par_chunk_map(task_pool, chunk_size, f)
    }
}

impl<S, T: Sync> ParallelSlice<T> for S where S: AsRef<[T]> {}

pub trait ParallelSliceMut<T: Send>: AsMut<[T]> {
    fn par_chunk_map_mut<F, R>(&mut self, task_pool: &TaskPool, chunk_size: usize, f: F) -> Vec<R>
    where
        F: Fn(&mut [T]) -> R + Send + Sync,
        R: Send + 'static,
    {
        let slice = self.as_mut();
        let f = &f;
        task_pool.scope(|scope| {
            for chunk in slice.chunks_mut(chunk_size) {
                scope.spawn(async move { f(chunk) });
            }
        })
    }

    fn par_splat_map_mut<F, R>(
        &mut self,
        task_pool: &TaskPool,
        max_tasks: Option<usize>,
        f: F,
    ) -> Vec<R>
    where
        F: Fn(&mut [T]) -> R + Send + Sync,
        R: Send + 'static,
    {
        let mut slice = self.as_mut();
        let chunk_size = std::cmp::max(
            1,
            std::cmp::max(
                slice.len() / task_pool.thread_num(),
                slice.len() / max_tasks.unwrap_or(usize::MAX),
            ),
        );

        slice.par_chunk_map_mut(task_pool, chunk_size, f)
    }
}

impl<S, T: Send> ParallelSliceMut<T> for S where S: AsMut<[T]> {}

#[cfg(test)]
mod tests {
    use crate::*;

    #[test]
    fn test_par_chunks_map() {
        let v = vec![42; 1000];
        let task_pool = TaskPool::new();
        let outputs = v.par_splat_map(&task_pool, None, |numbers| -> i32 { numbers.iter().sum() });

        let mut sum = 0;
        for output in outputs {
            sum += output;
        }

        assert_eq!(sum, 1000 * 42);
    }

    #[test]
    fn test_par_chunks_map_mut() {
        let mut v = vec![42; 1000];
        let task_pool = TaskPool::new();

        let outputs = v.par_splat_map_mut(&task_pool, None, |numbers| -> i32 {
            for number in numbers.iter_mut() {
                *number *= 2;
            }
            numbers.iter().sum()
        });

        let mut sum = 0;
        for output in outputs {
            sum += output;
        }

        assert_eq!(sum, 1000 * 42 * 2);
        assert_eq!(v[0], 84);
    }
}
