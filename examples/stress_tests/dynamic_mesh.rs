//! Sculpts a large terrain mesh every frame, which makes Bevy re-upload the whole mesh to the GPU
//! each frame, like a game with editable terrain.
//!
//! `cargo run --example dynamic_mesh --release`

use core::f32::consts::FRAC_PI_2;

use argh::FromArgs;
use bevy::{
    asset::RenderAssetUsages,
    color::{
        palettes::css::{LIME, RED},
        Mix,
    },
    diagnostic::{Diagnostic, DiagnosticsStore, FrameTimeDiagnosticsPlugin, LogDiagnosticsPlugin},
    math::ops,
    mesh::{Indices, PrimitiveTopology, VertexAttributeValues},
    prelude::*,
    window::PresentMode,
    winit::WinitSettings,
};

const TERRAIN_SIZE: f32 = 100.0;
const BRUSH_RADIUS: f32 = 8.0;
const BRUSH_SPEED: f32 = 0.6;
const MIN_HEIGHT: f32 = -6.0;
const MAX_HEIGHT: f32 = 18.0;
const WATER_HEIGHT: f32 = -1.5;
const SKY_COLOR: Color = Color::srgb(0.55, 0.7, 0.9);

#[derive(FromArgs, Resource)]
/// `dynamic_mesh` stress test
struct Args {
    /// the number of vertices along each side of the terrain
    #[argh(option, default = "1024")]
    grid_size: usize,
}

#[derive(Resource)]
struct Terrain {
    mesh: Handle<Mesh>,
    size: usize,
    heights: Vec<f32>,
}

impl Terrain {
    fn spacing(&self) -> f32 {
        TERRAIN_SIZE / (self.size - 1) as f32
    }

    fn height(&self, x: usize, z: usize) -> f32 {
        self.heights[z * self.size + x]
    }

    fn position(&self, x: usize, z: usize) -> Vec3 {
        let spacing = self.spacing();
        Vec3::new(
            x as f32 * spacing - TERRAIN_SIZE / 2.0,
            self.height(x, z),
            z as f32 * spacing - TERRAIN_SIZE / 2.0,
        )
    }

    fn normal(&self, x: usize, z: usize) -> Vec3 {
        let last = self.size - 1;
        let dx = self.height((x + 1).min(last), z) - self.height(x.saturating_sub(1), z);
        let dz = self.height(x, (z + 1).min(last)) - self.height(x, z.saturating_sub(1));
        Vec3::new(-dx, 2.0 * self.spacing(), -dz).normalize()
    }

    fn color(&self, x: usize, z: usize) -> [f32; 4] {
        let stops = [
            (MIN_HEIGHT, Color::srgb(0.3, 0.22, 0.15)),
            (-2.0, Color::srgb(0.76, 0.7, 0.5)),
            (0.0, Color::srgb(0.35, 0.6, 0.2)),
            (5.0, Color::srgb(0.12, 0.35, 0.12)),
            (9.0, Color::srgb(0.42, 0.38, 0.34)),
            (13.0, Color::srgb(0.97, 0.97, 1.0)),
        ];
        let height = self.height(x, z);
        let mut color = stops[0].1;
        for pair in stops.windows(2) {
            let ((start, from), (end, to)) = (pair[0], pair[1]);
            if height > start {
                color = from.mix(&to, ((height - start) / (end - start)).min(1.0));
            }
        }
        color.to_linear().to_f32_array()
    }
}

#[derive(Component)]
struct InfoText;

fn main() {
    #[cfg(not(target_arch = "wasm32"))]
    let args: Args = argh::from_env();
    #[cfg(target_arch = "wasm32")]
    let args = Args::from_args(&[], &[]).unwrap();

    App::new()
        .add_plugins((
            DefaultPlugins.set(WindowPlugin {
                primary_window: Some(Window {
                    present_mode: PresentMode::AutoNoVsync,
                    ..default()
                }),
                ..default()
            }),
            FrameTimeDiagnosticsPlugin::default(),
            LogDiagnosticsPlugin::default(),
        ))
        .insert_resource(WinitSettings::continuous())
        .insert_resource(ClearColor(SKY_COLOR))
        .insert_resource(args)
        .add_systems(Startup, setup)
        .add_systems(Update, (sculpt, orbit_camera, update_text))
        .run();
}

