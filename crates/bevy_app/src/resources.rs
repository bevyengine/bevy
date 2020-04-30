use crate::{EventReader, GetEventReader};
use legion::prelude::Resources;

pub trait FromResources {
    fn from_resources(resources: &mut Resources) -> Self;
}

impl<T> FromResources for T
where
    T: Default,
{
    default fn from_resources(_resources: &mut Resources) -> Self {
        Self::default()
    }
}

impl<T> FromResources for EventReader<T> where T: Send + Sync + 'static {
    fn from_resources(resources: &mut Resources) -> Self {
        resources.get_event_reader::<T>()
    }
}
