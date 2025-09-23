use core::any::{Any, TypeId};

use bevy_ptr::{MovingPtr, OwningPtr, Ptr};
use core::mem::MaybeUninit;
use variadics_please::all_tuples_enumerated;

use crate::{
    bundle::{Bundle, BundleFromComponents, DynamicBundle, NoBundleEffect},
    component::{Component, ComponentId, Components, ComponentsRegistrator, StorageType},
    fragmenting_value::{FragmentingValueComponent, FragmentingValueV2Borrowed},
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

    #[inline]
    fn get_fragmenting_values<'a>(
        &'a self,
        components: &Components,
        values: &mut impl FnMut(Option<FragmentingValueV2Borrowed<'a>>),
    ) {
        if let Some(component) = (self as &dyn Any).downcast_ref::<C::Key>() {
            values(FragmentingValueV2Borrowed::from_component(
                components, component,
            ));
        }
    }

    #[inline]
    fn count_fragmenting_values() -> usize {
        if TypeId::of::<C>() == TypeId::of::<C::Key>() {
            1
        } else {
            0
        }
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

            fn get_fragmenting_values<'a>(&'a self, components: &Components, values: &mut impl FnMut(Option<FragmentingValueV2Borrowed<'a>>)) {
                #[allow(
                    non_snake_case,
                    reason = "The names of these variables are provided by the caller, not by us."
                )]
                let ($($name,)*) = &self;
                $(
                    $name.get_fragmenting_values(components, &mut *values);
                )*

            }

            #[inline(always)]
            fn count_fragmenting_values() -> usize {
                0 $(+ <$name as Bundle>::count_fragmenting_values())*
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
                // SAFETY: Caller ensures requirements for calling `get_components` are met.
                $( $name::get_components($alias, func); )*
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
                // SAFETY: Caller ensures requirements for calling `apply_effect` are met.
                $( $name::apply_effect($alias, entity); )*
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
