use bevy::prelude::*;

const MAX_WIDTH: f32 = 400.;
const MAX_HEIGHT: f32 = 400.;

fn main() {
    App::new()
        .insert_resource(WindowDescriptor {
            width: MAX_WIDTH,
            height: MAX_HEIGHT,
            scale_factor_override: Some(1.),
            ..Default::default()
        })
        .add_plugins(DefaultPlugins)
        .insert_resource(Phase::ContractingY)
        .add_system(change_window_size)
        .add_startup_system(setup)
        .run();
}

fn setup(mut commands: Commands) {
    commands.spawn_bundle(OrthographicCameraBundle::new_3d());
}

enum Phase {
    ContractingY,
    ContractingX,
    ExpandingY,
    ExpandingX,
}

use Phase::*;

fn change_window_size(mut windows: ResMut<Windows>, mut phase: ResMut<Phase>) {
    let primary = windows.get_primary_mut().unwrap();
    let height = primary.height();
    let width = primary.width();
    match *phase {
        Phase::ContractingY => {
            if height <= 0.5 {
                *phase = ContractingX;
            }
            primary.set_resolution(width, (height - 4.).max(0.0))
        }
        Phase::ContractingX => {
            if width <= 0.5 {
                *phase = ExpandingY;
            }
            primary.set_resolution((width - 4.).max(0.0), height)
        }
        Phase::ExpandingY => {
            if height >= MAX_HEIGHT {
                *phase = ExpandingX;
            }
            primary.set_resolution(width, height + 4.)
        }
        Phase::ExpandingX => {
            if width >= MAX_WIDTH {
                *phase = ContractingY;
            }
            primary.set_resolution(width + 4., height)
        }
    }
}
