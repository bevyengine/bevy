use bevy_ecs::component::Component;
use bevy_ecs::system::SystemParam;
use core::fmt::Debug;
use variadics_please::all_tuples;
use bevy_ecs::query::QueryFilter;

/// A message describing a system parameter validation error that occurred during const evaluation.
/// Contains information about the conflicting parameter access types and their names for
/// displaying helpful error messages to users.
#[derive(Copy, Clone, Debug)]
pub struct SystemPanicMessage {
    /// The type of access (Ref/Mut) for the left-hand side component in a conflict
    pub lhs_access_type: AccessType,
    /// The name of the left-hand side component involved in the conflict
    pub lhs_name: &'static str,
    /// The type of access (Ref/Mut) for the right-hand side component in a conflict
    pub rhs_access_type: AccessType,
    /// The name of the right-hand side component involved in the conflict
    pub rhs_name: &'static str,
}

/// Describes how a component is accessed within a system - either ignored
/// or used with a specific access type (Ref/Mut) and optional name.
#[derive(Copy, Clone, Debug)]
pub struct ComponentAccess {
    /// Unique compile-time identifier for the component type
    pub type_id: u128,
    /// The type of access (reference or mutable) to the component
    pub access: AccessType,
    /// The component's name if available, used for error reporting
    pub name: &'static str,
}
impl ComponentAccess {
    const fn conflicts(&self, rhs: &ComponentAccess) -> Option<SystemPanicMessage> {
        if self.type_id != rhs.type_id {
            return None;
        }
        match (self.access, rhs.access) {
            (AccessType::Mut, _) | (_, AccessType::Mut) => {
                //panic!("OwO");
                Some(SystemPanicMessage {
                    lhs_access_type: self.access,
                    lhs_name: self.name,
                    rhs_access_type: rhs.access,
                    rhs_name: rhs.name,
                })
            }
            _ => None,
        }
    }
}
/// The type of access to a component - either read-only reference or mutable reference.
#[derive(Copy, Clone, Debug)]
pub enum AccessType {
    /// Immutable reference access (&T)
    Ref,
    /// Mutable reference access (&mut T)
    Mut,
}

impl AccessType {
    /// String representation of AccessType::Ref
    const REF_STR: &'static str = "&";
    /// String representation of AccessType::Mut
    const MUT_STR: &'static str = "&mut";
    /// Converts to String representation of AccessType.
    pub fn as_str(&self) -> &'static str {
        match self {
            AccessType::Ref => Self::REF_STR,
            AccessType::Mut => Self::MUT_STR,
        }
    }
}

/// CONST_UNSTABLE_TYPEID for a component within a Without<> filter
#[derive(Clone, Copy, Debug)]
pub struct WithoutId(pub u128);
/// CONST_UNSTABLE_TYPEID for a component within a With<>, Added<>, and Changed<> filter
#[derive(Clone, Copy, Debug)]
pub struct WithId(pub u128);

/// An unordered tree created at compile time.
/// The reason it's a tree instead of a list is because we are unable to construct an arbitrarily
/// sized nil cons list in stable rust with the data we want.
#[derive(Copy, Clone, Debug)]
pub enum ConstTreeInner<T: Copy + Clone + Debug + 'static> {
    /// The tree is empty
    Empty,
    /// Leaf node
    Leaf(T),
    /// Unordered list as a tree because we have no choice
    Node(ConstTree<T>, ConstTree<T>),
    /// Panic message that gets bubbled up
    PanicMessage(SystemPanicMessage),
}
/// For cleaner code, the only version of a `ConstTreeInner<T>` that will ever actually exist
/// will be a `&'static ConstTreeInner<T>`; so we give it an easy name.
pub type ConstTree<T> = &'static ConstTreeInner<T>;

/// A collection of all the ConstTrees in a system parameter.
pub struct ConstTrees {
    /// All `Component` accesses in queries.
    pub component_tree: ConstTree<ComponentAccess>,
    /// All `Without<T>`
    pub without_tree: ConstTree<WithoutId>,
    /// All `With<T>`/`Added<T>`/`Changed<T>`/`Option<T>` etc.
    pub with_tree: ConstTree<WithId>,
}

