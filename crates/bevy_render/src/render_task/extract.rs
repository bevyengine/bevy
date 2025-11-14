use super::RenderTask;
use crate::{sync_world::RenderEntity, Extract};
use bevy_camera::Camera;
use bevy_ecs::system::{Commands, Query};

// TODO: Use SyncComponentPlugin or ExtractComponentPlugin?
// TODO: Handle Camera also being removed?
// TODO: Allow extracting/removing additional types
// TODO: Allow mutating T in main world
pub fn extract_render_task<T: RenderTask>(
    query: Extract<Query<(Option<&T>, &Camera, RenderEntity)>>,
    mut commands: Commands,
) {
    for (task, camera, entity) in &query {
        let mut entity_commands = commands
            .get_entity(entity)
            .expect("Camera entity wasn't synced.");

        if let Some(task) = task
            && camera.is_active
        {
            entity_commands.insert(task.clone());
        } else {
            entity_commands.remove::<T>();
        }
    }
}
