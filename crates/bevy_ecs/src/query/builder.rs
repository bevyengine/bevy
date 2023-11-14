use std::marker::PhantomData;

use crate::{component::ComponentId, prelude::*};

use super::{FilteredAccess, ReadOnlyWorldQuery, WorldQuery};

/// Builder struct to create [`QueryState`] instances at runtime.
///
/// ```
/// # use bevy_ecs::prelude::*;
/// #
/// # #[derive(Component)]
/// # struct A;
/// #
/// # #[derive(Component)]
/// # struct B;
/// #
/// # #[derive(Component)]
/// # struct C;
/// #
/// let mut world = World::new();
/// let entity_a = world.spawn((A, B)).id();
/// let entity_b = world.spawn((A, C)).id();
///
/// // Instantiate the builder using the type signature of the iterator you will consume
/// let mut query = QueryBuilder::<(Entity, &B)>::new(&mut world)
/// // Add additional terms through builder methods
///     .with::<A>()
///     .without::<C>()
///     .build();
///
/// // Consume the QueryState
/// let (entity, b) = query.single(&world);
///```
pub struct QueryBuilder<'w, Q: WorldQuery = (), F: ReadOnlyWorldQuery = ()> {
    access: FilteredAccess<ComponentId>,
    world: &'w mut World,
    or: bool,
    first: bool,
    _marker: PhantomData<(Q, F)>,
}

impl<'w, Q: WorldQuery, F: ReadOnlyWorldQuery> QueryBuilder<'w, Q, F> {
    /// Creates a new builder with the accesses required for `Q` and `F`
    pub fn new(world: &'w mut World) -> Self {
        let fetch_state = Q::init_state(world);
        let filter_state = F::init_state(world);

        let mut access = FilteredAccess::default();
        Q::update_component_access(&fetch_state, &mut access);

        // Use a temporary empty FilteredAccess for filters. This prevents them from conflicting with the
        // main Query's `fetch_state` access. Filters are allowed to conflict with the main query fetch
        // because they are evaluated *before* a specific reference is constructed.
        let mut filter_access = FilteredAccess::default();
        F::update_component_access(&filter_state, &mut filter_access);

        // Merge the temporary filter access with the main access. This ensures that filter access is
        // properly considered in a global "cross-query" context (both within systems and across systems).
        access.extend(&filter_access);

        Self {
            access,
            world,
            or: false,
            first: false,
            _marker: PhantomData,
        }
    }

    /// Returns a reference to the world passed to [`Self::new`].
    pub fn world(&self) -> &World {
        self.world
    }

    /// Returns a reference to the world passed to [`Self::new`].
    pub fn world_mut(&mut self) -> &mut World {
        self.world
    }

    /// Adds access to self's underlying [`FilteredAccess`] respecting [`Self::or`] and [`Self::and`]
    pub fn extend_access(&mut self, mut access: FilteredAccess<ComponentId>) {
        if self.or {
            if self.first {
                access.required.clear();
                self.access.extend(&access);
                self.first = false;
            } else {
                self.access.append_or(&access);
            }
        } else {
            self.access.extend(&access);
        }
    }

    /// Adds accesses required for `T` to self.
    pub fn push<T: WorldQuery>(&mut self) -> &mut Self {
        let state = T::init_state(self.world);
        let mut access = FilteredAccess::default();
        T::update_component_access(&state, &mut access);
        self.extend_access(access);
        self
    }

    /// Adds [`With<T>`] to the [`FilteredAccess`] of self.
    pub fn with<T: Component>(&mut self) -> &mut Self {
        self.push::<With<T>>();
        self
    }

    /// Adds [`With<T>`] to the [`FilteredAccess`] of self from a runtime [`ComponentId`].
    pub fn with_id(&mut self, id: ComponentId) -> &mut Self {
        let mut access = FilteredAccess::default();
        access.and_with(id);
        self.extend_access(access);
        self
    }

    /// Adds [`Without<T>`] to the [`FilteredAccess`] of self.
    pub fn without<T: Component>(&mut self) -> &mut Self {
        self.push::<Without<T>>();
        self
    }

    /// Adds [`Without<T>`] to the [`FilteredAccess`] of self from a runtime [`ComponentId`].
    pub fn without_id(&mut self, id: ComponentId) -> &mut Self {
        let mut access = FilteredAccess::default();
        access.and_without(id);
        self.extend_access(access);
        self
    }

    /// Adds `&T` to the [`FilteredAccess`] of self.
    pub fn ref_id(&mut self, id: ComponentId) -> &mut Self {
        self.with_id(id);
        self.access.add_read(id);
        self
    }