/// Checks a system parameter for a conflicts using recursive const data structures.
pub const fn check_system_parameters_for_conflicts(
    left: ConstTrees,
    right: ConstTrees,
) -> Option<SystemPanicMessage> {
    // First thing we do is return any panic messages, these should only exist on the component trees
    // But later we might want some sort of check to ensure we aren't doubling up on filters in the filters.
    if let ConstTreeInner::PanicMessage(panic_message) = left.component_tree {
        return Some(*panic_message);
    }
    if let ConstTreeInner::PanicMessage(panic_message) = right.component_tree {
        return Some(*panic_message);
    }

    // REGION: We check for any intersections between components we require and components we anti-require.
    if right.component_tree.intersects(left.without_tree) {
        return None;
    }
    if left.component_tree.intersects(right.without_tree) {
        return None;
    }
    if right.with_tree.intersects(left.without_tree) {
        return None;
    }
    if left.with_tree.intersects(right.without_tree) {
        return None;
    }
    // END-REGION

    right.component_tree.self_intersects(left.component_tree)
}
impl ConstTreeInner<WithoutId> {
    /// Simply combines with trees, we do this in case later we want to do some
    /// error checking, like preventing multiple of the same filter.
    pub const fn combine(lhs: ConstTree<WithoutId>, rhs: ConstTree<WithoutId>) -> Self {
        Self::Node(lhs, rhs)
    }
}
impl ConstTreeInner<WithId> {
    /// Simply combines with trees, we do this in case later we want to do some
    /// error checking, like preventing multiple of the same filter.
    pub const fn combine(lhs: ConstTree<WithId>, rhs: ConstTree<WithId>) -> Self {
        Self::Node(lhs, rhs)
    }
    const fn intersects(&'static self, without_tree: ConstTree<WithoutId>) -> bool {
        match without_tree {
            ConstTreeInner::Empty => false,
            ConstTreeInner::Leaf(without_id) => match self {
                ConstTreeInner::Empty => false,
                ConstTreeInner::Leaf(with_id) => with_id.0 == without_id.0,
                ConstTreeInner::Node(component_tree_lhs, component_tree_rhs) => {
                    component_tree_lhs.intersects(without_tree)
                        || component_tree_rhs.intersects(without_tree)
                }
                ConstTreeInner::PanicMessage(_) => unreachable!(),
            },
            ConstTreeInner::Node(without_lhs, without_rhs) => {
                self.intersects(without_lhs) || self.intersects(without_rhs)
            }
            ConstTreeInner::PanicMessage(_) => unreachable!(),
        }
    }
}

impl ConstTreeInner<ComponentAccess> {
    /// Combines together two Component Access trees and looks for conflicts
    pub const fn combine(lhs: ConstTree<ComponentAccess>, rhs: ConstTree<ComponentAccess>) -> Self {
        if let Some(panic_message) = lhs.self_intersects(rhs) {
            return Self::PanicMessage(panic_message);
        }
        Self::Node(lhs, rhs)
    }
    const fn intersects(&'static self, without_tree: ConstTree<WithoutId>) -> bool {
        match without_tree {
            ConstTreeInner::Empty => false,
            ConstTreeInner::Leaf(without_id) => match self {
                ConstTreeInner::Empty => false,
                ConstTreeInner::Leaf(component_access) => component_access.type_id == without_id.0,
                ConstTreeInner::Node(component_tree_lhs, component_tree_rhs) => {
                    component_tree_lhs.intersects(without_tree)
                        || component_tree_rhs.intersects(without_tree)
                }
                ConstTreeInner::PanicMessage(_) => unreachable!(),
            },
            ConstTreeInner::Node(without_lhs, without_rhs) => {
                self.intersects(without_lhs) || self.intersects(without_rhs)
            }
            ConstTreeInner::PanicMessage(_) => unreachable!(),
        }
    }
    const fn self_intersects(
        &'static self,
        right: ConstTree<ComponentAccess>,
    ) -> Option<SystemPanicMessage> {
        match self {
            ConstTreeInner::Empty => {
                if let ConstTreeInner::PanicMessage(panic_message) = right {
                    Some(*panic_message)
                } else {
                    None
                }
            }
            ConstTreeInner::Leaf(component_access) => match right {
                ConstTreeInner::Empty => None,
                ConstTreeInner::Leaf(component_access_2) => {
                    component_access.conflicts(component_access_2)
                }
                ConstTreeInner::Node(rhs, lhs) => {
                    if let Some(panic_message) = self.self_intersects(rhs) {
                        Some(panic_message)
                    } else {
                        self.self_intersects(lhs)
                    }
                }
                ConstTreeInner::PanicMessage(panic_message) => Some(*panic_message),
            },
            ConstTreeInner::Node(lhs, rhs) => {
                if let Some(panic_message) = right.self_intersects(rhs) {
                    Some(panic_message)
                } else {
                    right.self_intersects(lhs)
                }
            }
            ConstTreeInner::PanicMessage(panic_message) => Some(*panic_message),
        }
    }
}

