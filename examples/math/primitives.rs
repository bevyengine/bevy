//! This example demonstrates how each of Bevy's math primitives look like in 2D and 3D with meshes
//! and with gizmos
#![allow(clippy::match_same_arms)]

use bevy::{
    input::common_conditions::input_just_pressed,
    prelude::*,
    sprite::{Anchor, MaterialMesh2dBundle},
};

const LEFT_RIGHT_OFFSET_2D: f32 = 200.0;
const LEFT_RIGHT_OFFSET_3D: f32 = 2.0;

fn main() {
    let mut app = App::new();

    app.add_plugins(DefaultPlugins)
        .init_state::<PrimitiveSelected>()
        .init_state::<CameraActive>();

    // cameras
    app.add_systems(Startup, setup_cameras).add_systems(
        Update,
        (
            update_active_cameras.run_if(state_changed::<CameraActive>),
            switch_cameras.run_if(input_just_pressed(KeyCode::KeyC)),
        ),
    );

    // text
    app.add_systems(Startup, setup_text).add_systems(
        Update,
        (update_text.run_if(state_changed::<PrimitiveSelected>),),
    );

    // primitives
    app.add_systems(Startup, spawn_primitive_2d).add_systems(
        Update,
        (
            switch_to_next_primitive.run_if(input_just_pressed(KeyCode::ArrowUp)),
            switch_to_previous_primitive.run_if(input_just_pressed(KeyCode::ArrowDown)),
            draw_gizmos_2d.run_if(in_mode(CameraActive::Dim2)),
            draw_gizmos_3d.run_if(in_mode(CameraActive::Dim3)),
            update_primitive_meshes
                .run_if(state_changed::<PrimitiveSelected>.or_else(state_changed::<CameraActive>)),
            rotate_primitive_meshes,
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
    ConicalFrustrum,
    Torus,
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
    const ALL: [Self; 15] = [
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
        Self::ConicalFrustrum,
        Self::Torus,
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

const TRIANGLE: Triangle2d = Triangle2d {
    vertices: [
        Vec2::new(SMALL_2D, 0.0),
        Vec2::new(0.0, SMALL_2D),
        Vec2::new(-SMALL_2D, 0.0),
    ],
};

const PLANE_2D: Plane2d = Plane2d {
    normal: Direction2d::Y,
};
const PLANE_3D: Plane3d = Plane3d {
    normal: Direction3d::Y,
};

const LINE2D: Line2d = Line2d {
    direction: Direction2d::X,
};
const LINE3D: Line3d = Line3d {
    direction: Direction3d::X,
};

const SEGMENT_2D: Segment2d = Segment2d {
    direction: Direction2d::X,
    half_length: BIG_2D,
};
const SEGMENT_3D: Segment3d = Segment3d {
    direction: Direction3d::X,
    half_length: BIG_3D,
};

const POLYLINE_2D: Polyline2d<4> = Polyline2d {
    vertices: [
        Vec2::new(-BIG_2D, -SMALL_2D),
        Vec2::new(-SMALL_2D, SMALL_2D),
        Vec2::new(SMALL_2D, -SMALL_2D),
        Vec2::new(BIG_2D, SMALL_2D),
    ],
};
const POLYLINE_3D: Polyline3d<4> = Polyline3d {
    vertices: [
        Vec3::new(-BIG_3D, -SMALL_3D, -SMALL_3D),
        Vec3::new(SMALL_3D, SMALL_3D, 0.0),
        Vec3::new(-SMALL_3D, -SMALL_3D, 0.0),
        Vec3::new(BIG_3D, SMALL_3D, SMALL_3D),
    ],
};

const POLYGON_2D: Polygon<5> = Polygon {
    vertices: [
        Vec2::new(-BIG_2D, -SMALL_2D),
        Vec2::new(BIG_2D, -SMALL_2D),
        Vec2::new(BIG_2D, SMALL_2D),
        Vec2::new(0.0, 0.0),
        Vec2::new(-BIG_2D, SMALL_2D),
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

const CONICAL_FRUSTRUM: ConicalFrustum = ConicalFrustum {
    radius_top: BIG_3D,
    radius_bottom: SMALL_3D,
    height: BIG_3D,
};

const TORUS: Torus = Torus {
    minor_radius: SMALL_3D / 2.0,
    major_radius: SMALL_3D * 1.5,
};

fn setup_cameras(mut commands: Commands) {
    let start_in_2d = true;
    let make_camera = |is_active| Camera {
        is_active,
        ..Default::default()
    };

    commands.spawn(Camera2dBundle {
        camera: make_camera(start_in_2d),
        ..Default::default()
    });

    commands.spawn(Camera3dBundle {
        camera: make_camera(!start_in_2d),
        transform: Transform::from_xyz(0.0, 10.0, 0.0).looking_at(Vec3::ZERO, Vec3::Z),
        ..Default::default()
    });
}

fn update_active_cameras(
    state: Res<State<CameraActive>>,
    mut cameras: Query<(&mut Camera, Has<Camera2d>, Has<Camera3d>)>,
) {
    match state.get() {
        CameraActive::Dim2 => cameras.iter_mut().for_each(|(mut camera, has_2d, _)| {
            camera.is_active = has_2d;
        }),
        CameraActive::Dim3 => cameras.iter_mut().for_each(|(mut camera, _, has_3d)| {
            camera.is_active = has_3d;
        }),
    }
}

/// Marker component for header text
#[derive(Debug, Clone, Component, Default, Reflect)]
pub struct Header;

fn switch_cameras(current: Res<State<CameraActive>>, mut next: ResMut<NextState<CameraActive>>) {
    let next_state = match current.get() {
        CameraActive::Dim2 => CameraActive::Dim3,
        CameraActive::Dim3 => CameraActive::Dim2,
    };
    next.set(next_state);
}

fn setup_text(mut commands: Commands, asset_server: Res<AssetServer>, window: Query<&Window>) {
    let text = format!("{text}", text = PrimitiveSelected::default());
    let font_size = 24.0;
    let font: Handle<Font> = asset_server.load("fonts/FiraMono-Medium.ttf");
    let guessed_line_gap = 5.0;
    let style = TextStyle {
        font,
        font_size,
        color: Color::WHITE,
    };
    let instructions = "Press 'C' to switch between 2D and 3D mode\n\
        Press 'Up' or 'Down' to switch to the next/last primitive";
    let text = Text::from_sections([
        TextSection::new("Primitive: ", style.clone()),
        TextSection::new(text, style.clone()),
        TextSection::new("\n\n", style.clone()),
        TextSection::new(instructions, style.clone()),
        TextSection::new("\n\n", style.clone()),
        TextSection::new(
            "(If nothing is displayed, there's not rendering support yet)",
            style.clone(),
        ),
    ])
    .with_justify(JustifyText::Center);

    let window_height = window.get_single().map_or(0.0, |window| window.height());
    let text_offset = text.sections.len() as f32 * font_size;

    // 2d
    commands.spawn((
        Header,
        Text2dBundle {
            text: text.clone(),
            text_anchor: Anchor::Center,
            transform: Transform::from_translation(Vec3::new(0.0, window_height / 2.0, 0.0)),
            ..Default::default()
        },
    ));

    // 3d
    commands.spawn((
        Header,
        TextBundle {
            text,
            style: Style {
                top: Val::Px(text_offset / 2.0),
                align_self: AlignSelf::Start,
                justify_self: JustifySelf::Center,
                ..Default::default()
            },
            ..Default::default()
        },
    ));
}

fn update_text(
    primitive_state: Res<State<PrimitiveSelected>>,
    mut header: Query<&mut Text, With<Header>>,
) {
    let new_text = format!("{text}", text = primitive_state.get());
    header.iter_mut().for_each(|mut header_text| {
        if let Some(kind) = header_text.sections.get_mut(1) {
            kind.value = new_text.clone();
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
    let angle = time.elapsed_seconds();
    let color = Color::WHITE;

    match state.get() {
        PrimitiveSelected::RectangleAndCuboid => {
            gizmos.primitive_2d(RECTANGLE, POSITION, angle, color)
        }
        PrimitiveSelected::CircleAndSphere => gizmos.primitive_2d(CIRCLE, POSITION, angle, color),
        PrimitiveSelected::Ellipse => gizmos.primitive_2d(ELLIPSE, POSITION, angle, color),
        PrimitiveSelected::Triangle => gizmos.primitive_2d(TRIANGLE, POSITION, angle, color),
        PrimitiveSelected::Plane => gizmos.primitive_2d(PLANE_2D, POSITION, angle, color),
        PrimitiveSelected::Line => drop(gizmos.primitive_2d(LINE2D, POSITION, angle, color)),
        PrimitiveSelected::Segment => drop(gizmos.primitive_2d(SEGMENT_2D, POSITION, angle, color)),
        PrimitiveSelected::Polyline => gizmos.primitive_2d(POLYLINE_2D, POSITION, angle, color),
        PrimitiveSelected::Polygon => gizmos.primitive_2d(POLYGON_2D, POSITION, angle, color),
        PrimitiveSelected::RegularPolygon => {
            gizmos.primitive_2d(REGULAR_POLYGON, POSITION, angle, color);
        }
        PrimitiveSelected::Capsule => gizmos.primitive_2d(CAPSULE_2D, POSITION, angle, color),
        PrimitiveSelected::Cylinder => {}
        PrimitiveSelected::Cone => {}
        PrimitiveSelected::ConicalFrustrum => {}
        PrimitiveSelected::Torus => {}
    }
}

/// Marker for primitive meshes to record in which state they should be visible in
#[derive(Debug, Clone, Component, Default, Reflect)]
pub struct PrimitiveData {
    camera_mode: CameraActive,
    primitive_state: PrimitiveSelected,
}

fn spawn_primitive_2d(
    mut commands: Commands,
    mut materials: ResMut<Assets<ColorMaterial>>,
    mut meshes: ResMut<Assets<Mesh>>,
) {
    const POSITION: Vec3 = Vec3::new(LEFT_RIGHT_OFFSET_2D, 0.0, 0.0);
    let material: Handle<ColorMaterial> = materials.add(Color::WHITE);
    let camera_mode = CameraActive::Dim2;
    [
        Some(RECTANGLE.mesh()),
        Some(CIRCLE.mesh().build()),
        Some(ELLIPSE.mesh().build()),
        Some(TRIANGLE.mesh()),
        None, // plane
        None, // line
        None, // segment
        None, // polyline
        None, // polygon
        Some(REGULAR_POLYGON.mesh()),
        Some(CAPSULE_2D.mesh().build()),
        None, // cylinder
        None, // cone
        None, // conical frustrum
        None, // torus
    ]
    .into_iter()
    .zip(PrimitiveSelected::ALL)
    .for_each(|(maybe_mesh, state)| {
        if let Some(mesh) = maybe_mesh {
            commands.spawn((
                PrimitiveData {
                    camera_mode,
                    primitive_state: state,
                },
                MaterialMesh2dBundle {
                    mesh: meshes.add(mesh).into(),
                    material: material.clone(),
                    transform: Transform::from_translation(POSITION),
                    ..Default::default()
                },
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

fn rotate_primitive_meshes(
    mut primitives: Query<(&mut Transform, &ViewVisibility), With<PrimitiveData>>,
    time: Res<Time>,
) {
    let rotation = Quat::from_mat3(&Mat3::from_angle(time.elapsed_seconds()));
    primitives
        .iter_mut()
        .filter(|(_, vis)| vis.get())
        .for_each(|(mut transform, _)| {
            transform.rotation = rotation;
        });
}

fn draw_gizmos_3d(mut gizmos: Gizmos, state: Res<State<PrimitiveSelected>>, time: Res<Time>) {
    const POSITION: Vec3 = Vec3::new(LEFT_RIGHT_OFFSET_3D, 0.0, 0.0);
    let rotation = Quat::from_rotation_arc(
        Vec3::Z,
        Vec3::new(
            time.elapsed_seconds().sin(),
            time.elapsed_seconds().cos(),
            time.elapsed_seconds().sin() * 0.5,
        )
        .try_normalize()
        .unwrap_or(Vec3::Z),
    );
    let color = Color::WHITE;
    let segments = 10;

    match state.get() {
        PrimitiveSelected::RectangleAndCuboid => {
            gizmos.primitive_3d(CUBOID, POSITION, rotation, color)
        }
        PrimitiveSelected::CircleAndSphere => drop(
            gizmos
                .primitive_3d(SPHERE, POSITION, rotation, color)
                .segments(segments),
        ),
        PrimitiveSelected::Ellipse => {}
        PrimitiveSelected::Triangle => {}
        PrimitiveSelected::Plane => drop(gizmos.primitive_3d(PLANE_3D, POSITION, rotation, color)),
        PrimitiveSelected::Line => gizmos.primitive_3d(LINE3D, POSITION, rotation, color),
        PrimitiveSelected::Segment => gizmos.primitive_3d(SEGMENT_3D, POSITION, rotation, color),
        PrimitiveSelected::Polyline => gizmos.primitive_3d(POLYLINE_3D, POSITION, rotation, color),
        PrimitiveSelected::Polygon => {}
        PrimitiveSelected::RegularPolygon => {}
        PrimitiveSelected::Capsule => drop(
            gizmos
                .primitive_3d(CAPSULE_3D, POSITION, rotation, color)
                .segments(segments),
        ),
        PrimitiveSelected::Cylinder => drop(
            gizmos
                .primitive_3d(CYLINDER, POSITION, rotation, color)
                .segments(segments),
        ),
        PrimitiveSelected::Cone => drop(
            gizmos
                .primitive_3d(CONE, POSITION, rotation, color)
                .segments(segments),
        ),
        PrimitiveSelected::ConicalFrustrum => {
            gizmos.primitive_3d(CONICAL_FRUSTRUM, POSITION, rotation, color);
        }

        PrimitiveSelected::Torus => drop(
            gizmos
                .primitive_3d(TORUS, POSITION, rotation, color)
                .minor_segments(segments)
                .major_segments(segments),
        ),
    }
}
