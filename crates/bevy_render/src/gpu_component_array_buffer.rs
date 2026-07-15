//! Lightweight storage of per-mesh-instance data.

use crate::storage::{ShaderBuffer, ShaderBufferData};

use alloc::borrow::Cow;
use bevy_app::{App, Plugin, PostUpdate};
use bevy_asset::{Assets, Handle, RenderAssetUsages};
use bevy_ecs::{
    component::Component,
    entity::{EntityHashMap, EntityHashSet},
    lifecycle::RemovedComponents,
    prelude::Entity,
    query::{QueryFilter, QueryItem, ReadOnlyQueryData},
    resource::Resource,
    system::{Commands, If, Local, Query, ResMut},
};
use bevy_mesh::MeshTag;
use bytemuck::Pod;
use core::marker::PhantomData;
use encase::ShaderType;
use std::iter;
use wgpu::BufferUsages;

/// Buffer sizes are rounded up to the nearest power of this value.
///
/// The value of 1.5 was chosen to provide a good balance between performance
/// and fragmentation.
const BUFFER_ALLOCATION_GROWTH_FACTOR: f64 = 1.5;

pub struct GpuComponentArrayBufferPlugin<C>(PhantomData<C>)
where
    C: GpuComponentArrayBuffer;

/// A lightweight mechanism to expose component data that may differ between
/// mesh instances to the GPU in the form of a [`ShaderBuffer`].
pub trait GpuComponentArrayBuffer: Component + Send + Sync + 'static {
    /// The query to perform to update the component data.
    type QueryData: ReadOnlyQueryData;
    /// An optional filter to apply to the query.
    type QueryFilter: QueryFilter;
    /// The packed GPU data.
    type Out: Pod + ShaderType + Default;

    /// Packs the component into a form suitable for the GPU.
    ///
    /// If `None` is returned, then the component data is removed from the
    /// component array buffer entirely.
    fn extract_component(item: QueryItem<'_, '_, Self::QueryData>) -> Option<Self::Out>;

    /// Override this function to supply a custom debugging label for the
    /// buffer.
    fn label() -> Cow<'static, str> {
        Cow::Borrowed("GPU component array")
    }

    /// Override this function to provide a custom set of allowed buffer usages
    /// for the buffer.
    fn buffer_usage() -> BufferUsages {
        BufferUsages::STORAGE | BufferUsages::COPY_DST | BufferUsages::COPY_SRC
    }
}

/// A resource, part of the main world, that stores the [`ShaderBuffer`] for a
/// [`GpuComponentArrayBuffer`] and maintains the mapping between entities and
/// entries within it.
#[derive(Resource)]
pub struct GpuComponentArray<C>
where
    C: GpuComponentArrayBuffer,
{
    pub buffer: Handle<ShaderBuffer>,
    entity_to_tag: EntityHashMap<u32>,
    tag_to_entity: Vec<Entity>,
    phantom: PhantomData<C>,
}

impl<C> Plugin for GpuComponentArrayBufferPlugin<C>
where
    C: GpuComponentArrayBuffer,
{
    fn build(&self, app: &mut App) {
        app.add_systems(PostUpdate, update_components::<C>);
    }
}

impl<C> Default for GpuComponentArrayBufferPlugin<C>
where
    C: GpuComponentArrayBuffer,
{
    fn default() -> Self {
        Self(PhantomData::<C>)
    }
}

impl<C> GpuComponentArray<C>
where
    C: GpuComponentArrayBuffer,
{
    /// Creates a new [`GpuComponentArray`] and the [`ShaderBuffer`] it writes
    /// into.
    pub fn new(shader_buffer_assets: &mut Assets<ShaderBuffer>) -> Self {
        let buffer = shader_buffer_assets.add(ShaderBuffer {
            data: ShaderBufferData::Initialized(vec![0; size_of::<C::Out>()]),
            label: C::label(),
            buffer_usage: C::buffer_usage(),
            asset_usage: RenderAssetUsages::MAIN_WORLD | RenderAssetUsages::RENDER_WORLD,
            copy_on_resize: true,
        });

        GpuComponentArray {
            buffer,
            entity_to_tag: EntityHashMap::default(),
            tag_to_entity: vec![],
            phantom: PhantomData,
        }
    }
}

