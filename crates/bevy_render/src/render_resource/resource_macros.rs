#[cfg(debug_assertions)]
#[macro_export]
macro_rules! render_resource_type {
    ($wgpu_type:ty) => {
        Arc<Box<()>>
    }
}
#[cfg(debug_assertions)]
#[macro_export]
macro_rules! render_resource_ref {
    ($value:expr, $wgpu_type:ty) => {
        unsafe { &std::mem::transmute::<&Box<()>, &Box<$wgpu_type>>($value.as_ref()) }
    };
}
#[cfg(debug_assertions)]
#[macro_export]
macro_rules! render_resource_new {
    ($value:expr) => {
        Arc::new(unsafe { std::mem::transmute(Box::new($value)) })
    };
}
#[cfg(debug_assertions)]
#[macro_export]
macro_rules! render_resource_drop {
    ($value:expr, $wgpu_type:ty) => {
        let _counter: Arc<Box<$wgpu_type>> = unsafe { std::mem::transmute(std::mem::take($value)) };
    };
}
#[cfg(debug_assertions)]
#[macro_export]
macro_rules! render_resource_try_unwrap {
    ($value:expr, $wgpu_type:ty) => {{
        match Arc::try_unwrap($value) {
            Ok(boxed) => {
                let typed_box: Box<$wgpu_type> = unsafe { std::mem::transmute(boxed) };
                Some(*typed_box)
            }
            Err(arc) => {
                let _ = unsafe { std::mem::transmute::<_, Arc<Box<$wgpu_type>>>(arc) };
                None
            }
        }
    }};
}

#[cfg(not(debug_assertions))]
#[macro_export]
macro_rules! render_resource_type {
    ($wgpu_type:ty) => {
        Arc<$wgpu_type>
    }
}
#[cfg(not(debug_assertions))]
#[macro_export]
macro_rules! render_resource_ref {
    ($value:expr, $wgpu_type:ty) => {
        &$value
    };
}
#[cfg(not(debug_assertions))]
#[macro_export]
macro_rules! render_resource_new {
    ($value:expr) => {
        Arc::new($value)
    };
}
#[cfg(not(debug_assertions))]
#[macro_export]
macro_rules! render_resource_drop {
    ($value:expr, $wgpu_type:ty) => {};
}
#[cfg(not(debug_assertions))]
#[macro_export]
macro_rules! render_resource_try_unwrap {
    ($value:expr, $wgpu_type:ty) => {
        Arc::try_unwrap($value).ok()
    };
}

pub use render_resource_drop;
pub use render_resource_new;
pub use render_resource_ref;
pub use render_resource_try_unwrap;
pub use render_resource_type;
