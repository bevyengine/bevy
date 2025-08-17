//! This example demonstrates how each of Bevy's math primitives look like in 2D and 3D with meshes
//! and with gizmos

use bevy::{input::common_conditions::input_just_pressed, math::Isometry2d, prelude::*};

const LEFT_RIGHT_OFFSET_2D: f32 = 200.0;
const LEFT_RIGHT_OFFSET_3D: f32 = 2.0;

fn main() {
    let mut app = App::new();

    app.add_plugins(DefaultPlugins)
        .init_state::<PrimitiveSelected>()
        .init_state::<CameraActive>();

    // cameras
    app.add_systems(Startup, (setup_cameras, setup_lights, setup_ambient_light))
        .add_systems(
            Update,
            (
                update_active_cameras.run_if(state_changed::<CameraActive>),
                switch_cameras.run_if(input_just_pressed(KeyCode::KeyC)),
            ),
        );

    // text

    // PostStartup since we need the cameras to exist
    app.add_systems(PostStartup, setup_text);
    app.add_systems(
        Update,
        (update_text.run_if(state_changed::<PrimitiveSelected>),),
    );

    // primitives
    app.add_systems(Startup, (spawn_primitive_2d, spawn_primitive_3d))
        .add_systems(
            Update,
            (
                switch_to_next_primitive.run_if(input_just_pressed(KeyCode::ArrowUp)),
                switch_to_previous_primitive.run_if(input_just_pressed(KeyCode::ArrowDown)),
                draw_gizmos_2d.run_if(in_mode(CameraActive::Dim2)),
                draw_gizmos_3d.run_if(in_mode(CameraActive::Dim3)),
                update_primitive_meshes
                    .run_if(state_changed::<PrimitiveSelected>.or(state_changed::<CameraActive>)),
                rotate_primitive_2d_meshes,
                rotate_primitive_3d_meshes,
            ),
        );

    app.run();
}

/// State for tracking which of the two cameras (2D & 3D) is currently active
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, States, Default, Reflect)]
enum CameraActive {
    #[default]
    /// 2D Camera is active
    Dim2,
    /// 3D Camera is active
    Dim3,
}

/// State for tracking which primitives are currently displayed
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, States, Default, Reflect)]
enum PrimitiveSelected {
    #[default]
    RectangleAndCuboid,
    CircleAndSphere,
    Ellipse,
    Triangle,
    Plane,
    Line,
    Segment,
    Polyline,
    Polygon,
    RegularPolygon,
    Capsule,
    Cylinder,
    Cone,
    ConicalFrustum,
    Torus,
    Tetrahedron,
    Arc,
    CircularSector,
    CircularSegment,
}

impl std::fmt::Display for PrimitiveSelected {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let name = match self {
            PrimitiveSelected::RectangleAndCuboid => String::from("Rectangle/Cuboid"),
            PrimitiveSelected::CircleAndSphere => String::from("Circle/Sphere"),
            other => format!("{other:?}"),
        };
        write!(f, "{name}")
    }
}

impl PrimitiveSelected {
    const ALL: [Self; 19] = [
        Self::RectangleAndCuboid,
        Self::CircleAndSphere,
        Self::Ellipse,
        Self::Triangle,
        Self::Plane,
        Self::Line,
        Self::Segment,
        Self::Polyline,
        Self::Polygon,
        Self::RegularPolygon,
        Self::Capsule,
        Self::Cylinder,
        Self::Cone,
        Self::ConicalFrustum,
        Self::Torus,
        Self::Tetrahedron,
        Self::Arc,
        Self::CircularSector,
        Self::CircularSegment,
    ];

    fn next(self) -> Self {
        Self::ALL
            .into_iter()
            .cycle()
            .skip_while(|&x| x != self)
            .nth(1)
            .unwrap()
    }

    fn previous(self) -> Self {
        Self::ALL
            .into_iter()
            .rev()
            .cycle()
            .skip_while(|&x| x != self)
            .nth(1)
            .unwrap()
    }
}

const SMALL_2D: f32 = 50.0;
const BIG_2D: f32 = 100.0;

