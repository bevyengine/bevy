// structs containing wgpu types take a long time to compile. this is particularly bad for generic
// structs containing wgpu structs. we avoid that in debug builds (and for cargo check and rust analyzer)
// by boxing and type-erasing with the `render_resource_wrapper` macro.
// analysis from https://github.com/bevyengine/bevy/pull/5950#issuecomment-1243473071 indicates this is
// due to `evaluate_obligations`. we should check if this can be removed after a fix lands for
// https://github.com/rust-lang/rust/issues/99188 (and after other `evaluate_obligations`-related changes).
#[cfg(debug_assertions)]
#[macro_export]
macro_rules! render_resource_wrapper {
    ($wrapper_type:ident, $wgpu_type:ty) => {
        #[derive(Clone, Debug)]
        pub struct $wrapper_type(Option<std::sync::Arc<Box<()>>>);

        impl $wrapper_type {
            pub fn new(value: $wgpu_type) -> Self {
                unsafe {
                    Self(Some(std::sync::Arc::new(std::mem::transmute(Box::new(
                        value,
                    )))))
                }
            }

            pub fn try_unwrap(mut self) -> Option<$wgpu_type> {
                let inner = self.0.take();
                if let Some(inner) = inner {
                    match std::sync::Arc::try_unwrap(inner) {
                        Ok(untyped_box) => {
                            let typed_box = unsafe {
                                std::mem::transmute::<Box<()>, Box<$wgpu_type>>(untyped_box)
                            };
                            Some(*typed_box)
                        }
                        Err(inner) => {
                            let _ = unsafe {
                                std::mem::transmute::<
                                    std::sync::Arc<Box<()>>,
                                    std::sync::Arc<Box<$wgpu_type>>,
                                >(inner)
                            };
                            None
                        }
                    }
                } else {
                    None
                }
            }
        }

        impl std::ops::Deref for $wrapper_type {
            type Target = $wgpu_type;

            fn deref(&self) -> &Self::Target {
                let untyped_box = self
                    .0
                    .as_ref()
                    .expect("render_resource_wrapper inner value has already been taken (via drop or try_unwrap")
                    .as_ref();

                let typed_box =
                    unsafe { std::mem::transmute::<&Box<()>, &Box<$wgpu_type>>(untyped_box) };
                typed_box.as_ref()
            }
        }

        impl Drop for $wrapper_type {
            fn drop(&mut self) {
                let inner = self.0.take();
                if let Some(inner) = inner {
                    let _ = unsafe {
                        std::mem::transmute::<
                            std::sync::Arc<Box<()>>,
                            std::sync::Arc<Box<$wgpu_type>>,
                        >(inner)
                    };
                }
            }
        }

        // Arc<Box<()>> and Arc<()> will be Sync and Send even when $wgpu_type is not Sync or Send.
        // We ensure correctness by checking that $wgpu_type does implement Send and Sync.
        // If in future there is a case where a wrapper is required for a non-send/sync type
        // we can implement a macro variant that also does `impl !Send for $wrapper_type {}` and
        // `impl !Sync for $wrapper_type {}`
        const _: () = {
            trait AssertSendSyncBound: Send + Sync {}
            impl AssertSendSyncBound for $wgpu_type {}
        };
    };
}

#[cfg(not(debug_assertions))]
#[macro_export]
macro_rules! render_resource_wrapper {
    ($wrapper_type:ident, $wgpu_type:ty) => {
        #[derive(Clone, Debug)]
        pub struct $wrapper_type(std::sync::Arc<$wgpu_type>);

        impl $wrapper_type {
            pub fn new(value: $wgpu_type) -> Self {
                Self(std::sync::Arc::new(value))
            }

            pub fn try_unwrap(self) -> Option<$wgpu_type> {
                std::sync::Arc::try_unwrap(self.0).ok()
            }
        }

        impl std::ops::Deref for $wrapper_type {
            type Target = $wgpu_type;

            fn deref(&self) -> &Self::Target {
                self.0.as_ref()
            }
        }

        const _: () = {
            trait AssertSendSyncBound: Send + Sync {}
            impl AssertSendSyncBound for $wgpu_type {}
        };
    };
}

pub use render_resource_wrapper;
