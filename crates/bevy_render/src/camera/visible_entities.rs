use super::{Camera, DepthCalculation};
use crate::{draw::OutsideFrustum, prelude::Visible};
use bevy_core::FloatOrd;
use bevy_ecs::{
    entity::Entity,
    query::{Changed, Or, With, Without},
    reflect::ReflectComponent,
    system::{Commands, Query},
};
use bevy_reflect::Reflect;
use bevy_transform::prelude::{Children, GlobalTransform, Parent};

// This struct reflects Visible and is used to store the effective value.
// The only one reason why it's not stored in the original struct is the `is_transparent` field.
// Having both that field and the effective value in Visible complicates creation of that struct.
#[derive(Default, Debug, Reflect)]
#[reflect(Component)]
pub struct VisibleEffective {
    #[reflect(ignore)]
    pub is_visible: bool,
    #[reflect(ignore)]
    pub is_transparent: bool,
}

#[derive(Debug)]
pub struct VisibleEntity {
    pub entity: Entity,
    pub order: FloatOrd,
}

#[derive(Default, Debug, Reflect)]
#[reflect(Component)]
pub struct VisibleEntities {
    #[reflect(ignore)]
    pub value: Vec<VisibleEntity>,
}

impl VisibleEntities {
    pub fn iter(&self) -> impl DoubleEndedIterator<Item = &VisibleEntity> {
        self.value.iter()
    }
}

type LayerMask = u32;

/// An identifier for a rendering layer.
pub type Layer = u8;

/// Describes which rendering layers an entity belongs to.
///
/// Cameras with this component will only render entities with intersecting
/// layers.
///
/// There are 32 layers numbered `0` - [`TOTAL_LAYERS`](RenderLayers::TOTAL_LAYERS). Entities may
/// belong to one or more layers, or no layer at all.
///
/// The [`Default`] instance of `RenderLayers` contains layer `0`, the first layer.
///
/// An entity with this component without any layers is invisible.
///
/// Entities without this component belong to layer `0`.
#[derive(Copy, Clone, Reflect, PartialEq, Eq, PartialOrd, Ord)]
#[reflect(Component, PartialEq)]
pub struct RenderLayers(LayerMask);

impl std::fmt::Debug for RenderLayers {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("RenderLayers")
            .field(&self.iter().collect::<Vec<_>>())
            .finish()
    }
}

impl std::iter::FromIterator<Layer> for RenderLayers {
    fn from_iter<T: IntoIterator<Item = Layer>>(i: T) -> Self {
        i.into_iter().fold(Self::none(), |mask, g| mask.with(g))
    }
}

/// Defaults to containing to layer `0`, the first layer.
impl Default for RenderLayers {
    fn default() -> Self {
        RenderLayers::layer(0)
    }
}

impl RenderLayers {
    /// The total number of layers supported.
    pub const TOTAL_LAYERS: usize = std::mem::size_of::<LayerMask>() * 8;

    /// Create a new `RenderLayers` belonging to the given layer.
    pub fn layer(n: Layer) -> Self {
        RenderLayers(0).with(n)
    }

    /// Create a new `RenderLayers` that belongs to all layers.
    pub fn all() -> Self {
        RenderLayers(u32::MAX)
    }

    /// Create a new `RenderLayers` that belongs to no layers.
    pub fn none() -> Self {
        RenderLayers(0)
    }

    /// Create a `RenderLayers` from a list of layers.
    pub fn from_layers(layers: &[Layer]) -> Self {
        layers.iter().copied().collect()
    }

    /// Add the given layer.
    ///
    /// This may be called multiple times to allow an entity to belong
    /// to multiple rendering layers. The maximum layer is `TOTAL_LAYERS - 1`.
    ///
    /// # Panics
    /// Panics when called with a layer greater than `TOTAL_LAYERS - 1`.
    pub fn with(mut self, layer: Layer) -> Self {
        assert!(usize::from(layer) < Self::TOTAL_LAYERS);
        self.0 |= 1 << layer;
        self
    }

    /// Removes the given rendering layer.
    ///
    /// # Panics
    /// Panics when called with a layer greater than `TOTAL_LAYERS - 1`.
    pub fn without(mut self, layer: Layer) -> Self {
        assert!(usize::from(layer) < Self::TOTAL_LAYERS);
        self.0 |= 0 << layer;
        self
    }

    /// Get an iterator of the layers.
    pub fn iter(&self) -> impl Iterator<Item = Layer> {
        let total: Layer = std::convert::TryInto::try_into(Self::TOTAL_LAYERS).unwrap();
        let mask = *self;
        (0..total).filter(move |g| RenderLayers::layer(*g).intersects(&mask))
    }

