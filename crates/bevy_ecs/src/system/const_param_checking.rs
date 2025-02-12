use bevy_ecs::component::Component;
use bevy_ecs::system::SystemParam;
use variadics_please::all_tuples;

#[derive(Copy, Clone, Debug)]
pub enum ComponentAccess {
    Ignore,
    Use { type_id: u128, access: AccessType },
}
impl ComponentAccess {
    const fn invalid(&self, rhs: &ComponentAccess) -> bool {
        match self {
            ComponentAccess::Ignore => false,
            ComponentAccess::Use { type_id, access } => {
                let type_id_self = type_id;
                let access_self = access;
                match rhs {
                    ComponentAccess::Ignore => false,
                    ComponentAccess::Use { type_id, access } => {
                        *type_id_self == *type_id
                            && (matches!(access_self, AccessType::Mut)
                                || matches!(access, AccessType::Mut))
                    }
                }
            }
        }
    }
}
#[derive(Copy, Clone, Debug)]
pub enum AccessType {
    Ref,
    Mut,
}

#[derive(Copy, Clone, Debug)]
pub struct ComponentAccessTree {
    pub this: ComponentAccess,
    pub left: Option<&'static ComponentAccessTree>,
    pub right: Option<&'static ComponentAccessTree>,
}

impl ComponentAccessTree {
    pub const fn combine(
        left: &'static ComponentAccessTree,
        right: &'static ComponentAccessTree,
    ) -> ComponentAccessTree {
        left.check_list(right);
        ComponentAccessTree {
            this: ComponentAccess::Ignore,
            left: Some(left),
            right: Some(right),
        }
    }
    const fn check_list(&self, rhs: &ComponentAccessTree) {
        self.check(rhs.this);
        if let Some(right) = rhs.right {
            self.check_list(right);
        }
        if let Some(left) = rhs.left {
            self.check_list(left);
        }
    }
    const fn check(&self, component_access: ComponentAccess) {
        assert!(!self.this.invalid(&component_access));
        if let Some(right) = self.right {
            right.check(component_access);
        }
        if let Some(left) = self.left {
            left.check(component_access);
        }
    }
}

pub trait AccessTreeContainer {
    const COMPONENT_ACCESS_TREE: ComponentAccessTree;
}

impl<T: Component> AccessTreeContainer for &T {
    const COMPONENT_ACCESS_TREE: ComponentAccessTree = ComponentAccessTree {
        this: ComponentAccess::Use {
            type_id: T::UNSTABLE_TYPE_ID,
            access: AccessType::Ref,
        },
        left: None,
        right: None,
    };
}

impl<T: Component> AccessTreeContainer for &mut T {
    const COMPONENT_ACCESS_TREE: ComponentAccessTree = ComponentAccessTree {
        this: ComponentAccess::Use {
            type_id: T::UNSTABLE_TYPE_ID,
            access: AccessType::Mut,
        },
        left: None,
        right: None,
    };
}

pub trait ValidSystemParams<SystemParams> {
    const COMPONENT_ACCESS_TREE: ComponentAccessTree;
}
macro_rules! impl_valid_system_params {
    ($($t:ident),+) => {
        impl<

            $($t: AccessTreeContainer,)*
        > ValidSystemParams<($($t,)*)> for ($($t,)*) {
            const COMPONENT_ACCESS_TREE: ComponentAccessTree = impl_valid_system_params!(@nest $($t),+);
        }
    };

    (@nest $t0:ident) => {
        $t0::COMPONENT_ACCESS_TREE
    };

    (@nest $t0:ident, $t1:ident) => {
        ComponentAccessTree::combine(
            &$t0::COMPONENT_ACCESS_TREE,
            &$t1::COMPONENT_ACCESS_TREE,
        )
    };

    (@nest $t0:ident, $t1:ident, $($rest:ident),+) => {
        ComponentAccessTree::combine(
            &$t0::COMPONENT_ACCESS_TREE,
            &ComponentAccessTree::combine(
                &$t1::COMPONENT_ACCESS_TREE,
                &impl_valid_system_params!(@nest $($rest),+)
            )
        )
    };
}
// Usage example:
impl_valid_system_params!(T0, T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11);
impl_valid_system_params!(T0, T1, T2, T3, T4, T5, T6, T7, T8, T9, T10);
impl_valid_system_params!(T0, T1, T2, T3, T4, T5, T6, T7, T8, T9);
impl_valid_system_params!(T0, T1, T2, T3, T4, T5, T6, T7, T8);
impl_valid_system_params!(T0, T1, T2, T3, T4, T5, T6, T7);
impl_valid_system_params!(T0, T1, T2, T3, T4, T5, T6);
impl_valid_system_params!(T0, T1, T2, T3, T4, T5);
impl_valid_system_params!(T0, T1, T2, T3, T4);
impl_valid_system_params!(T0, T1, T2, T3);
impl_valid_system_params!(T0, T1, T2);
impl_valid_system_params!(T0, T1);
impl_valid_system_params!(T0);
use crate::system::SystemParamItem;
macro_rules! impl_valid_system_params_for_fn {
    ($($param:ident),+) => {
        impl<Func, Out, $($param: SystemParam),*> ValidSystemParams<($($param,)*)>
        for Func
    where
        Func: Send + Sync + 'static,
        for<'a> &'a mut Func: FnMut($($param),*) -> Out + FnMut($(SystemParamItem<$param>),*) -> Out,
    {
        const COMPONENT_ACCESS_TREE: ComponentAccessTree = impl_valid_system_params_for_fn!(@nest $($param),+);
    }
    };

    (@nest $t0:ident) => {
        $t0::COMPONENT_ACCESS_TREE
    };

    (@nest $t0:ident, $t1:ident) => {
        ComponentAccessTree::combine(
            &$t0::COMPONENT_ACCESS_TREE,
            &$t1::COMPONENT_ACCESS_TREE,
        )
    };

    (@nest $t0:ident, $t1:ident, $($rest:ident),+) => {
        ComponentAccessTree::combine(
            &$t0::COMPONENT_ACCESS_TREE,
            &ComponentAccessTree::combine(
                &$t1::COMPONENT_ACCESS_TREE,
                &impl_valid_system_params!(@nest $($rest),+)
            )
        )
    };
}

impl_valid_system_params_for_fn!(T0, T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11);
impl_valid_system_params_for_fn!(T0, T1, T2, T3, T4, T5, T6, T7, T8, T9, T10);
impl_valid_system_params_for_fn!(T0, T1, T2, T3, T4, T5, T6, T7, T8, T9);
impl_valid_system_params_for_fn!(T0, T1, T2, T3, T4, T5, T6, T7, T8);
impl_valid_system_params_for_fn!(T0, T1, T2, T3, T4, T5, T6, T7);
impl_valid_system_params_for_fn!(T0, T1, T2, T3, T4, T5, T6);
impl_valid_system_params_for_fn!(T0, T1, T2, T3, T4, T5);
impl_valid_system_params_for_fn!(T0, T1, T2, T3, T4);
impl_valid_system_params_for_fn!(T0, T1, T2, T3);
impl_valid_system_params_for_fn!(T0, T1, T2);
impl_valid_system_params_for_fn!(T0, T1);
impl_valid_system_params_for_fn!(T0);
