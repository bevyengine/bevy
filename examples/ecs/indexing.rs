use bevy::prelude::*;

#[derive(Debug, Hash, PartialEq, Eq, Clone, Copy)]
struct Position {
    x: i32,
    y: i32,
}
#[derive(Debug)]
struct Selected(Position);
struct Number(i32);

fn main() {
    App::build()
        .add_plugins(DefaultPlugins)
        .add_index::<Position>()
        .init_resource::<Materials>()
        .insert_resource(Selected(Position { x: 0, y: 0 }))
        .add_startup_system(setup.system())
        .add_system(update.system())
        .run();
}

fn setup(mut commands: Commands, materials: Res<Materials>, asset_server: Res<AssetServer>) {
    commands.spawn(UiCameraBundle::default());
    let font = asset_server.load("fonts/FiraSans-Bold.ttf");
    for x in 0..8 {
        for y in 0..8 {
            commands
                .spawn(NodeBundle {
                    style: Style {
                        position_type: PositionType::Absolute,
                        position: Rect {
                            left: Val::Px(100. * x as f32),
                            top: Val::Px(100. * y as f32),
                            ..Default::default()
                        },
                        size: Size::new(Val::Px(100.), Val::Px(100.)),
                        ..Default::default()
                    },
                    material: if (x + y) % 2 == 1 {
                        materials.white.clone()
                    } else {
                        materials.black.clone()
                    },
                    ..Default::default()
                })
                .with_children(|parent| {
                    parent
                        .spawn(TextBundle {
                            text: Text::with_section(
                                "0",
                                TextStyle {
                                    font: font.clone(),
                                    font_size: 90.,
                                    color: Color::RED,
                                },
                                TextAlignment {
                                    vertical: VerticalAlign::Center,
                                    horizontal: HorizontalAlign::Center,
                                },
                            ),
                            ..Default::default()
                        })
                        .with(Position { x, y })
                        .with(Number(0));
                });
        }
    }
}

struct Materials {
    black: Handle<ColorMaterial>,
    white: Handle<ColorMaterial>,
    green: Handle<ColorMaterial>,
}

impl FromWorld for Materials {
    fn from_world(world: &mut World) -> Self {
        let mut materials = world.get_resource_mut::<Assets<ColorMaterial>>().unwrap();
        Self {
            white: materials.add(Color::WHITE.into()),
            black: materials.add(Color::BLACK.into()),
            green: materials.add(Color::GREEN.into()),
        }
    }
}

fn update(
    mut query: Query<(&Position, &Parent, &mut Number, &mut Text)>,
    mut parent_query: Query<&mut Handle<ColorMaterial>>,
    index: Res<Index<Position>>,
    mut selected: ResMut<Selected>,
    input: Res<Input<KeyCode>>,
    mats: Res<Materials>,
) {
    let old_selected = selected.0;
    let selected = &mut selected.0;
    let mut increment = 0;
    for c in input.get_just_pressed() {
        match c {
            KeyCode::Left => selected.x -= 1,
            KeyCode::Right => selected.x += 1,
            KeyCode::Up => selected.y -= 1,
            KeyCode::Down => selected.y += 1,
            KeyCode::Return => increment += 1,
            KeyCode::Back => increment -= 1,
            _ => (),
        }
    }
    selected.x = selected.x.clamp(0, 7);
    selected.y = selected.y.clamp(0, 7);
    dbg!(&selected);
    if let Some((_, parent, mut num, mut text)) = index
        .get(&selected)
        .and_then(|i| i.iter().next())
        .and_then(|e| query.get_mut(*e).ok())
    {
        let mut mat = parent_query.get_mut(parent.0).unwrap();
        num.0 += increment;
        text.sections[0].value = num.0.to_string();

        if *selected != old_selected {
            *mat = mats.green.clone();
            if let Some((pos, parent, _, _)) = index
                .get(&old_selected)
                .and_then(|i| i.iter().next())
                .and_then(|e| query.get_mut(*e).ok())
            {
                let mut mat = parent_query.get_mut(parent.0).unwrap();
                *mat = if (pos.x + pos.y) % 2 == 1 {
                    mats.white.clone()
                } else {
                    mats.black.clone()
                };
            }
        }
    }
}
