use legion::prelude::Resources;

pub trait FromResources {
    fn from_resources(resources: &Resources) -> Self;
}

impl<T> FromResources for T
where
    T: Default,
{
    default fn from_resources(_resources: &Resources) -> Self {
        Self::default()
    }
}