const SMALL_3D: f32 = 0.5;
const BIG_3D: f32 = 1.0;

// primitives
const RECTANGLE: Rectangle = Rectangle {
    half_size: Vec2::new(SMALL_2D, BIG_2D),
};
const CUBOID: Cuboid = Cuboid {
    half_size: Vec3::new(BIG_3D, SMALL_3D, BIG_3D),
};

const CIRCLE: Circle = Circle { radius: BIG_2D };
const SPHERE: Sphere = Sphere { radius: BIG_3D };

const ELLIPSE: Ellipse = Ellipse {
    half_size: Vec2::new(BIG_2D, SMALL_2D),
};

const TRIANGLE_2D: Triangle2d = Triangle2d {
    vertices: [
        Vec2::new(BIG_2D, 0.0),
        Vec2::new(0.0, BIG_2D),
        Vec2::new(-BIG_2D, 0.0),
    ],
};

const TRIANGLE_3D: Triangle3d = Triangle3d {
    vertices: [
        Vec3::new(BIG_3D, 0.0, 0.0),
        Vec3::new(0.0, BIG_3D, 0.0),
        Vec3::new(-BIG_3D, 0.0, 0.0),
    ],
};

const PLANE_2D: Plane2d = Plane2d { normal: Dir2::Y };
const PLANE_3D: Plane3d = Plane3d {
    normal: Dir3::Y,
    half_size: Vec2::new(BIG_3D, BIG_3D),
};

const LINE2D: Line2d = Line2d { direction: Dir2::X };
const LINE3D: Line3d = Line3d { direction: Dir3::X };

const SEGMENT_2D: Segment2d = Segment2d {
    vertices: [Vec2::new(-BIG_2D / 2., 0.), Vec2::new(BIG_2D / 2., 0.)],
};

const SEGMENT_3D: Segment3d = Segment3d {
    vertices: [
        Vec3::new(-BIG_3D / 2., 0., 0.),
        Vec3::new(BIG_3D / 2., 0., 0.),
    ],
};

const REGULAR_POLYGON: RegularPolygon = RegularPolygon {
    circumcircle: Circle { radius: BIG_2D },
    sides: 5,
};

const CAPSULE_2D: Capsule2d = Capsule2d {
    radius: SMALL_2D,
    half_length: SMALL_2D,
};
const CAPSULE_3D: Capsule3d = Capsule3d {
    radius: SMALL_3D,
    half_length: SMALL_3D,
};

const CYLINDER: Cylinder = Cylinder {
    radius: SMALL_3D,
    half_height: SMALL_3D,
};

const CONE: Cone = Cone {
    radius: BIG_3D,
    height: BIG_3D,
};

const CONICAL_FRUSTUM: ConicalFrustum = ConicalFrustum {
    radius_top: BIG_3D,
    radius_bottom: SMALL_3D,
    height: BIG_3D,
};

const ANNULUS: Annulus = Annulus {
    inner_circle: Circle { radius: SMALL_2D },
    outer_circle: Circle { radius: BIG_2D },
};

const TORUS: Torus = Torus {
    minor_radius: SMALL_3D / 2.0,
    major_radius: SMALL_3D * 1.5,
};

const TETRAHEDRON: Tetrahedron = Tetrahedron {
    vertices: [
        Vec3::new(-BIG_3D, 0.0, 0.0),
        Vec3::new(BIG_3D, 0.0, 0.0),
        Vec3::new(0.0, 0.0, -BIG_3D * 1.67),
        Vec3::new(0.0, BIG_3D * 1.67, -BIG_3D * 0.5),
    ],
};

const ARC: Arc2d = Arc2d {
    radius: BIG_2D,
    half_angle: std::f32::consts::FRAC_PI_4,
};

const CIRCULAR_SECTOR: CircularSector = CircularSector {
    arc: Arc2d {
        radius: BIG_2D,
        half_angle: std::f32::consts::FRAC_PI_4,
    },
};

const CIRCULAR_SEGMENT: CircularSegment = CircularSegment {
    arc: Arc2d {
        radius: BIG_2D,
        half_angle: std::f32::consts::FRAC_PI_4,
    },
};

