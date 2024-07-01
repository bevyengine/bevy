//! System parameter for computing up-to-date [`GlobalTransform`]s.

use bevy_ecs::{
    prelude::Entity,
    query::QueryEntityError,
    system::{Query, SystemParam},
};
use bevy_hierarchy::{HierarchyQueryExt, Parent};
use thiserror::Error;

use crate::components::{GlobalTransform, Transform};

/// System parameter for computing up-to-date [`GlobalTransform`]s.
///
/// Computing an entity's [`GlobalTransform`] can be expensive so it is recommended
/// you use the [`GlobalTransform`] component stored on the entity, unless you need
/// a [`GlobalTransform`] that reflects the changes made to any [`Transform`]s since
/// the last time the transform propagation systems ran.
#[derive(SystemParam)]
pub struct TransformHelper<'w, 's> {
    parent_query: Query<'w, 's, &'static Parent>,
    transform_query: Query<'w, 's, &'static Transform>,
}

impl<'w, 's> TransformHelper<'w, 's> {
    /// Computes the [`GlobalTransform`] of the given entity from the [`Transform`] component on it and its ancestors.
    pub fn compute_global_transform(
        &self,
        entity: Entity,
    ) -> Result<GlobalTransform, ComputeGlobalTransformError> {
        let transform = self
            .transform_query
            .get(entity)
            .map_err(|err| map_error(err, false))?;

        let mut global_transform = GlobalTransform::from(*transform);

        for entity in self.parent_query.iter_ancestors(entity) {
            let transform = self
                .transform_query
                .get(entity)
                .map_err(|err| map_error(err, true))?;

            global_transform = *transform * global_transform;
        }

        Ok(global_transform)
    }
}

fn map_error(err: QueryEntityError, ancestor: bool) -> ComputeGlobalTransformError {
    use ComputeGlobalTransformError::*;
    match err {
        QueryEntityError::QueryDoesNotMatch(entity) => MissingTransform(entity),
        QueryEntityError::NoSuchEntity(entity) => {
            if ancestor {
                MalformedHierarchy(entity)
            } else {
                NoSuchEntity(entity)
            }
        }
        QueryEntityError::AliasedMutability(_) => unreachable!(),
    }
}

/// Error returned by [`TransformHelper::compute_global_transform`].
#[derive(Debug, Error)]
pub enum ComputeGlobalTransformError {
    /// The entity or one of its ancestors is missing the [`Transform`] component.
    #[error("The entity {0:?} or one of its ancestors is missing the `Transform` component")]
    MissingTransform(Entity),
    /// The entity does not exist.
    #[error("The entity {0:?} does not exist")]
    NoSuchEntity(Entity),
    /// An ancestor is missing.
    /// This probably means that your hierarchy has been improperly maintained.
    #[error("The ancestor {0:?} is missing")]
    MalformedHierarchy(Entity),
}

#[cfg(test)]
mod tests {
    use std::f32::consts::TAU;

    use bevy_app::App;
    use bevy_ecs::system::SystemState;
    use bevy_hierarchy::BuildWorldChildren;
    use bevy_math::{Quat, Vec3};

    use crate::{
        bundles::TransformBundle,
        components::{GlobalTransform, Transform},
        helper::TransformHelper,
        plugins::TransformPlugin,
    };

    #[test]
    fn match_transform_propagation_systems() {
        // Single transform
        match_transform_propagation_systems_inner(vec![Transform::from_translation(Vec3::X)
            .with_rotation(Quat::from_rotation_y(TAU / 4.))
            .with_scale(Vec3::splat(2.))]);

        // Transform hierarchy
        match_transform_propagation_systems_inner(vec![
            Transform::from_translation(Vec3::X)
                .with_rotation(Quat::from_rotation_y(TAU / 4.))
                .with_scale(Vec3::splat(2.)),
            Transform::from_translation(Vec3::Y)
                .with_rotation(Quat::from_rotation_z(TAU / 3.))
                .with_scale(Vec3::splat(1.5)),
            Transform::from_translation(Vec3::Z)
                .with_rotation(Quat::from_rotation_x(TAU / 2.))
                .with_scale(Vec3::splat(0.3)),
        ]);
    }

    fn match_transform_propagation_systems_inner(transforms: Vec<Transform>) {
        let mut app = App::new();
        app.add_plugins(TransformPlugin);

        let mut entity = None;

        for transform in transforms {
            let mut e = app.world_mut().spawn(TransformBundle::from(transform));

            if let Some(entity) = entity {
                e.set_parent(entity);
            }

            entity = Some(e.id());
        }

        let leaf_entity = entity.unwrap();

        app.update();

        let transform = *app.world().get::<GlobalTransform>(leaf_entity).unwrap();

        let mut state = SystemState::<TransformHelper>::new(app.world_mut());
        let helper = state.get(app.world());

        let computed_transform = helper.compute_global_transform(leaf_entity).unwrap();

        approx::assert_abs_diff_eq!(transform.affine(), computed_transform.affine());
    }
}