    /// Determine if a `RenderLayers` intersects another.
    ///
    /// `RenderLayers`s intersect if they share any common layers.
    ///
    /// A `RenderLayers` with no layers will not match any other
    /// `RenderLayers`, even another with no layers.
    pub fn intersects(&self, other: &RenderLayers) -> bool {
        (self.0 & other.0) > 0
    }
}

#[cfg(test)]
mod rendering_mask_tests {
    use super::{Layer, RenderLayers};

    #[test]
    fn rendering_mask_sanity() {
        assert_eq!(
            RenderLayers::TOTAL_LAYERS,
            32,
            "total layers is what we think it is"
        );
        assert_eq!(RenderLayers::layer(0).0, 1, "layer 0 is mask 1");
        assert_eq!(RenderLayers::layer(1).0, 2, "layer 1 is mask 2");
        assert_eq!(RenderLayers::layer(0).with(1).0, 3, "layer 0 + 1 is mask 3");
        assert!(
            RenderLayers::layer(1).intersects(&RenderLayers::layer(1)),
            "layers match like layers"
        );
        assert!(
            RenderLayers::layer(0).intersects(&RenderLayers(1)),
            "a layer of 0 means the mask is just 1 bit"
        );

        assert!(
            RenderLayers::layer(0)
                .with(3)
                .intersects(&RenderLayers::layer(3)),
            "a mask will match another mask containing any similar layers"
        );

        assert!(
            RenderLayers::default().intersects(&RenderLayers::default()),
            "default masks match each other"
        );

        assert_eq!(
            RenderLayers::layer(0).intersects(&RenderLayers::layer(1)),
            false,
            "masks with differing layers do not match"
        );
        assert_eq!(
            RenderLayers(0).intersects(&RenderLayers(0)),
            false,
            "empty masks don't match"
        );
        assert_eq!(
            RenderLayers::from_layers(&[0, 2, 16, 30])
                .iter()
                .collect::<Vec<_>>(),
            vec![0, 2, 16, 30],
            "from_layers and get_layers should roundtrip"
        );
        assert_eq!(
            format!("{:?}", RenderLayers::from_layers(&[0, 1, 2, 3])).as_str(),
            "RenderLayers([0, 1, 2, 3])",
            "Debug instance shows layers"
        );
        assert_eq!(
            RenderLayers::from_layers(&[0, 1, 2]),
            <RenderLayers as std::iter::FromIterator<Layer>>::from_iter(vec![0, 1, 2]),
            "from_layers and from_iter are equivalent"
        )
    }
}

//Computes effective visibility for entities.
//
//In hirerarchies the actual visiblity of an entity isn't only the current value,
//but also depends on ancestors of the entity./ To avoid traversing each hierarchy
//and determine the effective visibility for each entity, this system listens to
//visiblity and hierarchy changes and only then computes a value to be cached and
//used by other systems.
pub fn visible_effective_system(
    children_query: Query<&Children>,
    changes_query: Query<
        (Entity, Option<&Parent>),
        (With<Visible>, Or<(Changed<Visible>, Changed<Parent>)>),
    >,
    mut visible_query: Query<(&Visible, Option<&mut VisibleEffective>)>,
    mut commands: Commands,
) {
    fn update_effective(
        entity: Entity,
        is_visible_parent: bool,
        children_query: &Query<&Children>,
        mut visible_query: &mut Query<(&Visible, Option<&mut VisibleEffective>)>,
        mut commands: &mut Commands,
    ) {
        if let Ok((visible, maybe_visible_effective)) = visible_query.get_mut(entity) {
            let is_visible = visible.is_visible & is_visible_parent;
            if let Some(mut visible_effective) = maybe_visible_effective {
                visible_effective.is_transparent = visible.is_transparent;

                if visible_effective.is_visible == is_visible {
                    return;
                }

                visible_effective.is_visible = is_visible;
            } else {
                commands.entity(entity).insert(VisibleEffective {
                    is_visible,
                    is_transparent: visible.is_transparent,
                });
            }

            if let Ok(children) = children_query.get(entity) {
                for child in children.iter() {
                    update_effective(
                        *child,
                        is_visible,
                        &children_query,
                        &mut visible_query,
                        &mut commands,
                    );
                }
            }
        }
    }

    for (entity, maybe_parent) in changes_query.iter() {
        if let Ok(is_visible) = match maybe_parent {
            None => Ok(true),
            Some(parent) => visible_query
                .get_component::<VisibleEffective>(parent.0)
                .map(|v| v.is_visible),
        } {
            update_effective(
                entity,
                is_visible,
                &children_query,
                &mut visible_query,
                &mut commands,
            );
        }
    }
}

