use core::{any::TypeId, iter};

use bevy_ptr::{MovingPtr, OwningPtr};
use core::mem::MaybeUninit;
use variadics_please::all_tuples_enumerated;

use crate::{
    bundle::{Bundle, BundleFromComponents, DynamicBundle, NoBundleEffect},
    component::{Component, ComponentId, ComponentIdDictator, Components, StorageType},
    world::EntityWorldMut,
};

// SAFETY:
// - `Bundle::component_ids` calls `ids` for C's component id (and nothing else)
// - `Bundle::get_components` is called exactly once for C and passes the component's storage type based on its associated constant.
unsafe impl<C: Component> Bundle for C {
    fn component_ids<Components: ComponentIdDictator>(
        components: &mut Components,
    ) -> impl Iterator<Item = ComponentId> + use<C, Components> {
        iter::once(components.determine_component_id_of::<C>())
    }

    fn get_component_ids(components: &Components) -> impl Iterator<Item = Option<ComponentId>> {
        iter::once(components.get_id(TypeId::of::<C>()))
    }
}

// SAFETY:
// - `Bundle::from_components` calls `func` exactly once for C, which is the exact value returned by `Bundle::component_ids`.
unsafe impl<C: Component> BundleFromComponents for C {
    unsafe fn from_components<T, F>(ctx: &mut T, func: &mut F) -> Self
    where
        // Ensure that the `OwningPtr` is used correctly
        F: for<'a> FnMut(&'a mut T) -> OwningPtr<'a>,
        Self: Sized,
    {
        let ptr = func(ctx);
        // Safety: The id given in `component_ids` is for `Self`
        unsafe { ptr.read() }
    }
}

impl<C: Component> DynamicBundle for C {
    type Effect = ();
    #[inline]
    unsafe fn get_components(
        ptr: MovingPtr<'_, Self>,
        func: &mut impl FnMut(StorageType, OwningPtr<'_>),
    ) -> Self::Effect {
        func(C::STORAGE_TYPE, OwningPtr::from(ptr));
    }

    #[inline]
    unsafe fn apply_effect(_ptr: MovingPtr<'_, MaybeUninit<Self>>, _entity: &mut EntityWorldMut) {}
}

macro_rules! tuple_impl {
    ($(#[$meta:meta])* $(($index:tt, $name: ident, $alias: ident)),*) => {
        #[expect(
            clippy::allow_attributes,
            reason = "This is a tuple-related macro; as such, the lints below may not always apply."
        )]
        #[allow(
            unused_mut,
            unused_variables,
            reason = "Zero-length tuples won't use any of the parameters."
        )]
        $(#[$meta])*
        // SAFETY:
        // - `Bundle::component_ids` calls `ids` for each component type in the
        // bundle, in the exact order that `DynamicBundle::get_components` is called.
        // - `Bundle::from_components` calls `func` exactly once for each `ComponentId` returned by `Bundle::component_ids`.
        // - `Bundle::get_components` is called exactly once for each member. Relies on the above implementation to pass the correct
        //   `StorageType` into the callback.
        unsafe impl<$($name: Bundle),*> Bundle for ($($name,)*) {
            fn component_ids<'a, Components: ComponentIdDictator>(components: &'a mut Components) -> impl Iterator<Item = ComponentId> + use<$($name,)* Components> {
                iter::empty()$(.chain(<$name as Bundle>::component_ids(components)))*
            }

            fn get_component_ids(components: &Components) -> impl Iterator<Item = Option<ComponentId>> {
                iter::empty()$(.chain(<$name as Bundle>::get_component_ids(components)))*
            }
        }

