use legion::prelude::{Resources, Runnable, Schedulable, World};
use std::borrow::Cow;
pub enum System {
    Schedulable(Box<dyn Schedulable>),
    ThreadLocal(Box<dyn Runnable>),
    ThreadLocalFn((&'static str, Box<dyn FnMut(&mut World, &mut Resources)>)),
}

impl System {
    pub fn name(&self) -> Cow<'static, str> {
        match *self {
            System::Schedulable(ref schedulable) => schedulable.name().name(),
            System::ThreadLocal(ref runnable) => runnable.name().name(),
            System::ThreadLocalFn((ref name, ref _thread_local_fn)) => Cow::Borrowed(name),
        }
    }
}

impl From<Box<dyn Schedulable>> for System {
    fn from(system: Box<dyn Schedulable>) -> Self {
        System::Schedulable(system)
    }
}

impl From<Box<dyn Runnable>> for System {
    fn from(system: Box<dyn Runnable>) -> Self {
        System::ThreadLocal(system)
    }
}

impl<T> From<T> for System
where
    T: FnMut(&mut World, &mut Resources) + 'static,
{
    fn from(system: T) -> Self {
        System::ThreadLocalFn((std::any::type_name::<T>(), Box::new(system)))
    }
}