fn setup_cameras(mut commands: Commands) {
    let start_in_2d = true;
    let make_camera = |is_active| Camera {
        is_active,
        ..Default::default()
    };

    commands.spawn((Camera2d, make_camera(start_in_2d)));

    commands.spawn((
        Camera3d::default(),
        make_camera(!start_in_2d),
        Transform::from_xyz(0.0, 10.0, 0.0).looking_at(Vec3::ZERO, Vec3::Z),
    ));
}

fn setup_ambient_light(mut ambient_light: ResMut<AmbientLight>) {
    ambient_light.brightness = 50.0;
}

fn setup_lights(mut commands: Commands) {
    commands.spawn((
        PointLight {
            intensity: 5000.0,
            ..default()
        },
        Transform::from_translation(Vec3::new(-LEFT_RIGHT_OFFSET_3D, 2.0, 0.0))
            .looking_at(Vec3::new(-LEFT_RIGHT_OFFSET_3D, 0.0, 0.0), Vec3::Y),
    ));
}

/// Marker component for header text
#[derive(Debug, Clone, Component, Default, Reflect)]
pub struct HeaderText;

/// Marker component for header node
#[derive(Debug, Clone, Component, Default, Reflect)]
pub struct HeaderNode;

fn update_active_cameras(
    state: Res<State<CameraActive>>,
    camera_2d: Single<(Entity, &mut Camera), With<Camera2d>>,
    camera_3d: Single<(Entity, &mut Camera), (With<Camera3d>, Without<Camera2d>)>,
    mut text: Query<&mut UiTargetCamera, With<HeaderNode>>,
) {
    let (entity_2d, mut cam_2d) = camera_2d.into_inner();
    let (entity_3d, mut cam_3d) = camera_3d.into_inner();
    let is_camera_2d_active = matches!(*state.get(), CameraActive::Dim2);

    cam_2d.is_active = is_camera_2d_active;
    cam_3d.is_active = !is_camera_2d_active;

    let active_camera = if is_camera_2d_active {
        entity_2d
    } else {
        entity_3d
    };

    text.iter_mut().for_each(|mut target_camera| {
        *target_camera = UiTargetCamera(active_camera);
    });
}

fn switch_cameras(current: Res<State<CameraActive>>, mut next: ResMut<NextState<CameraActive>>) {
    let next_state = match current.get() {
        CameraActive::Dim2 => CameraActive::Dim3,
        CameraActive::Dim3 => CameraActive::Dim2,
    };
    next.set(next_state);
}

fn setup_text(mut commands: Commands, cameras: Query<(Entity, &Camera)>) {
    let active_camera = cameras
        .iter()
        .find_map(|(entity, camera)| camera.is_active.then_some(entity))
        .expect("run condition ensures existence");
    commands.spawn((
        HeaderNode,
        Node {
            justify_self: JustifySelf::Center,
            top: Val::Px(5.0),
            ..Default::default()
        },
        UiTargetCamera(active_camera),
        children![(
            Text::default(),
            HeaderText,
            TextLayout::new_with_justify(Justify::Center),
            children![
                TextSpan::new("Primitive: "),
                TextSpan(format!("{text}", text = PrimitiveSelected::default())),
                TextSpan::new("\n\n"),
                TextSpan::new(
                    "Press 'C' to switch between 2D and 3D mode\n\
                    Press 'Up' or 'Down' to switch to the next/previous primitive",
                ),
                TextSpan::new("\n\n"),
                TextSpan::new("(If nothing is displayed, there's no rendering support yet)",),
            ]
        )],
    ));
}

fn update_text(
    primitive_state: Res<State<PrimitiveSelected>>,
    header: Query<Entity, With<HeaderText>>,
    mut writer: TextUiWriter,
) {
    let new_text = format!("{text}", text = primitive_state.get());
    header.iter().for_each(|header_text| {
        if let Some(mut text) = writer.get_text(header_text, 2) {
            (*text).clone_from(&new_text);
        };
    });
}

fn switch_to_next_primitive(
    current: Res<State<PrimitiveSelected>>,
    mut next: ResMut<NextState<PrimitiveSelected>>,
) {
    let next_state = current.get().next();
    next.set(next_state);
}

