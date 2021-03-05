use std::{
    borrow::Cow,
    future::Future,
    sync::Arc,
    task::{Poll, Waker},
};

use async_channel::{Receiver, Sender};
use bevy_tasks::{AsyncComputeTaskPool, TaskPool};
use bevy_utils::BoxedFuture;
use futures_lite::pin;
use parking_lot::Mutex;

use crate::{
    prelude::World,
    system::{ExclusiveSystem, SystemId},
};

use super::{AccessorTrait, AsyncSystemHandle, AsyncSystemOutput, AsyncSystemOutputError};

#[derive(Clone)]
pub struct ExclusiveAccessor {
    channel: Sender<Box<dyn GenericAccess>>,
}

impl AccessorTrait for ExclusiveAccessor {
    type AccessSystem = ExclusiveAccessorRunnerSystem;

    fn new() -> (Self, Self::AccessSystem) {
        let (tx, rx) = async_channel::unbounded();
        (
            ExclusiveAccessor { channel: tx },
            ExclusiveAccessorRunnerSystem {
                channel: rx,
                name: Cow::Borrowed(std::any::type_name::<Self>()),
                id: SystemId::new(),
            },
        )
    }
}

pub trait AccessFn<'a, 'env, Out> {
    fn call(self: Box<Self>, world: &'a mut World) -> Out;
}

impl<'a, 'env, Out, F> AccessFn<'a, 'env, Out> for F
where
    F: FnOnce(&'a mut World) -> Out + 'env,
{
    fn call(self: Box<Self>, world: &'a mut World) -> Out {
        self(world)
    }
}

impl ExclusiveAccessor {
    pub fn access<'env, R: Send + Sync + 'static>(
        &mut self,
        sync: impl for<'a> AccessFn<'a, 'env, R> + Send + Sync + 'env,
    ) -> impl Future<Output = R> + Send + Sync + 'env {
        AccessFuture {
            state: AccessFutureState::FirstPoll {
                boxed: Box::new(sync),
                tx: self.channel.clone(),
            },
        }
    }
}

struct Access<'env, Out> {
    inner: Arc<Mutex<Option<Box<dyn for<'a> AccessFn<'a, 'env, Out> + Send + Sync + 'env>>>>,
    tx: Sender<Out>,
    waker: Waker,
}

trait GenericAccess: Send + Sync {
    fn run(self: Box<Self>, world: &mut World);
}

impl<'env, Out: Send + Sync + 'env> GenericAccess for Access<'env, Out> {
    fn run(self: Box<Self>, world: &mut World) {
        if let Some(sync) = self.inner.lock().take() {
            self.tx.try_send(sync.call(world)).unwrap();
        }
        self.waker.wake();
    }
}

enum AccessFutureState<'env, R> {
    FirstPoll {
        boxed: Box<dyn for<'a> AccessFn<'a, 'env, R> + Send + Sync + 'env>,
        tx: Sender<Box<dyn GenericAccess>>,
    },
    WaitingForCompletion(
        Receiver<R>,
        Arc<Mutex<Option<Box<dyn for<'a> AccessFn<'a, 'env, R> + Send + Sync + 'env>>>>,
    ),
}

pub struct AccessFuture<'env, R> {
    state: AccessFutureState<'env, R>,
}

impl<'env, R: Send + Sync + 'env> Future for AccessFuture<'env, R> {
    type Output = R;

    fn poll(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Self::Output> {
        match &mut self.state {
            AccessFutureState::FirstPoll { .. } => {
                let (tx, rx) = async_channel::bounded(1);
                let arc = Arc::new(Mutex::new(None));
                if let AccessFutureState::FirstPoll { boxed, tx: mtx } = std::mem::replace(
                    &mut self.state,
                    AccessFutureState::WaitingForCompletion(rx, arc.clone()),
                ) {
                    *arc.lock() = Some(boxed);
                    let msg = Access {
                        inner: arc,
                        tx,
                        waker: cx.waker().clone(),
                    };
                    let boxed: Box<dyn GenericAccess + 'env> = Box::new(msg);
                    let boxed: Box<dyn GenericAccess + 'static> =
                    // Safe: the reference will only live as long as this struct, as the drop impl will drop the references
                        unsafe { std::mem::transmute(boxed) };
                    mtx.try_send(boxed).unwrap();
                    Poll::Pending
                } else {
                    unreachable!()
                }
            }
            AccessFutureState::WaitingForCompletion(rx, _) => {
                let future = rx.recv();
                pin!(future);
                future.poll(cx).map(|v| v.unwrap())
            }
        }
    }
}

impl<'env, R> Drop for AccessFuture<'env, R> {
    fn drop(&mut self) {
        if let AccessFutureState::WaitingForCompletion(_, arc) = &self.state {
            *arc.lock() = None;
        }
    }
}

pub struct ExclusiveAccessorRunnerSystem {
    channel: Receiver<Box<dyn GenericAccess>>,
    name: Cow<'static, str>,
    id: SystemId,
}

impl ExclusiveSystem for ExclusiveAccessorRunnerSystem {
    fn name(&self) -> Cow<'static, str> {
        self.name.clone()
    }

    fn id(&self) -> SystemId {
        self.id
    }

    fn run(&mut self, world: &mut World) {
        loop {
            match self.channel.try_recv() {
                Ok(sync) => sync.run(world),
                Err(async_channel::TryRecvError::Closed) => panic!(),
                Err(async_channel::TryRecvError::Empty) => break,
            }
        }
    }

    fn initialize(&mut self, _: &mut World) {}
}

pub trait ExclusiveAccessSystemsTuple: Send + Sync + 'static {
    fn initialize(&mut self, world: &mut World);
    fn run(&mut self, world: &mut World);
}

pub struct ExclusiveAsyncChainSystem<Systems> {
    pub(super) inner_systems: Systems,
    pub(super) handle: AsyncSystemHandle<(), ()>,
    pub(super) return_handle: Option<AsyncSystemOutput<()>>,
    pub(super) name: Cow<'static, str>,
    pub(super) id: SystemId,
    pub(super) startup_future:
        Option<Box<dyn FnOnce(TaskPool) -> BoxedFuture<'static, ()> + Send + Sync + 'static>>,
}

impl<Systems: ExclusiveAccessSystemsTuple> ExclusiveSystem for ExclusiveAsyncChainSystem<Systems> {
    fn name(&self) -> Cow<'static, str> {
        self.name.clone()
    }

    fn id(&self) -> SystemId {
        self.id
    }

    fn initialize(&mut self, world: &mut World) {
        if let Some(fut) = self.startup_future.take() {
            let tp = world.get_resource_mut::<AsyncComputeTaskPool>().unwrap();
            tp.spawn((fut)(tp.clone().0)).detach();
        }
        self.inner_systems.initialize(world);
    }

    fn run(&mut self, world: &mut World) {
        self.inner_systems.run(world);
        if let Some(ref mut handle) = &mut self.return_handle {
            match handle.get() {
                Ok(_) => {
                    self.return_handle = Some(self.handle.fire(()));
                }
                Err(AsyncSystemOutputError::SystemNotFinished) => (),
                Err(AsyncSystemOutputError::OutputMoved) => panic!(),
            }
        } else {
            self.return_handle = Some(self.handle.fire(()));
        }
    }
}
