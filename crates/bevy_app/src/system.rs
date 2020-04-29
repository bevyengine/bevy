use legion::{
    prelude::{
        Resources,
        Runnable, Schedulable, World,
    },
};
pub enum System {
    Schedulable(Box<dyn Schedulable>),
    ThreadLocal(Box<dyn Runnable>),
    ThreadLocalFn(Box<dyn FnMut(&mut World, &mut Resources)>),
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
        System::ThreadLocalFn(Box::new(system))
    }
}