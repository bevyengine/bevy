//! Shows the behavior of having multiple [`DirectionalLight`]
//! with [`Atmosphere`] enabled

use bevy::{
    anti_alias::fxaa::Fxaa,
    camera::Exposure,
    core_pipeline::tonemapping::Tonemapping,
    input::common_conditions::input_just_pressed,
    light::{AtmosphereEnvironmentMapLight, VolumetricLight},
    pbr::{Atmosphere, AtmosphereSettings, ScatteringMedium},
    post_process::bloom::Bloom,
    prelude::*,
};

fn main() {
    let mut app = App::new();

    app.add_plugins(DefaultPlugins);

    app.add_systems(Startup, setup);
    // Setup systems to change which light will have the VolumetricLight
    // component
    app.add_systems(
        Update,
        (
            change_directional_lights_with_scattering::<0>
                .run_if(input_just_pressed(KeyCode::Digit0)),
            change_directional_lights_with_scattering::<1>
                .run_if(input_just_pressed(KeyCode::Digit1)),
            change_directional_lights_with_scattering::<2>
                .run_if(input_just_pressed(KeyCode::Digit2)),
            change_directional_lights_with_scattering::<3>
                .run_if(input_just_pressed(KeyCode::Digit3)),
            change_directional_lights_with_scattering::<4>
                .run_if(input_just_pressed(KeyCode::Digit4)),
            change_directional_lights_with_scattering::<5>
                .run_if(input_just_pressed(KeyCode::Digit5)),
            change_directional_lights_with_scattering::<6>
                .run_if(input_just_pressed(KeyCode::Digit6)),
            change_directional_lights_with_scattering::<7>
                .run_if(input_just_pressed(KeyCode::Digit7)),
        ),
    );

    app.run();
}

#[derive(Component)]
struct DirectionalLightFlag(u8);

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut scattering_mediums: ResMut<Assets<ScatteringMedium>>,
) {
    commands.spawn((
        Camera3d::default(),
        Transform::from_translation(Vec3::new(0., 5., 15.)),
        // get the default `Atmosphere` component
        Atmosphere::earthlike(scattering_mediums.add(ScatteringMedium::default())),
        // Can be adjusted to change the scene scale and rendering quality
        AtmosphereSettings::default(),
        // The directional light illuminance used in this scene
        // (the one recommended for use with this feature) is
        // quite bright, so raising the exposure compensation helps
        // bring the scene to a nicer brightness range.
        Exposure { ev100: 13.0 },
        // Tonemapper chosen just because it looked good with the scene, any
        // tonemapper would be fine :)
        Tonemapping::AcesFitted,
        // Bloom gives the sun a much more natural look.
        Bloom::NATURAL,
        // Enables the atmosphere to drive reflections and ambient lighting (IBL) for this view
        AtmosphereEnvironmentMapLight::default(),
        Msaa::Off,
        Fxaa::default(),
    ));

    // Some objects for the scene
    let material = materials.add(StandardMaterial::from_color(Color::WHITE));
    commands.spawn((
        Mesh3d(meshes.add(Plane3d::new(Vec3::Y, Vec2::splat(10.)))),
        MeshMaterial3d(material.clone()),
    ));
    commands.spawn((
        Mesh3d(meshes.add(Cuboid::new(2., 2., 2.))),
        MeshMaterial3d(material.clone()),
        Transform::from_translation(Vec3::new(0., 1., 0.)),
    ));

    // Directional lights
    commands.spawn((
        DirectionalLight {
            color: Color::srgb(1., 0., 0.),
            shadows_enabled: true,
            ..Default::default()
        },
        Transform::from_translation(Vec3::new(0., 5., -15.)).looking_at(Vec3::ZERO, Vec3::Y),
        VolumetricLight,
        DirectionalLightFlag(0b1),
    ));
    commands.spawn((
        DirectionalLight {
            color: Color::srgb(0., 1., 0.),
            shadows_enabled: true,
            ..Default::default()
        },
        Transform::from_translation(Vec3::new(-5., 5., -15.)).looking_at(Vec3::ZERO, Vec3::Y),
        DirectionalLightFlag(0b10),
    ));
    commands.spawn((
        DirectionalLight {
            color: Color::srgb(0., 0., 1.),
            shadows_enabled: true,
            ..Default::default()
        },
        Transform::from_translation(Vec3::new(5., 5., -15.)).looking_at(Vec3::ZERO, Vec3::Y),
        DirectionalLightFlag(0b100),
    ));

    // Legend
    commands.spawn((
        Node {
            top: Val::Px(0.),
            left: Val::Px(0.),
            padding: UiRect::all(Val::Px(4.)),
            flex_direction: FlexDirection::Column,
            row_gap: Val::Px(4.),
            ..Default::default()
        },
        children![
            Text::new("[1]~[7] Change directional light with VolumetricLight"),
            Text::new("[0] Disable VolumetricLight for all lights")
        ],
    ));
}

/// Changes which [`DirectionalLight`] will have [`VolumetricLight`].
/// The [`DirectionalLightFlag`] is compared with the generic N to determinate
/// which of the lights will receive [`VolumetricLight`] and which will
/// have it remove.
fn change_directional_lights_with_scattering<const N: u8>(
    mut commands: Commands,
    directional_lights: Query<(Entity, &DirectionalLightFlag), With<DirectionalLight>>,
) {
    for (entity, flag) in directional_lights {
        if (flag.0 & N) != 0 {
            commands.entity(entity).insert(VolumetricLight);
        } else {
            commands.entity(entity).remove::<VolumetricLight>();
        }
    }
}
