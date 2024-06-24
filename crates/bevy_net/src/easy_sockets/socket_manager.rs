use std::ops::DerefMut;
use std::sync::Weak;
use bevy_internal::tasks::TaskPool;
use crate::easy_sockets::{AsyncHandler, Buffer};
use crate::easy_sockets::spin_lock::SpinLock;

struct SocketManger<B, S> {
    handler: AsyncHandler<Weak<SpinLock<(B, Option<S>)>>>
}

impl<B, S> SocketManger<B, S>
where B: Buffer<InnerSocket = S> {
    fn update_buffers(&mut self, pool: &TaskPool) -> Vec<Result<Option<(Result<usize, ErrorAction<<B as Buffer>::Error>>, Result<usize, ErrorAction<<B as Buffer>::Error>>)>, usize>> {
        self.handler.for_each_async_mut(|index, weak| async move {
            if let Some(lock) = weak.upgrade() {
                let mut guard = lock.lock_async().await.unwrap();
                let mut inner = guard.deref_mut();
                
                if let Some(socket) = &mut inner.1 {
                    return Ok(
                        Some(
                            (inner.0.flush_write_bufs(socket).await,
                             inner.0.fill_read_bufs(socket).await)
                        )
                    )
                }
                return Ok(None)
            }

            Err(index)
        }, pool)
    }
}

enum ErrorAction<E> {
    /// Drop the socket.
    Drop,
    /// Present the error to the end user.
    Present(E),
    /// Take no automated action.
    /// However, you may wish to take
    /// your own corrective action.
    None
}