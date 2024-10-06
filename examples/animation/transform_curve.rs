//! Create and play an animation defined by a curve valued in `Transform`.

use bevy::{
    animation::{AnimationTarget, AnimationTargetId},
    math::vec3,
    prelude::*,
};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .insert_resource(AmbientLight {
            color: Color::WHITE,
            brightness: 150.0,
        })
        .add_systems(Startup, setup)
        .run();
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
    node_indices: Vec<AnimationNodeIndex>,
}

impl AnimationInfo {
    // Programmatically creates the UI animation.
    fn create(
        animation_graphs: &mut Assets<AnimationGraph>,
        animation_clips: &mut Assets<AnimationClip>,
    ) -> AnimationInfo {
        // Create an ID that identifies the thing we're going to animate.
        let animation_target_name = Name::new("Ship");
        let animation_target_id = AnimationTargetId::from_name(&animation_target_name);

        // Allocate an animation clip.
        let mut main_clip = AnimationClip::default();

        let wobbly_circle_curve =
            function_curve(Interval::new(0.0, std::f32::consts::TAU).unwrap(), |t| {
                vec3(t.sin() * 5.0, t.sin() * 1.5, t.cos() * 5.0)
            });

        let transform_curve = wobbly_circle_curve.map(|position| {
            Transform::from_translation(position).aligned_by(
                Dir3::NEG_X,
                vec3(0.0, -2.0, 0.0) - position,
                Dir3::Y,
                Dir3::Y,
            )
        });

        main_clip.add_curve_to_target(animation_target_id, TransformCurve(transform_curve));

        // Set up an additional additive clip to blend with the first.
        let mut additive_clip = AnimationClip::default();

        let turbulence_curve =
            function_curve(Interval::new(0.0, std::f32::consts::TAU).unwrap(), |t| {
                vec3(f32::cos(20.0 * t), 0.0, f32::sin(20.0 * t))
            });

        additive_clip.add_curve_to_target(animation_target_id, TranslationCurve(turbulence_curve));

        // Save our animation clips as assets.
        let main_clip_handle = animation_clips.add(main_clip);
        let additive_clip_handle = animation_clips.add(additive_clip);

        let mut node_indices = vec![];

        // Build the animation graph:
        let mut animation_graph = AnimationGraph::new();
        let blend_node = animation_graph.add_additive_blend(1.0, animation_graph.root);
        node_indices.push(animation_graph.add_clip(main_clip_handle, 1.0, blend_node));
        node_indices.push(animation_graph.add_clip(additive_clip_handle, 0.01, blend_node));

        let animation_graph_handle = animation_graphs.add(animation_graph);

        AnimationInfo {
            target_name: animation_target_name,
            target_id: animation_target_id,
            graph: animation_graph_handle,
            node_indices,
        }
    }
}

fn setup(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut animations: ResMut<Assets<AnimationClip>>,
    mut graphs: ResMut<Assets<AnimationGraph>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    // Create the animation.
    let AnimationInfo {
        target_name: animation_target_name,
        target_id: animation_target_id,
        graph: animation_graph,
        node_indices: animation_node_indices,
    } = AnimationInfo::create(&mut graphs, &mut animations);

    // Camera
    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(-4.0, 5.0, 10.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));

    // A light source
    commands.spawn((
        PointLight {
            shadows_enabled: true,
            ..default()
        },
        Transform::from_xyz(4.0, 7.0, -4.0),
    ));

    // A plane that we can use to situate ourselves
    commands.spawn((
        Mesh3d(meshes.add(Circle::new(20.0))),
        MeshMaterial3d(materials.add(Color::srgb(0.3, 0.5, 0.3))),
        Transform::from_xyz(0., -2., 0.)
            .with_rotation(Quat::from_rotation_x(-std::f32::consts::FRAC_PI_2)),
    ));

    // Create the animation player, and set it to repeat.
    let mut player = AnimationPlayer::default();
    for index in animation_node_indices {
        player.play(index).repeat();
    }

    // Finally, our ship that is going to be animated.
    let ship_entity = commands
        .spawn((
            SceneRoot(
                asset_server
                    .load(GltfAssetLabel::Scene(0).from_asset("models/ship/craft_speederD.gltf")),
            ),
            animation_target_name,
            animation_graph,
            player,
        ))
        .id();

    commands.entity(ship_entity).insert(AnimationTarget {
        id: animation_target_id,
        player: ship_entity,
    });
}