fn setup(
    mut commands: Commands,
    args: Res<Args>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let size = args.grid_size.max(2);
    let mut terrain = Terrain {
        mesh: Handle::default(),
        size,
        heights: vec![0.0; size * size],
    };
    for z in 0..size {
        for x in 0..size {
            let p = terrain.position(x, z);
            terrain.heights[z * size + x] = 3.0 * ops::sin(p.x * 0.07) * ops::cos(p.z * 0.05)
                + 2.0 * ops::sin(p.x * 0.13 + p.z * 0.11)
                + 0.8 * ops::cos(p.x * 0.31 - p.z * 0.27);
        }
    }

    let vertex_count = size * size;
    let mut positions = Vec::with_capacity(vertex_count);
    let mut normals = Vec::with_capacity(vertex_count);
    let mut uvs = Vec::with_capacity(vertex_count);
    let mut colors = Vec::with_capacity(vertex_count);
    for z in 0..size {
        for x in 0..size {
            positions.push(terrain.position(x, z).to_array());
            normals.push(terrain.normal(x, z).to_array());
            uvs.push([x as f32, z as f32].map(|v| v / (size - 1) as f32));
            colors.push(terrain.color(x, z));
        }
    }

    let row = size as u32;
    let mut indices = Vec::with_capacity((size - 1) * (size - 1) * 6);
    for z in 0..row - 1 {
        for x in 0..row - 1 {
            let i = z * row + x;
            indices.extend([i, i + row, i + 1, i + 1, i + row, i + row + 1]);
        }
    }

    let mesh = Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::default(),
    )
    .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, positions)
    .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, normals)
    .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, uvs)
    .with_inserted_attribute(Mesh::ATTRIBUTE_COLOR, colors)
    .with_inserted_indices(Indices::U32(indices));
    terrain.mesh = meshes.add(mesh);

    commands.spawn((
        Mesh3d(terrain.mesh.clone()),
        MeshMaterial3d(materials.add(StandardMaterial {
            perceptual_roughness: 0.9,
            ..default()
        })),
    ));
    commands.insert_resource(terrain);

    commands.spawn((
        Mesh3d(meshes.add(Plane3d::default().mesh().size(TERRAIN_SIZE, TERRAIN_SIZE))),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::srgb(0.15, 0.35, 0.55),
            perceptual_roughness: 0.15,
            ..default()
        })),
        Transform::from_xyz(0.0, WATER_HEIGHT, 0.0),
    ));

    commands.spawn((
        DirectionalLight::default(),
        Transform::from_xyz(4.0, 3.0, 2.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));
    commands.spawn((
        Camera3d::default(),
        DistanceFog {
            color: SKY_COLOR,
            falloff: FogFalloff::Linear {
                start: 120.0,
                end: 260.0,
            },
            ..default()
        },
    ));

    commands.spawn((
        InfoText,
        Text::default(),
        Node {
            position_type: PositionType::Absolute,
            top: px(12),
            left: px(12),
            padding: UiRect::all(px(10)),
            ..default()
        },
        BackgroundColor(Color::BLACK.with_alpha(0.6)),
    ));
}

fn sculpt(
    time: Res<Time>,
    mut terrain: ResMut<Terrain>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut gizmos: Gizmos,
) {
    let t = time.elapsed_secs();
    let center = Vec2::new(ops::sin(t * 0.21), ops::sin(t * 0.16 + 1.0)) * TERRAIN_SIZE * 0.35;
    let raising = t % 9.0 < 6.0;
    let target = if raising { MAX_HEIGHT } else { MIN_HEIGHT };
    let strength = BRUSH_SPEED * time.delta_secs();

    let size = terrain.size;
    let last = size - 1;
    let to_grid = |v: f32| (v + TERRAIN_SIZE / 2.0) / terrain.spacing();
    let x_min = to_grid(center.x - BRUSH_RADIUS).max(0.0) as usize;
    let x_max = (to_grid(center.x + BRUSH_RADIUS) as usize + 1).min(last);
    let z_min = to_grid(center.y - BRUSH_RADIUS).max(0.0) as usize;
    let z_max = (to_grid(center.y + BRUSH_RADIUS) as usize + 1).min(last);

    let mut ring_height = MIN_HEIGHT;
    for z in z_min..=z_max {
        for x in x_min..=x_max {
            let p = terrain.position(x, z);
            let r = Vec2::new(p.x, p.z).distance(center) / BRUSH_RADIUS;
            if r < 1.0 {
                let falloff = (1.0 - r * r) * (1.0 - r * r);
                let height = &mut terrain.heights[z * size + x];
                *height += (target - *height) * strength * falloff;
                ring_height = ring_height.max(*height);
            }
        }
    }

    gizmos.circle(
        Isometry3d::new(
            Vec3::new(center.x, ring_height + 0.3, center.y),
            Quat::from_rotation_x(FRAC_PI_2),
        ),
        BRUSH_RADIUS,
        if raising { LIME } else { RED },
    );

    let Some(mut mesh) = meshes.get_mut(&terrain.mesh) else {
        return;
    };
    let region = || {
        (z_min.saturating_sub(1)..=(z_max + 1).min(last)).flat_map(move |z| {
            (x_min.saturating_sub(1)..=(x_max + 1).min(last)).map(move |x| (x, z))
        })
    };
    if let Some(VertexAttributeValues::Float32x3(positions)) =
        mesh.attribute_mut(Mesh::ATTRIBUTE_POSITION)
    {
        for (x, z) in region() {
            positions[z * size + x][1] = terrain.height(x, z);
        }
    }
    if let Some(VertexAttributeValues::Float32x3(normals)) =
        mesh.attribute_mut(Mesh::ATTRIBUTE_NORMAL)
    {
        for (x, z) in region() {
            normals[z * size + x] = terrain.normal(x, z).to_array();
        }
    }
    if let Some(VertexAttributeValues::Float32x4(colors)) =
        mesh.attribute_mut(Mesh::ATTRIBUTE_COLOR)
    {
        for (x, z) in region() {
            colors[z * size + x] = terrain.color(x, z);
        }
    }
}

fn orbit_camera(time: Res<Time>, mut camera: Single<&mut Transform, With<Camera3d>>) {
    let angle = time.elapsed_secs() * 0.1;
    **camera = Transform::from_xyz(ops::cos(angle) * 85.0, 50.0, ops::sin(angle) * 85.0)
        .looking_at(Vec3::ZERO, Vec3::Y);
}

fn update_text(diagnostics: Res<DiagnosticsStore>, mut text: Single<&mut Text, With<InfoText>>) {
    let frame_time = diagnostics
        .get(&FrameTimeDiagnosticsPlugin::FRAME_TIME)
        .and_then(Diagnostic::smoothed)
        .unwrap_or_default();
    let fps = if frame_time > 0.0 {
        1000.0 / frame_time
    } else {
        0.0
    };
    text.0 = format!("Frame time: {frame_time:.1} ms ({fps:.0} FPS)");
}
