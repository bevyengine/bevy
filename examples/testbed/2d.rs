//! 2d testbed
//!
//! You can switch scene by pressing the spacebar

mod helpers;

use bevy::prelude::*;
use helpers::Next;

fn main() {
    let mut app = App::new();
    app.add_plugins((DefaultPlugins,))
        .init_state::<Scene>()
        .add_systems(OnEnter(Scene::Shapes), shapes::setup)
        .add_systems(OnEnter(Scene::Bloom), bloom::setup)
        .add_systems(OnEnter(Scene::Text), text::setup)
        .add_systems(OnEnter(Scene::Sprite), sprite::setup)
        .add_systems(OnEnter(Scene::Gizmos), gizmos::setup)
        .add_systems(Update, switch_scene)
        .add_systems(Update, gizmos::draw_gizmos.run_if(in_state(Scene::Gizmos)));

    #[cfg(feature = "bevy_ci_testing")]
    app.add_systems(Update, helpers::switch_scene_in_ci::<Scene>);

    app.run();
}

#[derive(Debug, Clone, Eq, PartialEq, Hash, States, Default)]
enum Scene {
    #[default]
    Shapes,
    Bloom,
    Text,
    Sprite,
    Gizmos,
}

impl Next for Scene {
    fn next(&self) -> Self {
        match self {
            Scene::Shapes => Scene::Bloom,
            Scene::Bloom => Scene::Text,
            Scene::Text => Scene::Sprite,
            Scene::Sprite => Scene::Gizmos,
            Scene::Gizmos => Scene::Shapes,
        }
    }
}

fn switch_scene(
    keyboard: Res<ButtonInput<KeyCode>>,
    scene: Res<State<Scene>>,
    mut next_scene: ResMut<NextState<Scene>>,
) {
    if keyboard.just_pressed(KeyCode::Space) {
        info!("Switching scene");
        next_scene.set(scene.get().next());
    }
}

mod shapes {
    use bevy::prelude::*;

    const X_EXTENT: f32 = 900.;

    pub fn setup(
        mut commands: Commands,
        mut meshes: ResMut<Assets<Mesh>>,
        mut materials: ResMut<Assets<ColorMaterial>>,
    ) {
        commands.spawn((Camera2d, DespawnOnExit(super::Scene::Shapes)));

        let shapes = [
            meshes.add(Circle::new(50.0)),
            meshes.add(CircularSector::new(50.0, 1.0)),
            meshes.add(CircularSegment::new(50.0, 1.25)),
            meshes.add(Ellipse::new(25.0, 50.0)),
            meshes.add(Annulus::new(25.0, 50.0)),
            meshes.add(Capsule2d::new(25.0, 50.0)),
            meshes.add(Rhombus::new(75.0, 100.0)),
            meshes.add(Rectangle::new(50.0, 100.0)),
            meshes.add(RegularPolygon::new(50.0, 6)),
            meshes.add(Triangle2d::new(
                Vec2::Y * 50.0,
                Vec2::new(-50.0, -50.0),
                Vec2::new(50.0, -50.0),
            )),
        ];
        let num_shapes = shapes.len();

        for (i, shape) in shapes.into_iter().enumerate() {
            // Distribute colors evenly across the rainbow.
            let color = Color::hsl(360. * i as f32 / num_shapes as f32, 0.95, 0.7);

            commands.spawn((
                Mesh2d(shape),
                MeshMaterial2d(materials.add(color)),
                Transform::from_xyz(
                    // Distribute shapes from -X_EXTENT/2 to +X_EXTENT/2.
                    -X_EXTENT / 2. + i as f32 / (num_shapes - 1) as f32 * X_EXTENT,
                    0.0,
                    0.0,
                ),
                DespawnOnExit(super::Scene::Shapes),
            ));
        }
    }
}

mod bloom {
    use bevy::{
        core_pipeline::{bloom::Bloom, tonemapping::Tonemapping},
        prelude::*,
    };

    pub fn setup(
        mut commands: Commands,
        mut meshes: ResMut<Assets<Mesh>>,
        mut materials: ResMut<Assets<ColorMaterial>>,
    ) {
        commands.spawn((
            Camera2d,
            Tonemapping::TonyMcMapface,
            Bloom::default(),
            DespawnOnExit(super::Scene::Bloom),
        ));

        commands.spawn((
            Mesh2d(meshes.add(Circle::new(100.))),
            MeshMaterial2d(materials.add(Color::srgb(7.5, 0.0, 7.5))),
            Transform::from_translation(Vec3::new(-200., 0., 0.)),
            DespawnOnExit(super::Scene::Bloom),
        ));

        commands.spawn((
            Mesh2d(meshes.add(RegularPolygon::new(100., 6))),
            MeshMaterial2d(materials.add(Color::srgb(6.25, 9.4, 9.1))),
            Transform::from_translation(Vec3::new(200., 0., 0.)),
            DespawnOnExit(super::Scene::Bloom),
        ));
    }
}

mod text {
    use bevy::color::palettes;
    use bevy::prelude::*;
    use bevy::sprite::Anchor;
    use bevy::text::TextBounds;

    pub fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
        commands.spawn((Camera2d, DespawnOnExit(super::Scene::Text)));

