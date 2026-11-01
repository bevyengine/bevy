#[cfg(not(all(target_arch = "wasm32", target_feature = "atomics")))]
use wgpu::{
    Adapter, AdapterInfo, BindGroup, BindGroupLayout, Buffer, CommandBuffer, CommandEncoder,
    ComputePipeline, Device, Error, ErrorSource, Instance, PipelineCache, PipelineLayout, Queue,
    RenderPipeline, Sampler, ShaderModule, Surface, SurfaceTexture, Texture, TextureView,
};

/// Marker for types that are `Send + Sync` on every target where [`wgpu_wrapper!`]
/// asserts this bound, i.e. everywhere except wasm with atomics.
///
/// The trait deliberately has no `Send + Sync` supertraits: requiring them here
/// would reintroduce the deep auto-trait proof at every impl.
///
/// # Safety
///
/// Implementors must be `Send + Sync` on those targets. `wgpu` handles satisfy this,
/// but proving it through their auto-trait chains can exceed the trait solver's
/// recursion limit; the wrapper uses this marker to check the bound with a shallow
/// lookup.
#[cfg(not(all(target_arch = "wasm32", target_feature = "atomics")))]
pub(crate) unsafe trait WgpuSendSync {}

/// Implements [`WgpuSendSync`] for the given types.
///
/// The caller must ensure each type is `Send + Sync` on every target where
/// [`wgpu_wrapper!`] asserts the bound. A direct `Send + Sync` assertion here
/// would recreate the overflow this marker exists to avoid.
#[cfg(not(all(target_arch = "wasm32", target_feature = "atomics")))]
macro_rules! impl_wgpu_send_sync {
    ($($ty:ty),+ $(,)?) => {
        $(
            // SAFETY: the caller guarantees the type is `Send + Sync`; the
            // invocation documents why.
            unsafe impl $crate::renderer::WgpuSendSync for $ty {}
        )+
    };
}
#[cfg(not(all(target_arch = "wasm32", target_feature = "atomics")))]
pub(crate) use impl_wgpu_send_sync;

// SAFETY: every handle below is `Send + Sync` on every target where
// `wgpu_wrapper!` asserts the bound; wgpu statically asserts this. `AdapterInfo`
// is plain data and `ErrorSource` is a boxed `Send + Sync` error.
#[cfg(not(all(target_arch = "wasm32", target_feature = "atomics")))]
impl_wgpu_send_sync!(
    Adapter,
    AdapterInfo,
    BindGroup,
    BindGroupLayout,
    Buffer,
    CommandBuffer,
    CommandEncoder,
    ComputePipeline,
    Device,
    Error,
    ErrorSource,
    Instance,
    PipelineCache,
    PipelineLayout,
    Queue,
    RenderPipeline,
    Sampler,
    ShaderModule,
    Surface<'static>,
    SurfaceTexture,
    Texture,
    TextureView,
);

// `AdapterInfo` and `ErrorSource` are not wgpu handles, so prove them directly.
#[cfg(not(all(target_arch = "wasm32", target_feature = "atomics")))]
const _: () = {
    const fn assert_send_sync<T: Send + Sync>() {}
    assert_send_sync::<AdapterInfo>();
    assert_send_sync::<ErrorSource>();
};

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
            // At the type level this does effectively nothing, but it gives the trait
            // solver an explicit impl to use instead of recursing through wgpu's
            // auto-trait chain, which substantially improves compile times.
            const _: () = {
                const fn assert_send_sync<T: $crate::renderer::WgpuSendSync>() {}
                assert_send_sync::<$wgputy>()
            };
            // SAFETY: `$wgputy` implements `WgpuSendSync`, whose contract requires
            // `Send + Sync`.
            #[expect(unsafe_code, reason = "Blanket-impl Send requires unsafe.")]
            unsafe impl Send for $name {}
            // SAFETY: `$wgputy` implements `WgpuSendSync`, whose contract requires
            // `Send + Sync`.
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

                #[allow(clippy::allow_attributes, unused, reason = "This is not used on all wrappers.")]
                /// Unwraps the value.
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
        )+
    };
}

pub(crate) use wgpu_wrapper;
