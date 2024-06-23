use std::future::Future;
use bevy_ecs::system::Resource;
use bevy_internal::tasks::futures_lite::{AsyncRead, AsyncWrite};
use bevy_internal::tasks::{IoTaskPool, TaskPool};
use bevy_internal::utils::hashbrown::hash_map::{Iter, IterMut};
use bevy_internal::utils::HashMap;
mod tcp_registory;

pub struct AsyncHandler<T>(pub Vec<T>);

impl<T> AsyncHandler<T> {
    pub fn for_each_async_mut<'a, F, U, Fut>(&'a mut self, function: F, io: &TaskPool) -> Vec<U>
        where F: Fn(usize, &'a mut T) -> Fut,
              Fut: Future<Output = U>, U: Send + 'static {
        io.scope(|s| {
            for (index, value) in self.0.iter_mut().enumerate() {
                s.spawn(function(index, value))
            }
        })
    }
}

