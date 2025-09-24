//! Shows how to use animation clips to animate UI properties.

use bevy::{
    animation::{
        animated_field, AnimationEntityMut, AnimationEvaluationError, AnimationTarget,
        AnimationTargetId,
    },
    prelude::*,
};
use std::any::TypeId;

// Holds information about the animation we programmatically create.
struct AnimationInfo {
    // The name of the animation target (in this case, the text).
    target_name: Name,
    // The ID of the animation target, derived from the name.
    target_id: AnimationTargetId,
    // The animation graph asset.
    graph: Handle<AnimationGraph>,
    // The index of the node within that graph.
    node_index: AnimationNodeIndex,
}

// The entry point.
fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        // Note that we don't need any systems other than the setup system,
        // because Bevy automatically updates animations every frame.
        .add_systems(Startup, setup)
        .run();
}

impl AnimationInfo {
    // Programmatically creates the UI animation.
    fn create(
        animation_graphs: &mut Assets<AnimationGraph>,
        animation_clips: &mut Assets<AnimationClip>,
    ) -> AnimationInfo {
        // Create an ID that identifies the text node we're going to animate.
        let animation_target_name = Name::new("Text");
        let animation_target_id = AnimationTargetId::from_name(&animation_target_name);

        // Allocate an animation clip.
        let mut animation_clip = AnimationClip::default();

        // Create a curve that animates font size.
        animation_clip.add_curve_to_target(
            animation_target_id,
            AnimatableCurve::new(
                animated_field!(TextFont::font_size),
                AnimatableKeyframeCurve::new(
                    [0.0, 0.5, 1.0, 1.5, 2.0, 2.5, 3.0]
                        .into_iter()
                        .zip([24.0, 80.0, 24.0, 80.0, 24.0, 80.0, 24.0]),
                )
                .expect(
                    "should be able to build translation curve because we pass in valid samples",
                ),
            ),
        );

        // Create a curve that animates font color. Note that this should have
        // the same time duration as the previous curve.
        //
        // This time we use a "custom property", which in this case animates TextColor under the assumption
        // that it is in the "srgba" format.
        animation_clip.add_curve_to_target(
            animation_target_id,
            AnimatableCurve::new(
                TextColorProperty,
                AnimatableKeyframeCurve::new([0.0, 1.0, 2.0, 3.0].into_iter().zip([
                    Srgba::RED,
                    Srgba::GREEN,
                    Srgba::BLUE,
                    Srgba::RED,
                ]))
                .expect(
                    "should be able to build translation curve because we pass in valid samples",
                ),
            ),
        );

        // Save our animation clip as an asset.
        let animation_clip_handle = animation_clips.add(animation_clip);

        // Create an animation graph with that clip.
        let (animation_graph, animation_node_index) =
            AnimationGraph::from_clip(animation_clip_handle);
        let animation_graph_handle = animation_graphs.add(animation_graph);

        AnimationInfo {
            target_name: animation_target_name,
            target_id: animation_target_id,
            graph: animation_graph_handle,
            node_index: animation_node_index,
        }
    }
}

// Creates all the entities in the scene.
fn setup(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut animation_graphs: ResMut<Assets<AnimationGraph>>,
    mut animation_clips: ResMut<Assets<AnimationClip>>,
) {
    // Create the animation.
    let AnimationInfo {
        target_name: animation_target_name,
        target_id: animation_target_id,
        graph: animation_graph,
        node_index: animation_node_index,
    } = AnimationInfo::create(&mut animation_graphs, &mut animation_clips);

    // Build an animation player that automatically plays the UI animation.
    let mut animation_player = AnimationPlayer::default();
    animation_player.play(animation_node_index).repeat();

    // Add a camera.
    commands.spawn(Camera2d);

    // Build the UI. We have a parent node that covers the whole screen and
    // contains the `AnimationPlayer`, as well as a child node that contains the
    // text to be animated.
    let mut entity = commands.spawn((
        // Cover the whole screen, and center contents.
        Node {
            position_type: PositionType::Absolute,
            top: px(0),
            left: px(0),
            right: px(0),
            bottom: px(0),
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            ..default()
        },
        animation_player,
        AnimationGraphHandle(animation_graph),
    ));

    let player = entity.id();
    entity.insert(children![(
        Text::new("Bevy"),
        TextFont {
            font: asset_server.load("fonts/FiraSans-Bold.ttf"),
            font_size: 24.0,
            ..default()
        },
        TextColor(Color::Srgba(Srgba::RED)),
        TextLayout::new_with_justify(Justify::Center),
        AnimationTarget {
            id: animation_target_id,
            player,
        },
        animation_target_name,
    )]);
}

// A type that represents the color of the first text section.
//
// We implement `AnimatableProperty` on this to define custom property accessor logic
#[derive(Clone)]
struct TextColorProperty;

impl AnimatableProperty for TextColorProperty {
    type Property = Srgba;

    fn evaluator_id(&self) -> EvaluatorId<'_> {
        EvaluatorId::Type(TypeId::of::<Self>())
    }

    fn get_mut<'a>(
        &self,
        entity: &'a mut AnimationEntityMut,
    ) -> Result<&'a mut Self::Property, AnimationEvaluationError> {
        let text_color = entity
            .get_mut::<TextColor>()
            .ok_or(AnimationEvaluationError::ComponentNotPresent(TypeId::of::<
                TextColor,
            >(
            )))?
            .into_inner();
        match text_color.0 {
            Color::Srgba(ref mut color) => Ok(color),
            _ => Err(AnimationEvaluationError::PropertyNotPresent(TypeId::of::<
                Srgba,
            >(
            ))),
        }
    }
}