        for (i, justify) in [
            Justify::Left,
            Justify::Right,
            Justify::Center,
            Justify::Justified,
        ]
        .into_iter()
        .enumerate()
        {
            let y = 230. - 150. * i as f32;
            spawn_anchored_text(&mut commands, -300. * Vec3::X + y * Vec3::Y, justify, None);
            spawn_anchored_text(
                &mut commands,
                300. * Vec3::X + y * Vec3::Y,
                justify,
                Some(TextBounds::new(150., 60.)),
            );
        }

        let sans_serif = TextFont::from(asset_server.load("fonts/FiraSans-Bold.ttf"));

        const NUM_ITERATIONS: usize = 10;
        for i in 0..NUM_ITERATIONS {
            let fraction = i as f32 / (NUM_ITERATIONS - 1) as f32;

            commands.spawn((
                Text2d::new("Bevy"),
                sans_serif.clone(),
                Transform::from_xyz(0.0, fraction * 200.0, i as f32)
                    .with_scale(1.0 + Vec2::splat(fraction).extend(1.))
                    .with_rotation(Quat::from_rotation_z(fraction * core::f32::consts::PI)),
                TextColor(Color::hsla(fraction * 360.0, 0.8, 0.8, 0.8)),
                DespawnOnExit(super::Scene::Text),
            ));
        }

        commands.spawn((
            Text2d::new("This text is invisible."),
            Visibility::Hidden,
            DespawnOnExit(super::Scene::Text),
        ));
    }

    fn spawn_anchored_text(
        commands: &mut Commands,
        dest: Vec3,
        justify: Justify,
        bounds: Option<TextBounds>,
    ) {
        commands.spawn((
            Sprite {
                color: palettes::css::YELLOW.into(),
                custom_size: Some(5. * Vec2::ONE),
                ..Default::default()
            },
            Transform::from_translation(dest),
            DespawnOnExit(super::Scene::Text),
        ));

        for anchor in [
            Anchor::TOP_LEFT,
            Anchor::TOP_RIGHT,
            Anchor::BOTTOM_RIGHT,
            Anchor::BOTTOM_LEFT,
        ] {
            let mut text = commands.spawn((
                Text2d::new("L R\n"),
                TextLayout::new_with_justify(justify),
                Transform::from_translation(dest + Vec3::Z),
                anchor,
                DespawnOnExit(super::Scene::Text),
                ShowAabbGizmo {
                    color: Some(palettes::tailwind::AMBER_400.into()),
                },
                children![
                    (
                        TextSpan::new(format!("{}, {}\n", anchor.x, anchor.y)),
                        TextFont::from_font_size(14.0),
                        TextColor(palettes::tailwind::BLUE_400.into()),
                    ),
                    (
                        TextSpan::new(format!("{justify:?}")),
                        TextFont::from_font_size(14.0),
                        TextColor(palettes::tailwind::GREEN_400.into()),
                    ),
                ],
            ));
            if let Some(bounds) = bounds {
                text.insert(bounds);

                commands.spawn((
                    Sprite {
                        color: palettes::tailwind::GRAY_900.into(),
                        custom_size: Some(Vec2::new(bounds.width.unwrap(), bounds.height.unwrap())),
                        ..Default::default()
                    },
                    Transform::from_translation(dest - Vec3::Z),
                    anchor,
                    DespawnOnExit(super::Scene::Text),
                ));
            }
        }
    }
}

mod sprite {
    use bevy::color::palettes::css::{BLUE, LIME, RED};
    use bevy::prelude::*;
    use bevy::sprite::Anchor;

    pub fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
        commands.spawn((Camera2d, DespawnOnExit(super::Scene::Sprite)));
        for (anchor, flip_x, flip_y, color) in [
            (Anchor::BOTTOM_LEFT, false, false, Color::WHITE),
            (Anchor::BOTTOM_RIGHT, true, false, RED.into()),
            (Anchor::TOP_LEFT, false, true, LIME.into()),
            (Anchor::TOP_RIGHT, true, true, BLUE.into()),
        ] {
            commands.spawn((
                Sprite {
                    image: asset_server.load("branding/bevy_logo_dark.png"),
                    flip_x,
                    flip_y,
                    color,
                    ..default()
                },
                anchor,
                DespawnOnExit(super::Scene::Sprite),
            ));
        }
    }
}

mod gizmos {
    use bevy::{color::palettes::css::*, prelude::*};

    pub fn setup(mut commands: Commands) {
        commands.spawn((Camera2d, DespawnOnExit(super::Scene::Gizmos)));
    }

    pub fn draw_gizmos(mut gizmos: Gizmos) {
        gizmos.rect_2d(
            Isometry2d::from_translation(Vec2::new(-200.0, 0.0)),
            Vec2::new(200.0, 200.0),
            RED,
        );
        gizmos
            .circle_2d(
                Isometry2d::from_translation(Vec2::new(-200.0, 0.0)),
                200.0,
                GREEN,
            )
            .resolution(64);

        // 2d grids with all variations of outer edges on or off
        for i in 0..4 {
            let x = 200.0 * (1.0 + (i % 2) as f32);
            let y = 150.0 * (0.5 - (i / 2) as f32);
            let mut grid = gizmos.grid(
                Vec3::new(x, y, 0.0),
                UVec2::new(5, 4),
                Vec2::splat(30.),
                Color::WHITE,
            );
            if i & 1 > 0 {
                grid = grid.outer_edges_x();
            }
            if i & 2 > 0 {
                grid.outer_edges_y();
            }
        }
    }
}