fn switch_to_previous_primitive(
    current: Res<State<PrimitiveSelected>>,
    mut next: ResMut<NextState<PrimitiveSelected>>,
) {
    let next_state = current.get().previous();
    next.set(next_state);
}

fn in_mode(active: CameraActive) -> impl Fn(Res<State<CameraActive>>) -> bool {
    move |state| *state.get() == active
}

fn draw_gizmos_2d(mut gizmos: Gizmos, state: Res<State<PrimitiveSelected>>, time: Res<Time>) {
    const POSITION: Vec2 = Vec2::new(-LEFT_RIGHT_OFFSET_2D, 0.0);
    let angle = time.elapsed_secs();
    let isometry = Isometry2d::new(POSITION, Rot2::radians(angle));
    let color = Color::WHITE;

    #[expect(
        clippy::match_same_arms,
        reason = "Certain primitives don't have any 2D rendering support yet."
    )]
    match state.get() {
        PrimitiveSelected::RectangleAndCuboid => {
            gizmos.primitive_2d(&RECTANGLE, isometry, color);
        }
        PrimitiveSelected::CircleAndSphere => {
            gizmos.primitive_2d(&CIRCLE, isometry, color);
        }
        PrimitiveSelected::Ellipse => drop(gizmos.primitive_2d(&ELLIPSE, isometry, color)),
        PrimitiveSelected::Triangle => gizmos.primitive_2d(&TRIANGLE_2D, isometry, color),
        PrimitiveSelected::Plane => gizmos.primitive_2d(&PLANE_2D, isometry, color),
        PrimitiveSelected::Line => drop(gizmos.primitive_2d(&LINE2D, isometry, color)),
        PrimitiveSelected::Segment => {
            drop(gizmos.primitive_2d(&SEGMENT_2D, isometry, color));
        }
        PrimitiveSelected::Polyline => gizmos.primitive_2d(
            &Polyline2d {
                vertices: vec![
                    Vec2::new(-BIG_2D, -SMALL_2D),
                    Vec2::new(-SMALL_2D, SMALL_2D),
                    Vec2::new(SMALL_2D, -SMALL_2D),
                    Vec2::new(BIG_2D, SMALL_2D),
                ],
            },
            isometry,
            color,
        ),
        PrimitiveSelected::Polygon => gizmos.primitive_2d(
            &Polygon {
                vertices: vec![
                    Vec2::new(-BIG_2D, -SMALL_2D),
                    Vec2::new(BIG_2D, -SMALL_2D),
                    Vec2::new(BIG_2D, SMALL_2D),
                    Vec2::new(0.0, 0.0),
                    Vec2::new(-BIG_2D, SMALL_2D),
                ],
            },
            isometry,
            color,
        ),
        PrimitiveSelected::RegularPolygon => {
            gizmos.primitive_2d(&REGULAR_POLYGON, isometry, color);
        }
        PrimitiveSelected::Capsule => gizmos.primitive_2d(&CAPSULE_2D, isometry, color),
        PrimitiveSelected::Cylinder => {}
        PrimitiveSelected::Cone => {}
        PrimitiveSelected::ConicalFrustum => {}
        PrimitiveSelected::Torus => drop(gizmos.primitive_2d(&ANNULUS, isometry, color)),
        PrimitiveSelected::Tetrahedron => {}
        PrimitiveSelected::Arc => gizmos.primitive_2d(&ARC, isometry, color),
        PrimitiveSelected::CircularSector => {
            gizmos.primitive_2d(&CIRCULAR_SECTOR, isometry, color);
        }
        PrimitiveSelected::CircularSegment => {
            gizmos.primitive_2d(&CIRCULAR_SEGMENT, isometry, color);
        }
    }
}

/// Marker for primitive meshes to record in which state they should be visible in
#[derive(Debug, Clone, Component, Default, Reflect)]
pub struct PrimitiveData {
    camera_mode: CameraActive,
    primitive_state: PrimitiveSelected,
}

/// Marker for meshes of 2D primitives
#[derive(Debug, Clone, Component, Default)]
pub struct MeshDim2;

