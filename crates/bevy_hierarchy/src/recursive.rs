use bevy_ecs::{
    entity::Entity,
    query::{QueryData, QueryFilter, With, WorldQuery},
    system::{Query, SystemParam},
};

use crate::{Children, Parent};

/// [`SystemParam`] that provide mutable hierarchical access to data stored in the world.
///
/// When queried, we recursively look for entities from parent to child, along the queries defined by the user.
///
/// [`QueryRecursive`] is a generic data structure that accepts `5` type parameters:
///
/// * `QShared` is a [`QueryData`] present on all entities, a read only reference of this item is passed down from parent to child during iteration.
/// * `QRoot` is a [`QueryData`] present on root entities only.
/// * `QChild` is a [`QueryData`] present on child entities only.
/// * `FRoot` is a [`QueryFilter`] for root entities.
/// * `FChild` is a [`QueryFilter`] for child entities, [`With<Parent>`] is automatically added.
///
/// The user is responsible for excluding all child entities in `root`
/// and make sure entities in `root` are not ancestors of each other,
/// for example using `FRoot` `Without<Parent>`.
///
/// # Example
///
/// A naive transform pipeline implementation.
/// ```
/// # use bevy_ecs::prelude::*;
/// # use bevy_hierarchy::prelude::*;
/// # #[derive(Clone, Copy, Component)] pub struct Transform;
/// # #[derive(Clone, Copy, Component)] pub struct GlobalTransform;
/// #
/// # impl Transform {
/// #    fn into(self) -> GlobalTransform {
/// #        GlobalTransform
/// #    }
/// # }
/// #
/// # impl GlobalTransform {
/// #    fn mul_transform(self, global: Transform) -> GlobalTransform {
/// #        GlobalTransform
/// #    }
/// # }
/// fn propagate_transforms(mut query: QueryRecursive<
///     (&Transform, &mut GlobalTransform),
///     (),
///     (),
///     Without<Parent>,
///     ()
/// >) {
///     query.for_each_mut(
///         |(transform, global_transform), ()|
///             **global_transform = (**transform).into(),
///         |(_, parent), (transform, global_transform), ()|
///             **global_transform = parent.mul_transform(**transform),
///     );
/// }
/// ```
///
/// # Panics
///
/// If hierarchy is malformed, for example if a parent child mismatch or a cycle is found.
#[derive(SystemParam)]
pub struct QueryRecursive<
    'w,
    's,
    QShared: QueryData + 'static,
    QRoot: QueryData + 'static,
    QChild: QueryData + 'static,
    FRoot: QueryFilter + 'static,
    FChild: QueryFilter + 'static,
