use bevy::prelude::*;

fn main() {
    App::build()
        .add_plugins(DefaultPlugins)
        .add_startup_system(setup.system())
        .add_system(sprite_movement.system())
        .run();
}

struct BevyLogo {
    rising: bool,
}

fn setup(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    let texture_handle = asset_server.load("branding/icon.png");
    commands.spawn_bundle(OrthographicCameraBundle::new_2d());
    commands
        .spawn_bundle(SpriteBundle {
            material: materials.add(texture_handle.into()),
            ..Default::default()
        })
        .insert(BevyLogo { rising: true });
}

fn sprite_movement(time: Res<Time>, mut sprite_position: Query<(&mut BevyLogo, &mut Transform)>) {
    for (mut logo, mut transform) in sprite_position.iter_mut() {
        if logo.rising {
            transform.translation.y += 150. * time.delta_seconds();
        } else {
            transform.translation.y -= 150. * time.delta_seconds();
        }

        if transform.translation.y > 200. {
            logo.rising = false;
        } else if transform.translation.y < -200. {
            logo.rising = true;
        }
    }
}
