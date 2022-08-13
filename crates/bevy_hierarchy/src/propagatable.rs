use bevy_ecs::prelude::*;

use crate::{Children, Parent};

/// Marks a component as propagatable thrown hierachy alike `Transform`/`GlobalTransorm`
/// or `Visibility`/`ComputedVisibility`.
pub trait Propagatable: Component {
    /// The computed version of this component.
    type Computed: Component;
    /// The payload passed to children for computation.
    type Payload;

    /// If set to `false`, children are computed only if the hierarchy is changed
    /// or their local component changed.
    /// Otherwise, always compute all components.
    const ALWAYS_PROPAGATE: bool;

    /// Update computed component for root entity from it's local component.
    fn compute_root(computed: &mut Self::Computed, local: &Self);

    /// Update computed component from the parent's payload and the local component.
    fn compute(computed: &mut Self::Computed, payload: &Self::Payload, local: &Self);

    /// Compute the payload to pass to children from the computed component.
    fn payload(computed: &Self::Computed) -> Self::Payload;
}

type LocalQuery<'w, 's, 'a, T> = Query<
    'w,
    's,
    (
        &'a T,
        Changed<T>,
        &'a mut <T as Propagatable>::Computed,
        &'a Parent,
    ),
>;
type ChildrenQuery<'w, 's, 'a, T> = Query<
    'w,
    's,
    (&'a Children, Changed<Children>),
    (With<Parent>, With<<T as Propagatable>::Computed>),
>;

/// Update `T::Computed` component of entities based on entity hierarchy and
/// `T` component.
pub fn propagate_system<T: Propagatable>(
    mut root_query: Query<
        (
            Option<(&Children, Changed<Children>)>,
            &T,
            Changed<T>,
            &mut T::Computed,
            Entity,
        ),
        Without<Parent>,
    >,
    mut local_query: LocalQuery<T>,
    children_query: ChildrenQuery<T>,
) {
    for (children, local, local_changed, mut computed, entity) in root_query.iter_mut() {
        let mut changed = local_changed;
        if T::ALWAYS_PROPAGATE | changed {
            T::compute_root(computed.as_mut(), local);
        }

        if let Some((children, changed_children)) = children {
            // If our `Children` has changed, we need to recalculate everything below us
            changed |= changed_children;
            let payload = T::payload(computed.as_ref());
            for child in children {
                let _ = propagate_recursive(
                    &payload,
                    &mut local_query,
                    &children_query,
                    *child,
                    entity,
                    changed,
                );
            }
        }
    }
}

fn propagate_recursive<T: Propagatable>(
    payload: &T::Payload,
    local_query: &mut LocalQuery<T>,
    children_query: &ChildrenQuery<T>,
    entity: Entity,
    expected_parent: Entity,
    mut changed: bool,
    // BLOCKED: https://github.com/rust-lang/rust/issues/31436
    // We use a result here to use the `?` operator. Ideally we'd use a try block instead
) -> Result<(), ()> {
    let payload = {
        let (local, local_changed, mut computed, child_parent) =
            local_query.get_mut(entity).map_err(drop)?;
        // Note that for parallelising, this check cannot occur here, since there is an `&mut GlobalTransform` (in global_transform)
        assert_eq!(
            child_parent.get(), expected_parent,
            "Malformed hierarchy. This probably means that your hierarchy has been improperly maintained, or contains a cycle"
        );
        changed |= local_changed;
        if T::ALWAYS_PROPAGATE | changed {
            T::compute(computed.as_mut(), payload, local);
        }
        T::payload(computed.as_ref())
    };

    let (children, changed_children) = children_query.get(entity).map_err(drop)?;
    // If our `Children` has changed, we need to recalculate everything below us
    changed |= changed_children;
    for child in children {
        let _ = propagate_recursive(
            &payload,
            local_query,
            children_query,
            *child,
            entity,
            changed,
        );
    }
    Ok(())
}

#[cfg(test)]
mod test {
    use bevy_app::App;
    use bevy_ecs::prelude::*;
    use bevy_ecs::system::CommandQueue;

    use crate::{
        propagate_system, BuildChildren, BuildWorldChildren, Children, Parent, Propagatable,
    };

    #[derive(Default, Component)]
    struct MyComponent(i32);

    #[derive(Default, Component, Clone, Copy)]
    struct MyComputedComponent(i32);

    impl Propagatable for MyComponent {
        type Computed = MyComputedComponent;
        type Payload = MyComputedComponent;
        const ALWAYS_PROPAGATE: bool = false;

        fn compute_root(computed: &mut Self::Computed, local: &Self) {
            computed.0 = local.0;
        }

        fn compute(computed: &mut Self::Computed, payload: &Self::Payload, local: &Self) {
            computed.0 = payload.0 * local.0;
        }

        fn payload(computed: &Self::Computed) -> Self::Payload {
            *computed
        }
    }

    #[test]
    fn did_propagate() {
        let mut world = World::default();

        let mut update_stage = SystemStage::parallel();
        update_stage.add_system(propagate_system::<MyComponent>);

        let mut schedule = Schedule::default();
        schedule.add_stage("update", update_stage);

        const ROOT_VALUE: i32 = 5;
        const CHILDREN_0_VALUE: i32 = 3;
        const CHILDREN_1_VALUE: i32 = -2;

        let mut children = Vec::new();
        world
            .spawn()
            .insert_bundle((MyComponent(ROOT_VALUE), MyComputedComponent::default()))
            .with_children(|parent| {
                children.push(
                    parent
                        .spawn_bundle((
                            MyComponent(CHILDREN_0_VALUE),
                            MyComputedComponent::default(),
                        ))
                        .id(),
                );
                children.push(
                    parent
                        .spawn_bundle((
                            MyComponent(CHILDREN_1_VALUE),
                            MyComputedComponent::default(),
                        ))
                        .id(),
                );
            });
        schedule.run(&mut world);

        assert_eq!(
            world.get::<MyComputedComponent>(children[0]).unwrap().0,
            ROOT_VALUE * CHILDREN_0_VALUE
        );

        assert_eq!(
            world.get::<MyComputedComponent>(children[1]).unwrap().0,
            ROOT_VALUE * CHILDREN_1_VALUE
        );
    }

    #[test]
    fn did_propagate_command_buffer() {
        let mut world = World::default();
        let mut update_stage = SystemStage::parallel();
        update_stage.add_system(propagate_system::<MyComponent>);

        let mut schedule = Schedule::default();
        schedule.add_stage("update", update_stage);

        const ROOT_VALUE: i32 = 5;
        const CHILDREN_0_VALUE: i32 = 3;
        const CHILDREN_1_VALUE: i32 = -2;

        // Root entity
        let mut queue = CommandQueue::default();
        let mut commands = Commands::new(&mut queue, &world);
        let mut children = Vec::new();
        commands
            .spawn_bundle((MyComponent(ROOT_VALUE), MyComputedComponent::default()))
            .with_children(|parent| {
                children.push(
                    parent
                        .spawn_bundle((
                            MyComponent(CHILDREN_0_VALUE),
                            MyComputedComponent::default(),
                        ))
                        .id(),
                );
                children.push(
                    parent
                        .spawn_bundle((
                            MyComponent(CHILDREN_1_VALUE),
                            MyComputedComponent::default(),
                        ))
                        .id(),
                );
            });
        queue.apply(&mut world);
        schedule.run(&mut world);

        assert_eq!(
            world.get::<MyComputedComponent>(children[0]).unwrap().0,
            ROOT_VALUE * CHILDREN_0_VALUE
        );

        assert_eq!(
            world.get::<MyComputedComponent>(children[1]).unwrap().0,
            ROOT_VALUE * CHILDREN_1_VALUE
        );
    }

    #[test]
    fn correct_children() {
        let mut world = World::default();

        let mut update_stage = SystemStage::parallel();
        update_stage.add_system(propagate_system::<MyComponent>);

        let mut schedule = Schedule::default();
        schedule.add_stage("update", update_stage);

        // Add parent entities
        let mut children = Vec::new();
        let parent = {
            let mut command_queue = CommandQueue::default();
            let mut commands = Commands::new(&mut command_queue, &world);
            let parent = commands.spawn().insert(MyComponent::default()).id();
            commands.entity(parent).with_children(|parent| {
                children.push(parent.spawn().insert(MyComponent::default()).id());
                children.push(parent.spawn().insert(MyComponent::default()).id());
            });
            command_queue.apply(&mut world);
            schedule.run(&mut world);
            parent
        };

        assert_eq!(
            world
                .get::<Children>(parent)
                .unwrap()
                .iter()
                .cloned()
                .collect::<Vec<_>>(),
            children,
        );

        // Parent `e1` to `e2`.
        {
            let mut command_queue = CommandQueue::default();
            let mut commands = Commands::new(&mut command_queue, &world);
            commands.entity(children[1]).add_child(children[0]);
            command_queue.apply(&mut world);
            schedule.run(&mut world);
        }

        assert_eq!(
            world
                .get::<Children>(parent)
                .unwrap()
                .iter()
                .cloned()
                .collect::<Vec<_>>(),
            vec![children[1]]
        );

        assert_eq!(
            world
                .get::<Children>(children[1])
                .unwrap()
                .iter()
                .cloned()
                .collect::<Vec<_>>(),
            vec![children[0]]
        );

        assert!(world.despawn(children[0]));

        schedule.run(&mut world);

        assert_eq!(
            world
                .get::<Children>(parent)
                .unwrap()
                .iter()
                .cloned()
                .collect::<Vec<_>>(),
            vec![children[1]]
        );
    }

    #[test]
    #[should_panic]
    fn panic_when_hierarchy_cycle() {
        // We cannot directly edit Parent and Children, so we use a temp world to break
        // the hierarchy's invariants.
        let mut temp = World::new();
        let mut app = App::new();

        app.add_system(propagate_system::<MyComponent>);

        fn setup_world(world: &mut World) -> (Entity, Entity) {
            let mut grandchild = Entity::from_raw(0);
            let child = world
                .spawn()
                .insert_bundle((MyComponent::default(), MyComputedComponent::default()))
                .with_children(|builder| {
                    grandchild = builder
                        .spawn()
                        .insert_bundle((MyComponent::default(), MyComputedComponent::default()))
                        .id();
                })
                .id();
            (child, grandchild)
        }

        let (temp_child, temp_grandchild) = setup_world(&mut temp);
        let (child, grandchild) = setup_world(&mut app.world);

        assert_eq!(temp_child, child);
        assert_eq!(temp_grandchild, grandchild);

        app.world
            .spawn()
            .insert_bundle((MyComponent::default(), MyComputedComponent::default()))
            .push_children(&[child]);
        std::mem::swap(
            &mut *app.world.get_mut::<Parent>(child).unwrap(),
            &mut *temp.get_mut::<Parent>(grandchild).unwrap(),
        );

        app.update();
    }
}
