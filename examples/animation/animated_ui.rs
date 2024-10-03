//! Shows how to use animation clips to animate UI properties.

use bevy::{
    animation::{AnimationTarget, AnimationTargetId},
    prelude::*,
};

// A type that represents the font size of the first text section.
//
// We implement `AnimatableProperty` on this.
#[derive(Reflect)]
struct FontSizeProperty;

// A type that represents the color of the first text section.
//
// We implement `AnimatableProperty` on this.
#[derive(Reflect)]
struct TextColorProperty;

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

impl AnimatableProperty for FontSizeProperty {
    type Component = TextStyle;

    type Property = f32;

    fn get_mut(component: &mut Self::Component) -> Option<&mut Self::Property> {
        Some(&mut component.font_size)
    }
}

impl AnimatableProperty for TextColorProperty {
    type Component = TextStyle;

    type Property = Srgba;

    fn get_mut(component: &mut Self::Component) -> Option<&mut Self::Property> {
        match component.color {
            Color::Srgba(ref mut color) => Some(color),
            _ => None,
        }
    }
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
        //
        // The curve itself is a `Curve<f32>`, and `f32` is `FontSizeProperty::Property`,
        // which is required by `AnimatableCurve::from_curve`.
        animation_clip.add_curve_to_target(
            animation_target_id,
            AnimatableKeyframeCurve::new(
                [0.0, 0.5, 1.0, 1.5, 2.0, 2.5, 3.0]
                    .into_iter()
                    .zip([24.0, 80.0, 24.0, 80.0, 24.0, 80.0, 24.0]),
            )
            .map(AnimatableCurve::<FontSizeProperty, _>::from_curve)
            .expect("should be able to build translation curve because we pass in valid samples"),
        );

        // Create a curve that animates font color. Note that this should have
        // the same time duration as the previous curve.
        //
        // Similar to the above, the curve itself is a `Curve<Srgba>`, and `Srgba` is
        // `TextColorProperty::Property`, which is required by the `from_curve` method.
        animation_clip.add_curve_to_target(
            animation_target_id,
            AnimatableKeyframeCurve::new([0.0, 1.0, 2.0, 3.0].into_iter().zip([
                Srgba::RED,
                Srgba::GREEN,
                Srgba::BLUE,
                Srgba::RED,
            ]))
            .map(AnimatableCurve::<TextColorProperty, _>::from_curve)
            .expect("should be able to build translation curve because we pass in valid samples"),
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
    commands.spawn(Camera2dBundle::default());

    // Build the UI. We have a parent node that covers the whole screen and
    // contains the `AnimationPlayer`, as well as a child node that contains the
    // text to be animated.
    commands
        .spawn(NodeBundle {
            // Cover the whole screen, and center contents.
            style: Style {
                position_type: PositionType::Absolute,
                top: Val::Px(0.0),
                left: Val::Px(0.0),
                right: Val::Px(0.0),
                bottom: Val::Px(0.0),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
            },
            ..default()
        })
        .insert(animation_player)
        .insert(animation_graph)
        .with_children(|builder| {
            // Build the text node.
            let player = builder.parent_entity();
            builder
                .spawn((
                    TextNEW::new("Bevy"),
                    TextStyle {
                        font: asset_server.load("fonts/FiraSans-Bold.ttf"),
                        font_size: 24.0,
                        color: Color::Srgba(Srgba::RED),
                    },
                    TextBlock::new_with_justify(JustifyText::Center),
                ))
                // Mark as an animation target.
                .insert(AnimationTarget {
                    id: animation_target_id,
                    player,
                })
                .insert(animation_target_name);
        });
}
