use crate::components::{GlobalTransform, Transform, TransformTreeChanged};
use bevy_ecs::{prelude::*, hierarchy_propagate::{hierarchy_propagate_simple, mark_dirty_trees as mark_dirty_trees_generic, hierarchy_propagate_complex, DownPropagate}};

// Transform propagation implementation
#[derive(Component)]
pub struct TransformPropagate;

impl DownPropagate for TransformPropagate {
    type Input = Transform;
    type Output = GlobalTransform;
    type TreeChanged = TransformTreeChanged;
    
    fn down_propagate(parent: &GlobalTransform, input: &Transform) -> GlobalTransform {
        *parent * *input
    }
    
    fn input_to_output(input: &Transform) -> GlobalTransform {
        GlobalTransform::from(*input)
    }
}

/// Update [`GlobalTransform`] component of entities that aren't in the hierarchy
///
/// Third party plugins should ensure that this is used in concert with
/// [`propagate_parent_transforms`] and [`mark_dirty_trees`].
pub fn sync_simple_transforms(
    queries: ParamSet<(
        Query<
            (&Transform, &mut GlobalTransform),
            (
                Or<(Changed<Transform>, Added<GlobalTransform>)>,
                Without<ChildOf>,
                Without<Children>,
            ),
        >,
        Query<(Ref<Transform>, &mut GlobalTransform), (Without<ChildOf>, Without<Children>)>,
    )>,
    orphaned: RemovedComponents<ChildOf>,
) {
    hierarchy_propagate_simple::<TransformPropagate>(queries, orphaned)
}

/// Optimization for static scenes. Propagates a "dirty bit" up the hierarchy towards ancestors.
/// Transform propagation can ignore entire subtrees of the hierarchy if it encounters an entity
/// without the dirty bit.
pub fn mark_dirty_trees(
    changed_transforms: Query<
        Entity,
        Or<(Changed<Transform>, Changed<ChildOf>, Added<GlobalTransform>)>,
    >,
    orphaned: RemovedComponents<ChildOf>,
    transforms: Query<(Option<&ChildOf>, &mut TransformTreeChanged)>,
) {
    mark_dirty_trees_generic::<TransformPropagate>(changed_transforms, orphaned, transforms)
}

#[cfg(not(feature = "std"))]
pub fn propagate_parent_transforms(
    mut root_query: Query<
        (Entity, &Children, Ref<Transform>, &mut GlobalTransform),
        Without<ChildOf>,
    >,
    mut orphaned: RemovedComponents<ChildOf>,
    transform_query: Query<
        (Ref<Transform>, &mut GlobalTransform, Option<&Children>),
        With<ChildOf>,
    >,
    child_query: Query<(Entity, Ref<ChildOf>), With<GlobalTransform>>,
    mut orphaned_entities: Local<Vec<Entity>>,
) {
    hierarchy_propagate_complex::<TransformPropagate>(
        root_query,
        orphaned,
        transform_query,
        child_query,
        orphaned_entities,
)
}

/// Update [`GlobalTransform`] component of entities based on entity hierarchy and [`Transform`]
/// component.
///
/// This is now implemented using the generic hierarchy propagation framework.
/// For direct usage, consider using `hierarchy_propagate_complex::<TransformPropagate>` instead.
#[cfg(feature = "std")]
use bevy_ecs::hierarchy_propagate::parallel::{WorkQueue, NodeQuery};
#[cfg(feature = "std")]
pub fn propagate_parent_transforms(
    mut queue: Local<WorkQueue>,
    mut roots: Query<
        (Entity, Ref<Transform>, &mut GlobalTransform, &Children),
        (Without<ChildOf>, Changed<TransformTreeChanged>),
    >,
    nodes: NodeQuery<'_, '_, TransformPropagate>,
) {
    hierarchy_propagate_complex::<TransformPropagate>(queue, roots, nodes)
}

