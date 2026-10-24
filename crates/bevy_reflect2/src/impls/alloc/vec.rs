use crate::{kind::impl_reflect_list, ListAccess, Reflect, Typed};
use alloc::{boxed::Box, vec::Vec};

// SAFETY: every item is a `T`, and inserted values are type checked
unsafe impl<T: Typed> ListAccess for Vec<T> {
    fn len(&self) -> usize {
        Vec::len(self)
    }

    fn get(&self, index: usize) -> Option<&dyn Reflect> {
        <[T]>::get(self, index).map(|value| value as &dyn Reflect)
    }

    fn get_mut(&mut self, index: usize) -> Option<&mut dyn Reflect> {
        <[T]>::get_mut(self, index).map(|value| value as &mut dyn Reflect)
    }

    fn insert(&mut self, index: usize, value: Box<dyn Reflect>) -> Result<(), Box<dyn Reflect>> {
        Vec::insert(self, index, value.take::<T>()?);
        Ok(())
    }

    fn push(&mut self, value: Box<dyn Reflect>) -> Result<(), Box<dyn Reflect>> {
        Vec::push(self, value.take::<T>()?);
        Ok(())
    }

    fn remove(&mut self, index: usize) -> Option<Box<dyn Reflect>> {
        (index < Vec::len(self)).then(|| Box::new(Vec::remove(self, index)) as Box<dyn Reflect>)
    }

    fn pop(&mut self) -> Option<Box<dyn Reflect>> {
        Vec::pop(self).map(|value| Box::new(value) as Box<dyn Reflect>)
    }

    fn clear(&mut self) {
        Vec::clear(self);
    }
}

impl_reflect_list!(
    impl<T> for Vec<T>,
    type_path: ("alloc::vec", "Vec"),
    item: T
);

#[cfg(test)]
mod tests {
    use crate::{Kind, Reflect, TypeRegistry, Typed};
    use alloc::{boxed::Box, format, string::String, vec, vec::Vec};
    use core::any::TypeId;

    #[test]
    fn vec_type_info() {
        assert_eq!(Vec::<u32>::TYPE.type_path().full(), "alloc::vec::Vec<u32>");
        assert_eq!(Vec::<u32>::TYPE.type_path().short(), "Vec<u32>");
        assert_eq!(Vec::<u32>::TYPE.type_path().type_ident(), "Vec");
        assert_eq!(
            Vec::<u32>::TYPE.type_path().module_path(),
            Some("alloc::vec")
        );
        assert_eq!(Vec::<u32>::TYPE.type_path().crate_name(), Some("alloc"));
        assert_eq!(
            Vec::<Vec<String>>::TYPE.type_path().short(),
            "Vec<Vec<String>>"
        );
        let Kind::List(list_type) = Vec::<u32>::TYPE.kind() else {
            panic!("expected list kind");
        };
        assert_eq!(list_type.item().get().type_id(), TypeId::of::<u32>());
    }

    #[test]
    fn vec_access() {
        let mut value = vec![1u32, 2, 3];
        let reflect = &mut value as &mut dyn Reflect;
        {
            let list = reflect.list_ref().unwrap();
            assert_eq!(list.len(), 3);
            assert_eq!(list.get(0).unwrap().downcast_ref::<u32>(), Some(&1));
            assert!(list.get(3).is_none());
            let items = list
                .iter()
                .map(|item| *item.downcast_ref::<u32>().unwrap())
                .collect::<Vec<_>>();
            assert_eq!(items, [1, 2, 3]);
        }
        {
            let mut list = reflect.list_mut().unwrap();
            list.push(Box::new(4u32)).unwrap();
            let rejected = list.push(Box::new(5u64)).unwrap_err();
            assert_eq!(rejected.downcast_ref::<u64>(), Some(&5));
            list.insert(0, Box::new(0u32)).unwrap();
            *list.get_mut(1).unwrap().downcast_mut::<u32>().unwrap() = 10;
            let removed = list.remove(2).unwrap();
            assert_eq!(removed.downcast_ref::<u32>(), Some(&2));
            assert!(list.remove(10).is_none());
            let popped = list.pop().unwrap();
            assert_eq!(popped.downcast_ref::<u32>(), Some(&4));
        }
        assert_eq!(value, [0, 10, 3]);
        (&mut value as &mut dyn Reflect).list_mut().unwrap().clear();
        assert!(value.is_empty());
    }

    #[test]
    fn vec_builtins() {
        let a = &vec![1, 2, 3] as &dyn Reflect;
        let b = &vec![1, 2, 3] as &dyn Reflect;
        let c = &vec![3, 2, 1] as &dyn Reflect;
        assert_eq!(a.reflect_partial_eq(b), Some(true));
        assert_eq!(a.reflect_partial_eq(c), Some(false));
        assert_eq!(a.reflect_partial_eq(&vec![1, 2]), Some(false));
        assert_eq!(a.reflect_partial_cmp(c), Some(core::cmp::Ordering::Less));
        assert_eq!(
            a.reflect_partial_cmp(&vec![1, 2]),
            Some(core::cmp::Ordering::Greater)
        );
        assert_eq!(a.reflect_hash(), b.reflect_hash());
        assert_ne!(a.reflect_hash(), c.reflect_hash());
        assert_eq!(format!("{a:?}"), "[1, 2, 3]");
        let clone = a.reflect_clone().unwrap();
        assert_eq!(clone.downcast_ref::<Vec<i32>>(), Some(&vec![1, 2, 3]));
        let default = Vec::<i32>::TYPE.default().unwrap();
        assert_eq!(default.downcast_ref::<Vec<i32>>(), Some(&Vec::new()));
        // Floats can't be hashed
        assert!((&vec![1.0f32] as &dyn Reflect).reflect_hash().is_none());
    }

    #[test]
    fn vec_registration() {
        let mut registry = TypeRegistry::empty();
        registry.register::<Vec<Vec<u32>>>();
        assert!(registry.contains(TypeId::of::<Vec<Vec<u32>>>()));
        assert!(registry.contains(TypeId::of::<Vec<u32>>()));
        assert!(registry.contains(TypeId::of::<u32>()));
        assert!(registry
            .get_with_type_path("alloc::vec::Vec<u32>")
            .is_some());
    }
}
