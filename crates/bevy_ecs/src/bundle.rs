pub use bevy_ecs_macros::Bundle;

use crate::{
    archetype::ComponentStatus,
    component::{Component, ComponentId, ComponentTicks, Components, StorageType, TypeInfo},
    entity::Entity,
    storage::{SparseSetIndex, SparseSets, Table},
};
use bevy_ecs_macros::all_tuples;
use std::{any::TypeId, collections::HashMap};

/// An ordered collection of components, commonly used for spawning entities, and adding and
/// removing components in bulk.
///
/// In order to query for components in a bundle use [crate::query::WithBundle].
///
/// Typically, you will simply use `#[derive(Bundle)]` when creating your own `Bundle`.
/// The `Bundle` trait is automatically implemented for tuples of components:
/// `(ComponentA, ComponentB)` is a very convenient shorthand when working with one-off collections
/// of components. Note that both `()` and `(ComponentA, )` are valid tuples.
///
/// You can nest bundles like so:
/// ```
/// # use bevy_ecs::bundle::Bundle;
///
/// #[derive(Bundle)]
/// struct A {
///     x: i32,
///     y: u64,
/// }
///
/// #[derive(Bundle)]
/// struct B {
///     #[bundle]
///     a: A,
///     z: String,
/// }
/// ```
///
/// # Safety
/// [Bundle::type_info] must return the TypeInfo for each component type in the bundle, in the
/// _exact_ order that [Bundle::get_components] is called.
/// [Bundle::from_components] must call `func` exactly once for each [TypeInfo] returned by
/// [Bundle::type_info]
pub unsafe trait Bundle: Send + Sync + 'static {
    /// Gets this [Bundle]'s components type info, in the order of this bundle's Components
    fn type_info() -> Vec<TypeInfo>;

    /// Calls `func`, which should return data for each component in the bundle, in the order of
    /// this bundle's Components
    ///
    /// # Safety
    /// Caller must return data for each component in the bundle, in the order of this bundle's
    /// Components
    unsafe fn from_components(func: impl FnMut() -> *mut u8) -> Self
    where
        Self: Sized;

    /// Calls `func` on each value, in the order of this bundle's Components. This will
    /// "mem::forget" the bundle fields, so callers are responsible for dropping the fields if
    /// that is desirable.
    fn get_components(self, func: impl FnMut(*mut u8));
}

macro_rules! tuple_impl {
    ($($name: ident),*) => {
        /// SAFE: TypeInfo is returned in tuple-order. [Bundle::from_components] and [Bundle::get_components] use tuple-order
        unsafe impl<$($name: Component),*> Bundle for ($($name,)*) {
            fn type_info() -> Vec<TypeInfo> {
                vec![$(TypeInfo::of::<$name>()),*]
            }

            #[allow(unused_variables, unused_mut)]
            unsafe fn from_components(mut func: impl FnMut() -> *mut u8) -> Self {
                #[allow(non_snake_case)]
                let ($(mut $name,)*) = (
                    $(func().cast::<$name>(),)*
                );
                ($($name.read(),)*)
            }

            #[allow(unused_variables, unused_mut)]
            fn get_components(self, mut func: impl FnMut(*mut u8)) {
                #[allow(non_snake_case)]
                let ($(mut $name,)*) = self;
                $(
                    func((&mut $name as *mut $name).cast::<u8>());
                    std::mem::forget($name);
                )*
            }
        }
    }
}

all_tuples!(tuple_impl, 0, 15, C);

#[derive(Debug, Clone, Copy)]
pub struct BundleId(usize);

impl BundleId {
    #[inline]
    pub fn index(self) -> usize {
        self.0
    }
}

impl SparseSetIndex for BundleId {
    #[inline]
    fn sparse_set_index(&self) -> usize {
        self.index()
    }

    fn get_sparse_set_index(value: usize) -> Self {
        Self(value)
    }
}