#[cfg(test)]
mod test {
    use alloc::{vec, vec::Vec};
    use bevy_app::prelude::*;
    use bevy_ecs::{prelude::*, world::CommandQueue};
    use bevy_math::{vec3, Vec3};
    use bevy_tasks::{ComputeTaskPool, TaskPool};

    use crate::systems::*;

    #[test]
    fn correct_parent_removed() {
        ComputeTaskPool::get_or_init(TaskPool::default);
        let mut world = World::default();
        let offset_global_transform =
            |offset| GlobalTransform::from(Transform::from_xyz(offset, offset, offset));
        let offset_transform = |offset| Transform::from_xyz(offset, offset, offset);

        let mut schedule = Schedule::default();
        schedule.add_systems(
            (
                mark_dirty_trees,
                sync_simple_transforms,
                propagate_parent_transforms,
            )
                .chain(),
        );

        let mut command_queue = CommandQueue::default();
        let mut commands = Commands::new(&mut command_queue, &world);
        let root = commands.spawn(offset_transform(3.3)).id();
        let parent = commands.spawn(offset_transform(4.4)).id();
        let child = commands.spawn(offset_transform(5.5)).id();
        commands.entity(parent).insert(ChildOf(root));
        commands.entity(child).insert(ChildOf(parent));
        command_queue.apply(&mut world);
        schedule.run(&mut world);

        assert_eq!(
            world.get::<GlobalTransform>(parent).unwrap(),
            &offset_global_transform(4.4 + 3.3),
            "The transform systems didn't run, ie: `GlobalTransform` wasn't updated",
        );

        // Remove parent of `parent`
        let mut command_queue = CommandQueue::default();
        let mut commands = Commands::new(&mut command_queue, &world);
        commands.entity(parent).remove::<ChildOf>();
        command_queue.apply(&mut world);
        schedule.run(&mut world);

        assert_eq!(
            world.get::<GlobalTransform>(parent).unwrap(),
            &offset_global_transform(4.4),
            "The global transform of an orphaned entity wasn't updated properly",
        );

        // Remove parent of `child`
        let mut command_queue = CommandQueue::default();
        let mut commands = Commands::new(&mut command_queue, &world);
        commands.entity(child).remove::<ChildOf>();
        command_queue.apply(&mut world);
        schedule.run(&mut world);

        assert_eq!(
            world.get::<GlobalTransform>(child).unwrap(),
            &offset_global_transform(5.5),
            "The global transform of an orphaned entity wasn't updated properly",
        );
    }

    #[test]
    fn did_propagate() {
        ComputeTaskPool::get_or_init(TaskPool::default);
        let mut world = World::default();

        let mut schedule = Schedule::default();
        schedule.add_systems(
            (
                mark_dirty_trees,
                sync_simple_transforms,
                propagate_parent_transforms,
            )
                .chain(),
        );

        // Root entity
        world.spawn(Transform::from_xyz(1.0, 0.0, 0.0));

        let mut children = Vec::new();
        world
            .spawn(Transform::from_xyz(1.0, 0.0, 0.0))
            .with_children(|parent| {
                children.push(parent.spawn(Transform::from_xyz(0.0, 2.0, 0.)).id());
                children.push(parent.spawn(Transform::from_xyz(0.0, 0.0, 3.)).id());
            });
        schedule.run(&mut world);

        assert_eq!(
            *world.get::<GlobalTransform>(children[0]).unwrap(),
            GlobalTransform::from_xyz(1.0, 0.0, 0.0) * Transform::from_xyz(0.0, 2.0, 0.0)
        );

        assert_eq!(
            *world.get::<GlobalTransform>(children[1]).unwrap(),
            GlobalTransform::from_xyz(1.0, 0.0, 0.0) * Transform::from_xyz(0.0, 0.0, 3.0)
        );
    }

