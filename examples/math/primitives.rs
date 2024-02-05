//! This example demonstrates how each of Bevy's math primitives look like in 2D and 3D with meshes
//! and with gizmos
#![allow(clippy::match_same_arms)]

use bevy::{
    input::common_conditions::input_just_pressed, prelude::*, sprite::MaterialMesh2dBundle,
};

const LEFT_RIGHT_OFFSET_2D: f32 = 200.0;
const LEFT_RIGHT_OFFSET_3D: f32 = 2.0;

fn main() {
    let mut app = App::new();

    app.add_plugins(DefaultPlugins)
        .init_state::<PrimitiveState>()
        .init_state::<CameraState>();

    // cameras
    app.add_systems(Startup, setup_cameras).add_systems(
        Update,
        (
            update_active_cameras.run_if(state_changed::<CameraState>),
            switch_cameras.run_if(input_just_pressed(KeyCode::KeyC)),
        ),
    );

    // text
    app.add_systems(Startup, setup_text).add_systems(
        Update,
        (update_text.run_if(state_changed::<PrimitiveState>),),
    );

    // primitives
    app.add_systems(Startup, spawn_primitive_2d).add_systems(
        Update,
        (
            switch_to_next_primitive.run_if(input_just_pressed(KeyCode::ArrowUp)),
            switch_to_last_primitive.run_if(input_just_pressed(KeyCode::ArrowDown)),
            draw_gizmos_2d.run_if(in_mode(CameraState::D2)),
            draw_gizmos_3d.run_if(in_mode(CameraState::D3)),
            update_primtive_meshes
                .run_if(state_changed::<PrimitiveState>.or_else(state_changed::<CameraState>)),
            rotate_primtive_meshes,
        ),
    );

    app.run();
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, States, Default, Reflect)]
enum CameraState {
    #[default]
    D2,
    D3,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, States, Default, Reflect)]
