//! This example demonstrates how you can implement a cooldown in UI.
//! We create three buttons with 2, 1, and 5 seconds cooldown.

use std::{any::TypeId, time::Duration};

use bevy::{
    animation::{AnimationEntityMut, AnimationEvaluationError, AnimationTarget, AnimationTargetId},
    color::palettes::tailwind::RED_400,
    prelude::*,
};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_systems(Startup, setup)
        .add_systems(Update, activate_ability)
        .run();
}

fn setup(
    mut commands: Commands,
    mut animation_graphs: ResMut<Assets<AnimationGraph>>,
    mut animation_clips: ResMut<Assets<AnimationClip>>,
    asset_server: Res<AssetServer>,
) {
    commands.spawn(Camera2d);
    let abilities = [
        (
            "Hat Guy",
            Duration::from_secs(2),
            asset_server.load("textures/rpg/chars/hat-guy/hat-guy.png"),
        ),
        (
            "Sensei",
            Duration::from_secs(1),
            asset_server.load("textures/rpg/chars/sensei/sensei.png"),
        ),
        (
            "Bee",
            Duration::from_secs(4),
            asset_server.load("textures/rpg/mobs/boss_bee.png"),
        ),
    ];
    commands
        .spawn(Node {
            width: Val::Percent(100.),
            height: Val::Percent(100.),
            align_items: AlignItems::Center,
            justify_content: JustifyContent::Center,
            column_gap: Val::Px(15.),
            ..default()
        })
        .with_children(|root| {
            // we need to get an entity to target for the animation
            for ability in abilities {
                let mut button = root.spawn(());
                let button_id = button.id();
                button.insert(build_ability(
                    ability,
                    &mut animation_graphs,
                    &mut animation_clips,
                    button_id,
                ));
            }
        });
    commands.spawn((
        Text::new("*Click an ability to activate it*"),
        Node {
            position_type: PositionType::Absolute,
            top: Val::Px(5.0),
            left: Val::Px(15.0),
            ..default()
        },
    ));
}

type Ability = (&'static str, Duration, Handle<Image>);

fn build_ability(
    ability: Ability,
    animation_graphs: &mut Assets<AnimationGraph>,
    animation_clips: &mut Assets<AnimationClip>,
    target: Entity,
) -> impl Bundle {
    let (name, cooldown, icon) = ability;
    let name = Name::new(name);
    let animation_target_id = AnimationTargetId::from_name(&name);

    let mut animation_clip = AnimationClip::default();

    // Create a curve that animates the cooldown UI
    animation_clip.add_curve_to_target(
        animation_target_id,
        AnimatableCurve::new(
            CooldownProperty,
            AnimatableKeyframeCurve::new([(0.0, 100.), (1.0, 0.)]).expect(
                "should be able to build translation curve because we pass in valid samples",
            ),
        ),
    );
    animation_clip.add_event_fn(1.0, |commands, entity, _, _| {
        commands.entity(entity).remove::<AbilityDeactivated>();
    });
    // Save our animation clip as an asset.
    let animation_clip_handle = animation_clips.add(animation_clip);

    // Create an animation graph with that clip.
    let (animation_graph, animation_node_index) = AnimationGraph::from_clip(animation_clip_handle);
    let animation_graph_handle = animation_graphs.add(animation_graph);

    (
        Node {
            width: Val::Px(80.0),
            height: Val::Px(80.0),
            flex_direction: FlexDirection::ColumnReverse,
            overflow: Overflow::clip(),
            overflow_clip_margin: OverflowClipMargin::content_box(),
            ..default()
        },
        BackgroundColor(RED_400.into()),
        Button,
        AnimationPlayer::default(),
        AnimationGraphHandle(animation_graph_handle),
        HeightAnimationNode(animation_node_index),
        ImageNode::new(icon),
        Cooldown(cooldown),
        children![(
            cooldown_cover(),
            AnimationTarget {
                id: AnimationTargetId::from_name(&name),
                player: target,
            }
        )],
        name,
    )
}

#[derive(Component)]
struct HeightAnimationNode(AnimationNodeIndex);

fn cooldown_cover() -> impl Bundle {
    return (
        Node {
            width: Val::Percent(100.),
            height: Val::Percent(0.),
            ..default()
        },
        BackgroundColor(Color::srgba(1.0, 1.0, 1.0, 0.5)),
    );
}

#[derive(Component)]
struct Cooldown(Duration);

#[derive(Clone)]
struct CooldownProperty;

#[derive(Component)]
#[component(storage = "SparseSet")]
struct AbilityDeactivated;

impl AnimatableProperty for CooldownProperty {
    type Property = f32;

    fn evaluator_id(&self) -> EvaluatorId {
        EvaluatorId::Type(TypeId::of::<Self>())
    }

    fn get_mut<'a>(
        &self,
        entity: &'a mut AnimationEntityMut,
    ) -> Result<&'a mut Self::Property, AnimationEvaluationError> {
        let node = entity
            .get_mut::<Node>()
            .ok_or(AnimationEvaluationError::ComponentNotPresent(TypeId::of::<
                Node,
            >(
            )))?
            .into_inner();

        match node.height {
            Val::Percent(ref mut percent) => Ok(percent),
            _ => Err(AnimationEvaluationError::PropertyNotPresent(TypeId::of::<
                f32,
            >(
            ))),
        }
    }
}

fn activate_ability(
    mut commands: Commands,
    mut interaction_query: Query<
        (
            Entity,
            &Interaction,
            &Cooldown,
            &mut AnimationPlayer,
            &HeightAnimationNode,
            &Name,
        ),
        (
            Changed<Interaction>,
            With<Button>,
            Without<AbilityDeactivated>,
        ),
    >,
    mut text: Query<&mut Text>,
) -> Result {
    for (entity, interaction, cooldown, mut player, node_id, name) in &mut interaction_query {
        match *interaction {
            Interaction::Pressed => {
                // our animation curve is one second long
                player
                    .play(node_id.0)
                    .set_speed(1. / cooldown.0.as_secs_f32())
                    .replay();
                commands.entity(entity).insert(AbilityDeactivated);
                **text.single_mut()? = format!("Activated {name}");
            }
            _ => (),
        }
    }

    Ok(())
}
