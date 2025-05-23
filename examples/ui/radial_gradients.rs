//! Simple example demonstrating radial gradients.

use bevy::color::palettes::css::GREEN;
use bevy::color::palettes::css::NAVY;
use bevy::color::palettes::css::RED;
use bevy::prelude::*;
use bevy::ui::ColorStop;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_systems(Startup, setup_grid)
        .run();
}

const CELL_SIZE: f32 = 100.;
const GAP: f32 = 10.;

fn setup_grid(mut commands: Commands) {
    let color_stops = vec![
        ColorStop::new(Color::BLACK, Val::Px(5.)),
        ColorStop::new(Color::WHITE, Val::Px(5.)),
        ColorStop::new(Color::WHITE, Val::Percent(100.)),
        ColorStop::auto(RED),
    ];

    commands.spawn(Camera2d);
    commands
        .spawn((
            Node {
                width: Val::Percent(100.),
                height: Val::Percent(100.),
                display: Display::Grid,
                align_items: AlignItems::Start,
                align_content: AlignContent::Start,
                grid_template_columns: vec![RepeatedGridTrack::px(
                    GridTrackRepetition::AutoFill,
                    CELL_SIZE,
                )],
                grid_auto_flow: GridAutoFlow::Row,
                row_gap: Val::Px(GAP),
                column_gap: Val::Px(GAP),
                margin: UiRect::all(Val::Px(GAP)),
                ..Default::default()
            },
            BackgroundColor(NAVY.into()),
        ))
        .with_children(|commands| {
            for shape in [
                RadialGradientShape::ClosestSide,
                RadialGradientShape::FarthestSide,
                RadialGradientShape::Circle(Val::Percent(55.)),
                RadialGradientShape::FarthestCorner,
            ] {
                for position in [
                    Position::TOP_LEFT,
                    Position::LEFT,
                    Position::BOTTOM_LEFT,
                    Position::TOP,
                    Position::CENTER,
                    Position::BOTTOM,
                    Position::TOP_RIGHT,
                    Position::RIGHT,
                    Position::BOTTOM_RIGHT,
                ] {
                    for (w, h) in [(100., 100.), (100., 50.)] {
                        commands
                            .spawn((
                                BackgroundColor(GREEN.into()),
                                Node {
                                    display: Display::Grid,
                                    width: Val::Px(CELL_SIZE),
                                    ..Default::default()
                                },
                            ))
                            .with_children(|commands| {
                                commands.spawn((
                                    Text(format!("{shape:#?}\n{position:#?}")),
                                    TextFont::from_font_size(10.),
                                ));
                                commands.spawn((
                                    Node {
                                        width: Val::Px(w),
                                        height: Val::Px(h),
                                        ..default()
                                    },
                                    BackgroundGradient::from(RadialGradient {
                                        stops: color_stops.clone(),
                                        position,
                                        shape,
                                        ..Default::default()
                                    }),
                                ));
                            });
                    }
                }
            }
        });
}
