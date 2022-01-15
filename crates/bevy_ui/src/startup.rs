use crate::prelude::UiCameraBundle;
use crate::ui_node::Node;
use crate::CAMERA_UI;
use bevy_ecs::prelude::{Commands, Entity, Query, With};
use bevy_render::prelude::{Camera, OrthographicProjection};

pub fn add_default_ui_cam_if_needed(
    mut commands: Commands,
    node_query: Query<Entity, With<Node>>,
    ui_cam_query: Query<&Camera, With<OrthographicProjection>>,
) {
    let world_contains_nodes = node_query.iter().next().is_some();
    let world_contains_ui_cam = ui_cam_query
        .iter()
        .any(|cam| cam.name.as_deref() == Some(CAMERA_UI));

    if world_contains_nodes && !world_contains_ui_cam {
        commands.spawn_bundle(UiCameraBundle::default());
    }
}

#[cfg(test)]
mod tests {
    use crate::entity::{NodeBundle, UiCameraBundle};
    use crate::startup::add_default_ui_cam_if_needed;
    use bevy_ecs::prelude::{Stage, SystemStage, World};
    use bevy_render::prelude::{Camera, OrthographicProjection};

    #[test]
    fn no_ui_camera_added_when_no_ui_nodes_exist() {
        let mut world = World::default();

        let mut startup_stage = SystemStage::parallel();
        startup_stage.add_system(add_default_ui_cam_if_needed);

        startup_stage.run(&mut world);

        assert_eq!(
            world
                .query::<(&Camera, &OrthographicProjection)>()
                .iter(&world)
                .len(),
            0
        );
    }

    #[test]
    fn ui_camera_added_when_ui_nodes_exist() {
        let mut world = World::default();

        world.spawn().insert_bundle(NodeBundle::default());

        let mut startup_stage = SystemStage::parallel();
        startup_stage.add_system(add_default_ui_cam_if_needed);

        startup_stage.run(&mut world);

        assert_eq!(
            world
                .query::<(&Camera, &OrthographicProjection)>()
                .iter(&world)
                .len(),
            1
        );
    }

    #[test]
    fn no_duplicate_ui_camera_added_when_one_is_already_present() {
        let mut world = World::default();

        let cam_id = world.spawn().insert_bundle(UiCameraBundle::default()).id();

        assert_eq!(
            world
                .query::<(&Camera, &OrthographicProjection)>()
                .iter(&world)
                .len(),
            1
        );

        let mut startup_stage = SystemStage::parallel();
        startup_stage.add_system(add_default_ui_cam_if_needed);

        startup_stage.run(&mut world);

        assert_eq!(
            world
                .query::<(&Camera, &OrthographicProjection)>()
                .iter(&world)
                .len(),
            1
        );

        assert!(world.get::<Camera>(cam_id).is_some());
    }
}