/// A main-world system that runs the [`GpuComponentArrayBuffer`] query,
/// extracts the components, and writes them into the associated
/// [`ShaderBuffer`].
fn update_components<C>(
    mut commands: Commands,
    query: Query<(Entity, Option<&MeshTag>, C::QueryData), C::QueryFilter>,
    mut component_array: If<ResMut<GpuComponentArray<C>>>,
    mut shader_buffers: ResMut<Assets<ShaderBuffer>>,
    mut removed_components: RemovedComponents<C>,
    mut processed_entities: Local<EntityHashSet>,
) where
    C: GpuComponentArrayBuffer,
{
    let Some(mut buffer) = shader_buffers.get_mut(&mut component_array.buffer) else {
        return;
    };

    processed_entities.clear();

    for (entity, maybe_tag, item) in &query {
        match C::extract_component(item) {
            None => {
                if let Some((displaced_entity, displaced_entity_new_tag)) =
                    component_array.remove(&mut buffer, entity)
                {
                    commands
                        .entity(displaced_entity)
                        .insert(MeshTag(displaced_entity_new_tag));
                }
            }
            Some(data) => match maybe_tag {
                None => {
                    let tag = component_array.len();
                    component_array.push(&mut buffer, entity, data);
                    commands.entity(entity).insert(MeshTag(tag as u32));
                }
                Some(tag) => {
                    component_array.set(&mut buffer, tag.0, data);
                }
            },
        }

        processed_entities.insert(entity);
    }

    // Only remove from the component array if we didn't pick up the entity
    // above.
    // It's possible that the component was removed and then re-added in the
    // same frame.
    for entity in removed_components
        .read()
        .filter(|entity| !processed_entities.contains(entity))
    {
        if let Some((displaced_entity, displaced_entity_new_tag)) =
            component_array.remove(&mut buffer, entity)
        {
            commands
                .entity(displaced_entity)
                .insert(MeshTag(displaced_entity_new_tag));
        }
    }
}

impl<C> GpuComponentArray<C>
where
    C: GpuComponentArrayBuffer,
{
    /// Returns the number of items that this [`GpuComponentArray`] is managing.
    fn len(&self) -> usize {
        self.tag_to_entity.len()
    }

    /// Returns true if this [`GpuComponentArray`] is managing no items or false
    /// if it's managing at least one item.
    fn is_empty(&self) -> bool {
        self.tag_to_entity.is_empty()
    }

    /// Adds the `data` corresponding to the given entity to the end of the
    /// given shader buffer.
    ///
    /// Data for the given entity must not already be present in the buffer.
    fn push(&mut self, buffer: &mut ShaderBuffer, entity: Entity, data: C::Out) {
        let ShaderBufferData::Initialized(ref mut data_buffer) = buffer.data else {
            panic!(
                "Shader buffers created for use in a `GpuComponentArrayBuffer` must have been \
                created with `ShaderBufferData::Initialized`"
            );
        };
        if self.is_empty() {
            data_buffer.clear();
        }

        let tag = self.tag_to_entity.len() as u32;

        let needed_buffer_len = tag as usize + 1;
        let current_buffer_len = data_buffer.len() / size_of::<C::Out>();
        if needed_buffer_len > current_buffer_len {
            let next_buffer_len = round_buffer_size_up(needed_buffer_len);
            data_buffer.extend(iter::repeat_n(
                0,
                next_buffer_len * size_of::<C::Out>() - data_buffer.len(),
            ));
        }
        bytemuck::cast_slice_mut(data_buffer.as_mut_slice())[tag as usize] = data;

        let prev_tag = self.entity_to_tag.insert(entity, tag);
        self.tag_to_entity.push(entity);

        debug_assert!(prev_tag.is_none());
    }

    fn get<'a>(&'_ self, buffer: &'a ShaderBuffer, tag: u32) -> &'a C::Out {
        let ShaderBufferData::Initialized(ref data_buffer) = buffer.data else {
            panic!(
                "Shader buffers created for use in a `GpuComponentArrayBuffer` must have been \
                created with `ShaderBufferData::Initialized`"
            );
        };
        &bytemuck::cast_slice(data_buffer.as_slice())[tag as usize]
    }

    fn set(&mut self, buffer: &mut ShaderBuffer, tag: u32, data: C::Out) {
        let ShaderBufferData::Initialized(ref mut data_buffer) = buffer.data else {
            panic!(
                "Shader buffers created for use in a `GpuComponentArrayBuffer` must have been \
                created with `ShaderBufferData::Initialized`"
            );
        };
        bytemuck::cast_slice_mut(data_buffer.as_mut_slice())[tag as usize] = data;
    }

    fn remove(
        &mut self,
        buffer: &mut ShaderBuffer,
        entity_to_remove: Entity,
    ) -> Option<(Entity, u32)> {
        let displaced_tag = self.tag_to_entity.len() as u32 - 1;
        let displaced_data = *self.get(buffer, displaced_tag);
        let displaced_entity = *self.tag_to_entity.last().unwrap();

        let tag_to_remove = self.entity_to_tag.remove(&entity_to_remove)?;
        let removed_entity = self.tag_to_entity.swap_remove(tag_to_remove as usize);
        debug_assert_eq!(entity_to_remove, removed_entity);

        if tag_to_remove == displaced_tag {
            return None;
        }

        *self.entity_to_tag.get_mut(&displaced_entity).unwrap() = tag_to_remove;
        self.set(buffer, tag_to_remove, displaced_data);
        Some((displaced_entity, tag_to_remove))
    }
}

