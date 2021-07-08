use bevy::{
    math::{
        curves::{Curve, KeyframeIndex, CurveVariable, TangentControl},
        interpolation::Interpolation,
    },
    prelude::*,
    render::pipeline::PrimitiveTopology,
};

struct CurveCursorTag;

struct CurveTargetTag;

#[derive(Default)]
struct CurveMesh {
    timer: Timer,
    curve: CurveVariable<f32>,
}

fn main() {
    App::build()
        .insert_resource(CurveMesh::default())
        .add_plugins(DefaultPlugins)
        .add_startup_system(setup.system())
        .add_system(animate.system())
        .run();
}

fn values<T: Copy>(length: usize, default: T) -> Vec<T> {
    let mut v = vec![];
    v.resize(length, default);
    v
}

fn line(a: [f32; 3], b: [f32; 3]) -> Mesh {
    let mut mesh = Mesh::new(PrimitiveTopology::LineStrip);
    mesh.set_attribute(Mesh::ATTRIBUTE_POSITION, vec![a, b]);
    mesh.set_attribute(Mesh::ATTRIBUTE_NORMAL, values(2, [0.0f32, 0.0, 1.0]));
    mesh.set_attribute(Mesh::ATTRIBUTE_TANGENT, values(2, [0.0f32, 1.0, 0.0]));
    mesh.set_attribute(Mesh::ATTRIBUTE_UV_0, values(2, [0.0f32; 2]));
    mesh
}

fn setup(
    mut commands: Commands,
    mut curve_mesh: ResMut<CurveMesh>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    // Create curve
    curve_mesh.curve = CurveVariable::with_tangents_and_mode(
        vec![0.0, 1.0, 1.3, 1.6, 1.7, 1.8, 1.9, 2.0],
        vec![3.0, 0.0, 1.0, 0.0, 0.5, 0.0, 0.25, 0.0],
        TangentControl::Auto,
        Interpolation::Hermite,
    )
    .unwrap();
    // Create timer
    curve_mesh.timer = Timer::from_seconds(2.5, true);

    // Create curve mesh
    const DIVS: usize = 1024;
    let mut mesh = Mesh::new(PrimitiveTopology::LineStrip);
    mesh.set_attribute(
        Mesh::ATTRIBUTE_POSITION,
        (0..DIVS)
            .into_iter()
            .map(|i| {
                let time = (i as f32 / (DIVS - 1) as f32) * curve_mesh.curve.duration();
                [time, curve_mesh.curve.sample(time), 0.0]
            })
            .collect::<Vec<_>>(),
    );
    mesh.set_attribute(Mesh::ATTRIBUTE_NORMAL, values(DIVS, [0.0f32, 0.0, 1.0]));
    mesh.set_attribute(Mesh::ATTRIBUTE_TANGENT, values(DIVS, [0.0f32, 1.0, 0.0]));
    mesh.set_attribute(Mesh::ATTRIBUTE_UV_0, values(DIVS, [0.0f32; 2]));
    let mesh = meshes.add(mesh);

    let material = materials.add(Color::RED.into());
    let keyframe_mesh = meshes.add(Mesh::from(shape::Cube { size: 0.05 }));

    // Animated sphere
    commands
        .spawn_bundle(PbrBundle {
            mesh,
            transform: Transform::from_translation(Vec3::new(-3.5, 0.0, 0.0)),
            material: material.clone(),
            ..Default::default()
        })
        .with_children(|parent| {
            // Create keyframes
            let tangent_material = materials.add(Color::BLACK.into());
            for (index, (t, k)) in curve_mesh.curve.iter().enumerate() {
                parent
                    .spawn_bundle(PbrBundle {
                        mesh: keyframe_mesh.clone(),
                        transform: Transform::from_translation(Vec3::new(t, *k, 0.0)),
                        material: material.clone(),
                        ..Default::default()
                    })
                    .with_children(|parent| {
                        // tangents
                        let (a, b) = curve_mesh.curve.get_in_out_tangent(index as KeyframeIndex);
                        let (ay, ax) = a.atan().sin_cos();
                        let (by, bx) = b.atan().sin_cos();
                        parent.spawn_bundle(PbrBundle {
                            mesh: meshes.add(line([0.0, 0.0, 0.0], [ax * -0.2, ay * -0.2, 0.0])),
                            material: tangent_material.clone(),
                            ..Default::default()
                        });
                        parent.spawn_bundle(PbrBundle {
                            mesh: meshes.add(line([0.0, 0.0, 0.0], [bx * 0.2, by * 0.2, 0.0])),
                            material: tangent_material.clone(),
                            ..Default::default()
                        });
                    });
            }

            // Create time cursor
            parent
                .spawn_bundle(PbrBundle {
                    mesh: meshes.add(line([0.0, 4.0, 0.0], [0.0, -2.0, 0.0])),
                    material: materials.add(Color::BLUE.into()),
                    ..Default::default()
                })
                .insert(CurveCursorTag);
        });

    commands
        .spawn_bundle(PbrBundle {
            mesh: meshes.add(Mesh::from(shape::Icosphere {
                radius: 0.5,
                subdivisions: 3,
            })),
            transform: Transform::from_translation(Vec3::new(2.0, 0.0, 0.0)),
            material: materials.add(Color::BEIGE.into()),
            ..Default::default()
        })
        .insert(CurveTargetTag);

    // Camera and Light
    commands.spawn_bundle(PointLightBundle {
        transform: Transform::from_translation(Vec3::new(4.0, 8.0, 4.0)),
        ..Default::default()
    });

    commands.spawn_bundle(PerspectiveCameraBundle {
        transform: Transform::from_matrix(Mat4::face_toward(
            Vec3::new(-3.0, 5.0, 8.0),
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(0.0, 1.0, 0.0),
        )),
        ..Default::default()
    });
}

fn animate(
    mut curve_mesh: ResMut<CurveMesh>,
    time: Res<Time>,
    mut query_set: QuerySet<(
        Query<(&mut Transform, &CurveTargetTag)>,
        Query<(&mut Transform, &CurveCursorTag)>,
    )>,
) {
    let t = curve_mesh.timer.elapsed_secs();

    for (mut transform, _) in query_set.q0_mut().iter_mut() {
        let y = curve_mesh.curve.sample(t);
        transform.translation.y = y;
    }

    for (mut transform, _) in query_set.q1_mut().iter_mut() {
        transform.translation.x = t;
    }

    curve_mesh.timer.tick(time.delta());
}
