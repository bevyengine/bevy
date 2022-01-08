use bevy::prelude::*;

fn main() {
    App::new()
        .insert_resource(WindowDescriptor {
            width: 200.,
            height: 200.,
            scale_factor_override: Some(1.),
            ..Default::default()
        })
        .add_plugins(DefaultPlugins)
        .insert_resource(Phase::ContractingY)
        .add_system(change_window_size)
        .run();
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
            if height >= 200. {
                *phase = ExpandingX;
            }
            primary.set_resolution(width, height + 4.)
        }
        Phase::ExpandingX => {
            if width >= 200. {
                *phase = ContractingY;
            }
            primary.set_resolution(width + 4., height)
        }
    }
}
