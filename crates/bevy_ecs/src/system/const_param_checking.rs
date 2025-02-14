use bevy_ecs::component::Component;
use bevy_ecs::system::SystemParam;
use core::fmt::Debug;

/// A message describing a system parameter validation error that occurred during const evaluation.
/// Contains information about the conflicting parameter access types and their names for
/// displaying helpful error messages to users.
#[derive(Copy, Clone, Debug)]
pub struct SystemPanicMessage {
    pub lhs_access_type: AccessType,
    pub lhs_name: &'static str,
    pub rhs_access_type: AccessType,
    pub rhs_name: &'static str,
}

/// Describes how a component is accessed within a system - either ignored
/// or used with a specific access type (Ref/Mut) and optional name.
#[derive(Copy, Clone, Debug)]
pub enum ComponentAccess {
    Ignore,
    Use {
        type_id: u128,
        access: AccessType,
        name: Option<&'static str>,
    },
}
impl ComponentAccess {
    const fn invalid(&self, rhs: &ComponentAccess) -> bool {
        match self {
            ComponentAccess::Ignore => false,
            ComponentAccess::Use {
                type_id,
                access,
                name,
            } => {
                let type_id_self = type_id;
                let access_self = access;
                match rhs {
                    ComponentAccess::Ignore => false,
                    ComponentAccess::Use {
                        type_id,
                        access,
                        name,
                    } => {
                        *type_id_self == *type_id
                            && (matches!(access_self, AccessType::Mut)
                                || matches!(access, AccessType::Mut))
                    }
                }
            }
        }
    }
    const fn invalid_2(&self, rhs: &ComponentAccess) -> Option<SystemPanicMessage> {
        match self {
            ComponentAccess::Ignore => None,
            ComponentAccess::Use {
                type_id,
                access,
                name,
            } => {
                let type_id_self = type_id;
                let access_self = access;
                let name_self = name;
                match rhs {
                    ComponentAccess::Ignore => None,
                    ComponentAccess::Use {
                        type_id,
                        access,
                        name,
                    } => {
                        if *type_id_self != *type_id {
                            return None;
                        }
                        if matches!(access_self, AccessType::Mut) {
                            if let Some(name_self) = name_self {
                                let name_self = *name_self;
                                if let Some(name) = name {
                                    let name = *name;
                                    return Some(SystemPanicMessage {
                                        lhs_access_type: *access_self,
                                        lhs_name: name_self,
                                        rhs_access_type: *access,
                                        rhs_name: name,
                                    });
                                }
                                return Some(SystemPanicMessage {
                                    lhs_access_type: *access_self,
                                    lhs_name: "",
                                    rhs_access_type: *access,
                                    rhs_name: "",
                                });
                            }
                        } else {
                            if matches!(access, AccessType::Mut) {
                                if let Some(name_self) = name_self {
                                    let name_self = *name_self;
                                    if let Some(name) = name {
                                        let name = *name;
                                        return Some(SystemPanicMessage {
                                            lhs_access_type: *access_self,
                                            lhs_name: name_self,
                                            rhs_access_type: *access,
                                            rhs_name: name,
                                        });
                                    }
                                    return Some(SystemPanicMessage {
                                        lhs_access_type: *access_self,
                                        lhs_name: "",
                                        rhs_access_type: *access,
                                        rhs_name: "",
                                    });
                                }
                            }
                        }
                        None
                    }
                }
            }
        }
    }
}
/// The type of access to a component - either read-only reference or mutable reference.
#[derive(Copy, Clone, Debug)]
pub enum AccessType {
    Ref,
    Mut,
}

/// A tree structure representing the `With<T>`, `Added<T>`, and `Changed<T>` filters
/// applied to a query, tracking which component types must be present. Used for compile-time
/// validation of query compatibility. The tree allows recursively checking filter constraints
/// across complex query combinations.
#[derive(Copy, Clone, Debug)]
pub struct WithFilterTree {
    /// `WithId`
    pub this: WithId,
    /// The lhs of the tree structure
    pub left: &'static Option<WithFilterTree>,
    /// The rhs of the tree structure
    pub right: &'static Option<WithFilterTree>,
}

