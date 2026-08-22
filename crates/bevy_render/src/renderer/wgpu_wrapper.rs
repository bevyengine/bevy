#[cfg(not(all(target_arch = "wasm32", target_feature = "atomics")))]
macro_rules! wgpu_wrapper {
    ($( $(#[$($attrs:tt)*])* $vis:vis struct $name:ident ($wgputy:ty) );+ $(;)?) => {
        $(
            $( #[$($attrs)*] )*
            $vis struct $name ($wgputy);

            impl $name {
                /// Constructs a new instance of `WgpuWrapper` which will wrap the specified value.
                pub fn new(t: $wgputy) -> Self {
                    Self(t)
                }
            }

            const _: () = {
                const fn assert_sync_send<T: Sync + Send>() {}
                assert_sync_send::<$wgputy>()
            };

            // SAFETY: We just asserted that $wgputy is Send and Sync
            #[expect(unsafe_code, reason = "Blanket-impl Send requires unsafe.")]
            unsafe impl Send for $name {}
            // SAFETY: We just asserted that $wgputy is Send and Sync
            #[expect(unsafe_code, reason = "Blanket-impl Send requires unsafe.")]
            unsafe impl Sync for $name {}

            impl ::core::ops::Deref for $name {
                type Target = $wgputy;

                fn deref(&self) -> &Self::Target {
                    &self.0
                }
            }

            impl ::core::ops::DerefMut for $name {
                fn deref_mut(&mut self) -> &mut Self::Target {
                    &mut self.0
                }
            }

            impl Into<$wgputy> for $name {
                fn into(self) -> $wgputy {
                    self.0
                }
            }
        )+
    };
}

#[cfg(all(target_arch = "wasm32", target_feature = "atomics"))]
macro_rules! wgpu_wrapper {
    ($( $(#[$($attrs:tt)*])* $vis:vis struct $name:ident ($wgputy:ty) );+ $(;)?) => {
        $(
            $( #[$($attrs)*] )*
            $vis struct $name (send_wrapper::SendWrapper<$wgputy>);

            impl $name {
                /// Constructs a new instance of `WgpuWrapper` which will wrap the specified value.
                pub fn new(t: $wgputy) -> Self {
                    Self(send_wrapper::SendWrapper::new(t))
                }
            }

            impl ::core::ops::Deref for $name {
                type Target = $wgputy;

                fn deref(&self) -> &Self::Target {
                    &self.0
                }
            }

            impl ::core::ops::DerefMut for $name {
                fn deref_mut(&mut self) -> &mut Self::Target {
                    &mut self.0
                }
            }

            impl Into<$wgputy> for $name {
                fn into(self) -> $wgputy {
                    self.0.take()
                }
            }
        )+
    };
}

pub(crate) use wgpu_wrapper;