/// Marker for meshes of 3D primitives
#[derive(Debug, Clone, Component, Default)]
pub struct MeshDim3;

fn spawn_primitive_2d(
    mut commands: Commands,
    mut materials: ResMut<Assets<ColorMaterial>>,
    mut meshes: ResMut<Assets<Mesh>>,
) {
    const POSITION: Vec3 = Vec3::new(LEFT_RIGHT_OFFSET_2D, 0.0, 0.0);
    let material: Handle<ColorMaterial> = materials.add(Color::WHITE);
    let camera_mode = CameraActive::Dim2;
    [
        Some(RECTANGLE.mesh().build()),
        Some(CIRCLE.mesh().build()),
        Some(ELLIPSE.mesh().build()),
        Some(TRIANGLE_2D.mesh().build()),
        None, // plane
        None, // line
        None, // segment
        None, // polyline
        None, // polygon
        Some(REGULAR_POLYGON.mesh().build()),
        Some(CAPSULE_2D.mesh().build()),
        None, // cylinder
        None, // cone
        None, // conical frustum
        Some(ANNULUS.mesh().build()),
        None, // tetrahedron
    ]
    .into_iter()
    .zip(PrimitiveSelected::ALL)
    .for_each(|(maybe_mesh, state)| {
        if let Some(mesh) = maybe_mesh {
            commands.spawn((
                MeshDim2,
                PrimitiveData {
                    camera_mode,
                    primitive_state: state,
                },
                Mesh2d(meshes.add(mesh)),
                MeshMaterial2d(material.clone()),
                Transform::from_translation(POSITION),
            ));
        }
    });
}

fn spawn_primitive_3d(
    mut commands: Commands,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut meshes: ResMut<Assets<Mesh>>,
) {
    const POSITION: Vec3 = Vec3::new(-LEFT_RIGHT_OFFSET_3D, 0.0, 0.0);
    let material: Handle<StandardMaterial> = materials.add(Color::WHITE);
    let camera_mode = CameraActive::Dim3;
    [
        Some(CUBOID.mesh().build()),
        Some(SPHERE.mesh().build()),
        None, // ellipse
        Some(TRIANGLE_3D.mesh().build()),
        Some(PLANE_3D.mesh().build()),
        None, // line
        None, // segment
        None, // polyline
        None, // polygon
        None, // regular polygon
        Some(CAPSULE_3D.mesh().build()),
        Some(CYLINDER.mesh().build()),
        None, // cone
        None, // conical frustum
        Some(TORUS.mesh().build()),
        Some(TETRAHEDRON.mesh().build()),
    ]
    .into_iter()
    .zip(PrimitiveSelected::ALL)
    .for_each(|(maybe_mesh, state)| {
        if let Some(mesh) = maybe_mesh {
            commands.spawn((
                MeshDim3,
                PrimitiveData {
                    camera_mode,
                    primitive_state: state,
                },
                Mesh3d(meshes.add(mesh)),
                MeshMaterial3d(material.clone()),
                Transform::from_translation(POSITION),
            ));
        }
    });
}

