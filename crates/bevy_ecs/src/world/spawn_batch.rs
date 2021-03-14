use crate::{
    archetype::{Archetype, ArchetypeId},
    bundle::{Bundle, BundleInfo},
    entity::{Entities, Entity},
    storage::{SparseSets, Table},
    world::{add_bundle_to_archetype, World},
};

pub struct SpawnBatchIter<'w, I>
where
    I: Iterator,
    I::Item: Bundle,
{
    inner: I,
    entities: &'w mut Entities,
    archetype: &'w mut Archetype,
    table: &'w mut Table,
    sparse_sets: &'w mut SparseSets,
    bundle_info: &'w BundleInfo,
}

impl<'w, I> SpawnBatchIter<'w, I>
where
    I: Iterator,
    I::Item: Bundle,
{
    #[inline]
    pub(crate) fn new(world: &'w mut World, iter: I) -> Self {
        // Ensure all entity allocations are accounted for so `self.entities` can realloc if
        // necessary
        world.flush();

        let (lower, upper) = iter.size_hint();

        let bundle_info = world.bundles.init_info::<I::Item>(&mut world.components);

        let length = upper.unwrap_or(lower);
        // SAFE: empty archetype exists and bundle components were initialized above
        let archetype_id = unsafe {
            add_bundle_to_archetype(
                &mut world.archetypes,
                &mut world.storages,
                &mut world.components,
                ArchetypeId::empty(),
                bundle_info,
            )
        };
        let archetype = &mut world.archetypes[archetype_id];
        let table = &mut world.storages.tables[archetype.table_id()];
        archetype.reserve(length);
        table.reserve(length);
        world.entities.reserve(length as u32);
        Self {
            inner: iter,
            entities: &mut world.entities,
            archetype,
            table,
            sparse_sets: &mut world.storages.sparse_sets,
            bundle_info,
        }
    }
}

impl<I> Drop for SpawnBatchIter<'_, I>
where
    I: Iterator,
    I::Item: Bundle,
{
    fn drop(&mut self) {
        for _ in self {}
    }
}

impl<I> Iterator for SpawnBatchIter<'_, I>
where
    I: Iterator,
    I::Item: Bundle,
{
    type Item = Entity;

    fn next(&mut self) -> Option<Entity> {
        let bundle = self.inner.next()?;
        let entity = self.entities.alloc();
        // SAFE: component values are immediately written to relevant storages (which have been
        // allocated)
        unsafe {
            let table_row = self.table.allocate(entity);
            let location = self.archetype.allocate(entity, table_row);
            let from_bundle = self
                .archetype
                .edges()
                .get_from_bundle(self.bundle_info.id)
                .unwrap();
            self.bundle_info.write_components(
                self.sparse_sets,
                entity,
                self.table,
                table_row,
                &from_bundle.bundle_flags,
                bundle,
            );
            self.entities.meta[entity.id as usize].location = location;
        }
        Some(entity)
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.inner.size_hint()
    }
}

impl<I, T> ExactSizeIterator for SpawnBatchIter<'_, I>
where
    I: ExactSizeIterator<Item = T>,
    T: Bundle,
{
    fn len(&self) -> usize {
        self.inner.len()
    }
}
