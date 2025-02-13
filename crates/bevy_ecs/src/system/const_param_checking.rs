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
pub struct WithFilterTree {
    pub this: WithId,
    pub left: Option<&'static WithFilterTree>,
    pub right: Option<&'static WithFilterTree>,
}

impl WithFilterTree {
    const fn is_filtered_out_list(&self, rhs: &WithoutFilterTree) -> bool {
        if self.is_filtered_out(rhs.this) {
            return true;
        }
        if let Some(right) = rhs.right {
            if self.is_filtered_out_list(right) {
                return true;
            }
        }
        if let Some(left) = rhs.left {
            if self.is_filtered_out_list(left) {
                return true;
            }
        }
        return false;
    }
    const fn is_filtered_out(&self, without_id: WithoutId) -> bool {
        if self.this.0 == without_id.0 {
            return true;
        }
        if let Some(right) = self.right {
            if right.is_filtered_out(without_id) {
                return true;
            }
        }
        if let Some(left) = self.left {
            if left.is_filtered_out(without_id) {
                return true;
            }
        }
        return false;
    }
}

#[derive(Copy, Clone, Debug)]
pub struct WithoutFilterTree {
    pub this: WithoutId,
    pub left: Option<&'static WithoutFilterTree>,
    pub right: Option<&'static WithoutFilterTree>,
}

#[derive(Clone, Copy, Debug)]
pub struct WithoutId(pub u128);
#[derive(Clone, Copy, Debug)]
pub struct WithId(pub u128);

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

    pub const fn filter_check(
        left: (
            &'static ComponentAccessTree,
            &'static Option<WithFilterTree>,
            &'static Option<WithoutFilterTree>,
        ),
        right: (
            &'static ComponentAccessTree,
            &'static Option<WithFilterTree>,
            &'static Option<WithoutFilterTree>,
        ),
    ) {
        let (with_tree, without_tree, maybe_with_tree, maybe_without_tree): (
            &WithFilterTree,
            &WithoutFilterTree,
            &Option<WithFilterTree>,
            &Option<WithoutFilterTree>,
        ) = match (left.1, left.2, right.1, right.2) {
            (Some(left_with), left_without, right_with, Some(right_without)) => {
                (left_with, right_without, right_with, left_without)
            }
            (left_with, Some(left_without), Some(right_with), right_without) => {
                (right_with, left_without, left_with, right_without)
            }
            _ => {
                Self::combine(left.0, right.0);
                return;
            }
        };

        if with_tree.is_filtered_out_list(without_tree) {
            return;
        }
        if let Some(with_tree) = maybe_with_tree {
            if let Some(without_tree) = maybe_without_tree {
                if with_tree.is_filtered_out_list(without_tree) {
                    return;
                }
            }
        }
        Self::combine(left.0, right.0);
        return;
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
        const COMPONENT_ACCESS_TREE: ComponentAccessTree = {
                impl_valid_system_params_for_fn!(@check_all $($param),+);
                ComponentAccessTree {
                    this: ComponentAccess::Ignore,
                    left: None,
                    right: None,
                }
            };
    }
    };

    (@check_all $t0:ident) => {
        // Single element has no pairs to check
    };

    (@check_all $t0:ident, $($rest:ident),+) => {
        // Check t0 against all remaining elements
        $(
            ComponentAccessTree::filter_check(
                (&$t0::COMPONENT_ACCESS_TREE, &$t0::WITH_FILTER_TREE, &$t0::WITHOUT_FILTER_TREE),
                (&$rest::COMPONENT_ACCESS_TREE, &$rest::WITH_FILTER_TREE, &$rest::WITHOUT_FILTER_TREE)
            );
        )*
        // Recursively check remaining elements
        impl_valid_system_params_for_fn!(@check_all $($rest),+);
    };
}

impl_valid_system_params_for_fn!(T0, T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14, T15, T16);
impl_valid_system_params_for_fn!(T0, T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14, T15);
impl_valid_system_params_for_fn!(T0, T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14);
impl_valid_system_params_for_fn!(T0, T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13);
impl_valid_system_params_for_fn!(T0, T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12);
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

impl<Func, Out> ValidSystemParams<()> for Func
where
    Func: Send + Sync + 'static,
    for<'a> &'a mut Func: FnMut() -> Out + FnMut() -> Out,
{
    const COMPONENT_ACCESS_TREE: ComponentAccessTree = {
        ComponentAccessTree {
            this: ComponentAccess::Ignore,
            left: None,
            right: None,
        }
    };
}
