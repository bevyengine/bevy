use bevy_hierarchy::Propagatable;

use crate::{GlobalTransform, Transform};

impl Propagatable for Transform {
    type Computed = GlobalTransform;
    type Payload = GlobalTransform;

    const ALWAYS_PROPAGATE: bool = false;

    #[inline]
    fn compute_root(computed: &mut Self::Computed, local: &Self) {
        *computed = GlobalTransform::from(*local);
    }

    #[inline]
    fn compute(computed: &mut Self::Computed, payload: &Self::Payload, local: &Self) {
        *computed = payload.mul_transform(*local);
    }

    #[inline]
    fn payload(computed: &Self::Computed) -> Self::Payload {
        *computed
    }
}

#[cfg(test)]
mod test {
    use bevy_app::prelude::*;
    use bevy_ecs::prelude::*;
    use bevy_math::vec3;

    use crate::components::{GlobalTransform, Transform};
    use bevy_hierarchy::{BuildWorldChildren, Children};

    #[test]
    fn correct_transforms_when_no_children() {
        let mut app = App::new();

        app.add_system(bevy_hierarchy::propagate_system::<Transform>);

        let translation = vec3(1.0, 0.0, 0.0);

        // These will be overwritten.
        let mut child = Entity::from_raw(0);
        let mut grandchild = Entity::from_raw(1);
        let parent = app
            .world
            .spawn()
            .insert(Transform::from_translation(translation))
            .insert(GlobalTransform::default())
            .with_children(|builder| {
                child = builder
                    .spawn_bundle((Transform::identity(), GlobalTransform::default()))
                    .with_children(|builder| {
                        grandchild = builder
                            .spawn_bundle((Transform::identity(), GlobalTransform::default()))
                            .id();
                    })
                    .id();
            })
            .id();

        app.update();

        // check the `Children` structure is spawned
        assert_eq!(&**app.world.get::<Children>(parent).unwrap(), &[child]);
        assert_eq!(&**app.world.get::<Children>(child).unwrap(), &[grandchild]);
        // Note that at this point, the `GlobalTransform`s will not have updated yet, due to `Commands` delay
        app.update();

        let mut state = app.world.query::<&GlobalTransform>();
        for global in state.iter(&app.world) {
            assert_eq!(global, &GlobalTransform::from_translation(translation));
        }
    }
}
