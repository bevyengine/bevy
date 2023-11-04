use bevy_app::{App, Plugin, PostUpdate};
use bevy_ecs::{
    component::Component,
    entity::Entity,
    query::{Changed, With},
    schedule::{IntoSystemConfigs, SystemSet},
    system::Query,
};
use bevy_hierarchy::{Children, Parent};
use bevy_render::extract_component::{ExtractComponent, ExtractComponentPlugin};

pub struct SortingPlugin;

impl Plugin for SortingPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(ExtractComponentPlugin::<ComputedSorting>::default())
            .add_systems(
                PostUpdate,
                sorting_propagate_system.in_set(SortingPropagation),
            );
    }
}

#[derive(SystemSet, Debug, Hash, PartialEq, Eq, Clone)]
pub struct SortingPropagation;

/// A [`Component`] that controls the sorting of entities for rendering in 2D.
#[derive(Component, Clone, Debug, PartialEq)]
pub struct Sorting {
    /// The order in which this entity will be sorted.
    ///
    /// Entities with a higher order render on top of entities with a lower order.
    ///
    /// Defaults to `0`.
    pub order: f32,
    /// Whether the order is relative to the parent's order.
    ///
    /// Defaults to `true`.
    pub relative: bool,
}

impl Default for Sorting {
    fn default() -> Self {
        Sorting {
            order: 0.,
            relative: true,
        }
    }
}

impl Sorting {
    pub fn from_order(order: f32) -> Self {
        Sorting {
            order,
            ..Default::default()
        }
    }
}

/// The render order computed from the [`Sorting`] component on this entity and it's ancestors.
#[derive(Component, Default, Clone, Debug, PartialEq, ExtractComponent)]
pub struct ComputedSorting {
    /// The order in which this entity is sorted.
    pub order: f32,
}

fn sorting_propagate_system(
    changed: Query<
        (Entity, &Sorting, Option<&Parent>, Option<&Children>),
        (With<ComputedSorting>, Changed<Sorting>),
    >,
    mut sorting_query: Query<(&Sorting, &mut ComputedSorting)>,
    children_query: Query<&Children, (With<Sorting>, With<ComputedSorting>)>,
) {
    for (entity, sorting, parent, children) in &changed {
        let order = if sorting.relative {
            let parent_order = parent
                .and_then(|parent| sorting_query.get(parent.get()).ok())
                .map(|(_sorting, computed_sorting)| computed_sorting.order);
            sorting.order + parent_order.unwrap_or(0.)
        } else {
            sorting.order
        };

        let (_, mut computed_sorting) = sorting_query
            .get_mut(entity)
            .expect("With<InheritedSorting> ensures this query will return a value");

        // Only update the sorting if it has changed.
        // This will prevent the sorting from propagating multiple times in the same frame
        // if this entity's sorting has been updated recursively by its parent.
        if computed_sorting.order != order {
            computed_sorting.order = order;

            // Recursively update the sorting of each child.
            for &child in children.into_iter().flatten() {
                propagate_recursive(order, child, &mut sorting_query, &children_query);
            }
        }
    }
}

fn propagate_recursive(
    parent_order: f32,
    entity: Entity,
    sorting_query: &mut Query<(&Sorting, &mut ComputedSorting)>,
    children_query: &Query<&Children, (With<Sorting>, With<ComputedSorting>)>,
) {
    // Get the sorting components for the current entity.
    // If the entity does not have the required components, just return early.
    let Ok((sorting, mut computed_sorting)) = sorting_query.get_mut(entity) else {
        return;
    };

    if !sorting.relative {
        return;
    };

    let order = sorting.order + parent_order;

    // Only update the sorting if it has changed.
    if computed_sorting.order == order {
        return;
    }

    computed_sorting.order = order;

    // Recursively update the sorting of each child.
    for &child in children_query.get(entity).ok().into_iter().flatten() {
        propagate_recursive(order, child, sorting_query, children_query);
    }
}

#[cfg(test)]
mod tests {
    use bevy_app::App;
    use bevy_ecs::{entity::Entity, world::World};
    use bevy_hierarchy::BuildWorldChildren;

    use crate::sorting::{ComputedSorting, Sorting, SortingPlugin};

    #[test]
    fn propagation() {
        let mut app = App::new();
        app.add_plugins(SortingPlugin);

        fn spawn(world: &mut World, order: f32, relative: bool, parent: Option<Entity>) -> Entity {
            let mut entity = world.spawn((Sorting { order, relative }, ComputedSorting::default()));

            if let Some(parent) = parent {
                entity.set_parent(parent);
            }
            entity.id()
        }

        let a = spawn(&mut app.world, 1., true, None);
        let b = spawn(&mut app.world, 1., true, Some(a));
        let c = spawn(&mut app.world, 2., false, Some(a));
        let d = spawn(&mut app.world, 2., true, Some(c));

        app.update();

        let mut query = app.world.query::<&ComputedSorting>();

        let result = query
            .get_many(&app.world, [a, b, c, d])
            .unwrap()
            .map(|i| i.order);

        assert_eq!(result, [1., 2., 2., 4.]);

        let mut query = app.world.query::<&mut Sorting>();

        query.get_mut(&mut app.world, a).unwrap().order = -1.;
        query.get_mut(&mut app.world, c).unwrap().relative = true;

        app.update();

        let mut query = app.world.query::<&ComputedSorting>();

        let result = query
            .get_many(&app.world, [a, b, c, d])
            .unwrap()
            .map(|i| i.order);

        assert_eq!(result, [-1., 0., 1., 3.]);
    }
}