    /// Adds `&mut T` to the [`FilteredAccess`] of self.
    pub fn mut_id(&mut self, id: ComponentId) -> &mut Self {
        self.with_id(id);
        self.access.add_write(id);
        self
    }

    /// Takes a function over mutable access to a [`QueryBuilder`], calls that function
    /// on an empty builder and then adds all accesses from that builder to self as optional.
    pub fn optional(&mut self, f: impl Fn(&mut QueryBuilder)) -> &mut Self {
        let mut builder = QueryBuilder::new(self.world);
        f(&mut builder);
        self.access.extend_access(builder.access());
        self
    }

    /// Takes a function over mutable access to a [`QueryBuilder`], calls that function
    /// on an empty builder and then adds all accesses from that builder to self.
    ///
    /// Primarily used when inside a [`Self::or`] closure to group several terms.
    pub fn and(&mut self, f: impl Fn(&mut QueryBuilder)) -> &mut Self {
        let mut builder = QueryBuilder::new(self.world);
        f(&mut builder);
        let access = builder.access().clone();
        self.extend_access(access);
        self
    }

    /// Takes a function over mutable access to a [`QueryBuilder`], calls that function
    /// on an empty builder, all accesses added to that builder will become terms in an or expression.
    ///
    /// ```
    /// # use bevy_ecs::prelude::*;
    /// #
    /// # #[derive(Component)]
    /// # struct A;
    /// #
    /// # #[derive(Component)]
    /// # struct B;
    /// #
    /// # let mut world = World::new();
    /// #
    /// QueryBuilder::<Entity>::new(&mut world).or(|builder| {
    ///     builder.with::<A>();
    ///     builder.with::<B>();
    /// });
    /// // is equivalent to
    /// QueryBuilder::<Entity>::new(&mut world).push::<Or<(With<A>, With<B>)>>();
    /// ```
    pub fn or(&mut self, f: impl Fn(&mut QueryBuilder)) -> &mut Self {
        let mut builder = QueryBuilder::new(self.world);
        builder.or = true;
        builder.first = true;
        f(&mut builder);
        self.access.extend(builder.access());
        self
    }

    /// Returns a reference to the the [`FilteredAccess`] that will be provided to the built [`Query`].
    pub fn access(&self) -> &FilteredAccess<ComponentId> {
        &self.access
    }

    /// Transmute the existing builder adding erquired accesses.
    /// This will maintain all exisiting accesses.
    ///
    /// If including a filter type see [`Self::transmute_filtered`]
    pub fn transmute<NewQ: WorldQuery>(&mut self) -> &mut QueryBuilder<'w, NewQ> {
        self.transmute_filtered::<NewQ, ()>()
    }

    /// Transmute the existing builder adding erquired accesses.
    /// This will maintain all existing accesses.
    pub fn transmute_filtered<NewQ: WorldQuery, NewF: ReadOnlyWorldQuery>(
        &mut self,
    ) -> &mut QueryBuilder<'w, NewQ, NewF> {
        let mut fetch_state = NewQ::init_state(self.world);
        let filter_state = NewF::init_state(self.world);

        NewQ::set_access(&mut fetch_state, &self.access);

        let mut access = FilteredAccess::default();
        NewQ::update_component_access(&fetch_state, &mut access);
        NewF::update_component_access(&filter_state, &mut access);

        self.extend_access(access);
        // SAFETY:
        // - We have included all requried acceses for NewQ and NewF
        // - The layout of all QueryBuilder instances is the same
        unsafe { std::mem::transmute(self) }
    }

    /// Create a [`QueryState`] with the accesses of the builder.
    pub fn build(&mut self) -> QueryState<Q, F> {
        QueryState::<Q, F>::from_builder(self)
    }
}

#[cfg(test)]
mod tests {
    use crate as bevy_ecs;
    use crate::prelude::*;
    use crate::world::FilteredEntityRef;

    use super::QueryBuilder;

    #[derive(Component, PartialEq, Debug)]
    struct A(usize);

    #[derive(Component, PartialEq, Debug)]
    struct B(usize);

    #[derive(Component, PartialEq, Debug)]
    struct C(usize);

    #[test]
    fn builder_with_without_static() {
        let mut world = World::new();
        let entity_a = world.spawn((A(0), B(0))).id();
        let entity_b = world.spawn((A(0), C(0))).id();

        let mut query_a = QueryBuilder::<Entity>::new(&mut world)
            .with::<A>()
            .without::<C>()
            .build();
        assert_eq!(entity_a, query_a.single(&world));

        let mut query_b = QueryBuilder::<Entity>::new(&mut world)
            .with::<A>()
            .without::<B>()
            .build();
        assert_eq!(entity_b, query_b.single(&world));
    }