pub fn visible_entities_system(
    mut camera_query: Query<(
        &Camera,
        &GlobalTransform,
        &mut VisibleEntities,
        Option<&RenderLayers>,
    )>,
    visible_query: Query<
        (Entity, &VisibleEffective, Option<&RenderLayers>),
        Without<OutsideFrustum>,
    >,
    visible_transform_query: Query<&GlobalTransform, Without<OutsideFrustum>>,
) {
    for (camera, camera_global_transform, mut visible_entities, maybe_camera_mask) in
        camera_query.iter_mut()
    {
        visible_entities.value.clear();
        let camera_position = camera_global_transform.translation;
        let camera_mask = maybe_camera_mask.copied().unwrap_or_default();

        let mut no_transform_order = 0.0;
        let mut transparent_entities = Vec::new();
        for (entity, visible, maybe_entity_mask) in visible_query.iter() {
            if !visible.is_visible {
                continue;
            }

            let entity_mask = maybe_entity_mask.copied().unwrap_or_default();
            if !camera_mask.intersects(&entity_mask) {
                continue;
            }

            let order = if let Ok(global_transform) = visible_transform_query.get(entity) {
                let position = global_transform.translation;
                // smaller distances are sorted to lower indices by using the distance from the
                // camera
                FloatOrd(match camera.depth_calculation {
                    DepthCalculation::ZDifference => camera_position.z - position.z,
                    DepthCalculation::Distance => (camera_position - position).length_squared(),
                })
            } else {
                let order = FloatOrd(no_transform_order);
                no_transform_order += 0.1;
                order
            };

            if visible.is_transparent {
                transparent_entities.push(VisibleEntity { entity, order })
            } else {
                visible_entities.value.push(VisibleEntity { entity, order })
            }
        }

        // sort opaque entities front-to-back
        visible_entities.value.sort_by_key(|e| e.order);

        // sort transparent entities front-to-back
        transparent_entities.sort_by_key(|e| -e.order);
        visible_entities.value.extend(transparent_entities);

        // TODO: check for big changes in visible entities len() vs capacity() (ex: 2x) and resize
        // to prevent holding unneeded memory
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use bevy_ecs::{
        schedule::{Schedule, Stage, SystemStage},
        system::IntoSystem,
        world::World,
    };
    use bevy_transform::hierarchy::{parent_update_system, BuildWorldChildren};

    #[test]
    fn propagates_visibility() {
        let mut world = World::default();
        let mut update_stage = SystemStage::parallel();
        update_stage.add_system(parent_update_system.system());
        update_stage.add_system(visible_effective_system.system());

        let mut schedule = Schedule::default();
        schedule.add_stage("update", update_stage);

        let mut child1 = Option::<Entity>::None;
        let mut child2 = Option::<Entity>::None;
        let parent = world
            .spawn()
            .insert(Visible {
                is_visible: false,
                is_transparent: false,
            })
            .with_children(|parent| {
                child1 = Some(
                    parent
                        .spawn()
                        .insert(Visible::default())
                        .with_children(|parent| {
                            child2 = Some(parent.spawn().insert(Visible::default()).id())
                        })
                        .id(),
                )
            })
            .id();

        let child1 = child1.unwrap();
        let child2 = child2.unwrap();

        schedule.run(&mut world);
        assert_eq!(false, is_visible(&world, parent));
        assert_eq!(false, is_visible(&world, child1));
        assert_eq!(false, is_visible(&world, child2));

        world
            .get_mut::<Visible>(parent)
            .map(|mut v| v.is_visible = true)
            .unwrap();

        schedule.run(&mut world);
        assert_eq!(true, is_visible(&world, parent));
        assert_eq!(true, is_visible(&world, child1));
        assert_eq!(true, is_visible(&world, child2));

        world
            .get_mut::<Visible>(child1)
            .map(|mut v| v.is_visible = false)
            .unwrap();

        schedule.run(&mut world);
        assert_eq!(true, is_visible(&world, parent));
        assert_eq!(false, is_visible(&world, child1));
        assert_eq!(false, is_visible(&world, child2));

        world
            .get_mut::<Visible>(parent)
            .map(|mut v| v.is_visible = false)
            .unwrap();

        schedule.run(&mut world);
        assert_eq!(false, is_visible(&world, parent));
        assert_eq!(false, is_visible(&world, child1));
        assert_eq!(false, is_visible(&world, child2));

        world
            .get_mut::<Visible>(parent)
            .map(|mut v| v.is_visible = true)
            .unwrap();

        schedule.run(&mut world);
        assert_eq!(true, is_visible(&world, parent));
        assert_eq!(false, is_visible(&world, child1));
        assert_eq!(false, is_visible(&world, child2));

        fn is_visible(world: &World, entity: Entity) -> bool {
            world
                .get::<VisibleEffective>(entity)
                .map(|v| v.is_visible)
                .unwrap()
        }
    }
}
