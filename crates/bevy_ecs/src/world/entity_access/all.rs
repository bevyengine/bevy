use crate::{
    query::{has_conflicts, Access, QueryAccessError, ReadOnlyQueryData, ReleaseStateQueryData},
    world::{All, EntityMut, EntityRef, Filtered, FilteredEntityMut, FilteredEntityRef},
};

impl<'w> EntityRef<'w, All> {
    /// Consumes `self` and returns a [`FilteredEntityRef`] with read access to
    /// all components.
    pub fn into_filtered(self) -> FilteredEntityRef<'w, 'static> {
        // SAFETY:
        // - `Access:new_read_all` equals the read permissions of `self`'s `All` access.
        unsafe { EntityRef::new(self.cell, Filtered(const { &Access::new_read_all() })) }
    }

    /// Returns read-only components for the current entity that match the query `Q`.
    ///
    /// # Panics
    ///
    /// If the entity does not have the components required by the query `Q`.
    pub fn components<Q: ReadOnlyQueryData + ReleaseStateQueryData>(&self) -> Q::Item<'w, 'static> {
        self.get_components::<Q>()
            .expect("Query does not match the current entity")
    }

    /// Returns read-only components for the current entity that match the query `Q`,
    /// or `None` if the entity does not have the components required by the query `Q`.
    pub fn get_components<Q: ReadOnlyQueryData + ReleaseStateQueryData>(
        &self,
    ) -> Result<Q::Item<'w, 'static>, QueryAccessError> {
        // SAFETY:
        // - We have read-only access to all components of this entity.
        // - The query is read-only, and read-only references cannot have conflicts.
        unsafe { self.cell.get_components::<Q>() }
    }
}

impl<'w> From<EntityRef<'w, All>> for FilteredEntityRef<'w, 'static> {
    #[inline]
    fn from(entity: EntityRef<'w, All>) -> Self {
        entity.into_filtered()
    }
}

impl<'w> From<&EntityRef<'w, All>> for FilteredEntityRef<'w, 'static> {
    #[inline]
    fn from(entity: &EntityRef<'w, All>) -> Self {
        entity.into_filtered()
    }
}

impl<'w> EntityMut<'w, All> {
    /// Consumes `self` and returns a [`FilteredEntityMut`] with read and write
    /// access to all components.
    #[inline]
    pub fn into_filtered(self) -> FilteredEntityMut<'w, 'static> {
        // SAFETY:
        // - `Access::new_write_all` equals the read and write permissions of `entity`'s `All` access.
        // - Consuming `self` ensures there are no other accesses.
        unsafe { EntityMut::new(self.cell, Filtered(const { &Access::new_write_all() })) }
    }

    /// Returns read-only components for the current entity that match the query `Q`.
    ///
    /// # Panics
    ///
    /// If the entity does not have the components required by the query `Q`.
    pub fn components<Q: ReadOnlyQueryData + ReleaseStateQueryData>(&self) -> Q::Item<'_, 'static> {
        self.as_readonly().components::<Q>()
    }

    /// Returns read-only components for the current entity that match the query `Q`,
    /// or `None` if the entity does not have the components required by the query `Q`.
    pub fn get_components<Q: ReadOnlyQueryData + ReleaseStateQueryData>(
        &self,
    ) -> Result<Q::Item<'_, 'static>, QueryAccessError> {
        self.as_readonly().get_components::<Q>()
    }

    /// Returns components for the current entity that match the query `Q`.
    /// In the case of conflicting [`QueryData`](crate::query::QueryData), unregistered components, or missing components,
    /// this will return a [`QueryAccessError`]
    ///
    /// # Example
    ///
    /// ```
    /// # use bevy_ecs::prelude::*;
    /// #
    /// #[derive(Component)]
    /// struct X(usize);
    /// #[derive(Component)]
    /// struct Y(usize);
    ///
    /// # let mut world = World::default();
    /// let mut entity = world.spawn((X(0), Y(0))).into_mutable();
    /// // Get mutable access to two components at once
    /// // SAFETY: X and Y are different components
    /// let (mut x, mut y) = entity.get_components_mut::<(&mut X, &mut Y)>().unwrap();
    /// ```
    ///
    /// Note that this does a O(n^2) check that the [`QueryData`](crate::query::QueryData) does not conflict. If performance is a
    /// consideration you should use [`Self::get_components_mut_unchecked`] instead.
    pub fn get_components_mut<Q: ReleaseStateQueryData>(
        &mut self,
    ) -> Result<Q::Item<'_, 'static>, QueryAccessError> {
        self.reborrow().into_components_mut::<Q>()
    }

