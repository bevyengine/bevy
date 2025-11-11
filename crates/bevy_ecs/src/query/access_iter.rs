use crate::component::ComponentId;

/// The data storage type that is being accessed.
#[derive(Clone, Copy)]
pub enum EcsAccessType {
    /// Accesses [`Component`] data
    Component(EcsAccessLevel),
    /// Accesses [`Resource`] data
    Resource(ResourceAccessLevel),
}

/// The way the data will be accessed and whether we take access on all the components on
/// an entity or just one component.
#[derive(Clone, Copy)]
pub enum EcsAccessLevel {
    /// Reads [`Component`] with [`ComponentId`]
    Read(ComponentId),
    /// Writes [`Component`] with [`ComponentId`]
    Write(ComponentId),
    /// Potentially reads all [`Component`]'s in the [`World`]
    ReadAll,
    /// Potentially writes all [`Component`]'s in the [`World`]
    WriteAll,
    /// [`FilteredEntityRef`] captures it's access at the `SystemParam` level, so will
    /// not conflict with other `QueryData` in the same Query
    FilteredReadAll,
    /// [`FilteredEntityMut`] captures it's access at the `SystemParam` level, so will
    /// not conflict with other `QueryData` in the same Query
    FilteredWriteAll,
    /// Potentially reads all [`Components`]'s except [`ComponentId`]
    ReadAllExcept {
        /// used to group excepts from the same [`QueryData`] together
        index: usize,
        /// read all except this id
        component_id: ComponentId,
    },
    /// Potentially writes all [`Components`]'s except [`ComponentId`]
    WriteAllExcept {
        /// used to group excepts from the same [`QueryData`] together
        index: usize,
        /// write all except this id
        component_id: ComponentId,
    },
}

/// Access level needed by [`QueryData`] fetch to the resource.
#[derive(Copy, Clone)]
pub enum ResourceAccessLevel {
    /// Reads the resource with [`ComponentId`]
    Read(ComponentId),
    /// Writes the resource with [`ComponentId`]
    Write(ComponentId),
}

impl EcsAccessType {
    /// Returns true if the access between `self` and `other` do not conflict.
    pub fn is_compatible(&self, other: Self) -> bool {
        use EcsAccessLevel::*;
        use EcsAccessType::*;

        match (self, other) {
            (Component(ReadAll), Component(Write(_)))
            | (Component(Write(_)), Component(ReadAll))
            | (Component(_), Component(WriteAll))
            | (Component(WriteAll), Component(_)) 
            | (Component(WriteAllExcept { .. }), Component(ReadAllExcept { .. }))
            | (Component(ReadAllExcept { .. }), Component(WriteAllExcept { .. }))
            | (Component(WriteAllExcept { .. }), Component(ReadAll))
            | (Component(ReadAll), Component(WriteAllExcept { .. })) => false,

            (Component(Read(id)), Component(Write(id_other)))
            | (Component(Write(id)), Component(Read(id_other)))
            | (Component(Write(id)), Component(Write(id_other)))
            | (Resource(ResourceAccessLevel::Read(id)), Resource(ResourceAccessLevel::Write(id_other)))
            | (Resource(ResourceAccessLevel::Write(id)), Resource(ResourceAccessLevel::Read(id_other)))
            | (Resource(ResourceAccessLevel::Write(id)), Resource(ResourceAccessLevel::Write(id_other))) => *id != id_other,

            (Component(_), Resource(_))
            | (Resource(_), Component(_))
            | (Component(Read(_)), Component(Read(_)))
            | (Component(ReadAll), Component(Read(_)))
            | (Component(Read(_)), Component(ReadAll))
            | (Component(ReadAll), Component(ReadAll))
            | (Resource(ResourceAccessLevel::Read(_)), Resource(ResourceAccessLevel::Read(_)))
            // TODO: I think (FilterdReadAll, FilteredWriteAll) should probably conflict, but should
            // double check with the normal conflict check
            | (Component(FilteredReadAll), _)
            | (_, Component(FilteredReadAll))
            | (Component(FilteredWriteAll), _)
            | (_, Component(FilteredWriteAll))
            | (Component(ReadAllExcept { .. }), Component(Read(_))) 
            | (Component(Read(_)), Component(ReadAllExcept { .. })) 
            | (Component(ReadAllExcept { .. }), Component(ReadAll)) 
            | (Component(ReadAll), Component(ReadAllExcept { .. })) 
            | (Component(ReadAllExcept { .. }), Component(ReadAllExcept { .. }))=> true,

            (Component(ReadAllExcept { component_id: id, .. }), Component(Write(id_other))) |
            (Component(WriteAllExcept { component_id: id, .. }), Component(Read(id_other))) |
            (Component(WriteAllExcept { component_id: id, .. }), Component(Write(id_other))) | 
            (Component(Write(id)), Component(ReadAllExcept { component_id: id_other, .. })) |
            (Component(Read(id)), Component(WriteAllExcept { component_id: id_other, .. })) |
            (Component(Write(id)), Component(WriteAllExcept { component_id: id_other, .. })) => *id == id_other,
            
            (Component(WriteAllExcept { index, .. }), Component(WriteAllExcept { index: index_other, .. })) => *index == index_other, 
        }
    }
}
