use legion::{
    filter::EntityFilter,
    prelude::{
        into_resource_system, IntoQuery, ResourceSet, Resources, Runnable,
        Schedulable, World, into_resource_for_each_system,
    },
    query::{DefaultFilter, View},
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

impl System {
    pub fn resource_for_each<'a, Q, F, R, X>(name: &'static str, system: F) -> Self
    where
        Q: IntoQuery + DefaultFilter<Filter = R>,
        <Q as View<'a>>::Iter: Iterator<Item = Q> + 'a,
        F: FnMut(&mut X, Q) + Send + Sync + 'static,
        R: EntityFilter + Sync + 'static,
        X: ResourceSet<PreparedResources = X> + 'static,
    {
        into_resource_for_each_system(name, system).into()
    }

    pub fn resource<'a, F, X>(name: &'static str, system: F) -> Self
    where
        F: FnMut(&mut X) + Send + Sync + 'static,
        X: ResourceSet<PreparedResources = X> + 'static,
    {
        into_resource_system(name, system).into()
    }
}