    #[test]
    fn builder_with_without_dynamic() {
        let mut world = World::new();
        let entity_a = world.spawn((A(0), B(0))).id();
        let entity_b = world.spawn((A(0), C(0))).id();
        let component_id_a = world.init_component::<A>();
        let component_id_b = world.init_component::<B>();
        let component_id_c = world.init_component::<C>();

        let mut query_a = QueryBuilder::<Entity>::new(&mut world)
            .with_id(component_id_a)
            .without_id(component_id_c)
            .build();
        assert_eq!(entity_a, query_a.single(&world));

        let mut query_b = QueryBuilder::<Entity>::new(&mut world)
            .with_id(component_id_a)
            .without_id(component_id_b)
            .build();
        assert_eq!(entity_b, query_b.single(&world));
    }

    #[test]
    fn builder_or() {
        let mut world = World::new();
        world.spawn((A(0), B(0)));
        world.spawn(B(0));
        world.spawn(C(0));

        let mut query_a = QueryBuilder::<Entity>::new(&mut world)
            .or(|builder| {
                builder.with::<A>();
                builder.with::<B>();
            })
            .build();
        assert_eq!(2, query_a.iter(&world).count());

        let mut query_b = QueryBuilder::<Entity>::new(&mut world)
            .or(|builder| {
                builder.with::<A>();
                builder.without::<B>();
            })
            .build();
        dbg!(&query_b.component_access);
        assert_eq!(2, query_b.iter(&world).count());

        let mut query_c = QueryBuilder::<Entity>::new(&mut world)
            .or(|builder| {
                builder.with::<A>();
                builder.with::<B>();
                builder.with::<C>();
            })
            .build();
        assert_eq!(3, query_c.iter(&world).count());
    }

    #[test]
    fn builder_transmute() {
        let mut world = World::new();
        world.spawn(A(0));
        world.spawn((A(1), B(0)));
        let mut query = QueryBuilder::<()>::new(&mut world)
            .with::<B>()
            .transmute::<&A>()
            .build();

        query.iter(&world).for_each(|a| assert_eq!(a.0, 1));
    }

    #[test]
    fn builder_static_components() {
        let mut world = World::new();
        let entity = world.spawn((A(0), B(1))).id();

        let mut query = QueryBuilder::<FilteredEntityRef>::new(&mut world)
            .push::<&A>()
            .push::<&B>()
            .build();

        let entity_ref = query.single(&world);

        assert_eq!(entity, entity_ref.id());

        let a = entity_ref.get::<A>().unwrap();
        let b = entity_ref.get::<B>().unwrap();

        assert_eq!(0, a.0);
        assert_eq!(1, b.0);
    }

    #[test]
    fn builder_dynamic_components() {
        let mut world = World::new();
        let entity = world.spawn((A(0), B(1))).id();
        let component_id_a = world.init_component::<A>();
        let component_id_b = world.init_component::<B>();

        let mut query = QueryBuilder::<FilteredEntityRef>::new(&mut world)
            .ref_id(component_id_a)
            .ref_id(component_id_b)
            .build();

        let entity_ref = query.single(&world);

        assert_eq!(entity, entity_ref.id());

        let a = entity_ref.get_by_id(component_id_a).unwrap();
        let b = entity_ref.get_by_id(component_id_b).unwrap();

        // SAFETY: We set these pointers to point to these components
        unsafe {
            assert_eq!(0, a.deref::<A>().0);
            assert_eq!(1, b.deref::<B>().0);
        }
    }

    #[test]
    fn builder_query_system() {
        let mut world = World::new();
        world.spawn(A(0));
        let entity = world.spawn((A(1), B(0))).id();

        let sys = move |query: Query<(Entity, &A)>| {
            let (e, a) = query.single();
            assert_eq!(e, entity);
            assert_eq!(1, a.0);
        };

        // Add additional terms that don't appear in the original query
        let query = QueryBuilder::<(Entity, &A)>::new(&mut world)
            .with::<B>()
            .build();
        let mut system = IntoSystem::into_system(sys);
        system.initialize(&mut world);

        // SAFETY: We know the system param we are modifying has a compatible type signature
        unsafe { system.state_mut().0 = query };
        system.run((), &mut world);

        // Alternatively truncate terms from a query to match the system
        let query = QueryBuilder::<(Entity, &A, &B)>::new(&mut world).build();
        let mut system = IntoSystem::into_system(sys);
        system.initialize(&mut world);

        // SAFETY: We know the system param we are modifying has a compatible type signature
        unsafe { system.state_mut().0 = query.transmute(&world) };
        system.run((), &mut world);
    }
}