impl WithFilterTree {
    pub const fn combine(
        left: &'static Option<WithFilterTree>,
        right: &'static Option<WithFilterTree>,
    ) -> Option<Self> {
        Some(Self {
            this: WithId(None),
            left,
            right,
        })
    }
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
        if let Some(this_id) = self.this.0 {
            if let Some(without_id) = without_id.0 {
                if this_id == without_id {
                    return true;
                }
            }
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

/// A tree structure representing the `Without<T>` filters applied to a query, tracking which
/// component types must be absent. Used for compile-time validation of query compatibility.
/// The tree allows recursively checking filter constraints across complex query combinations.
#[derive(Copy, Clone, Debug)]
pub struct WithoutFilterTree {
    /// `WithoutId`
    pub this: WithoutId,
    /// The lhs of the tree structure
    pub left: &'static Option<WithoutFilterTree>,
    /// The rhs of the tree structure
    pub right: &'static Option<WithoutFilterTree>,
}

impl WithoutFilterTree {
    pub const fn combine(
        left: &'static Option<WithoutFilterTree>,
        right: &'static Option<WithoutFilterTree>,
    ) -> Option<Self> {
        Some(Self {
            this: WithoutId(None),
            left,
            right,
        })
    }
    const fn is_filtered_out_component_list(&self, rhs: &ComponentAccessTree) -> bool {
        if self.is_filtered_out_component(rhs.this) {
            return true;
        }
        if let Some(right) = rhs.right {
            if self.is_filtered_out_component_list(right) {
                return true;
            }
        }
        if let Some(left) = rhs.left {
            if self.is_filtered_out_component_list(left) {
                return true;
            }
        }
        return false;
    }
    const fn is_filtered_out_component(&self, component_access: ComponentAccess) -> bool {
        match component_access {
            ComponentAccess::Ignore => false,
            ComponentAccess::Use {
                type_id,
                access,
                name,
            } => {
                if let Some(type_id_this) = self.this.0 {
                    if type_id_this == type_id {
                        return true;
                    }
                    if let Some(right) = self.right {
                        if right.is_filtered_out_component(component_access) {
                            return true;
                        }
                    }
                    if let Some(left) = self.left {
                        if left.is_filtered_out_component(component_access) {
                            return true;
                        }
                    }
                }
                false
            }
        }
    }
}

/// CONST_UNSTABLE_TYPEID for a component within a Without<> filter
#[derive(Clone, Copy, Debug)]
pub struct WithoutId(pub Option<u128>);
/// CONST_UNSTABLE_TYPEID for a component within a With<>, Added<>, and Changed<> filter
#[derive(Clone, Copy, Debug)]
pub struct WithId(pub Option<u128>);

/// A tree structure representing how components are accessed within a query or system.
/// Tracks access types and CONST_UNSTABLE_IDs to enable compile-time validation
/// of query compatibility and detection of conflicting accesses.
#[derive(Copy, Clone, Debug)]
pub struct ComponentAccessTree {
    pub this: ComponentAccess,
    pub left: Option<&'static ComponentAccessTree>,
    pub right: Option<&'static ComponentAccessTree>,
}

impl ComponentAccessTree {
    #[track_caller]
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
    ) -> Option<SystemPanicMessage> {
        if let Some(left_without_tree) = left.2 {
            if left_without_tree.is_filtered_out_component_list(right.0) {
                return None;
            }
        }
        if let Some(right_without_tree) = right.2 {
            if right_without_tree.is_filtered_out_component_list(left.0) {
                return None;
            }
        }

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
                return Self::check_list_with_output(left.0, right.0);
            }
        };

        if with_tree.is_filtered_out_list(without_tree) {
            return None;
        }
        if let Some(with_tree) = maybe_with_tree {
            if let Some(without_tree) = maybe_without_tree {
                if with_tree.is_filtered_out_list(without_tree) {
                    return None;
                }
            }
        }

