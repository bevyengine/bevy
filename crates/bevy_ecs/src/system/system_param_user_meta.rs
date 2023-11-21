use std::any::TypeId;
use std::mem;
use std::mem::MaybeUninit;

struct SystemParamUserMetaRequestImpl<T: 'static> {
    values: Vec<T>,
}

trait SystemParamUserMetaRequestDyn {
    fn item_type_id(&self) -> TypeId;
    unsafe fn provide_value(&mut self, value: *const ());
}

/// Request arbitrary metadata from [`SystemParam`](crate::system::SystemParam).
///
/// See [`SystemParam::user_meta`](crate::system::SystemParam::user_meta) for more information.
pub struct SystemParamUserMetaRequest<'a> {
    request: &'a mut dyn SystemParamUserMetaRequestDyn,
}

impl<T: 'static> SystemParamUserMetaRequestDyn for SystemParamUserMetaRequestImpl<T> {
    fn item_type_id(&self) -> TypeId {
        TypeId::of::<T>()
    }

    unsafe fn provide_value(&mut self, value: *const ()) {
        let value = value as *const T;
        self.values.push(mem::transmute_copy(&*value));
    }
}

impl<'a> SystemParamUserMetaRequest<'a> {
    /// Provide the metadata value.
    ///
    /// This is a shortcut for [`provide_value_with`](Self::provide_value_with).
    pub fn provide_value<T: 'static>(&mut self, value: T) {
        self.provide_value_with(|| value)
    }

    /// Provide the metadata value.
    pub fn provide_value_with<T: 'static>(&mut self, value: impl FnOnce() -> T) {
        unsafe {
            if self.request.item_type_id() == TypeId::of::<T>() {
                let value = value();
                let value = MaybeUninit::new(value);
                self.request.provide_value(value.as_ptr() as *const ());
            }
        }
    }

    pub(crate) fn with<T: 'static>(
        mut cb: impl FnMut(&mut SystemParamUserMetaRequest<'_>),
    ) -> Vec<T> {
        let mut req_typed = SystemParamUserMetaRequestImpl { values: Vec::new() };
        let mut req = SystemParamUserMetaRequest {
            request: &mut req_typed,
        };
        cb(&mut req);
        req_typed.values
    }
}

#[cfg(test)]
mod tests {
    use crate as bevy_ecs;
    use crate::component::Tick;
    use crate::prelude::IntoSystem;
    use crate::prelude::World;
    use crate::system::system::System;
    use crate::system::{
        ReadOnlySystemParam, Res, SystemMeta, SystemParam, SystemParamUserMetaRequest,
    };
    use crate::world::unsafe_world_cell::UnsafeWorldCell;
    use bevy_ecs_macros::Resource;
    use std::any;
    use std::marker::PhantomData;

    // Shortcut for test.
    fn system_param_user_meta_request<P: SystemParam, T: 'static>() -> Vec<T> {
        SystemParamUserMetaRequest::with(|req| P::user_meta(req))
    }

    struct MyTestParam<T: 'static>(PhantomData<T>);

    unsafe impl<T> SystemParam for MyTestParam<T> {
        type State = ();
        type Item<'world, 'state> = MyTestParam<T>;

        fn init_state(_world: &mut World, _system_meta: &mut SystemMeta) -> Self::State {
            unreachable!("not needed in test")
        }

        fn user_meta(request: &mut SystemParamUserMetaRequest) {
            // In test we provide metadata as strings.
            // In production we may provide something like `InternedSystemSet`.
            request.provide_value(format!("meta {}", any::type_name::<T>()));
            // We can provide multiple values. This is not used in test.
            request.provide_value(10);
        }

        unsafe fn get_param<'world, 'state>(
            _state: &'state mut Self::State,
            _system_meta: &SystemMeta,
            _world: UnsafeWorldCell<'world>,
            _change_tick: Tick,
        ) -> Self::Item<'world, 'state> {
            unreachable!("not needed in test")
        }
    }

    unsafe impl<T> ReadOnlySystemParam for MyTestParam<T> {}

    #[derive(Resource)]
    struct MyRes(f32);

    #[derive(SystemParam)]
    struct DerivedParam<'w, T: 'static> {
        _p0: MyTestParam<u32>,
        _p1: MyTestParam<T>,
        _p2: Res<'w, MyRes>,
    }

    #[test]
    fn test_param_meta() {
        // Simple
        assert_eq!(
            vec![format!("meta {}", any::type_name::<u32>())],
            system_param_user_meta_request::<MyTestParam<u32>, String>()
        );

        // Tuple
        assert_eq!(
            vec![
                format!("meta {}", any::type_name::<u32>()),
                format!("meta {}", any::type_name::<Vec<u32>>()),
            ],
            system_param_user_meta_request::<(MyTestParam<u32>, MyTestParam<Vec<u32>>), String>()
        );

        // Derive
        assert_eq!(
            vec![
                format!("meta {}", any::type_name::<u32>()),
                format!("meta {}", any::type_name::<Vec<u32>>()),
            ],
            system_param_user_meta_request::<DerivedParam<'_, Vec<u32>>, String>()
        );
    }

    #[test]
    fn test_system_param_meta() {
        fn my_system(_a: MyTestParam<u8>, _b: DerivedParam<Vec<u32>>) {}

        let my_system = IntoSystem::into_system(my_system);

        let my_system: &dyn System<In = (), Out = ()> = &my_system;

        assert_eq!(
            vec![
                format!("meta {}", any::type_name::<u8>()),
                format!("meta {}", any::type_name::<u32>()),
                format!("meta {}", any::type_name::<Vec<u32>>()),
            ],
            my_system.param_user_meta_dyn::<String>()
        );
    }
}
