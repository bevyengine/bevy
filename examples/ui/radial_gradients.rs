//! Example demonstrating gradients

use bevy::prelude::*;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_systems(Startup, setup)
        .run();
}

fn inner(commands: &mut ChildBuilder, c: RelativePosition, stops: &Vec<ColorStop>) {
    for s in [
        RadialGradientShape::Circle(Val::Percent(25.).into()),
        RadialGradientShape::Circle(Val::Percent(40.).into()),
        RadialGradientShape::Circle(RadialGradientExtent::ClosestSide),
        RadialGradientShape::Circle(RadialGradientExtent::FarthestSide),
        RadialGradientShape::ClosestCorner,
        RadialGradientShape::FarthestCorner,
        RadialGradientShape::Ellipse(Val::Percent(40.).into(), Val::Percent(20.).into()),
        RadialGradientShape::Ellipse(Val::Percent(20.).into(), Val::Percent(40.).into()),
        RadialGradientShape::Ellipse(
            RadialGradientExtent::ClosestSide,
            RadialGradientExtent::ClosestSide,
        ),
        RadialGradientShape::Ellipse(
            RadialGradientExtent::ClosestSide,
            RadialGradientExtent::FarthestSide,
        ),
        RadialGradientShape::Ellipse(
            RadialGradientExtent::FarthestSide,
            RadialGradientExtent::ClosestSide,
        ),
        RadialGradientShape::Ellipse(
            RadialGradientExtent::FarthestSide,
            RadialGradientExtent::FarthestSide,
        ),
    ] {
        commands.spawn(NodeBundle {
            style: Style {
                width: Val::Px(75.),
                height: Val::Px(50.),
                ..Default::default()
            },
            background_color: RadialGradient::new(c, s, stops.clone()).into(),
            ..Default::default()
        });
    }
}

fn setup(mut commands: Commands) {
    commands.spawn(Camera2dBundle::default());
    let root = commands
        .spawn(NodeBundle {
            style: Style {
                align_items: AlignItems::Start,
                justify_content: JustifyContent::Start,
                flex_direction: FlexDirection::Column,
                width: Val::Percent(100.),
                height: Val::Percent(100.),
                row_gap: Val::Px(20.),
                column_gap: Val::Px(10.),
                ..Default::default()
            },
            ..Default::default()
        })
        .id();

    let group = spawn_group(&mut commands);

    commands.entity(root).add_child(group);
}

fn spawn_group(commands: &mut Commands) -> Entity {
    commands
        .spawn(NodeBundle {
            style: Style {
                align_items: AlignItems::Start,
                justify_content: JustifyContent::Start,
                row_gap: Val::Px(10.),
                column_gap: Val::Px(10.),
                ..Default::default()
            },
            ..Default::default()
        })
        .with_children(|commands| {
            commands
                .spawn(NodeBundle {
                    style: Style {
                        flex_direction: FlexDirection::Column,
                        row_gap: Val::Px(10.),
                        column_gap: Val::Px(10.),
                        ..Default::default()
                    },
                    ..Default::default()
                })
                .with_children(|commands| {
                    for c in [
                        RelativePosition::CENTER,
                        RelativePosition::new(
                            RelativePositionAxis::CENTER,
                            RelativePositionAxis::Start(Val::Percent(25.)),
                        ),
                        RelativePosition::new(
                            RelativePositionAxis::CENTER,
                            RelativePositionAxis::End(Val::Percent(25.)),
                        ),
                        RelativePosition::new(
                            RelativePositionAxis::Start(Val::Percent(25.)),
                            RelativePositionAxis::CENTER,
                        ),
                        RelativePosition::new(
                            RelativePositionAxis::End(Val::Percent(25.)),
                            RelativePositionAxis::CENTER,
                        ),
                        RelativePosition::new(
                            RelativePositionAxis::Start(Val::Percent(25.)),
                            RelativePositionAxis::Start(Val::Percent(25.)),
                        ),
                        RelativePosition::new(
                            RelativePositionAxis::End(Val::Percent(25.)),
                            RelativePositionAxis::Start(Val::Percent(25.)),
                        ),
                        RelativePosition::new(
                            RelativePositionAxis::Start(Val::Percent(25.)),
                            RelativePositionAxis::End(Val::Percent(25.)),
                        ),
                        RelativePosition::new(
                            RelativePositionAxis::End(Val::Percent(25.)),
                            RelativePositionAxis::End(Val::Percent(25.)),
                        ),
                    ] {
                        for stops in [
                            vec![
                                (Color::WHITE, Val::Auto).into(),
                                (Color::BLACK, Val::Auto).into(),
                            ],
                            vec![
                                (Color::RED, Val::Percent(10.)).into(),
                                (Color::GREEN, Val::Percent(20.)).into(),
                                (Color::GREEN, Val::Percent(30.)).into(),
                                (Color::BLUE, Val::Percent(30.)).into(),
                                (Color::BLUE, Val::Percent(40.)).into(),
                                (Color::YELLOW, Val::Auto).into(),
                            ],
                        ] {
                            commands
                                .spawn(NodeBundle {
                                    style: Style {
                                        flex_direction: FlexDirection::Row,
                                        row_gap: Val::Px(10.),
                                        column_gap: Val::Px(10.),
                                        ..Default::default()
                                    },
                                    ..Default::default()
                                })
                                .with_children(|commands| {
                                    inner(commands, c, &stops);
                                });
                        }
                    }
                });
        })
        .id()
}
