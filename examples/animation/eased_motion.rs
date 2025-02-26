//! Demonstrates the application of easing curves to animate a transition.

use std::f32::consts::FRAC_PI_2;

use bevy::{
    animation::{animated_field, AnimationTarget, AnimationTargetId},
    color::palettes::css::{ORANGE, SILVER},
    math::vec3,
    prelude::*,
};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_systems(Startup, setup)
        .run();
}

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut animation_graphs: ResMut<Assets<AnimationGraph>>,
    mut animation_clips: ResMut<Assets<AnimationClip>>,
) {
    // Create the animation:
    let AnimationInfo {
        target_name: animation_target_name,
        target_id: animation_target_id,
        graph: animation_graph,
        node_index: animation_node_index,
    } = AnimationInfo::create(&mut animation_graphs, &mut animation_clips);

    // Build an animation player that automatically plays the animation.
    let mut animation_player = AnimationPlayer::default();
    animation_player.play(animation_node_index).repeat();

    // A cube together with the components needed to animate it
    let cube_entity = commands
        .spawn((
            Mesh3d(meshes.add(Cuboid::from_length(2.0))),
            MeshMaterial3d(materials.add(Color::from(ORANGE))),
            Transform::from_translation(vec3(-6., 2., 0.)),
            animation_target_name,
            animation_player,
            AnimationGraphHandle(animation_graph),
        ))
        .id();

    commands.entity(cube_entity).insert(AnimationTarget {
        id: animation_target_id,
        player: cube_entity,
    });

    // Some light to see something
    commands.spawn((
        PointLight {
            shadows_enabled: true,
            intensity: 10_000_000.,
            range: 100.0,
            ..default()
        },
        Transform::from_xyz(8., 16., 8.),
    ));

    // Ground plane
    commands.spawn((
        Mesh3d(meshes.add(Plane3d::default().mesh().size(50., 50.))),
        MeshMaterial3d(materials.add(Color::from(SILVER))),
    ));

    // The camera
    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(0., 6., 12.).looking_at(Vec3::new(0., 1.5, 0.), Vec3::Y),
    ));
}

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

impl AnimationInfo {
    // Programmatically creates the UI animation.
    fn create(
        animation_graphs: &mut Assets<AnimationGraph>,
        animation_clips: &mut Assets<AnimationClip>,
    ) -> AnimationInfo {
        // Create an ID that identifies the text node we're going to animate.
        let animation_target_name = Name::new("Cube");
        let animation_target_id = AnimationTargetId::from_name(&animation_target_name);

        // Allocate an animation clip.
        let mut animation_clip = AnimationClip::default();

        // Each leg of the translation motion should take 3 seconds.
        let animation_domain = interval(0.0, 3.0).unwrap();

        // The easing curve is parametrized over [0, 1], so we reparametrize it and
        // then ping-pong, which makes it spend another 3 seconds on the return journey.
        let translation_curve = EasingCurve::new(
            vec3(-6., 2., 0.),
            vec3(6., 2., 0.),
            EaseFunction::CubicInOut,
        )
        .reparametrize_linear(animation_domain)
        .expect("this curve has bounded domain, so this should never fail")
        .ping_pong()
        .expect("this curve has bounded domain, so this should never fail");

        // Something similar for rotation. The repetition here is an illusion caused
        // by the symmetry of the cube; it rotates on the forward journey and never
        // rotates back.
        let rotation_curve = EasingCurve::new(
            Quat::IDENTITY,
            Quat::from_rotation_y(FRAC_PI_2),
            EaseFunction::ElasticInOut,
        )
        .reparametrize_linear(interval(0.0, 4.0).unwrap())
        .expect("this curve has bounded domain, so this should never fail");

        animation_clip.add_curve_to_target(
            animation_target_id,
            AnimatableCurve::new(animated_field!(Transform::translation), translation_curve),
        );
        animation_clip.add_curve_to_target(
            animation_target_id,
            AnimatableCurve::new(animated_field!(Transform::rotation), rotation_curve),
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