    /// Returns components for the current entity that match the query `Q`,
    /// or `None` if the entity does not have the components required by the query `Q`.
    ///
    /// # Example
    ///
    /// ```
    /// # use bevy_ecs::prelude::*;
    /// #
    /// #[derive(Component)]
    /// struct X(usize);
    /// #[derive(Component)]
    /// struct Y(usize);
    ///
    /// # let mut world = World::default();
    /// let mut entity = world.spawn((X(0), Y(0))).into_mutable();
    /// // Get mutable access to two components at once
    /// // SAFETY: X and Y are different components
    /// let (mut x, mut y) =
    ///     unsafe { entity.get_components_mut_unchecked::<(&mut X, &mut Y)>() }.unwrap();
    /// *x = X(1);
    /// *y = Y(1);
    /// // This would trigger undefined behavior, as the `&mut X`s would alias:
    /// // entity.get_components_mut_unchecked::<(&mut X, &mut X)>();
    /// ```
    ///
    /// # Safety
    /// It is the caller's responsibility to ensure that
    /// the `QueryData` does not provide aliasing mutable references to the same component.
    pub unsafe fn get_components_mut_unchecked<Q: ReleaseStateQueryData>(
        &mut self,
    ) -> Result<Q::Item<'_, 'static>, QueryAccessError> {
        // SAFETY: Caller the `QueryData` does not provide aliasing mutable references to the same component
        unsafe { self.reborrow().into_components_mut_unchecked::<Q>() }
    }

    /// Consumes self and returns components for the current entity that match the query `Q` for the world lifetime `'w`,
    /// or `None` if the entity does not have the components required by the query `Q`.
    ///
    /// The checks for aliasing mutable references may be expensive.
    /// If performance is a concern, consider making multiple calls to [`Self::get_mut`].
    /// If that is not possible, consider using [`Self::into_components_mut_unchecked`] to skip the checks.
    ///
    /// # Panics
    ///
    /// If the `QueryData` provides aliasing mutable references to the same component.
    ///
    /// # Example
    ///
    /// ```
    /// # use bevy_ecs::prelude::*;
    /// #
    /// #[derive(Component)]
    /// struct X(usize);
    /// #[derive(Component)]
    /// struct Y(usize);
    ///
    /// # let mut world = World::default();
    /// let mut entity = world.spawn((X(0), Y(0))).into_mutable();
    /// // Get mutable access to two components at once
    /// let (mut x, mut y) = entity.into_components_mut::<(&mut X, &mut Y)>().unwrap();
    /// *x = X(1);
    /// *y = Y(1);
    /// ```
    ///
    /// ```should_panic
    /// # use bevy_ecs::prelude::*;
    /// #
    /// # #[derive(Component)]
    /// # struct X(usize);
    /// #
    /// # let mut world = World::default();
    /// let mut entity = world.spawn((X(0))).into_mutable();
    /// // This panics, as the `&mut X`s would alias:
    /// entity.into_components_mut::<(&mut X, &mut X)>();
    /// ```
    pub fn into_components_mut<Q: ReleaseStateQueryData>(
        self,
    ) -> Result<Q::Item<'w, 'static>, QueryAccessError> {
        has_conflicts::<Q>(self.cell.world().components())?;

        // SAFETY: we checked that there were not conflicting components above
        unsafe { self.into_components_mut_unchecked::<Q>() }
    }

    /// Consumes self and returns components for the current entity that match the query `Q` for the world lifetime `'w`,
    /// or `None` if the entity does not have the components required by the query `Q`.
    ///
    /// # Example
    ///
    /// ```
    /// # use bevy_ecs::prelude::*;
    /// #
    /// #[derive(Component)]
    /// struct X(usize);
    /// #[derive(Component)]
    /// struct Y(usize);
    ///
    /// # let mut world = World::default();
    /// let mut entity = world.spawn((X(0), Y(0))).into_mutable();
    /// // Get mutable access to two components at once
    /// // SAFETY: X and Y are different components
    /// let (mut x, mut y) =
    ///     unsafe { entity.into_components_mut_unchecked::<(&mut X, &mut Y)>() }.unwrap();
    /// *x = X(1);
    /// *y = Y(1);
    /// // This would trigger undefined behavior, as the `&mut X`s would alias:
    /// // entity.into_components_mut_unchecked::<(&mut X, &mut X)>();
    /// ```
    ///
    /// # Safety
    /// It is the caller's responsibility to ensure that
    /// the `QueryData` does not provide aliasing mutable references to the same component.
    ///
    /// # See also
    ///
    /// - [`Self::into_components_mut`] for the safe version that performs aliasing checks
    pub unsafe fn into_components_mut_unchecked<Q: ReleaseStateQueryData>(
        self,
    ) -> Result<Q::Item<'w, 'static>, QueryAccessError> {
        // SAFETY:
        // - We have mutable access to all components of this entity.
        // - Caller asserts the `QueryData` does not provide aliasing mutable references to the same component
        unsafe { self.cell.get_components::<Q>() }
    }
}

impl<'w> From<EntityMut<'w, All>> for FilteredEntityRef<'w, 'static> {
    #[inline]
    fn from(entity: EntityMut<'w, All>) -> Self {
        entity.into_readonly().into_filtered()
    }
}

impl<'w> From<&'w EntityMut<'_, All>> for FilteredEntityRef<'w, 'static> {
    #[inline]
    fn from(entity: &'w EntityMut<'_, All>) -> Self {
        entity.as_readonly().into_filtered()
    }
}

impl<'w> From<EntityMut<'w, All>> for FilteredEntityMut<'w, 'static> {
    #[inline]
    fn from(entity: EntityMut<'w, All>) -> Self {
        entity.into_filtered()
    }
}

impl<'w> From<&'w mut EntityMut<'_, All>> for FilteredEntityMut<'w, 'static> {
    #[inline]
    fn from(entity: &'w mut EntityMut<'_, All>) -> Self {
        entity.reborrow().into_filtered()
    }
}
