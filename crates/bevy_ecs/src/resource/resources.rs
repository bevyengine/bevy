use super::{FetchResource, Res, ResMut, ResourceQuery};
use crate::system::SystemId;
use core::any::TypeId;
use hecs::{Archetype, ComponentError, Ref, RefMut, TypeInfo};
use std::collections::HashMap;

pub trait Resource: Send + Sync + 'static {}
impl<T: Send + Sync + 'static> Resource for T {}

#[derive(Default)]
pub struct Resources {
    pub(crate) resource_archetypes: HashMap<TypeId, Archetype>,
    pub(crate) system_id_to_archetype_index: HashMap<u32, u32>,
}

impl Resources {
    pub fn insert<T: Resource>(&mut self, resource: T) {
        self.insert_index(resource, 0);
    }

    pub fn contains<T: Resource>(&self) -> bool {
        self.get_index::<T>(0).is_ok()
    }

    pub fn get<T: Resource>(&self) -> Result<Ref<'_, T>, ComponentError> {
        self.get_index(0)
    }

    pub fn get_mut<T: Resource>(&self) -> Result<RefMut<'_, T>, ComponentError> {
        self.get_index_mut(0)
    }

    pub fn get_local<'a, T: Resource>(
        &'a self,
        id: SystemId,
    ) -> Result<Ref<'a, T>, ComponentError> {
        self.system_id_to_archetype_index
            .get(&id.0)
            .ok_or_else(|| ComponentError::NoSuchEntity)
            .and_then(|index| self.get_index(*index))
    }

    pub fn get_local_mut<'a, T: Resource>(
        &'a self,
        id: SystemId,
    ) -> Result<RefMut<'a, T>, ComponentError> {
        self.system_id_to_archetype_index
            .get(&id.0)
            .ok_or_else(|| ComponentError::NoSuchEntity)
            .and_then(|index| self.get_index_mut(*index))
    }

    pub fn insert_local<T: Resource>(&mut self, id: SystemId, resource: T) {
        if let Some(index) = self.system_id_to_archetype_index.get(&id.0).cloned() {
            self.insert_index(resource, index);
        } else {
            let mut index = self.archetype_len::<T>();
            // index 0 is reserved for the global non-system resource
            if index == 0 {
                self.allocate_next::<T>();
                index += 1;
            }
            self.insert_index(resource, index);
            self.system_id_to_archetype_index.insert(id.0, index);
        }
    }

    fn insert_index<T: Resource>(&mut self, mut resource: T, index: u32) {
        let type_id = TypeId::of::<T>();
        let archetype = self.resource_archetypes.entry(type_id).or_insert_with(|| {
            let mut types = Vec::new();
            types.push(TypeInfo::of::<T>());
            Archetype::new(types)
        });

        if index == archetype.len() {
            unsafe { archetype.allocate(index) };
        } else if index > archetype.len() {
            panic!("attempted to access index beyond 'current_capacity + 1'")
        }

        unsafe {
            let resource_ptr = (&mut resource as *mut T).cast::<u8>();
            archetype.put_dynamic(resource_ptr, type_id, core::mem::size_of::<T>(), index);
            std::mem::forget(resource);
        }
    }

    fn allocate_next<T: Resource>(&mut self) {
        let type_id = TypeId::of::<T>();
        let archetype = self.resource_archetypes.entry(type_id).or_insert_with(|| {
            let mut types = Vec::new();
            types.push(TypeInfo::of::<T>());
            Archetype::new(types)
        });

        let index = archetype.len();
        unsafe { archetype.allocate(index) };
    }

    fn get_index<T: Resource>(&self, index: u32) -> Result<Ref<'_, T>, ComponentError> {
        self.resource_archetypes
            .get(&TypeId::of::<T>())
            .ok_or_else(|| ComponentError::NoSuchEntity)
            .and_then(|archetype| unsafe {
                Ref::new(archetype, index).map_err(|err| ComponentError::MissingComponent(err))
            })
    }

    fn get_index_mut<T: Resource>(&self, index: u32) -> Result<RefMut<'_, T>, ComponentError> {
        self.resource_archetypes
            .get(&TypeId::of::<T>())
            .ok_or_else(|| ComponentError::NoSuchEntity)
            .and_then(|archetype| unsafe {
                RefMut::new(archetype, index).map_err(|err| ComponentError::MissingComponent(err))
            })
    }

    fn archetype_len<T: Resource>(&self) -> u32 {
        self.resource_archetypes
            .get(&TypeId::of::<T>())
            .map_or(0, |a| a.len())
    }

    pub fn query<Q: ResourceQuery>(&self) -> <Q::Fetch as FetchResource>::Item {
        unsafe { Q::Fetch::get(&self, None) }
    }

    pub fn query_system<Q: ResourceQuery>(
        &self,
        id: SystemId,
    ) -> <Q::Fetch as FetchResource>::Item {
        unsafe { Q::Fetch::get(&self, Some(id)) }
    }

    #[inline]
    pub unsafe fn get_res<T: Resource>(&self) -> Res<'_, T> {
        let archetype = self
            .resource_archetypes
            .get(&TypeId::of::<T>())
            .unwrap_or_else(|| panic!("Resource does not exist {}", std::any::type_name::<T>()));
        Res::new(archetype, 0).expect("Resource does not exist")
    }

    #[inline]
    pub unsafe fn get_res_mut<T: Resource>(&self) -> ResMut<'_, T> {
        let archetype = self
            .resource_archetypes
            .get(&TypeId::of::<T>())
            .unwrap_or_else(|| panic!("Resource does not exist {}", std::any::type_name::<T>()));
        ResMut::new(archetype, 0).expect("Resource does not exist")
    }
}

unsafe impl Send for Resources {}
unsafe impl Sync for Resources {}

pub trait FromResources {
    fn from_resources(resources: &Resources) -> Self;
}

impl<T> FromResources for T
where
    T: Default,
{
    fn from_resources(_resources: &Resources) -> Self {
        Self::default()
    }
}

#[cfg(test)]
mod tests {
    use super::Resources;
    use crate::system::SystemId;

    #[test]
    fn resource() {
        let mut resources = Resources::default();
        assert!(resources.get::<i32>().is_err());

        resources.insert(123);
        assert_eq!(*resources.get::<i32>().expect("resource exists"), 123);

        resources.insert(456.0);
        assert_eq!(*resources.get::<f64>().expect("resource exists"), 456.0);

        resources.insert(789.0);
        assert_eq!(*resources.get::<f64>().expect("resource exists"), 789.0);

        {
            let mut value = resources.get_mut::<f64>().expect("resource exists");
            assert_eq!(*value, 789.0);
            *value = -1.0;
        }

        assert_eq!(*resources.get::<f64>().expect("resource exists"), -1.0);

        assert!(resources.get_local::<i32>(SystemId(0)).is_err());
        resources.insert_local(SystemId(0), 111);
        assert_eq!(
            *resources
                .get_local::<i32>(SystemId(0))
                .expect("resource exists"),
            111
        );
        assert_eq!(*resources.get::<i32>().expect("resource exists"), 123);
        resources.insert_local(SystemId(0), 222);
        assert_eq!(
            *resources
                .get_local::<i32>(SystemId(0))
                .expect("resource exists"),
            222
        );
        assert_eq!(*resources.get::<i32>().expect("resource exists"), 123);
    }

    #[test]
    #[should_panic(expected = "i32 already borrowed")]
    fn resource_double_mut_panic() {
        let mut resources = Resources::default();
        resources.insert(123);
        let _x = resources.get_mut::<i32>();
        let _y = resources.get_mut::<i32>();
    }
}