/// A trait for providing a const ComponentAccessTree on Components without adding them directly
/// to the Component type
pub(crate) trait AccessTreeContainer {
    const COMPONENT_ACCESS_TREE: ConstTree<ComponentAccess>;
}

impl<T: Component> AccessTreeContainer for &T {
    const COMPONENT_ACCESS_TREE: ConstTree<ComponentAccess> =
        &ConstTreeInner::Leaf(ComponentAccess {
            type_id: T::UNSTABLE_TYPE_ID,
            access: AccessType::Ref,
            #[cfg(feature = "diagnostic_component_names")]
            name: T::STRUCT_NAME,
            #[cfg(not(feature = "diagnostic_component_names"))]
            name: Some(""),
        });
}

impl<T: Component> AccessTreeContainer for &mut T {
    const COMPONENT_ACCESS_TREE: ConstTree<ComponentAccess> =
        &ConstTreeInner::Leaf(ComponentAccess {
            type_id: T::UNSTABLE_TYPE_ID,
            access: AccessType::Mut,
            #[cfg(feature = "diagnostic_component_names")]
            name: T::STRUCT_NAME,
            #[cfg(not(feature = "diagnostic_component_names"))]
            name: Some(""),
        });
}

/// A trait for validating system parameter combinations at compile time.
/// Implementing types provide a const evaluation mechanism to detect invalid
/// parameter combinations before runtime.
pub trait ValidSystemParams<SystemParams> {
    /// Compile-time error detection for system parameters
    /// Contains validation results from checking parameter compatibility
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
                #[allow(unused_mut)]
                let mut system_panic_message = None;
                impl_valid_system_params_for_fn!(@check_all system_panic_message, $($param),+);
                system_panic_message
            };
    }
    };

    (@check_all $system_panic_message:ident, $t0:ident) => {
        if let ConstTreeInner::PanicMessage(system_panic_msg) = $t0::COMPONENT_ACCESS_TREE {
            $system_panic_message = Some(*system_panic_msg);
        }
    };

    (@check_all $system_panic_message:ident, $t0:ident, $($rest:ident),+) => {
        // Check t0 against all remaining elements

        $(
            if let Some(system_panic_msg) = check_system_parameters_for_conflicts(
                $t0::CONST_TREES,
                $rest::CONST_TREES,
            ) {
                $system_panic_message = Some(system_panic_msg);
            }
        )*
        // Recursively check remaining elements
        impl_valid_system_params_for_fn!(@check_all $system_panic_message, $($rest),+);
    };
}

all_tuples!(impl_valid_system_params_for_fn, 1, 16, T);
impl<Func, Out> ValidSystemParams<()> for Func
where
    Func: Send + Sync + 'static,
    for<'a> &'a mut Func: FnMut() -> Out + FnMut() -> Out,
{
    const SYSTEM_PARAMS_COMPILE_ERROR: Option<SystemPanicMessage> = None;
}


