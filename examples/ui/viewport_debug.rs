//! An example for debugging viewport coordinates

use bevy::prelude::*;

const PALETTE: [Color; 10] = [
    Color::ORANGE,
    Color::BLUE,
    Color::WHITE,
    Color::BEIGE,
    Color::CYAN,
    Color::CRIMSON,
    Color::NAVY,
    Color::AZURE,
    Color::GREEN,
    Color::BLACK,
];

#[derive(Default, Debug, Hash, Eq, PartialEq, Clone, States)]
enum Coords {
    #[default]
    Viewport,
    Pixel,
}

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                resolution: [800., 600.].into(),
                title: "Viewport Coordinates Debug".to_string(),
                resizable: false,
                ..Default::default()
            }),
            ..Default::default()
        }))
        .add_state::<Coords>()
        .add_systems(Startup, setup)
        .add_systems(OnEnter(Coords::Viewport), spawn_with_viewport_coords)
        .add_systems(OnEnter(Coords::Pixel), spawn_with_pixel_coords)
        .add_systems(OnExit(Coords::Viewport), despawn_nodes)
        .add_systems(OnExit(Coords::Pixel), despawn_nodes)
        .add_systems(Update, update)
        .run();
}

fn despawn_nodes(mut commands: Commands, query: Query<Entity, With<Node>>) {
    for entity in query.iter() {
        commands.entity(entity).despawn();
    }
}

fn update(
    mut timer: Local<f32>,
    time: Res<Time>,
    state: Res<State<Coords>>,
    mut next_state: ResMut<NextState<Coords>>,
) {
    *timer += time.delta_seconds();
    if 1. <= *timer {
        *timer = 0.;
        next_state.set(if *state.get() == Coords::Viewport {
            Coords::Pixel
        } else {
            Coords::Viewport
        });
    }
}

fn setup(mut commands: Commands) {
    commands.spawn(Camera2dBundle::default());
}

fn spawn_with_viewport_coords(mut commands: Commands) {
    commands
        .spawn(NodeBundle {
            style: Style {
                width: Val::Vw(100.),
                height: Val::Vh(100.),
                border: UiRect::axes(Val::Vw(5.), Val::Vh(5.)),
                flex_wrap: FlexWrap::Wrap,
                ..default()
            },
            background_color: PALETTE[0].into(),
            border_color: PALETTE[1].into(),
            ..default()
        })
        .with_children(|builder| {
            builder.spawn(NodeBundle {
                style: Style {
                    width: Val::Vw(30.),
                    height: Val::Vh(30.),
                    border: UiRect::all(Val::VMin(5.)),
                    ..default()
                },
                background_color: PALETTE[2].into(),
                border_color: PALETTE[9].into(),
                ..default()
            });

            builder.spawn(NodeBundle {
                style: Style {
                    width: Val::Vw(60.),
                    height: Val::Vh(30.),
                    ..default()
                },
                background_color: PALETTE[3].into(),
                ..default()
            });

            builder.spawn(NodeBundle {
                style: Style {
                    width: Val::Vw(45.),
                    height: Val::Vh(30.),
                    border: UiRect::left(Val::VMax(45. / 2.)),
                    ..default()
                },
                background_color: PALETTE[4].into(),
                border_color: PALETTE[8].into(),
                ..default()
            });

            builder.spawn(NodeBundle {
                style: Style {
                    width: Val::Vw(45.),
                    height: Val::Vh(30.),
                    border: UiRect::right(Val::VMax(45. / 2.)),
                    ..default()
                },
                background_color: PALETTE[5].into(),
                border_color: PALETTE[8].into(),
                ..default()
            });

            builder.spawn(NodeBundle {
                style: Style {
                    width: Val::Vw(60.),
                    height: Val::Vh(30.),
                    ..default()
                },
                background_color: PALETTE[6].into(),
                ..default()
            });

            builder.spawn(NodeBundle {
                style: Style {
                    width: Val::Vw(30.),
                    height: Val::Vh(30.),
                    border: UiRect::all(Val::VMin(5.)),
                    ..default()
                },
                background_color: PALETTE[7].into(),
                border_color: PALETTE[9].into(),
                ..default()
            });
        });
}

fn spawn_with_pixel_coords(mut commands: Commands) {
    commands
        .spawn(NodeBundle {
            style: Style {
                width: Val::Px(800.),
                height: Val::Px(600.),
                border: UiRect::axes(Val::Px(40.), Val::Px(30.)),
                flex_wrap: FlexWrap::Wrap,
                ..default()
            },
            background_color: PALETTE[1].into(),
            border_color: PALETTE[0].into(),
            ..default()
        })
        .with_children(|builder| {
            builder.spawn(NodeBundle {
                style: Style {
                    width: Val::Px(240.),
                    height: Val::Px(180.),
                    border: UiRect::axes(Val::Px(30.), Val::Px(30.)),
                    ..default()
                },
                background_color: PALETTE[2].into(),
                border_color: PALETTE[9].into(),
                ..default()
            });

            builder.spawn(NodeBundle {
                style: Style {
                    width: Val::Px(480.),
                    height: Val::Px(180.),
                    ..default()
                },
                background_color: PALETTE[3].into(),
                ..default()
            });

            builder.spawn(NodeBundle {
                style: Style {
                    width: Val::Px(360.),
                    height: Val::Px(180.),
                    border: UiRect::left(Val::Px(180.)),
                    ..default()
                },
                background_color: PALETTE[4].into(),
                border_color: PALETTE[8].into(),
                ..default()
            });

            builder.spawn(NodeBundle {
                style: Style {
                    width: Val::Px(360.),
                    height: Val::Px(180.),
                    border: UiRect::right(Val::Px(180.)),
                    ..default()
                },
                background_color: PALETTE[5].into(),
                border_color: PALETTE[8].into(),
                ..default()
            });

            builder.spawn(NodeBundle {
                style: Style {
                    width: Val::Px(480.),
                    height: Val::Px(180.),
                    ..default()
                },
                background_color: PALETTE[6].into(),
                ..default()
            });

            builder.spawn(NodeBundle {
                style: Style {
                    width: Val::Px(240.),
                    height: Val::Px(180.),
                    border: UiRect::axes(Val::Px(30.), Val::Px(30.)),
                    ..default()
                },
                background_color: PALETTE[7].into(),
                border_color: PALETTE[9].into(),
                ..default()
            });
        });
}