    #[test]
    fn did_propagate_command_buffer() {
        let mut world = World::default();

        let mut schedule = Schedule::default();
        schedule.add_systems(
            (
                mark_dirty_trees,
                sync_simple_transforms,
                propagate_parent_transforms,
            )
                .chain(),
        );

        // Root entity
        let mut queue = CommandQueue::default();
        let mut commands = Commands::new(&mut queue, &world);
        let mut children = Vec::new();
        commands
            .spawn(Transform::from_xyz(1.0, 0.0, 0.0))
            .with_children(|parent| {
                children.push(parent.spawn(Transform::from_xyz(0.0, 2.0, 0.0)).id());
                children.push(parent.spawn(Transform::from_xyz(0.0, 0.0, 3.0)).id());
            });
        queue.apply(&mut world);
        schedule.run(&mut world);

        assert_eq!(
            *world.get::<GlobalTransform>(children[0]).unwrap(),
            GlobalTransform::from_xyz(1.0, 0.0, 0.0) * Transform::from_xyz(0.0, 2.0, 0.0)
        );

        assert_eq!(
            *world.get::<GlobalTransform>(children[1]).unwrap(),
            GlobalTransform::from_xyz(1.0, 0.0, 0.0) * Transform::from_xyz(0.0, 0.0, 3.0)
        );
    }