pub mod constime {
    use variadics_please::all_tuples;
    use bevy_ecs::prelude::Component;
    use bevy_ecs::query::QueryFilter;

    #[derive(Copy, Clone, Debug)]
    pub struct ComponentInfo {
        /// Unique compile-time identifier for the component type
        pub type_id: u128,
        /// The component's name if available, used for error reporting
        pub name: &'static str,
    }

    pub enum FilteredAccess {
        Empty,
        Leaf(&'static FilteredAccess),
        Node(&'static FilteredAccess, &'static FilteredAccess),
        ComponentReadAndWrites(ComponentInfo),
        ComponentWrites(ComponentInfo),
        ResourceReadsAndWrites(ComponentInfo),
        ResourceWriter(ComponentInfo),
        ComponentReadAndWritesInverted,
        ComponentWritesInverted,
        ReadsAllResources,
        WritesAllResources,
        With(ComponentInfo),
        Without(ComponentInfo),
    }

    pub trait FilteredAccessHolder {
        const FILTERED_ACCESS: &'static FilteredAccess;
    }
    impl<T: Component> FilteredAccessHolder for bevy_ecs::query::With<T> {
        const FILTERED_ACCESS: &'static FilteredAccess = &FilteredAccess::With(ComponentInfo {
            type_id: T::UNSTABLE_TYPE_ID,
            name: T::STRUCT_NAME,
        });
    }

    impl<T: Component> FilteredAccessHolder for bevy_ecs::query::Without<T> {
        const FILTERED_ACCESS: &'static FilteredAccess = &FilteredAccess::Without(ComponentInfo {
            type_id: T::UNSTABLE_TYPE_ID,
            name: T::STRUCT_NAME,
        });
    }

    impl<'__w, T: Component> FilteredAccessHolder for &'__w mut T {
        const FILTERED_ACCESS: &'static FilteredAccess = &FilteredAccess::Node(
            &FilteredAccess::ComponentReadAndWrites(ComponentInfo {
                type_id: T::UNSTABLE_TYPE_ID,
                name: T::STRUCT_NAME,
            }),
            &FilteredAccess::ComponentWrites(ComponentInfo {
                type_id: T::UNSTABLE_TYPE_ID,
                name: T::STRUCT_NAME,
            })
        );
    }

    impl<T: Component> FilteredAccessHolder for &T {
        const FILTERED_ACCESS: &'static FilteredAccess = &FilteredAccess::ComponentReadAndWrites(ComponentInfo {
            type_id: T::UNSTABLE_TYPE_ID,
            name: T::STRUCT_NAME,
        });
    }
    macro_rules! impl_or_query_filter {
        ($(#[$meta:meta])* $($filter: ident),*) => {
            $(#[$meta])*
            #[expect(
                clippy::allow_attributes,
                reason = "This is a tuple-related macro; as such the lints below may not always apply."
            )]
            #[allow(
                non_snake_case,
                reason = "The names of some variables are provided by the macro's caller, not by us."
            )]
            #[allow(
                unused_variables,
                reason = "Zero-length tuples won't use any of the parameters."
            )]
            #[allow(
                clippy::unused_unit,
                reason = "Zero-length tuples will generate some function bodies equivalent to `()`; however, this macro is meant for all applicable tuples, and as such it makes no sense to rewrite it just for that case."
            )]
            impl<$($filter: QueryFilter,)*> FilteredAccessHolder for bevy_ecs::prelude::Or<($($filter,)*)> {
                const FILTERED_ACCESS: &'static FilteredAccess = impl_or_query_filter!(@tree $($filter,)*);
            }
        };

            // Base case
        (@tree) => {
            &FilteredAccess::Empty
        };
        // Inductive case
        (@tree $t0:ident, $($rest:ident,)*) => {
            &FilteredAccess::Node(
                $t0::FILTERED_ACCESS,
                bevy_ecs::prelude::Or::<($($rest,)*)>::FILTERED_ACCESS,
            )
        };
    }
    all_tuples!(impl_or_query_filter, 0, 15, T);
}
