use bevy::{
    gltf::{Gltf, GltfNode},
    prelude::*,
};

fn main() {
    App::new()
        .insert_resource(AmbientLight {
            color: Color::WHITE,
            brightness: 1.0 / 5.0f32,
        })
        .add_plugins(DefaultPlugins)
        .add_startup_system(setup)
        .add_system(spawn_gltf_objects)
        .run();
}

fn setup(mut commands: Commands, assets: Res<AssetServer>) {
    let handle: Handle<Gltf> = assets.load("models/FlightHelmet/FlightHelmet.gltf");
    commands.insert_resource(handle);
    commands.insert_resource(false);

    commands.spawn_bundle(PerspectiveCameraBundle {
        transform: Transform::from_xyz(0.7, 0.7, 1.0).looking_at(Vec3::new(0.0, 0.3, 0.0), Vec3::Y),
        ..Default::default()
    });
    const HALF_SIZE: f32 = 1.0;
    commands.spawn_bundle(DirectionalLightBundle {
        directional_light: DirectionalLight {
            shadow_projection: OrthographicProjection {
                left: -HALF_SIZE,
                right: HALF_SIZE,
                bottom: -HALF_SIZE,
                top: HALF_SIZE,
                near: -10.0 * HALF_SIZE,
                far: 10.0 * HALF_SIZE,
                ..Default::default()
            },
            shadows_enabled: true,
            ..Default::default()
        },
        ..Default::default()
    });
}

pub fn spawn_gltf_objects(
    mut commands: Commands,
    mut done: ResMut<bool>,
    gltf_handle: Res<Handle<Gltf>>,
    assets_gltf: Res<Assets<Gltf>>,
    assets_gltfnode: Res<Assets<GltfNode>>,
) {
    if *done {
        return;
    }

    // if the GLTF has loaded, we can navigate its contents
    if let Some(gltf) = assets_gltf.get(gltf_handle.clone()) {
        // Bevy allows you to easily spawn the entire scene.
        // However since the Scene object here is a Bevy `Scene` object
        // the mapping between Scene and GltfNodes is impossible to recover.
        let scene_handle: &Handle<Scene> = &gltf.scenes[0];
        commands.spawn_scene(scene_handle.clone());
        // You can get a Vec of all top-level GltfNodes used in this scene
        // through the `scene_to_nodes` map. This also enables you to recurse through the
        // hierarchy by yourself should you need to do that.
        let nodes: Vec<&GltfNode> = gltf.scene_to_nodes[scene_handle]
            .iter()
            .filter_map(|handle| assets_gltfnode.get(handle))
            .collect::<Vec<_>>();

        info!(
            "The following nodes are currently being displayed {:#?}",
            nodes
        );
        *done = true;
    }
}

#[cfg(test)]
mod test {
    use bevy::app::AppExit;
    use bevy::{
        gltf::{Gltf, GltfNode},
        prelude::*,
    };
    #[test]
    fn test_scene_to_nodes() {
        App::new()
            .add_plugins(DefaultPlugins)
            .add_startup_system(setup)
            .add_system(spawn_gltf_objects)
            .run();
    }

    fn setup(mut commands: Commands, assets: Res<AssetServer>) {
        let handle: Handle<Gltf> = assets.load("models/FlightHelmet/FlightHelmet.gltf");
        commands.insert_resource(handle);
    }

    pub fn spawn_gltf_objects(
        gltf_handle: Res<Handle<Gltf>>,
        assets_gltf: Res<Assets<Gltf>>,
        assets_gltfnode: Res<Assets<GltfNode>>,
        mut exit: EventWriter<AppExit>,
    ) {
        // if the GLTF has loaded, we can navigate its contents
        if let Some(gltf) = assets_gltf.get(gltf_handle.clone()) {
            let scene_handle: &Handle<Scene> = &gltf.scenes[0];
            let nodes: Vec<&GltfNode> = gltf.scene_to_nodes[scene_handle]
                .iter()
                .filter_map(|handle| assets_gltfnode.get(handle))
                .collect::<Vec<_>>();
            assert_eq!(nodes.len(), 6);
            assert_eq!(nodes[0].children.len(), 0);
            // If the asserts ran successfully, we can exit the app
            exit.send(AppExit);
        }
    }
}
