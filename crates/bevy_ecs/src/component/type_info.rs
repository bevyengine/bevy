use std::alloc::Layout;

/// Metadata required to store a component.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TypeInfo {
    layout: Layout,
    drop: unsafe fn(*mut u8),
    type_name: &'static str,
    // SAFETY: This must remain private. It must only be set to "true" if this type is actually
    // Send + Sync
    is_send_and_sync: bool,
}

impl TypeInfo {
    /// Metadata for `T`.
    pub fn of<T: Send + Sync + 'static>() -> Self {
        Self {
            layout: Layout::new::<T>(),
            is_send_and_sync: true,
            drop: Self::drop_ptr::<T>,
            type_name: core::any::type_name::<T>(),
        }
    }

    pub fn of_non_send_and_sync<T: 'static>() -> Self {
        Self {
            layout: Layout::new::<T>(),
            is_send_and_sync: false,
            drop: Self::drop_ptr::<T>,
            type_name: core::any::type_name::<T>(),
        }
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

    pub(super) unsafe fn drop_ptr<T>(x: *mut u8) {
        x.cast::<T>().drop_in_place()
    }
}
