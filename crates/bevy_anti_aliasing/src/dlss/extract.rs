use super::{prepare::ViewDlssContext, Dlss};
use bevy_ecs::{
    query::With,
    system::{Commands, ResMut},
};
use bevy_render::{
    camera::{Camera, MainPassResolutionOverride, Projection},
    sync_world::RenderEntity,
    view::Hdr,
    MainWorld,
};

pub fn extract_dlss(mut commands: Commands, mut main_world: ResMut<MainWorld>) {
    let mut cameras_3d = main_world
        .query_filtered::<(RenderEntity, &Camera, &Projection, Option<&mut Dlss>), With<Hdr>>();

    for (entity, camera, camera_projection, mut dlss) in cameras_3d.iter_mut(&mut main_world) {
        let has_perspective_projection = matches!(camera_projection, Projection::Perspective(_));
        let mut entity_commands = commands
            .get_entity(entity)
            .expect("Camera entity wasn't synced.");
        if dlss.is_some() && camera.is_active && has_perspective_projection {
            entity_commands.insert(dlss.as_deref().unwrap().clone());
            dlss.as_mut().unwrap().reset = false;
        } else {
            entity_commands.remove::<(Dlss, ViewDlssContext, MainPassResolutionOverride)>();
        }
    }
}
