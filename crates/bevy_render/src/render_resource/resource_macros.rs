#[cfg(debug_assertions)]
use std::sync::Arc;

#[cfg(debug_assertions)]
#[derive(Clone, Debug)]
pub struct BlackBox(Option<Arc<Box<()>>>);

#[cfg(debug_assertions)]
impl BlackBox {
    pub fn new<T>(value: T) -> Self {
        unsafe { Self(Some(Arc::new(std::mem::transmute(Box::new(value))))) }
    }

    pub unsafe fn typed_ref<T>(&self) -> &T {
        let untyped_box = self
            .0
            .as_ref()
            .expect("BlackBox inner value has already been taken (via drop or try_unwrap")
            .as_ref();

        let typed_box = std::mem::transmute::<&Box<()>, &Box<T>>(untyped_box);
        typed_box.as_ref()
    }

    pub unsafe fn try_unwrap<T>(mut self) -> Option<T> {
        let inner = self.0.take();
        if let Some(inner) = inner {
            match Arc::try_unwrap(inner) {
                Ok(untyped_box) => {
                    let typed_box = std::mem::transmute::<Box<()>, Box<T>>(untyped_box);
                    Some(*typed_box)
                }
                Err(inner) => {
                    let _ = std::mem::transmute::<Arc<Box<()>>, Arc<Box<T>>>(inner);
                    None
                }
            }
        } else {
            None
        }
    }
    pub unsafe fn drop_inner<T>(&mut self) {
        let inner = self.0.take();
        if let Some(inner) = inner {
            let _ = std::mem::transmute::<Arc<Box<()>>, Arc<Box<T>>>(inner);
        }
    }
}

#[cfg(debug_assertions)]
impl Drop for BlackBox {
    fn drop(&mut self) {
        if let Some(inner) = &self.0 {
            if Arc::strong_count(&inner) == 1 {
                panic!("undropped inner");
            }
        }
    }
}

#[cfg(debug_assertions)]
#[macro_export]
macro_rules! render_resource_type {
    ($wgpu_type:ty) => {
        BlackBox
    };
}
#[cfg(debug_assertions)]
#[macro_export]
macro_rules! render_resource_ref {
    ($value:expr, $wgpu_type:ty) => {
        unsafe { $value.typed_ref::<$wgpu_type>() }
    };
}
#[cfg(debug_assertions)]
#[macro_export]
macro_rules! render_resource_new {
    ($value:expr) => {
        BlackBox::new($value)
    };
}
#[cfg(debug_assertions)]
#[macro_export]
macro_rules! render_resource_drop {
    ($value:expr, $wgpu_type:ty) => {
        unsafe {
            $value.drop_inner::<$wgpu_type>();
        }
    };
}
#[cfg(debug_assertions)]
#[macro_export]
macro_rules! render_resource_try_unwrap {
    ($value:expr, $wgpu_type:ty) => {{
        unsafe { $value.try_unwrap::<$wgpu_type>() }
    }};
}

#[cfg(not(debug_assertions))]
#[macro_export]
macro_rules! render_resource_type {
    ($wgpu_type:ty) => {
        std::sync::Arc<$wgpu_type>
    }
}
#[cfg(not(debug_assertions))]
#[macro_export]
macro_rules! render_resource_ref {
    ($value:expr, $wgpu_type:ty) => {
        $value
    };
}
#[cfg(not(debug_assertions))]
#[macro_export]
macro_rules! render_resource_new {
    ($value:expr) => {
        std::sync::Arc::new($value)
    };
}
#[cfg(not(debug_assertions))]
#[macro_export]
macro_rules! render_resource_drop {
    ($value:expr, $wgpu_type:ty) => {
        let _ = $value;
    };
}
#[cfg(not(debug_assertions))]
#[macro_export]
macro_rules! render_resource_try_unwrap {
    ($value:expr, $wgpu_type:ty) => {
        std::sync::Arc::try_unwrap($value).ok()
    };
}

pub use render_resource_drop;
pub use render_resource_new;
pub use render_resource_ref;
pub use render_resource_try_unwrap;
pub use render_resource_type;