        #[expect(
            clippy::allow_attributes,
            reason = "This is a tuple-related macro; as such, the lints below may not always apply."
        )]
        #[allow(
            unused_mut,
            unused_variables,
            reason = "Zero-length tuples won't use any of the parameters."
        )]
        $(#[$meta])*
        // SAFETY:
        // - `Bundle::component_ids` calls `ids` for each component type in the
        // bundle, in the exact order that `DynamicBundle::get_components` is called.
        // - `Bundle::from_components` calls `func` exactly once for each `ComponentId` returned by `Bundle::component_ids`.
        // - `Bundle::get_components` is called exactly once for each member. Relies on the above implementation to pass the correct
        //   `StorageType` into the callback.
        unsafe impl<$($name: BundleFromComponents),*> BundleFromComponents for ($($name,)*) {
            #[allow(
                clippy::unused_unit,
                reason = "Zero-length tuples will generate a function body equivalent to `()`; however, this macro is meant for all applicable tuples, and as such it makes no sense to rewrite it just for that case."
            )]
            unsafe fn from_components<T, F>(ctx: &mut T, func: &mut F) -> Self
            where
                F: FnMut(&mut T) -> OwningPtr<'_>
            {
                #[allow(
                    unused_unsafe,
                    reason = "Zero-length tuples will not run anything in the unsafe block. Additionally, rewriting this to move the () outside of the unsafe would require putting the safety comment inside the tuple, hurting readability of the code."
                )]
                // SAFETY: Rust guarantees that tuple calls are evaluated 'left to right'.
                // https://doc.rust-lang.org/reference/expressions.html#evaluation-order-of-operands
                unsafe { ($(<$name as BundleFromComponents>::from_components(ctx, func),)*) }
            }
        }

        #[expect(
            clippy::allow_attributes,
            reason = "This is a tuple-related macro; as such, the lints below may not always apply."
        )]
        #[allow(
            unused_mut,
            unused_variables,
            reason = "Zero-length tuples won't use any of the parameters."
        )]
        $(#[$meta])*
        impl<$($name: Bundle),*> DynamicBundle for ($($name,)*) {
            type Effect = ($($name::Effect,)*);
            #[allow(
                clippy::unused_unit,
                reason = "Zero-length tuples will generate a function body equivalent to `()`; however, this macro is meant for all applicable tuples, and as such it makes no sense to rewrite it just for that case."
            )]
            #[inline(always)]
            unsafe fn get_components(ptr: MovingPtr<'_, Self>, func: &mut impl FnMut(StorageType, OwningPtr<'_>)) {
                bevy_ptr::deconstruct_moving_ptr!({
                    let tuple { $($index: $alias,)* } = ptr;
                });
                #[allow(
                    unused_unsafe,
                    reason = "Zero-length tuples will generate a function body equivalatent to (); however, this macro is meant for all applicable tuples, and as such it makes no sense to rewrite it just for that case."
                )]
                // SAFETY: Caller ensures requirements for calling `get_components` are met.
                unsafe {
                    $( $name::get_components($alias, func); )*
                }
            }

            #[allow(
                clippy::unused_unit,
                reason = "Zero-length tuples will generate a function body equivalent to `()`; however, this macro is meant for all applicable tuples, and as such it makes no sense to rewrite it just for that case."
            )]
            #[inline(always)]
            unsafe fn apply_effect(ptr: MovingPtr<'_, MaybeUninit<Self>>, entity: &mut EntityWorldMut) {
                bevy_ptr::deconstruct_moving_ptr!({
                    let MaybeUninit::<tuple> { $($index: $alias,)* } = ptr;
                });
                #[allow(
                    unused_unsafe,
                    reason = "Zero-length tuples will generate a function body equivalent to `()`; however, this macro is meant for all applicable tuples, and as such it makes no sense to rewrite it just for that case."
                )]
                // SAFETY: Caller ensures requirements for calling `apply_effect` are met.
                unsafe {
                    $( $name::apply_effect($alias, entity); )*
                }
            }
        }

        $(#[$meta])*
        impl<$($name: NoBundleEffect),*> NoBundleEffect for ($($name,)*) {}
    }
}

all_tuples_enumerated!(
    #[doc(fake_variadic)]
    tuple_impl,
    0,
    15,
    B,
    field_
);