        return Self::check_list_with_output(left.0, right.0);
    }

    const fn check_list_with_output(
        &self,
        rhs: &ComponentAccessTree,
    ) -> Option<SystemPanicMessage> {
        if let Some(owo) = self.check_with_output(rhs.this) {
            return Some(owo);
        }
        if let Some(right) = rhs.right {
            if let Some(owo) = self.check_list_with_output(right) {
                return Some(owo);
            }
        }
        if let Some(left) = rhs.left {
            if let Some(owo) = self.check_list_with_output(left) {
                return Some(owo);
            }
        }
        None
    }

    const fn check_with_output(
        &self,
        component_access: ComponentAccess,
    ) -> Option<SystemPanicMessage> {
        if let Some(str) = self.this.invalid_2(&component_access) {
            return Some(str);
        }
        if let Some(right) = self.right {
            if let Some(owo) = right.check_with_output(component_access) {
                return Some(owo);
            }
        }
        if let Some(left) = self.left {
            if let Some(owo) = left.check_with_output(component_access) {
                return Some(owo);
            }
        }
        None
    }

    #[track_caller]
    const fn check_list(&self, rhs: &ComponentAccessTree) {
        self.check(rhs.this);
        if let Some(right) = rhs.right {
            self.check_list(right);
        }
        if let Some(left) = rhs.left {
            self.check_list(left);
        }
    }
    #[track_caller]
    const fn check(&self, component_access: ComponentAccess) {
        if let Some(invalid) = self.this.invalid_2(&component_access) {
            let lhs_access_type = match invalid.lhs_access_type {
                AccessType::Ref => "&",
                AccessType::Mut => "&mut",
            };
            let rhs_access_type = match invalid.rhs_access_type {
                AccessType::Ref => "&",
                AccessType::Mut => "&mut",
            };
            let lhs_name = invalid.lhs_name;
            let rhs_name = invalid.rhs_name;
            const_panic::concat_panic!(const_panic::FmtArg::DISPLAY; "\nInvalid System Queries, ", lhs_access_type," ", lhs_name, ", ", rhs_access_type," ", rhs_name, "\n");
        }
        if let Some(right) = self.right {
            right.check(component_access);
        }
        if let Some(left) = self.left {
            left.check(component_access);
        }
    }
}

/// A trait for providing a const ComponentAccessTree on Components without adding them directly
/// to the Component type
pub(crate) trait AccessTreeContainer {
    const COMPONENT_ACCESS_TREE: ComponentAccessTree;
}

impl<T: Component> AccessTreeContainer for &T {
    const COMPONENT_ACCESS_TREE: ComponentAccessTree = ComponentAccessTree {
        this: ComponentAccess::Use {
            type_id: T::UNSTABLE_TYPE_ID,
            access: AccessType::Ref,
            #[cfg(feature="diagnostic_component_names")]
            name: T::STRUCT_NAME,
            #[cfg(not(feature="diagnostic_component_names"))]
            name: Some(""),
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
            #[cfg(feature="diagnostic_component_names")]
            name: T::STRUCT_NAME,
            #[cfg(not(feature="diagnostic_component_names"))]
            name: Some(""),
        },
        left: None,
        right: None,
    };
}

/// A trait for validating system parameter combinations at compile time.
/// Implementing types provide a const evaluation mechanism to detect invalid
/// parameter combinations before runtime.
pub trait ValidSystemParams<SystemParams> {
    const SYSTEM_PARAMS_COMPILE_ERROR: Option<SystemPanicMessage>;
}
use crate::system::SystemParamItem;
macro_rules! impl_valid_system_params_for_fn {
    ($($param:ident),+) => {
        impl<Func, Out, $($param: SystemParam),*> ValidSystemParams<($($param,)*)>
        for Func
    where
        Func: Send + Sync + 'static,
        for<'a> &'a mut Func: FnMut($($param),*) -> Out + FnMut($(SystemParamItem<$param>),*) -> Out,
    {
        const SYSTEM_PARAMS_COMPILE_ERROR: Option<SystemPanicMessage> = const {
                let mut error = None;
                impl_valid_system_params_for_fn!(@check_all error, $($param),+);
                error
            };
    }
    };

    (@check_all $error:ident, $t0:ident) => {
        let _ = &$t0::COMPONENT_ACCESS_TREE;
    };

    (@check_all $error:ident, $t0:ident, $($rest:ident),+) => {
        // Check t0 against all remaining elements

        $(
            if let Some(err) = ComponentAccessTree::filter_check(
                (&$t0::COMPONENT_ACCESS_TREE, &$t0::WITH_FILTER_TREE, &$t0::WITHOUT_FILTER_TREE),
                (&$rest::COMPONENT_ACCESS_TREE, &$rest::WITH_FILTER_TREE, &$rest::WITHOUT_FILTER_TREE)
            ) {
                $error = Some(err);
            }
        )*
        // Recursively check remaining elements
        impl_valid_system_params_for_fn!(@check_all $error, $($rest),+);
    };
}

impl_valid_system_params_for_fn!(
    T0, T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14, T15, T16
);
impl_valid_system_params_for_fn!(
    T0, T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14, T15
);
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
    const SYSTEM_PARAMS_COMPILE_ERROR: Option<SystemPanicMessage> = None;
}
