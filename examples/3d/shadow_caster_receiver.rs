use bevy::{
    pbr::{NotShadowCaster, NotShadowReceiver},
    prelude::*,
};

fn main() {
    println!(
        "Controls:
    C      - toggle shadow casters (i.e. casters become not, and not casters become casters)
    R      - toggle shadow receivers (i.e. receivers become not, and not receivers become receivers)
    L      - switch between directional and point lights"
    );
    App::new()
        .add_plugins(DefaultPlugins)
        .add_startup_system(setup)
        .add_system(toggle_light)
        .add_system(toggle_shadows)
        .run();
}

/// set up a 3D scene to test shadow biases and perspective projections
fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let spawn_plane_depth = 500.0f32;
    let spawn_height = 2.0;
    let sphere_radius = 0.25;

    let white_handle = materials.add(StandardMaterial {
        base_color: Color::WHITE,
        perceptual_roughness: 1.0,
        ..Default::default()
    });
    let sphere_handle = meshes.add(Mesh::from(shape::Icosphere {
        radius: sphere_radius,
        ..Default::default()
    }));

    // sphere - initially a caster
    commands.spawn_bundle(PbrBundle {
        mesh: sphere_handle.clone(),
        material: materials.add(Color::RED.into()),
        transform: Transform::from_xyz(-1.0, spawn_height, 0.0),
        ..Default::default()
    });

    // sphere - initially not a caster
    commands
        .spawn_bundle(PbrBundle {
            mesh: sphere_handle,
            material: materials.add(Color::BLUE.into()),
            transform: Transform::from_xyz(1.0, spawn_height, 0.0),
            ..Default::default()
        })
        .insert(NotShadowCaster);

    // floating plane - initially not a shadow receiver and not a caster
    commands
        .spawn_bundle(PbrBundle {
            mesh: meshes.add(Mesh::from(shape::Plane { size: 20.0 })),
            material: materials.add(Color::GREEN.into()),
            transform: Transform::from_xyz(0.0, 1.0, -10.0),
            ..Default::default()
        })
        .insert_bundle((NotShadowCaster, NotShadowReceiver));

    // lower ground plane - initially a shadow receiver
    commands.spawn_bundle(PbrBundle {
        mesh: meshes.add(Mesh::from(shape::Plane { size: 20.0 })),
        material: white_handle,
        ..Default::default()
    });

    println!("Using DirectionalLight");

    commands.spawn_bundle(PointLightBundle {
        transform: Transform::from_xyz(5.0, 5.0, 0.0),
        point_light: PointLight {
            intensity: 0.0,
            range: spawn_plane_depth,
            color: Color::WHITE,
            shadows_enabled: true,
            ..Default::default()
        },
        ..Default::default()
    });

    let theta = std::f32::consts::FRAC_PI_4;
    let light_transform = Mat4::from_euler(EulerRot::ZYX, 0.0, std::f32::consts::FRAC_PI_2, -theta);
    commands.spawn_bundle(DirectionalLightBundle {
        directional_light: DirectionalLight {
            illuminance: 100000.0,
            shadow_projection: OrthographicProjection {
                left: -10.0,
                right: 10.0,
                bottom: -10.0,
                top: 10.0,
                near: -50.0,
                far: 50.0,
                ..Default::default()
            },
            shadows_enabled: true,
            ..Default::default()
        },
        transform: Transform::from_matrix(light_transform),
        ..Default::default()
    });

    // camera
    commands.spawn_bundle(PerspectiveCameraBundle {
        transform: Transform::from_xyz(-5.0, 5.0, 5.0)
            .looking_at(Vec3::new(-1.0, 1.0, 0.0), Vec3::Y),
        ..Default::default()
    });
}

fn toggle_light(
    input: Res<Input<KeyCode>>,
    mut point_lights: Query<&mut PointLight>,
    mut directional_lights: Query<&mut DirectionalLight>,
) {
    if input.just_pressed(KeyCode::L) {
        for mut light in point_lights.iter_mut() {
            light.intensity = if light.intensity == 0.0 {
                println!("Using PointLight");
                100000000.0
            } else {
                0.0
            };
        }
        for mut light in directional_lights.iter_mut() {
            light.illuminance = if light.illuminance == 0.0 {
                println!("Using DirectionalLight");
                100000.0
            } else {
                0.0
            };
        }
    }
}

fn toggle_shadows(
    mut commands: Commands,
    input: Res<Input<KeyCode>>,
    mut queries: QuerySet<(
        QueryState<Entity, (With<Handle<Mesh>>, With<NotShadowCaster>)>,
        QueryState<Entity, (With<Handle<Mesh>>, With<NotShadowReceiver>)>,
        QueryState<Entity, (With<Handle<Mesh>>, Without<NotShadowCaster>)>,
        QueryState<Entity, (With<Handle<Mesh>>, Without<NotShadowReceiver>)>,
    )>,
) {
    if input.just_pressed(KeyCode::C) {
        println!("Toggling casters");
        for entity in queries.q0().iter() {
            commands.entity(entity).remove::<NotShadowCaster>();
        }
        for entity in queries.q2().iter() {
            commands.entity(entity).insert(NotShadowCaster);
        }
    }
    if input.just_pressed(KeyCode::R) {
        println!("Toggling receivers");
        for entity in queries.q1().iter() {
            commands.entity(entity).remove::<NotShadowReceiver>();
        }
        for entity in queries.q3().iter() {
            commands.entity(entity).insert(NotShadowReceiver);
        }
    }
}
