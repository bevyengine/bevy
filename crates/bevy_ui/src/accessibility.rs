use bevy_a11y::{accesskit::Rect, AccessibilityNode};
use bevy_app::{App, Plugin};

use bevy_ecs::{
    query::{Changed, Or},
    system::Query,
};

use bevy_render::prelude::Camera;
use bevy_transform::prelude::GlobalTransform;

use crate::Node;

fn calc_bounds(
    camera: Query<(&Camera, &GlobalTransform)>,
    mut nodes: Query<
        (&mut AccessibilityNode, &Node, &GlobalTransform),
        Or<(Changed<Node>, Changed<GlobalTransform>)>,
    >,
) {
    if let Ok((camera, camera_transform)) = camera.get_single() {
        for (mut accessible, node, transform) in &mut nodes {
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

/// `AccessKit` integration for `bevy_ui`.
pub(crate) struct AccessibilityPlugin;

impl Plugin for AccessibilityPlugin {
    fn build(&self, app: &mut App) {
        app.add_system(calc_bounds);
    }
}