enum PrimitiveState {
    #[default]
    Rectangle,
    Circle,
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

impl PrimitiveState {
    const ALL: [Self; 15] = [
        Self::Rectangle,
        Self::Circle,
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

// primitives
const RECTANGLE: Rectangle = Rectangle {
    half_size: Vec2::new(50.0, 100.0),
};
const CUBOID: Cuboid = Cuboid {
    half_size: Vec3::new(1.0, 2.0, 1.0),
};

const CIRCLE: Circle = Circle { radius: 50.0 };
const SPHERE: Sphere = Sphere { radius: 1.0 };

const ELLIPSE: Ellipse = Ellipse {
    half_size: Vec2::new(50.0, 100.0),
};

const TRIANGLE: Triangle2d = Triangle2d {
    vertices: [
        Vec2::new(25.0, 0.0),
        Vec2::new(0.0, 25.0),
        Vec2::new(-25.0, 0.0),
    ],
};

const PLANE2D: Plane2d = Plane2d {
    normal: Direction2d::Y,
};
const PLANE3D: Plane3d = Plane3d {
    normal: Direction3d::Y,
};

const LINE2D: Line2d = Line2d {
    direction: Direction2d::X,
};
const LINE3D: Line3d = Line3d {
    direction: Direction3d::X,
};

const SEGMENT2D: Segment2d = Segment2d {
    direction: Direction2d::X,
    half_length: 50.0,
};
const SEGMENT3D: Segment3d = Segment3d {
    direction: Direction3d::X,
    half_length: 1.0,
};

const POLYLINE2D: Polyline2d<4> = Polyline2d {
    vertices: [
        Vec2::new(-50.0, -25.0),
        Vec2::new(-25.0, 25.0),
        Vec2::new(25.0, -25.0),
        Vec2::new(50.0, 25.0),
    ],
};
const POLYLINE3D: Polyline3d<4> = Polyline3d {
    vertices: [
        Vec3::new(-1.0, -0.5, -0.5),
        Vec3::new(0.5, 0.5, 0.0),
        Vec3::new(-0.5, -0.5, 0.0),
        Vec3::new(1.0, 0.5, 0.5),
    ],
};

const POLYGON2D: Polygon<5> = Polygon {
    vertices: [
        Vec2::new(-50.0, -25.0),
        Vec2::new(50.0, -25.0),
        Vec2::new(50.0, 25.0),
        Vec2::new(0.0, 0.0),
        Vec2::new(-50.0, 25.0),
    ],
};

const REGULAR_POLYGON: RegularPolygon = RegularPolygon {
    circumcircle: Circle { radius: 50.0 },
    sides: 5,
};

const CAPSULE2D: Capsule2d = Capsule2d {
    radius: 50.0,
    half_length: 50.0,
};
const CAPSULE3D: Capsule3d = Capsule3d {
    radius: 1.0,
    half_length: 1.0,
};

const CYLINDER: Cylinder = Cylinder {
    radius: 1.0,
    half_height: 1.0,
};

const CONE: Cone = Cone {
    radius: 1.0,
    height: 1.0,
};

const CONICAL_FRUSTRUM: ConicalFrustum = ConicalFrustum {
    radius_top: 1.0,
    radius_bottom: 0.5,
    height: 1.0,
};

const TORUS: Torus = Torus {
    minor_radius: 0.5,
    major_radius: 1.0,
};

fn setup_cameras(mut commands: Commands) {
    let start_in_2d = true;
    let mk_camera = |is_active| Camera {
        is_active,
        ..Default::default()
    };

    commands.spawn(Camera2dBundle {
        camera: mk_camera(start_in_2d),
        ..Default::default()
    });

    commands.spawn(Camera3dBundle {
        camera: mk_camera(!start_in_2d),
        transform: Transform::from_xyz(0.0, 10.0, 0.0).looking_at(Vec3::ZERO, Vec3::Z),
        ..Default::default()
    });
}

fn update_active_cameras(
    state: Res<State<CameraState>>,
    mut cameras: Query<(&mut Camera, Has<Camera2d>, Has<Camera3d>)>,
) {
    match state.get() {
        CameraState::D2 => cameras.iter_mut().for_each(|(mut camera, has_2d, _)| {
            camera.is_active = has_2d;
        }),
        CameraState::D3 => cameras.iter_mut().for_each(|(mut camera, _, has_3d)| {
            camera.is_active = has_3d;
        }),
    }
}

/// Marker component for header text
#[derive(Debug, Clone, Component, Default, Reflect)]
pub struct Header;

fn switch_cameras(current: Res<State<CameraState>>, mut next: ResMut<NextState<CameraState>>) {
    let next_state = match current.get() {
        CameraState::D2 => CameraState::D3,
        CameraState::D3 => CameraState::D2,
    };
    next.set(next_state);
}

fn setup_text(mut commands: Commands, asset_server: Res<AssetServer>, window: Query<&Window>) {
    let text = format!("{text:?}", text = PrimitiveState::default());
    let style = TextStyle {
        font: asset_server.load("fonts/FiraMono-Medium.ttf"),
        font_size: 24.,
        color: Color::WHITE,
    };
    let instructions = "Press 'C' to switch between 2D and 3D mode\n\
        Press 'Up' or 'Down' to switch to the next/last primitive";
    let text = Text::from_sections([
        TextSection::new("Primitive: ", style.clone()),
        TextSection::new(text, style.clone()),
        TextSection::new("\n\n", style.clone()),
        TextSection::new(instructions, style),
    ]);

    let window_height = window.get_single().map_or(0.0, |window| window.height());

    // yikes, getting the same position is a bit hard 🙈

    // 2d
    commands.spawn((
        Header,
        Text2dBundle {
            text: text.clone(),
            transform: Transform::from_xyz(0.0, window_height / 2.0, 0.0),
            ..Default::default()
        },
    ));

    // 3d
    commands.spawn((
        Header,
        TextBundle {
            text,
            style: Style {
                top: Val::Px(120.0),
                align_self: AlignSelf::Start,
                justify_self: JustifySelf::Center,
                ..Default::default()
            },
            ..Default::default()
        },
    ));
}

fn update_text(
    primitive_state: Res<State<PrimitiveState>>,
    mut header: Query<&mut Text, With<Header>>,
) {
    let new_text = format!("{text:?}", text = primitive_state.get());
    header.iter_mut().for_each(|mut header_text| {
        if let Some(kind) = header_text.sections.get_mut(1) {
            kind.value = new_text.clone();
        };
    });
}

fn switch_to_next_primitive(
    current: Res<State<PrimitiveState>>,
    mut next: ResMut<NextState<PrimitiveState>>,
) {
    let next_state = current.get().next();
    next.set(next_state);
}

fn switch_to_last_primitive(
    current: Res<State<PrimitiveState>>,
    mut next: ResMut<NextState<PrimitiveState>>,
) {
    let next_state = current.get().previous();
    next.set(next_state);
}

fn in_mode(active: CameraState) -> impl Fn(Res<State<CameraState>>) -> bool {
    move |state| *state.get() == active
}

fn draw_gizmos_2d(mut gizmos: Gizmos, state: Res<State<PrimitiveState>>, time: Res<Time>) {
    const POSITION: Vec2 = Vec2::new(-LEFT_RIGHT_OFFSET_2D, 0.0);
    let angle = time.elapsed_seconds();
    let color = Color::WHITE;

    match state.get() {
        PrimitiveState::Rectangle => gizmos.primitive_2d(RECTANGLE, POSITION, angle, color),
        PrimitiveState::Circle => gizmos.primitive_2d(CIRCLE, POSITION, angle, color),
        PrimitiveState::Ellipse => gizmos.primitive_2d(ELLIPSE, POSITION, angle, color),
        PrimitiveState::Triangle => gizmos.primitive_2d(TRIANGLE, POSITION, angle, color),
        PrimitiveState::Plane => gizmos.primitive_2d(PLANE2D, POSITION, angle, color),
        PrimitiveState::Line => drop(gizmos.primitive_2d(LINE2D, POSITION, angle, color)),
        PrimitiveState::Segment => drop(gizmos.primitive_2d(SEGMENT2D, POSITION, angle, color)),
        PrimitiveState::Polyline => gizmos.primitive_2d(POLYLINE2D, POSITION, angle, color),
        PrimitiveState::Polygon => gizmos.primitive_2d(POLYGON2D, POSITION, angle, color),
        PrimitiveState::RegularPolygon => {
            gizmos.primitive_2d(REGULAR_POLYGON, POSITION, angle, color);
        }
        PrimitiveState::Capsule => gizmos.primitive_2d(CAPSULE2D, POSITION, angle, color),
        PrimitiveState::Cylinder => {}
        PrimitiveState::Cone => {}
        PrimitiveState::ConicalFrustrum => {}
        PrimitiveState::Torus => {}
    }
}

/// marker for primitive meshes to record in which state they should be visible in
#[derive(Debug, Clone, Component, Default, Reflect)]
pub struct PrimitiveData {
    camera_mode: CameraState,
    primtive_state: PrimitiveState,
}

fn spawn_primitive_2d(
    mut commands: Commands,
    mut materials: ResMut<Assets<ColorMaterial>>,
    mut meshes: ResMut<Assets<Mesh>>,
) {
    const POSITION: Vec3 = Vec3::new(LEFT_RIGHT_OFFSET_2D, 0.0, 0.0);
    let material: Handle<ColorMaterial> = materials.add(Color::WHITE);
    let camera_mode = CameraState::D2;
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
        Some(CAPSULE2D.mesh().build()),
        None, // cylinder
        None, // cone
        None, // conical frustrum
        None, // torus
    ]
    .into_iter()
    .zip(PrimitiveState::ALL)
    .for_each(|(maybe_mesh, state)| {
        if let Some(mesh) = maybe_mesh {
            commands.spawn((
                PrimitiveData {
                    camera_mode,
                    primtive_state: state,
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

fn update_primtive_meshes(
    camera_state: Res<State<CameraState>>,
    primitive_state: Res<State<PrimitiveState>>,
    mut primitives: Query<(&mut Visibility, &PrimitiveData)>,
) {
    primitives.iter_mut().for_each(|(mut vis, primitive)| {
        let visible = primitive.camera_mode == *camera_state.get()
            && primitive.primtive_state == *primitive_state.get();
        *vis = if visible {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
    });
}

fn rotate_primtive_meshes(
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

fn draw_gizmos_3d(mut gizmos: Gizmos, state: Res<State<PrimitiveState>>, time: Res<Time>) {
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
        PrimitiveState::Rectangle => gizmos.primitive_3d(CUBOID, POSITION, rotation, color),
        PrimitiveState::Circle => drop(
            gizmos
                .primitive_3d(SPHERE, POSITION, rotation, color)
                .segments(segments),
        ),
        PrimitiveState::Ellipse => {}
        PrimitiveState::Triangle => {}
        PrimitiveState::Plane => drop(gizmos.primitive_3d(PLANE3D, POSITION, rotation, color)),
        PrimitiveState::Line => gizmos.primitive_3d(LINE3D, POSITION, rotation, color),
        PrimitiveState::Segment => gizmos.primitive_3d(SEGMENT3D, POSITION, rotation, color),
        PrimitiveState::Polyline => gizmos.primitive_3d(POLYLINE3D, POSITION, rotation, color),
        PrimitiveState::Polygon => {}
        PrimitiveState::RegularPolygon => {}
        PrimitiveState::Capsule => drop(
            gizmos
                .primitive_3d(CAPSULE3D, POSITION, rotation, color)
                .segments(segments),
        ),
        PrimitiveState::Cylinder => drop(
            gizmos
                .primitive_3d(CYLINDER, POSITION, rotation, color)
                .segments(segments),
        ),
        PrimitiveState::Cone => drop(
            gizmos
                .primitive_3d(CONE, POSITION, rotation, color)
                .segments(segments),
        ),
        PrimitiveState::ConicalFrustrum => {
            gizmos.primitive_3d(CONICAL_FRUSTRUM, POSITION, rotation, color);
        }

        PrimitiveState::Torus => drop(
            gizmos
                .primitive_3d(TORUS, POSITION, rotation, color)
                .minor_segments(segments)
                .major_segments(segments),
        ),
    }
}
