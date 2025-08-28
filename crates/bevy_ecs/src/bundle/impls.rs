use core::any::TypeId;

use bevy_ptr::OwningPtr;
use core::ptr::NonNull;
use core::mem::MaybeUninit;
use variadics_please::{all_tuples, all_tuples_enumerated};

use crate::{
    bundle::{Bundle, BundleEffect, BundleFromComponents, DynamicBundle, NoBundleEffect},
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

// SAFETY:
// - The pointer is only moved out of in `get_components`.
// - `Effect = () : NoBundleEffect` so `apply_effect` is a no-op.
unsafe impl<C: Component> DynamicBundle for C {
    type Effect = ();
    #[inline]
    unsafe fn get_components(
        ptr: *mut Self,
        func: &mut impl FnMut(StorageType, OwningPtr<'_>),
    ) -> Self::Effect {
        // SAFETY: The caller must ensure that `ptr` is not null.
        let ptr = unsafe { NonNull::new(ptr).debug_checked_unwrap().cast::<u8>() };
        // SAFETY:
        // - The caller must ensure that `ptr` must point to valid value of type `C`.
        // - The `A` type parameter is [`Aligned`] and the caller must ensure that `ptr` is aligned.
        // - `ptr` must has the correct provenance to allow read and writes of the pointee type: the caller
        //   must sure that it is owned.
        // - The lifetime of the produced `OwningPtr` is valid for the rest of this fucntion call and does not
        //   alias, assuming that `func` is sound.
        let ptr = unsafe { OwningPtr::new(ptr) };
        func(C::STORAGE_TYPE, ptr);
    }

    #[inline]
    unsafe fn apply_effect(_ptr: *mut MaybeUninit<Self>, _entity: &mut EntityWorldMut) {}
}

macro_rules! tuple_impl {
    ($(#[$meta:meta])* $(($index:tt, $name: ident)),*) => {
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
        // SAFETY:
        // Assuming each of the fields' types implement `DynamicBundle` correctly:
        // - Each of the implementations for each of the fields must move the components out of the `Bundle` exactly once between both
        //   `get_components` and `apply_effect`.
        // - If all of the individual tuple elements are `Effect: NoBundleEffect`, then the whole type's `Effect` will also be `NoBundleEffect`.
        //   then the implementation of `apply_effect` must also be a no-op.
        unsafe impl<$($name: Bundle),*> DynamicBundle for ($($name,)*) {
            type Effect = ($($name::Effect,)*);
            #[allow(
                clippy::unused_unit,
                reason = "Zero-length tuples will generate a function body equivalent to `()`; however, this macro is meant for all applicable tuples, and as such it makes no sense to rewrite it just for that case."
            )]
            #[inline(always)]
            unsafe fn get_components(ptr: *mut Self, func: &mut impl FnMut(StorageType, OwningPtr<'_>)) {
                $(
                    let field_ptr = &raw mut (*ptr).$index;
                    // SAFETY:
                    // - If `ptr` is aligned, then field_ptr is aligned properly
                    // - If a field is `NoBundleEffect`, it's `apply_effect` is a no-op
                    //   and cannot move any value out of an invalid instance after this call.
                    // - If a field is `!NoBundleEffect`, it must be valid since a safe
                    //   implementation of `DynamicBundle` only moves the value out only
                    //   once between `get_components` and `apply_effect`.
                    $name::get_components(field_ptr, &mut *func);
                )*
            }

            #[allow(
                clippy::unused_unit,
                reason = "Zero-length tuples will generate a function body equivalent to `()`; however, this macro is meant for all applicable tuples, and as such it makes no sense to rewrite it just for that case."
            )]
            #[inline(always)]
            unsafe fn apply_effect(ptr: *mut core::mem::MaybeUninit<Self>, entity: &mut EntityWorldMut) {
                $(
                    let field_ptr = ptr
                        .byte_add(core::mem::offset_of!(Self, $index))
                        .cast::<core::mem::MaybeUninit<$name>>();
                    // SAFETY:
                    // - If `ptr` is aligned, then field_ptr is aligned properly
                    // - If a field is `NoBundleEffect`, it's `apply_effect` is a no-op
                    //   and cannot move any value out of an invalid instance.
                    // - If a field is `!NoBundleEffect`, it must be valid since a safe
                    //   implementation of `DynamicBundle` only moves the value out only
                    //   once between `get_components` and `apply_effect`.
                    $name::apply_effect(field_ptr, entity);
                )*
            }
        }
    }
}

all_tuples_enumerated!(
    #[doc(fake_variadic)]
    tuple_impl,
    0,
    15,
    B
);

macro_rules! after_effect_impl {
    ($($after_effect: ident),*) => {
        #[expect(
            clippy::allow_attributes,
            reason = "This is a tuple-related macro; as such, the lints below may not always apply."
        )]
        impl<$($after_effect: BundleEffect),*> BundleEffect for ($($after_effect,)*) {
            #[allow(
                clippy::unused_unit,
                reason = "Zero-length tuples will generate a function body equivalent to `()`; however, this macro is meant for all applicable tuples, and as such it makes no sense to rewrite it just for that case.")
            ]
            fn apply(self, _entity: &mut EntityWorldMut) {
                #[allow(
                    non_snake_case,
                    reason = "The names of these variables are provided by the caller, not by us."
                )]
                let ($($after_effect,)*) = self;
                $($after_effect.apply(_entity);)*
            }
        }

        impl<$($after_effect: NoBundleEffect),*> NoBundleEffect for ($($after_effect,)*) { }
    }
}

all_tuples!(after_effect_impl, 0, 15, P);
