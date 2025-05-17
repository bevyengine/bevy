//! Demonstrates implementing a cooldown in UI.
//! We create three buttons with 2, 1, and 5 seconds cooldown.

use bevy::{
    color::palettes::tailwind,
    ecs::spawn::SpawnIter,
    prelude::*,
};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_systems(Startup, setup)
        .add_systems(Update, (activate_ability, animate_cooldowns))
        .run();
}

fn setup(
    mut commands: Commands,
    mut texture_atlas_layouts: ResMut<Assets<TextureAtlasLayout>>,
    asset_server: Res<AssetServer>,
) {
    commands.spawn(Camera2d);
    let texture = asset_server.load("textures/food_kenney.png");
    let layout = TextureAtlasLayout::from_grid(UVec2::splat(64), 7, 7, None, None);
    let texture_atlas_layout = texture_atlas_layouts.add(layout);
    commands.spawn((
        Node {
            width: Val::Percent(100.),
            height: Val::Percent(100.),
            align_items: AlignItems::Center,
            justify_content: JustifyContent::Center,
            column_gap: Val::Px(15.),
            ..default()
        },
        Children::spawn(SpawnIter(
            [
                Ability {
                    name: "an apple",
                    cooldown: 2.,
                    index: 2,
                },
                Ability {
                    name: "a burger",
                    cooldown: 1.,
                    index: 23,
                },
                Ability {
                    name: "chocolate",
                    cooldown: 10.,
                    index: 32,
                },
                Ability {
                    name: "cherries",
                    cooldown: 4.,
                    index: 41,
                },
            ]
            .into_iter()
            .map(move |ability| {
                build_ability(ability, texture.clone(), texture_atlas_layout.clone())
            }),
        )),
    ));
    commands.spawn((
        Text::new("*Click some food to eat it*"),
        Node {
            position_type: PositionType::Absolute,
            top: Val::Px(5.0),
            left: Val::Px(15.0),
            ..default()
        },
    ));
}

struct Ability {
    name: &'static str,
    cooldown: f32,
    index: usize,
}

fn build_ability(
    ability: Ability,
    texture: Handle<Image>,
    layout: Handle<TextureAtlasLayout>,
) -> impl Bundle {
    let Ability {
        name,
        cooldown,
        index,
    } = ability;
    let name = Name::new(name);

    (
        Node {
            width: Val::Px(80.0),
            height: Val::Px(80.0),
            flex_direction: FlexDirection::ColumnReverse,
            overflow: Overflow::clip(),
            overflow_clip_margin: OverflowClipMargin::content_box(),
            ..default()
        },
        BackgroundColor(SLATE_400.into()),
        Button,
        ImageNode::from_atlas_image(texture, TextureAtlas { layout, index }),
        Cooldown(Timer::from_seconds(cooldown, TimerMode::Once)),
        name,
        children![(
            Node {
                width: Val::Percent(100.),
                height: Val::Percent(0.),
                ..default()
            },
            BackgroundColor(SLATE_50.with_alpha(0.5).into()),
        )],
    )
}

#[derive(Component)]
struct Cooldown(Timer);

#[derive(Component)]
#[component(storage = "SparseSet")]
struct ActiveCooldown;

fn activate_ability(
    mut commands: Commands,
    mut interaction_query: Query<
        (Entity, &Interaction, &mut Cooldown, &Name),
        (Changed<Interaction>, With<Button>, Without<ActiveCooldown>),
    >,
    mut text: Query<&mut Text>,
) -> Result {
    for (entity, interaction, mut cooldown, name) in &mut interaction_query {
        if *interaction == Interaction::Pressed {
            cooldown.0.reset();
            commands.entity(entity).insert(ActiveCooldown);
            **text.single_mut()? = format!("You ate {name}");
        }
    }

    Ok(())
}

fn animate_cooldowns(
    time: Res<Time>,
    mut commands: Commands,
    buttons: Query<(Entity, &mut Cooldown, &Children), With<ActiveCooldown>>,
    mut nodes: Query<&mut Node>,
) -> Result {
    for (entity, mut timer, children) in buttons {
        timer.0.tick(time.delta());
        let cooldown = children.first().ok_or("No child")?;
        if timer.0.just_finished() {
            commands.entity(entity).remove::<ActiveCooldown>();
            nodes.get_mut(*cooldown)?.height = Val::Percent(0.);
        } else {
            nodes.get_mut(*cooldown)?.height = Val::Percent((1. - timer.0.fraction()) * 100.);
        }
    }

    Ok(())
}