> {
    root: Query<'w, 's, (Entity, QShared, QRoot, Option<&'static Children>), FRoot>,
    children: Query<'w, 's, (QShared, QChild, Option<&'static Children>), (FChild, With<Parent>)>,
    parent: Query<'w, 's, (Entity, &'static Parent)>,
}

type Item<'t, T> = <T as WorldQuery>::Item<'t>;
type ReadItem<'t, T> = <<T as QueryData>::ReadOnly as WorldQuery>::Item<'t>;

impl<
        QShared: QueryData + 'static,
        QRoot: QueryData + 'static,
        QChild: QueryData + 'static,
        FRoot: QueryFilter + 'static,
        FChild: QueryFilter + 'static,
    > QueryRecursive<'_, '_, QShared, QRoot, QChild, FRoot, FChild>
{
    /// Iterate through the [`QueryRecursive`] hierarchy.
    /// Children receives a readonly reference to their parent's `QShared` [`QueryData`] as the first argument.
    ///
    /// # Panics
    ///
    /// If hierarchy is malformed, for example if a parent child mismatch or a cycle is found.
    pub fn for_each(
        &self,
        root_fn: impl FnMut(&ReadItem<QShared>, ReadItem<QRoot>),
        mut child_fn: impl FnMut(&ReadItem<QShared>, &ReadItem<QShared>, ReadItem<QChild>),
    ) {
        self.for_each_with(root_fn, |a, _, b, c| child_fn(a, b, c));
    }

    /// Mutably iterate through the [`QueryRecursive`] hierarchy.
    /// Children receives a readonly reference to their parent's `QShared` [`QueryData`] as the first argument.
    ///
    /// # Panics
    ///
    /// If hierarchy is malformed, for example if a parent child mismatch or a cycle is found.
    pub fn for_each_mut(
        &mut self,
        root_fn: impl FnMut(&mut Item<QShared>, Item<QRoot>),
        mut child_fn: impl FnMut(&Item<QShared>, &mut Item<QShared>, Item<QChild>),
    ) {
        self.for_each_mut_with(root_fn, |a, _, b, c| child_fn(a, b, c));
    }

    /// Iterate through the [`QueryRecursive`] hierarchy while passing down an evaluation result from parent to child.
    /// Children also receives a readonly reference to their parent's `QShared` [`QueryData`] as the first argument.
    ///
    /// # Panics
    ///
    /// If hierarchy is malformed, for example if a parent child mismatch or a cycle is found.
    #[allow(unsafe_code)]
    pub fn for_each_with<T: 'static>(
        &self,
        mut root_fn: impl FnMut(&ReadItem<QShared>, ReadItem<QRoot>) -> T,
        mut child_fn: impl FnMut(&ReadItem<QShared>, &T, &ReadItem<QShared>, ReadItem<QChild>) -> T,
    ) {
        for (actual_root, shared, owned, children) in self.root.iter() {
            let info = root_fn(&shared, owned);
            let Some(children) = children else {
                continue;
            };
            for entity in children {
                // Safety: `self.children` is not fetched while this is running.
                unsafe {
                    propagate(
                        actual_root,
                        &shared,
                        &info,
                        &self.children.to_readonly(),
                        &self.parent,
                        *entity,
                        &mut |a, b, c, d| child_fn(a, b, c, d),
                    );
                };
            }
        }
    }

    /// Mutably iterate through the [`QueryRecursive`] hierarchy while passing down an evaluation result from parent to child.
    /// Children also receives a readonly reference to their parent's `QShared` [`QueryData`] as the first argument.
    ///
    /// # Panics
    ///
    /// If hierarchy is malformed, for example if a parent child mismatch or a cycle is found.
    #[allow(unsafe_code)]
    pub fn for_each_mut_with<T: 'static>(
        &mut self,
        mut root_fn: impl FnMut(&mut Item<QShared>, Item<QRoot>) -> T,
        mut child_fn: impl FnMut(&Item<QShared>, &T, &mut Item<QShared>, Item<QChild>) -> T,
    ) {
        for (actual_root, mut shared, owned, children) in self.root.iter_mut() {
            let info = root_fn(&mut shared, owned);
            let Some(children) = children else {
                continue;
            };
            for entity in children {
                // Safety: `self.children` is not fetched while this is running.
                unsafe {
                    propagate(
                        actual_root,
                        &shared,
                        &info,
                        &self.children,
                        &self.parent,
                        *entity,
                        &mut child_fn,
                    );
                };
            }
        }
    }

    // Note: if to be implemented in the future,
    // `par_for_each_mut` is always unsafe unless we
    // can guarantee root nodes are not ancestors of each other.
    // This can be guaranteed if `Without<Parent>` is forced or verified to exist.
}

/// Recursively run a function on descendants, passing immutable references of parent to child.
///
/// # Panics
///
/// If `entity`'s descendants have a malformed hierarchy, this function will panic.
///
/// # Safety
///
/// - While this function is running, `main_query` must not have any fetches for `entity`,
/// nor any of its descendants.
/// - The caller must ensure that the hierarchy leading to `entity`
/// is well-formed and must remain as a tree or a forest. Each entity must have at most one parent.
#[allow(unsafe_code)]
unsafe fn propagate<
    QShared: QueryData + 'static,
    QMain: QueryData + 'static,
    Filter: QueryFilter + 'static,
    Info: 'static,
>(
    actual_root: Entity,
    parent: &Item<QShared>,
    parent_info: &Info,
    main_query: &Query<(QShared, QMain, Option<&'static Children>), (Filter, With<Parent>)>,
    parent_query: &Query<(Entity, &Parent)>,
    entity: Entity,
    function: &mut impl FnMut(&Item<QShared>, &Info, &mut Item<QShared>, Item<QMain>) -> Info,
) {
    // SAFETY: This call cannot create aliased mutable references.
    //   - The top level iteration parallelizes on the roots of the hierarchy.
    //   - The caller ensures that each child has one and only one unique parent throughout the entire
    //     hierarchy.
    //
    // For example, consider the following malformed hierarchy:
    //
    //     A
    //   /   \
    //  B     C
    //   \   /
    //     D
    //
    // D has two parents, B and C. If the propagation passes through C, but the Parent component on D points to B,
    // the above check will panic as the origin parent does match the recorded parent.
    //
    // Also consider the following case, where A and B are roots:
    //
    //  A       B
    //   \     /
    //    C   D
    //     \ /
    //      E
    //
    // Even if these A and B start two separate tasks running in parallel, one of them will panic before attempting
    // to mutably access E.
    let Ok((mut shared, owned, children)) = (unsafe { main_query.get_unchecked(entity) }) else {
        return;
    };

    let info = function(parent, parent_info, &mut shared, owned);

    let Some(children) = children else { return };
    for (child, actual_parent) in parent_query.iter_many(children) {
        // Check if entities are chained properly.
        assert_eq!(
            actual_parent.get(), entity,
            "Malformed hierarchy. This probably means that your hierarchy has been improperly maintained."
        );
        // Since entities are chained properly, The only error that can occur is forming a circle with the root node.
        // This was not needed in `propagate_transform` since the root node did not have parents.
        assert_ne!(
            actual_root, entity,
            "Malformed hierarchy. Your hierarchy contains a cycle"
        );
        // SAFETY: The caller guarantees that `main_query` will not be fetched
        // for any descendants of `entity`, so it is safe to call `propagate_recursive` for each child.
        //
        // The above assertion ensures that each child has one and only one unique parent throughout the
        // entire hierarchy.
        unsafe {
            propagate(
                actual_root,
                &shared,
                &info,
                main_query,
                parent_query,
                child,
                function,
            );
        }
    }
}

#[cfg(test)]
mod test {
    use crate::prelude::*;
    use bevy_ecs::{prelude::*, system::RunSystemOnce};

    #[derive(Component)]
    pub struct ShouldBe(u32);
    #[derive(Component)]
    pub struct Transform(u32);
    #[derive(Component, Default)]
    pub struct GlobalTransform(u32);

    #[derive(Component, Default)]
    pub struct Trim;

    #[derive(Component, Default)]
    pub struct Trimmed;

    fn test_world() -> World {
        let mut world = World::new();
        world
            .spawn((Transform(1), GlobalTransform::default(), ShouldBe(1)))
            .with_children(|s| {
                s.spawn((Transform(1), GlobalTransform::default(), ShouldBe(2)))
                    .with_children(|s| {
                        s.spawn((Transform(2), GlobalTransform::default(), ShouldBe(4)));
                        s.spawn((
                            Transform(3),
                            GlobalTransform::default(),
                            ShouldBe(5),
                            Trim,
                            Trimmed,
                        ))
                        .with_children(|s| {
                            s.spawn((
                                Transform(0),
                                GlobalTransform::default(),
                                ShouldBe(5),
                                Trimmed,
                            ));
                            s.spawn((
                                Transform(2),
                                GlobalTransform::default(),
                                ShouldBe(7),
                                Trimmed,
                            ));
                        });
                    });
                s.spawn((Transform(2), GlobalTransform::default(), ShouldBe(3)));
                s.spawn((
                    Transform(3),
                    GlobalTransform::default(),
                    ShouldBe(4),
                    Trim,
                    Trimmed,
                ))
                .with_children(|s| {
                    s.spawn((
                        Transform(1),
                        GlobalTransform::default(),
                        ShouldBe(5),
                        Trimmed,
                    ));
                    s.spawn((
                        Transform(4),
                        GlobalTransform::default(),
                        ShouldBe(8),
                        Trimmed,
                    ));
                });
                s.spawn((
                    Transform(4),
                    GlobalTransform::default(),
                    ShouldBe(5),
                    Trim,
                    Trimmed,
                ));
            });
        world
    }

    #[test]
    pub fn test() {
        let mut world = test_world();
        world.run_system_once(
            |mut query: QueryRecursive<
                (&Transform, &mut GlobalTransform),
                (),
                (),
                Without<Parent>,
                (),
            >| {
                query.for_each_mut(
                    |(transform, global), ()| global.0 = transform.0,
                    |(_, parent), (transform, global), ()| global.0 = transform.0 + parent.0,
                );
            },
        );
        world
            .query::<(&GlobalTransform, &ShouldBe)>()
            .iter(&world)
            .for_each(|(a, b)| {
                assert_eq!(a.0, b.0);
            });

        let mut world = test_world();
        world.run_system_once(
            |mut query: QueryRecursive<
                &mut GlobalTransform,
                &Transform,
                &Transform,
                Without<Parent>,
                (),
            >| {
                query.for_each_mut(
                    |global, transform| global.0 = transform.0,
                    |parent, global, transform| global.0 = transform.0 + parent.0,
                );
            },
        );
        world
            .query::<(&GlobalTransform, &ShouldBe)>()
            .iter(&world)
            .for_each(|(a, b)| {
                assert_eq!(a.0, b.0);
            });

        let mut world = test_world();
        world.run_system_once(
            |mut query: QueryRecursive<
                &mut GlobalTransform,
                &Transform,
                &Transform,
                Without<Parent>,
                (),
            >| {
                query.for_each_mut_with(
                    |global, transform| {
                        global.0 = transform.0;
                        global.0
                    },
                    |_, parent, global, transform| {
                        global.0 = transform.0 + *parent;
                        global.0
                    },
                );
            },
        );

        world
            .query::<(&GlobalTransform, &ShouldBe)>()
            .iter(&world)
            .for_each(|(a, b)| {
                assert_eq!(a.0, b.0);
            });

        let mut world = test_world();
        world.run_system_once(
            |mut query: QueryRecursive<
                &mut GlobalTransform,
                &Transform,
                &Transform,
                (Without<Parent>, Without<Trim>),
                Without<Trim>,
            >| {
                query.for_each_mut(
                    |global, transform| global.0 = transform.0,
                    |parent, global, transform| global.0 = transform.0 + parent.0,
                );
            },
        );
        world
            .query_filtered::<(&GlobalTransform, &ShouldBe), Without<Trimmed>>()
            .iter(&world)
            .for_each(|(a, b)| {
                assert_eq!(a.0, b.0);
            });
        world
            .query_filtered::<&GlobalTransform, With<Trimmed>>()
            .iter(&world)
            .for_each(|a| {
                assert_eq!(a.0, 0);
            });

        let mut world = test_world();
        world.run_system_once(
            |mut query: QueryRecursive<
                &mut GlobalTransform,
                &Transform,
                &Transform,
                (Without<Parent>, Without<Trim>),
                Without<Trim>,
            >| {
                query.for_each_mut_with(
                    |global, transform| {
                        global.0 = transform.0;
                        global.0
                    },
                    |_, parent, global, transform| {
                        global.0 = transform.0 + *parent;
                        global.0
                    },
                );
            },
        );
        world
            .query_filtered::<(&GlobalTransform, &ShouldBe), Without<Trimmed>>()
            .iter(&world)
            .for_each(|(a, b)| {
                assert_eq!(a.0, b.0);
            });
        world
            .query_filtered::<&GlobalTransform, With<Trimmed>>()
            .iter(&world)
            .for_each(|a| {
                assert_eq!(a.0, 0);
            });
    }
}