pub struct BundleInfo {
    pub(crate) id: BundleId,
    pub(crate) component_ids: Vec<ComponentId>,
    pub(crate) storage_types: Vec<StorageType>,
}

impl BundleInfo {
    /// # Safety
    /// table row must exist, entity must be valid
    #[allow(clippy::too_many_arguments)]
    #[inline]
    pub(crate) unsafe fn write_components<T: Bundle>(
        &self,
        sparse_sets: &mut SparseSets,
        entity: Entity,
        table: &mut Table,
        table_row: usize,
        bundle_status: &[ComponentStatus],
        bundle: T,
        change_tick: u32,
    ) {
        // NOTE: get_components calls this closure on each component in "bundle order".
        // bundle_info.component_ids are also in "bundle order"
        let mut bundle_component = 0;
        bundle.get_components(|component_ptr| {
            let component_id = *self.component_ids.get_unchecked(bundle_component);
            match self.storage_types[bundle_component] {
                StorageType::Table => {
                    let column = table.get_column_mut(component_id).unwrap();
                    match bundle_status.get_unchecked(bundle_component) {
                        ComponentStatus::Added => {
                            column.initialize(
                                table_row,
                                component_ptr,
                                ComponentTicks::new(change_tick),
                            );
                        }
                        ComponentStatus::Mutated => {
                            column.replace(table_row, component_ptr, change_tick);
                        }
                    }
                }
                StorageType::SparseSet => {
                    let sparse_set = sparse_sets.get_mut(component_id).unwrap();
                    sparse_set.insert(entity, component_ptr, change_tick);
                }
            }
            bundle_component += 1;
        });
    }

    #[inline]
    pub fn id(&self) -> BundleId {
        self.id
    }

    #[inline]
    pub fn components(&self) -> &[ComponentId] {
        &self.component_ids
    }

    #[inline]
    pub fn storage_types(&self) -> &[StorageType] {
        &self.storage_types
    }
}

#[derive(Default)]
pub struct Bundles {
    bundle_infos: Vec<BundleInfo>,
    bundle_ids: HashMap<TypeId, BundleId>,
}

impl Bundles {
    #[inline]
    pub fn get(&self, bundle_id: BundleId) -> Option<&BundleInfo> {
        self.bundle_infos.get(bundle_id.index())
    }

    #[inline]
    pub fn get_id(&self, type_id: TypeId) -> Option<BundleId> {
        self.bundle_ids.get(&type_id).cloned()
    }

    pub(crate) fn init_info<'a, T: Bundle>(
        &'a mut self,
        components: &mut Components,
    ) -> &'a BundleInfo {
        let bundle_infos = &mut self.bundle_infos;
        let id = self.bundle_ids.entry(TypeId::of::<T>()).or_insert_with(|| {
            let type_info = T::type_info();
            let id = BundleId(bundle_infos.len());
            let bundle_info =
                initialize_bundle(std::any::type_name::<T>(), &type_info, id, components);
            bundle_infos.push(bundle_info);
            id
        });
        // SAFE: index either exists, or was initialized
        unsafe { self.bundle_infos.get_unchecked(id.0) }
    }
}

fn initialize_bundle(
    bundle_type_name: &'static str,
    type_info: &[TypeInfo],
    id: BundleId,
    components: &mut Components,
) -> BundleInfo {
    let mut component_ids = Vec::new();
    let mut storage_types = Vec::new();

    for type_info in type_info {
        let component_id = components.get_or_insert_with(type_info.type_id(), || type_info.clone());
        // SAFE: get_with_type_info ensures info was created
        let info = unsafe { components.get_info_unchecked(component_id) };
        component_ids.push(component_id);
        storage_types.push(info.storage_type());
    }

    let mut deduped = component_ids.clone();
    deduped.sort();
    deduped.dedup();
    if deduped.len() != component_ids.len() {
        panic!("Bundle {} has duplicate components", bundle_type_name);
    }

    BundleInfo {
        id,
        component_ids,
        storage_types,
    }
}
