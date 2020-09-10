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
        .with(Animator::<Translation> {
            splines: vec![
                Spline::from_vec(vec![
                    Key::new(0.0, 0.0, Interpolation::Cosine),
                    Key::new(1.0, -30.0, Interpolation::Cosine),
                    Key::new(2.0, 50.0, Interpolation::Cosine),
                    Key::new(3.0, 20.0, Interpolation::Cosine),
                ]),
                Spline::from_vec(vec![]),
                Spline::from_vec(vec![]),
            ],
            ..Default::default()
        });
}
