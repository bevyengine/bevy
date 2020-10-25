use bevy::prelude::*;

fn main() {
    App::build()
        .add_default_plugins()
        .add_startup_system(setup.system())
        .run();
}

fn setup(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    let texture_handle = asset_server.load("assets/branding/icon.png").unwrap();

    commands
        .spawn(Camera2dComponents::default())
        .spawn(SpriteComponents {
            material: materials.add(texture_handle.into()),
            ..Default::default()
        })
        .with(AnimationSplineTransform {
            translation_x: Spline::from_vec(vec![
                Key::new(0.0, 0.0, Interpolation::Cosine),
                Key::new(1.0, -150.0, Interpolation::Cosine),
                Key::new(2.0, 100.0, Interpolation::Cosine),
                Key::new(3.0, 0.0, Interpolation::Cosine),
            ]),
            translation_y: Spline::from_vec(vec![
                Key::new(0.0, 100.0, Interpolation::Linear),
                Key::new(1.5, -100.0, Interpolation::Linear),
                Key::new(3.0, 100.0, Interpolation::Linear),
            ]),
            rotation: Spline::from_vec(vec![
                Key::new(0.0, Quat::identity().into(), Interpolation::Linear),
                Key::new(0.5, Quat::from_xyzw(0., 0., 1., 0.).into(), Interpolation::Linear),
                Key::new(1.0, Quat::from_xyzw(0., 0., 0., -1.).into(), Interpolation::Linear),
                Key::new(1.5, Quat::from_xyzw(0., 0., -1., 0.).into(), Interpolation::Linear),
                Key::new(2.0, Quat::from_xyzw(0., 0., 0., 1.).into(), Interpolation::Linear),
            ]),
            translation_z: Spline::from_vec(vec![]),
            scale: Spline::from_vec(vec![
                Key::new(0.0, 1.0, Interpolation::Cosine),
                Key::new(0.5, 1.5, Interpolation::Cosine),
                Key::new(1.0, 1.0, Interpolation::Cosine),
            ]),
            loop_style: LoopStyle::PingPong,
            ..Default::default()
        });
}
