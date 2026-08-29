/// A macro producing wrappers to safely make `wgpu` types Send / Sync on web with atomics enabled.
///
/// On web with `atomics` enabled the inner value can only be accessed
/// or dropped on the `wgpu` thread or else a panic will occur.
/// On other platforms the wrapper simply contains the wrapped value.
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

                #[allow(clippy::allow_attributes, unused, reason = "This is not used on all wrappers.")]
                /// Unwraps the value.
                pub fn into_inner(self) -> $wgputy {
                    self.0
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

            // Short-circuit the `Send + Sync` implementation.
            // At the type-level this does effectively nothing, however in the compiler
            // this creates a short-circuit for the trait solver that reduces recursion depth
            // and substantially improves compile times.
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
        )+
    };
}

#[cfg(all(target_arch = "wasm32", target_feature = "atomics"))]
macro_rules! wgpu_wrapper {
    ($( $(#[$($attrs:tt)*])* $vis:vis struct $name:ident ($wgputy:ty) );+ $(;)?) => {
        $(
            // On web + atomics we use SendWrapper to make the type unconditionally Send + Sync,
            // but the value can only be accessed on the `wgpu` thread or it will panic.
            // We don't need short circuits here since `SendWrapper` does it for us.
            $( #[$($attrs)*] )*
            $vis struct $name (send_wrapper::SendWrapper<$wgputy>);

            impl $name {
                /// Constructs a new instance of `WgpuWrapper` which will wrap the specified value.
                pub fn new(t: $wgputy) -> Self {
                    Self(send_wrapper::SendWrapper::new(t))
                }

                pub fn into_inner(self) -> $wgputy {
                    self.0.take()
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

            impl From<$name> for $wgputy {
                fn from(t: $name) -> Self {
                    t.0.take()
                }
            }
        )+
    };
}

pub(crate) use wgpu_wrapper;
