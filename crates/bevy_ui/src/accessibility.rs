use crate::Node;
use bevy_a11y::{
    accesskit::Rect,
    AccessibilityNode,
};
use bevy_app::{App, Plugin, Update};
use bevy_ecs::{
    prelude::DetectChanges,
    system::Query,
    world::Ref,
};
use bevy_render::prelude::Camera;
use bevy_transform::prelude::GlobalTransform;

fn calc_bounds(
    camera: Query<(&Camera, &GlobalTransform)>,
    mut nodes: Query<(&mut AccessibilityNode, Ref<Node>, Ref<GlobalTransform>)>,
) {
    if let Ok((camera, camera_transform)) = camera.get_single() {
        for (mut accessible, node, transform) in &mut nodes {
            if node.is_changed() || transform.is_changed() {
                if let Some(translation) =
                    camera.world_to_viewport(camera_transform, transform.translation())
                {
                    let bounds = Rect::new(
                        translation.x.into(),
                        translation.y.into(),
                        (translation.x + node.calculated_size.x).into(),
                        (translation.y + node.calculated_size.y).into(),
                    );
                    accessible.set_bounds(bounds);
                }
            }
        }
    }
}

/// `AccessKit` integration for `bevy_ui`.
pub(crate) struct AccessibilityPlugin;

impl Plugin for AccessibilityPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            calc_bounds,
        );
    }
}
