use bevy::prelude::*;

fn main() {
    App::new()
        .insert_resource(ClearColor(Color::rgb(0.4, 0.4, 0.4)))
        .add_plugins(DefaultPlugins)
        .add_startup_system(setup)
        .run();
}

fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands.spawn_bundle(Camera2dBundle::default());

    commands
        .spawn_bundle(NodeBundle {
            // Root Panel, covers the entire screen
            style: Style {
                size: Size::new(Val::Percent(100.0), Val::Percent(100.0)),
                justify_content: JustifyContent::SpaceBetween,
                ..default()
            },
            color: Color::NONE.into(),
            ..default()
        })
        .with_children(|parent| {
            spawn_left_panel(parent, &asset_server);
        });
}

fn spawn_left_panel(parent: &mut ChildBuilder, asset_server: &Res<AssetServer>) {
    parent
        .spawn_bundle(NodeBundle {
            style: Style {
                flex_direction: FlexDirection::ColumnReverse,
                size: Size::new(Val::Percent(50.0), Val::Percent(100.0)),
                ..default()
            },
            color: Color::rgb(0.10, 0.10, 0.10).into(),
            ..default()
        })
        .with_children(|parent| {
            parent.spawn_bundle(
                TextBundle::from_section(
                    "This is a super long text, that I would hope would get wrapped. ".to_owned()
                        + "It should only fill half the container size",
                    TextStyle {
                        font: asset_server.load("fonts/FiraSans-Bold.ttf"),
                        font_size: 20.0,
                        color: Color::WHITE,
                    },
                )
                .with_style(Style {
                    max_size: Size::new(Val::Percent(50.0), Val::Undefined),
                    ..default()
                }),
            );
            parent.spawn_bundle(
                TextBundle::from_section(
                    "This is another super long text, that I would hope would get wrapped. "
                        .to_owned()
                        + "It should fill the whole container",
                    TextStyle {
                        font: asset_server.load("fonts/FiraSans-Bold.ttf"),
                        font_size: 20.0,
                        color: Color::GRAY,
                    },
                )
                .with_style(Style {
                    max_size: Size::new(Val::Percent(100.0), Val::Undefined),
                    ..default()
                }),
            );
        });
}