    #[test]
    fn correct_children() {
        ComputeTaskPool::get_or_init(TaskPool::default);
        let mut world = World::default();

        let mut schedule = Schedule::default();
        schedule.add_systems(
            (
                mark_dirty_trees,
                sync_simple_transforms,
                propagate_parent_transforms,
            )
                .chain(),
        );

        // Add parent entities
        let mut children = Vec::new();
        let parent = {
            let mut command_queue = CommandQueue::default();
            let mut commands = Commands::new(&mut command_queue, &world);
            let parent = commands.spawn(Transform::from_xyz(1.0, 0.0, 0.0)).id();
            commands.entity(parent).with_children(|parent| {
                children.push(parent.spawn(Transform::from_xyz(0.0, 2.0, 0.0)).id());
                children.push(parent.spawn(Transform::from_xyz(0.0, 3.0, 0.0)).id());
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
                .collect::<Vec<_>>(),
            vec![children[1]]
        );

        assert_eq!(
            world
                .get::<Children>(children[1])
                .unwrap()
                .iter()
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
                .collect::<Vec<_>>(),
            vec![children[1]]
        );
    }

    #[test]
    fn correct_transforms_when_no_children() {
        let mut app = App::new();
        ComputeTaskPool::get_or_init(TaskPool::default);

        app.add_systems(
            Update,
            (
                mark_dirty_trees,
                sync_simple_transforms,
                propagate_parent_transforms,
            )
                .chain(),
        );

        let translation = vec3(1.0, 0.0, 0.0);

        // These will be overwritten.
        let mut child = Entity::from_raw_u32(0).unwrap();
        let mut grandchild = Entity::from_raw_u32(1).unwrap();
        let parent = app
            .world_mut()
            .spawn(Transform::from_translation(translation))
            .with_children(|builder| {
                child = builder
                    .spawn(Transform::IDENTITY)
                    .with_children(|builder| {
                        grandchild = builder.spawn(Transform::IDENTITY).id();
                    })
                    .id();
            })
            .id();

        app.update();

        // check the `Children` structure is spawned
        assert_eq!(&**app.world().get::<Children>(parent).unwrap(), &[child]);
        assert_eq!(
            &**app.world().get::<Children>(child).unwrap(),
            &[grandchild]
        );
        // Note that at this point, the `GlobalTransform`s will not have updated yet, due to
        // `Commands` delay
        app.update();

        let mut state = app.world_mut().query::<&GlobalTransform>();
        for global in state.iter(app.world()) {
            assert_eq!(global, &GlobalTransform::from_translation(translation));
        }
    }

    #[test]
    #[should_panic]
    fn panic_when_hierarchy_cycle() {
        ComputeTaskPool::get_or_init(TaskPool::default);
        // We cannot directly edit ChildOf and Children, so we use a temp world to break the
        // hierarchy's invariants.
        let mut temp = World::new();
        let mut app = App::new();

        app.add_systems(
            Update,
            // It is unsound for this unsafe system to encounter a cycle without panicking. This
            // requirement only applies to systems with unsafe parallel traversal that result in
            // aliased mutability during a cycle.
            propagate_parent_transforms,
        );

        fn setup_world(world: &mut World) -> (Entity, Entity) {
            let mut grandchild = Entity::from_raw_u32(0).unwrap();
            let child = world
                .spawn(Transform::IDENTITY)
                .with_children(|builder| {
                    grandchild = builder.spawn(Transform::IDENTITY).id();
                })
                .id();
            (child, grandchild)
        }

        let (temp_child, temp_grandchild) = setup_world(&mut temp);
        let (child, grandchild) = setup_world(app.world_mut());

        assert_eq!(temp_child, child);
        assert_eq!(temp_grandchild, grandchild);

        app.world_mut()
            .spawn(Transform::IDENTITY)
            .add_children(&[child]);

        let mut child_entity = app.world_mut().entity_mut(child);

        let mut grandchild_entity = temp.entity_mut(grandchild);

        #[expect(
            unsafe_code,
            reason = "ChildOf is not mutable but this is for a test to produce a scenario that cannot happen"
        )]
        // SAFETY: ChildOf is not mutable but this is for a test to produce a scenario that
        // cannot happen
        let mut a = unsafe { child_entity.get_mut_assume_mutable::<ChildOf>().unwrap() };

        // SAFETY: ChildOf is not mutable but this is for a test to produce a scenario that
        // cannot happen
        #[expect(
            unsafe_code,
            reason = "ChildOf is not mutable but this is for a test to produce a scenario that cannot happen"
        )]
        let mut b = unsafe {
            grandchild_entity
                .get_mut_assume_mutable::<ChildOf>()
                .unwrap()
        };

        core::mem::swap(a.as_mut(), b.as_mut());

        app.update();
    }

    #[test]
    fn global_transform_should_not_be_overwritten_after_reparenting() {
        let translation = Vec3::ONE;
        let mut world = World::new();

        // Create transform propagation schedule
        let mut schedule = Schedule::default();
        schedule.add_systems(
            (
                mark_dirty_trees,
                propagate_parent_transforms,
                sync_simple_transforms,
            )
                .chain(),
        );

        // Spawn a `Transform` entity with a local translation of `Vec3::ONE`
        let mut spawn_transform_bundle =
            || world.spawn(Transform::from_translation(translation)).id();

        // Spawn parent and child with identical transform bundles
        let parent = spawn_transform_bundle();
        let child = spawn_transform_bundle();
        world.entity_mut(parent).add_child(child);

        // Run schedule to propagate transforms
        schedule.run(&mut world);

        // Child should be positioned relative to its parent
        let parent_global_transform = *world.entity(parent).get::<GlobalTransform>().unwrap();
        let child_global_transform = *world.entity(child).get::<GlobalTransform>().unwrap();
        assert!(parent_global_transform
            .translation()
            .abs_diff_eq(translation, 0.1));
        assert!(child_global_transform
            .translation()
            .abs_diff_eq(2. * translation, 0.1));

        // Reparent child
        world.entity_mut(child).remove::<ChildOf>();
        world.entity_mut(parent).add_child(child);

        // Run schedule to propagate transforms
        schedule.run(&mut world);

        // Translations should be unchanged after update
        assert_eq!(
            parent_global_transform,
            *world.entity(parent).get::<GlobalTransform>().unwrap()
        );
        assert_eq!(
            child_global_transform,
            *world.entity(child).get::<GlobalTransform>().unwrap()
        );
    }
}
