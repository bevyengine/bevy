use std::{alloc::Layout, any::TypeId};

/// Metadata required to store a component.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TypeInfo {
    type_id: TypeId,
    layout: Layout,
    drop: unsafe fn(*mut u8),
    type_name: &'static str,
    is_send_and_sync: bool,
}

impl TypeInfo {
    /// Metadata for `T`.
    pub fn of<T: Send + Sync + 'static>() -> Self {
        Self {
            type_id: TypeId::of::<T>(),
            layout: Layout::new::<T>(),
            is_send_and_sync: true,
            drop: Self::drop_ptr::<T>,
            type_name: core::any::type_name::<T>(),
        }
    }

    pub fn of_non_send_and_sync<T: 'static>() -> Self {
        Self {
            type_id: TypeId::of::<T>(),
            layout: Layout::new::<T>(),
            is_send_and_sync: false,
            drop: Self::drop_ptr::<T>,
            type_name: core::any::type_name::<T>(),
        }
    }

    #[inline]
    pub fn type_id(&self) -> TypeId {
        self.type_id
    }

    #[inline]
    pub fn layout(&self) -> Layout {
        self.layout
    }

    #[inline]
    pub fn drop(&self) -> unsafe fn(*mut u8) {
        self.drop
    }

    #[inline]
    pub fn is_send_and_sync(&self) -> bool {
        self.is_send_and_sync
    }

    #[inline]
    pub fn type_name(&self) -> &'static str {
        self.type_name
    }

    pub(crate) unsafe fn drop_ptr<T>(x: *mut u8) {
        x.cast::<T>().drop_in_place()
    }
}
