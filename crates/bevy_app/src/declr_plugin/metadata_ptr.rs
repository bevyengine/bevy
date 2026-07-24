use core::{
    alloc::Layout,
    any::TypeId,
    ptr::NonNull,
};
use std::{
    alloc::{alloc, dealloc},
    boxed::Box,
};

/// Fully type erased pointer that owns the data, knows the layout, knows the
/// [`TypeId`], and holds onto a copy of the drop implementation.
pub(crate) struct MetadataPtr {
    layout: Layout,
    ptr: NonNull<()>,
    drop_fn: Box<dyn Fn(NonNull<()>, Layout)>,
    type_id: TypeId,
    already_dropped: bool,
}

#[allow(unsafe_code)]
impl MetadataPtr {
    pub fn new<T: Sized + 'static>(data: T) -> Option<Self> {
        let layout = Layout::for_value(&data);
        // SAFETY: Initialization happens in the next unsafe block, there's no
        // branching other than null pointer checking before then. Null pointers
        // cannot be deallocated.
        let ptr = unsafe { alloc(layout) };
        let ptr = NonNull::new(ptr)?.cast();
        // SAFETY: This uses a Layout derived from T
        unsafe { ptr.write(data) };

        Some(MetadataPtr {
            layout,
            ptr: ptr.cast(),
            drop_fn: Box::new(|ptr, layout| {
                // SAFETY: this function is only ever passed the original layout.
                let data: T = unsafe { Self::move_then_deallocate(ptr.cast(), layout) };
                drop(data);
            }),
            type_id: TypeId::of::<T>(),
            already_dropped: false,
        })
    }

    pub fn try_reverse_erase<T: Sized + 'static>(mut self) -> Result<T, Self> {
        let layout = Layout::new::<T>();
        let type_id = TypeId::of::<T>();
        if layout == self.layout && type_id == self.type_id && !self.already_dropped {
            // SAFETY: we at least know if the data is the right shape and the type IDs are the same.
            let data: NonNull<T> = self.ptr.cast();
            // SAFETY: We are passing the original layout this type was constructed with.
            if self.layout.size() != 0 {
                self.already_dropped = true;
                Ok(unsafe { Self::move_then_deallocate(data, self.layout) })
            } else {
                Err(self)
            }
        } else {
            Err(self)
        }
    }

    pub fn visit<T: Sized + 'static, Y>(&self, peek: impl for<'b> Fn(&'b T) -> Y) -> Option<Y> {
        let layout = Layout::new::<T>();
        let type_id = TypeId::of::<T>();
        if layout == self.layout && type_id == self.type_id && !self.already_dropped {
            Some(peek(unsafe { self.ptr.cast().as_ref() }))
        } else {
            None
        }
    }

    /// SAFETY: The layout passed must be the same as what `ptr` was allocated with.
    unsafe fn move_then_deallocate<T>(ptr: NonNull<T>, layout: Layout) -> T {
        // SAFETY: we deallocate immediately after.
        let data_read = unsafe { ptr.read() };
        // SAFETY: the data is read to the stack already, we can free the ptr
        unsafe { dealloc(ptr.cast::<u8>().as_ptr(), layout) };
        data_read
    }

    pub(crate) fn inner_type_id(&self) -> TypeId {
        self.type_id
    }
}

impl Drop for MetadataPtr {
    fn drop(&mut self) {
        if !self.already_dropped {
            (self.drop_fn)(self.ptr, self.layout);
        }
    }
}

#[cfg(test)]
mod metadata_ptr_test {
    use super::MetadataPtr;
    use std::vec::Vec;

    #[test]
    fn basic() {
        let mut v: Vec<u8> = Vec::new();
        v.push(1);
        v.push(2);
        v.push(3);
        v.push(4);
        let erased = MetadataPtr::new(v.clone()).unwrap();
        let visit_res = erased.visit::<Vec<u8>, _>(|v| (v.len(), v.iter().fold(0, |a, b| a + b)));
        assert_eq!(Some((4, 10)), visit_res);
        let visit_res = erased.visit::<Vec<u8>, _>(|v| (v.len(), v.iter().fold(0, |a, b| a + b)));
        assert_eq!(Some((4, 10)), visit_res);
        let visit_none = erased.visit::<Vec<u16>, _>(|v| v.len());
        assert_eq!(None, visit_none);
        let _visit_take_ref = erased.visit::<Vec<u8>, _>(|v| v.len());
        let _un_erased = erased.try_reverse_erase::<Vec<u8>>();
    }

    #[test]
    fn nested() {
        let mut v: Vec<u8> = Vec::new();
        v.push(1);
        v.push(2);
        v.push(3);
        v.push(4);
        let nested = MetadataPtr::new(MetadataPtr::new(v).unwrap()).unwrap();
        let double_visit = nested
            .visit::<MetadataPtr, _>(|single| single.visit::<Vec<u8>, _>(|v| v.len()))
            .flatten();
        assert_eq!(double_visit, Some(4));
        let mut extremely_nested = nested;
        for _ in 0..1000 {
            extremely_nested = MetadataPtr::new(extremely_nested).unwrap();
        }
        let mut sum = 0;
        while let Ok(inner_nested) = extremely_nested.try_reverse_erase::<MetadataPtr>() {
            sum += 1;
            extremely_nested = inner_nested;
        }
        assert!(sum > 1000, "{sum}");
    }
}
