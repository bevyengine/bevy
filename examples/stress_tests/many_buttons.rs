use bevy::{
    diagnostic::{FrameTimeDiagnosticsPlugin, LogDiagnosticsPlugin},
    prelude::*,
    window::PresentMode,
};

// For a total of 110 * 110 = 12100 buttons with text
const ROW_COLUMN_COUNT: usize = 110;
const FONT_SIZE: f32 = 7.0;

/// This example shows what happens when there is a lot of buttons on screen.
fn main() {
    App::new()
        .insert_resource(WindowDescriptor {
            present_mode: PresentMode::Immediate,
            ..default()
        })
        .add_plugins(DefaultPlugins)
        .add_plugin(FrameTimeDiagnosticsPlugin::default())
        .add_plugin(LogDiagnosticsPlugin::default())
        .init_resource::<UiFont>()
        .add_startup_system(setup)
        .add_system(button_system)
        .run();
}

#[derive(Component)]
struct IdleColor(UiColor);

fn button_system(
    mut interaction_query: Query<(&Interaction, &mut UiColor, &IdleColor), Changed<Interaction>>,
) {
    for (interaction, mut material, IdleColor(idle_color)) in interaction_query.iter_mut() {
        if matches!(interaction, Interaction::Hovered) {
            *material = Color::ORANGE_RED.into();
        } else {
            *material = *idle_color;
        }
    }
}

struct UiFont(Handle<Font>);
impl FromWorld for UiFont {
    fn from_world(world: &mut World) -> Self {
        let asset_server = world.get_resource::<AssetServer>().unwrap();
        UiFont(asset_server.load("fonts/FiraSans-Bold.ttf"))
    }
}

fn setup(mut commands: Commands, font: Res<UiFont>) {
    let count = ROW_COLUMN_COUNT;
    let count_f = count as f32;
    let as_rainbow = |i: usize| Color::hsl((i as f32 / count_f) * 360.0, 0.9, 0.8);
    commands.spawn_bundle(Camera2dBundle::default());
    commands
        .spawn_bundle(NodeBundle {
            style: Style {
                size: Size::new(Val::Percent(100.0), Val::Percent(100.0)),
                ..default()
            },
            ..default()
        })
        .with_children(|commands| {
            for i in 0..count {
                for j in 0..count {
                    let color = as_rainbow(j % i.max(1)).into();
                    spawn_button(commands, font.0.clone_weak(), color, count_f, i, j);
                }
            }
        });
}
fn spawn_button(
    commands: &mut ChildBuilder,
    font: Handle<Font>,
    color: UiColor,
    total: f32,
    i: usize,
    j: usize,
) {
    let width = 90.0 / total;
    commands
        .spawn_bundle(ButtonBundle {
            style: Style {
                size: Size::new(Val::Percent(width), Val::Percent(width)),

                position: UiRect {
                    bottom: Val::Percent(100.0 / total * i as f32),
                    left: Val::Percent(100.0 / total * j as f32),
                    ..default()
                },
                align_items: AlignItems::Center,
                position_type: PositionType::Absolute,
                ..default()
            },
            color,
            ..default()
        })
        .with_children(|commands| {
            commands.spawn_bundle(TextBundle::from_section(
                format!("{i}, {j}"),
                TextStyle {
                    font,
                    font_size: FONT_SIZE,
                    color: Color::rgb(0.2, 0.2, 0.2),
                },
            ));
        })
        .insert(IdleColor(color));
}