/// Rounds the buffer size up to a round size in order to amortize reallocation
/// when the buffer repeatedly grows.
///
/// The nearest power of [`BUFFER_ALLOCATION_GROWTH_FACTOR`], which need not be
/// an integer, is chosen.
fn round_buffer_size_up(original_size: usize) -> usize {
    if original_size <= 1 {
        return 1;
    }

    // Round up to the nearest power of `BUFFER_ALLOCATION_GROWTH_FACTOR`.
    let exponent = (original_size as f64).ln() / BUFFER_ALLOCATION_GROWTH_FACTOR.ln();
    BUFFER_ALLOCATION_GROWTH_FACTOR.powi(exponent.ceil() as i32) as usize
}
#[cfg(test)]
mod tests {
    use std::marker::PhantomData;

    use bevy_asset::{Handle, RenderAssetUsages};
    use bevy_ecs::{
        component::Component,
        entity::{Entity, EntityHashMap, EntityIndex},
        query::QueryItem,
    };
    use bytemuck::{Pod, Zeroable};
    use encase::ShaderType;
    use nonmax::NonMaxU32;

    use crate::storage::ShaderBuffer;

    use super::{GpuComponentArray, GpuComponentArrayBuffer};

    /// A mock CPU-side component that we pretend to extract to the GPU.
    #[derive(Component)]
    struct MockComponent;

    /// The mock GPU-side version of [`MockComponent`].
    #[derive(Clone, Copy, Default, PartialEq, Pod, Zeroable, ShaderType, Debug)]
    #[repr(C)]
    struct MockComponentData {
        a: u32,
        b: u32,
    }

    impl MockComponentData {
        fn new(a: u32, b: u32) -> Self {
            Self { a, b }
        }
    }

    impl GpuComponentArrayBuffer for MockComponent {
        type QueryData = ();
        type QueryFilter = ();
        type Out = MockComponentData;

        fn extract_component(_: QueryItem<'_, '_, Self::QueryData>) -> Option<Self::Out> {
            None
        }
    }

    /// Shared data that we use to perform our unit tests.
    struct TestData {
        /// The component array that the [`MockComponent`]s will be packed into.
        gpu_component_array: GpuComponentArray<MockComponent>,
        /// The GPU buffer that [`Self::gpu_component_array`] writes into.
        buffer: ShaderBuffer,

        entity_a: Entity,
        entity_b: Entity,
        entity_c: Entity,

        entity_data_a: MockComponentData,
        entity_data_b: MockComponentData,
        entity_data_c: MockComponentData,
    }

    impl TestData {
        fn new() -> TestData {
            TestData {
                gpu_component_array: GpuComponentArray::<MockComponent> {
                    buffer: Handle::default(),
                    entity_to_tag: EntityHashMap::default(),
                    tag_to_entity: vec![],
                    phantom: PhantomData,
                },
                buffer: ShaderBuffer::new(vec![], RenderAssetUsages::all()),

                // Create some entities.
                entity_a: Entity::from_index(EntityIndex::new(NonMaxU32::new(1).unwrap())),
                entity_b: Entity::from_index(EntityIndex::new(NonMaxU32::new(2).unwrap())),
                entity_c: Entity::from_index(EntityIndex::new(NonMaxU32::new(3).unwrap())),

                // Create some data to go with the entities.
                entity_data_a: MockComponentData::new(11, 111),
                entity_data_b: MockComponentData::new(22, 222),
                entity_data_c: MockComponentData::new(33, 333),
            }
        }

        /// Verifies that the GPU component array contains the given expected
        /// data, in that order.
        fn check(&self, expected_data: &[(Entity, MockComponentData)]) {
            assert_eq!(
                self.gpu_component_array.entity_to_tag.len(),
                expected_data.len()
            );
            assert_eq!(
                self.gpu_component_array.tag_to_entity.len(),
                expected_data.len()
            );
            assert!(self.buffer.len() >= expected_data.len() * size_of::<MockComponentData>());

            for (tag, (entity, data)) in expected_data.iter().enumerate() {
                assert_eq!(
                    self.gpu_component_array.entity_to_tag.get(entity),
                    Some(&(tag as u32))
                );
                assert_eq!(
                    self.gpu_component_array.tag_to_entity.get(tag),
                    Some(entity)
                );
                assert_eq!(self.gpu_component_array.get(&self.buffer, tag as u32), data);
            }
        }
    }