fn update_primitive_meshes(
    camera_state: Res<State<CameraActive>>,
    primitive_state: Res<State<PrimitiveSelected>>,
    mut primitives: Query<(&mut Visibility, &PrimitiveData)>,
) {
    primitives.iter_mut().for_each(|(mut vis, primitive)| {
        let visible = primitive.camera_mode == *camera_state.get()
            && primitive.primitive_state == *primitive_state.get();
        *vis = if visible {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
    });
}

fn rotate_primitive_2d_meshes(
    mut primitives_2d: Query<
        (&mut Transform, &ViewVisibility),
        (With<PrimitiveData>, With<MeshDim2>),
    >,
    time: Res<Time>,
) {
    let rotation_2d = Quat::from_mat3(&Mat3::from_angle(time.elapsed_secs()));
    primitives_2d
        .iter_mut()
        .filter(|(_, vis)| vis.get())
        .for_each(|(mut transform, _)| {
            transform.rotation = rotation_2d;
        });
}

fn rotate_primitive_3d_meshes(
    mut primitives_3d: Query<
        (&mut Transform, &ViewVisibility),
        (With<PrimitiveData>, With<MeshDim3>),
    >,
    time: Res<Time>,
) {
    let rotation_3d = Quat::from_rotation_arc(
        Vec3::Z,
        Vec3::new(
            ops::sin(time.elapsed_secs()),
            ops::cos(time.elapsed_secs()),
            ops::sin(time.elapsed_secs()) * 0.5,
        )
        .try_normalize()
        .unwrap_or(Vec3::Z),
    );
    primitives_3d
        .iter_mut()
        .filter(|(_, vis)| vis.get())
        .for_each(|(mut transform, _)| {
            transform.rotation = rotation_3d;
        });
}

fn draw_gizmos_3d(mut gizmos: Gizmos, state: Res<State<PrimitiveSelected>>, time: Res<Time>) {
    const POSITION: Vec3 = Vec3::new(LEFT_RIGHT_OFFSET_3D, 0.0, 0.0);
    let rotation = Quat::from_rotation_arc(
        Vec3::Z,
        Vec3::new(
            ops::sin(time.elapsed_secs()),
            ops::cos(time.elapsed_secs()),
            ops::sin(time.elapsed_secs()) * 0.5,
        )
        .try_normalize()
        .unwrap_or(Vec3::Z),
    );
    let isometry = Isometry3d::new(POSITION, rotation);
    let color = Color::WHITE;
    let resolution = 10;

    #[expect(
        clippy::match_same_arms,
        reason = "Certain primitives don't have any 3D rendering support yet."
    )]
    match state.get() {
        PrimitiveSelected::RectangleAndCuboid => {
            gizmos.primitive_3d(&CUBOID, isometry, color);
        }
        PrimitiveSelected::CircleAndSphere => drop(
            gizmos
                .primitive_3d(&SPHERE, isometry, color)
                .resolution(resolution),
        ),
        PrimitiveSelected::Ellipse => {}
        PrimitiveSelected::Triangle => gizmos.primitive_3d(&TRIANGLE_3D, isometry, color),
        PrimitiveSelected::Plane => drop(gizmos.primitive_3d(&PLANE_3D, isometry, color)),
        PrimitiveSelected::Line => gizmos.primitive_3d(&LINE3D, isometry, color),
        PrimitiveSelected::Segment => gizmos.primitive_3d(&SEGMENT_3D, isometry, color),
        PrimitiveSelected::Polyline => gizmos.primitive_3d(
            &Polyline3d {
                vertices: vec![
                    Vec3::new(-BIG_3D, -SMALL_3D, -SMALL_3D),
                    Vec3::new(SMALL_3D, SMALL_3D, 0.0),
                    Vec3::new(-SMALL_3D, -SMALL_3D, 0.0),
                    Vec3::new(BIG_3D, SMALL_3D, SMALL_3D),
                ],
            },
            isometry,
            color,
        ),
        PrimitiveSelected::Polygon => {}
        PrimitiveSelected::RegularPolygon => {}
        PrimitiveSelected::Capsule => drop(
            gizmos
                .primitive_3d(&CAPSULE_3D, isometry, color)
                .resolution(resolution),
        ),
        PrimitiveSelected::Cylinder => drop(
            gizmos
                .primitive_3d(&CYLINDER, isometry, color)
                .resolution(resolution),
        ),
        PrimitiveSelected::Cone => drop(
            gizmos
                .primitive_3d(&CONE, isometry, color)
                .resolution(resolution),
        ),
        PrimitiveSelected::ConicalFrustum => {
            gizmos.primitive_3d(&CONICAL_FRUSTUM, isometry, color);
        }

        PrimitiveSelected::Torus => drop(
            gizmos
                .primitive_3d(&TORUS, isometry, color)
                .minor_resolution(resolution)
                .major_resolution(resolution),
        ),
        PrimitiveSelected::Tetrahedron => {
            gizmos.primitive_3d(&TETRAHEDRON, isometry, color);
        }

        PrimitiveSelected::Arc => {}
        PrimitiveSelected::CircularSector => {}
        PrimitiveSelected::CircularSegment => {}
    }
}
