use core::any::TypeId;

use bevy_ptr::{MovingPtr, OwningPtr};
use core::mem::MaybeUninit;
use variadics_please::all_tuples_enumerated;

use crate::{
    bundle::{Bundle, BundleFromComponents, DynamicBundle, NoBundleEffect},
    component::{Component, ComponentId, Components, ComponentsRegistrator, StorageType},
    query::DebugCheckedUnwrap,
    world::EntityWorldMut,
};

// SAFETY:
// - `Bundle::component_ids` calls `ids` for C's component id (and nothing else)
// - `Bundle::get_components` is called exactly once for C and passes the component's storage type based on its associated constant.
unsafe impl<C: Component> Bundle for C {
    fn component_ids(components: &mut ComponentsRegistrator, ids: &mut impl FnMut(ComponentId)) {
        ids(components.register_component::<C>());
    }

    fn get_component_ids(components: &Components, ids: &mut impl FnMut(Option<ComponentId>)) {
        ids(components.get_id(TypeId::of::<C>()));
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
            fn component_ids(components: &mut ComponentsRegistrator,  ids: &mut impl FnMut(ComponentId)){
                $(<$name as Bundle>::component_ids(components, ids);)*
            }

            fn get_component_ids(components: &Components, ids: &mut impl FnMut(Option<ComponentId>)){
                $(<$name as Bundle>::get_component_ids(components, ids);)*
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
                // SAFETY:
                // - All of the `move_field` calls all fetch distinct and valid fields within `Self`.
                // - If a field is `NoBundleEffect`, it's `apply_effect` is a no-op
                //   and cannot move any value out of an invalid instance after this call.
                // - If a field is `!NoBundleEffect`, it must be valid since a safe
                //   implementation of `DynamicBundle` only moves the value out only
                //   once between `get_components` and `apply_effect`.
                bevy_ptr::deconstruct_moving_ptr!(ptr => ($($index => $alias,)*));
                // SAFETY:
                // - If `ptr` is aligned, then field_ptr is aligned properly. Rust tuples cannot be `repr(packed)`.
                $( $name::get_components($alias.try_into().debug_checked_unwrap(), func); )*
            }

            #[allow(
                clippy::unused_unit,
                reason = "Zero-length tuples will generate a function body equivalent to `()`; however, this macro is meant for all applicable tuples, and as such it makes no sense to rewrite it just for that case."
            )]
            #[inline(always)]
            unsafe fn apply_effect(ptr: MovingPtr<'_, MaybeUninit<Self>>, entity: &mut EntityWorldMut) {
                // SAFETY:
                // - All of the `move_field` calls all fetch distinct and valid fields within `Self`.
                // - If a field is `NoBundleEffect`, it's `apply_effect` is a no-op
                //   and cannot move any value out of an invalid instance.
                // - If a field is `!NoBundleEffect`, it must be valid since a safe
                //   implementation of `DynamicBundle` only moves the value out only
                //   once between `get_components` and `apply_effect`.
                bevy_ptr::deconstruct_moving_ptr!(ptr: MaybeUninit => (
                    $($index => $alias,)*
                ));
                // SAFETY:
                // - If `ptr` is aligned, then field_ptr is aligned properly. Rust tuples cannot be `repr(packed)`.
                $( $name::apply_effect($alias.try_into().debug_checked_unwrap(), entity); )*
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