    /// Checks that an empty GPU component array is correct.
    #[test]
    fn empty_gpu_component_array() {
        let test_data = TestData::new();
        test_data.check(&[]);
    }

    /// Check that a GPU component array is correct after pushing data for some
    /// components onto it.
    #[test]
    fn push_onto_gpu_component_array() {
        let mut test_data = TestData::new();
        test_data.gpu_component_array.push(
            &mut test_data.buffer,
            test_data.entity_a,
            test_data.entity_data_a,
        );
        test_data.check(&[(test_data.entity_a, test_data.entity_data_a)]);
        test_data.gpu_component_array.push(
            &mut test_data.buffer,
            test_data.entity_b,
            test_data.entity_data_b,
        );
        test_data.check(&[
            (test_data.entity_a, test_data.entity_data_a),
            (test_data.entity_b, test_data.entity_data_b),
        ]);
        test_data.gpu_component_array.push(
            &mut test_data.buffer,
            test_data.entity_c,
            test_data.entity_data_c,
        );
        test_data.check(&[
            (test_data.entity_a, test_data.entity_data_a),
            (test_data.entity_b, test_data.entity_data_b),
            (test_data.entity_c, test_data.entity_data_c),
        ]);
    }

    // Check that a GPU component array is correct after pushing data for some
    // components onto it, then mutating one of those components.
    #[test]
    fn set_element_in_gpu_component_array() {
        let mut test_data = TestData::new();
        test_data.gpu_component_array.push(
            &mut test_data.buffer,
            test_data.entity_a,
            test_data.entity_data_a,
        );
        test_data.gpu_component_array.push(
            &mut test_data.buffer,
            test_data.entity_b,
            test_data.entity_data_b,
        );
        test_data.gpu_component_array.push(
            &mut test_data.buffer,
            test_data.entity_c,
            test_data.entity_data_c,
        );

        let entity_data_b_alt = MockComponentData::new(2222, 22222);

        test_data
            .gpu_component_array
            .set(&mut test_data.buffer, 1, entity_data_b_alt);
        test_data.check(&[
            (test_data.entity_a, test_data.entity_data_a),
            (test_data.entity_b, entity_data_b_alt),
            (test_data.entity_c, test_data.entity_data_c),
        ]);
    }

    // Check that a GPU component array is correct after pushing data for some
    // components onto it, then removing the final component.
    #[test]
    fn remove_element_from_end_of_gpu_component_array() {
        let mut test_data = TestData::new();
        test_data.gpu_component_array.push(
            &mut test_data.buffer,
            test_data.entity_a,
            test_data.entity_data_a,
        );
        test_data.gpu_component_array.push(
            &mut test_data.buffer,
            test_data.entity_b,
            test_data.entity_data_b,
        );
        test_data.gpu_component_array.push(
            &mut test_data.buffer,
            test_data.entity_c,
            test_data.entity_data_c,
        );

        let maybe_displaced_element = test_data
            .gpu_component_array
            .remove(&mut test_data.buffer, test_data.entity_c);
        assert_eq!(maybe_displaced_element, None);

        test_data.check(&[
            (test_data.entity_a, test_data.entity_data_a),
            (test_data.entity_b, test_data.entity_data_b),
        ]);
    }

    // Check that a GPU component array is correct after pushing data for some
    // components onto it, then removing the first component.
    #[test]
    fn remove_element_from_start_of_gpu_component_array() {
        let mut test_data = TestData::new();
        test_data.gpu_component_array.push(
            &mut test_data.buffer,
            test_data.entity_a,
            test_data.entity_data_a,
        );
        test_data.gpu_component_array.push(
            &mut test_data.buffer,
            test_data.entity_b,
            test_data.entity_data_b,
        );
        test_data.gpu_component_array.push(
            &mut test_data.buffer,
            test_data.entity_c,
            test_data.entity_data_c,
        );

        let maybe_displaced_element = test_data
            .gpu_component_array
            .remove(&mut test_data.buffer, test_data.entity_a);
        assert_eq!(maybe_displaced_element, Some((test_data.entity_c, 0)));

        test_data.check(&[
            (test_data.entity_c, test_data.entity_data_c),
            (test_data.entity_b, test_data.entity_data_b),
        ]);
    }
}
