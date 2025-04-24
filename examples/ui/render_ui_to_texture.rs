//! Shows how to render UI to a texture. Useful for displaying UI in 3D space.

use std::f32::consts::PI;

use bevy::{
    asset::uuid::Uuid,
    color::palettes::css::{BLUE, GOLD, RED},
    input::ButtonState,
    picking::{
        backend::ray::RayMap,
        pointer::{Location, PointerAction, PointerId, PointerInput},
        PickSet,
    },
    prelude::*,
    render::{
        camera::{ManualTextureViews, RenderTarget},
        mesh::Indices,
        mesh::VertexAttributeValues,
        render_asset::RenderAssetUsages,
        render_resource::{Extent3d, TextureDimension, TextureFormat, TextureUsages},
    },
    window::{PrimaryWindow, WindowEvent},
};

const CUBE_POINTER_ID: PointerId = PointerId::Custom(Uuid::from_u128(90870987));

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_systems(Startup, setup)
        .add_systems(Update, rotator_system)
        .add_systems(First, drive_diegetic_pointer.in_set(PickSet::Input))
        .run();
}

// Marks the cube, to which the UI texture is applied.
#[derive(Component)]
struct Cube;

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut images: ResMut<Assets<Image>>,
) {
    let size = Extent3d {
        width: 512,
        height: 512,
        ..default()
    };

    // This is the texture that will be rendered to.
    let mut image = Image::new_fill(
        size,
        TextureDimension::D2,
        &[0, 0, 0, 0],
        TextureFormat::Bgra8UnormSrgb,
        RenderAssetUsages::default(),
    );
    // You need to set these texture usage flags in order to use the image as a render target
    image.texture_descriptor.usage =
        TextureUsages::TEXTURE_BINDING | TextureUsages::COPY_DST | TextureUsages::RENDER_ATTACHMENT;

    let image_handle = images.add(image);

    // Light
    commands.spawn(DirectionalLight::default());

    let texture_camera = commands
        .spawn((
            Camera2d,
            Camera {
                target: RenderTarget::Image(image_handle.clone().into()),
                ..default()
            },
        ))
        .id();

    commands
        .spawn((
            Node {
                // Cover the whole image
                width: Val::Percent(100.),
                height: Val::Percent(100.),
                flex_direction: FlexDirection::Column,
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
            },
            BackgroundColor(GOLD.into()),
            UiTargetCamera(texture_camera),
        ))
        .with_children(|parent| {
            parent.spawn((
                Text::new("This is a cube"),
                TextFont {
                    font_size: 40.0,
                    ..default()
                },
                TextColor::BLACK,
            ));
            parent
                .spawn((
                    Node {
                        position_type: PositionType::Absolute,
                        width: Val::Px(100.),
                        height: Val::Px(100.),
                        ..default()
                    },
                    BackgroundColor(BLUE.into()),
                ))
                .observe(
                    |pointer: Trigger<Pointer<Drag>>, mut nodes: Query<&mut Node>| {
                        let mut node = nodes.get_mut(pointer.target()).unwrap();
                        node.left = Val::Px(pointer.pointer_location.position.x - 50.0);
                        node.top = Val::Px(pointer.pointer_location.position.y - 50.0);
                    },
                )
                .observe(
                    |pointer: Trigger<Pointer<Over>>, mut colors: Query<&mut BackgroundColor>| {
                        colors.get_mut(pointer.target()).unwrap().0 = RED.into();
                    },
                )
                .observe(
                    |pointer: Trigger<Pointer<Out>>, mut colors: Query<&mut BackgroundColor>| {
                        colors.get_mut(pointer.target()).unwrap().0 = BLUE.into();
                    },
                );
        });

    let cube_size = 4.0;
    let cube_handle = meshes.add(Torus::new(cube_size / 2.0, cube_size));

    // This material has the texture that has been rendered.
    let material_handle = materials.add(StandardMaterial {
        base_color_texture: Some(image_handle),
        reflectance: 0.02,
        unlit: false,

        ..default()
    });

    // Cube with material containing the rendered UI texture.
    commands.spawn((
        Mesh3d(cube_handle),
        MeshMaterial3d(material_handle),
        Transform::from_xyz(0.0, 0.0, 1.5).with_rotation(Quat::from_rotation_x(-PI / 5.0)),
        Cube,
    ));

    // The main pass camera.
    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(0.0, 0.0, 15.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));

    commands.spawn((CUBE_POINTER_ID, CubePointer));
}

const ROTATION_SPEED: f32 = 0.1;

fn rotator_system(time: Res<Time>, mut query: Query<&mut Transform, With<Cube>>) {
    for mut transform in &mut query {
        transform.rotate_x(1.0 * time.delta_secs() * ROTATION_SPEED);
        transform.rotate_y(0.7 * time.delta_secs() * ROTATION_SPEED);
    }
}

#[derive(Component)]
struct CubePointer;

fn drive_diegetic_pointer(
    mut cursor_last: Local<Vec2>,
    mut raycast: MeshRayCast,
    rays: Res<RayMap>,
    cubes: Query<&Mesh3d, With<Cube>>,
    meshes: Res<Assets<Mesh>>,
    ui_camera: Query<&Camera, With<Camera2d>>,
    primary_window: Query<Entity, With<PrimaryWindow>>,
    windows: Query<(Entity, &Window)>,
    images: Res<Assets<Image>>,
    manual_texture_views: Res<ManualTextureViews>,
    mut window_events: EventReader<WindowEvent>,
    mut pointer_input: EventWriter<PointerInput>,
) -> Result {
    let target = ui_camera
        .single()?
        .target
        .normalize(primary_window.single().ok())
        .unwrap();
    let target_info = target
        .get_render_target_info(windows, &images, &manual_texture_views)
        .unwrap();
    let size = target_info.physical_size.as_vec2();

    let settings = MeshRayCastSettings {
        visibility: RayCastVisibility::VisibleInView,
        filter: &|entity| cubes.contains(entity),
        early_exit_test: &|_| false,
    };

    for (_id, ray) in rays.iter() {
        for (cube, hit) in raycast.cast_ray(*ray, &settings) {
            let mesh = meshes.get(cubes.get(*cube)?).unwrap();
            let uvs = mesh.attribute(Mesh::ATTRIBUTE_UV_0);
            let Some(VertexAttributeValues::Float32x2(uvs)) = uvs else {
                continue;
            };

            let uvs: [Vec2; 3] = if let Some(indices) = mesh.indices() {
                let i = hit.triangle_index.unwrap() * 3;
                match indices {
                    Indices::U16(indices) => [
                        Vec2::from(uvs[indices[i] as usize]),
                        Vec2::from(uvs[indices[i + 1] as usize]),
                        Vec2::from(uvs[indices[i + 2] as usize]),
                    ],
                    Indices::U32(indices) => [
                        Vec2::from(uvs[indices[i] as usize]),
                        Vec2::from(uvs[indices[i + 1] as usize]),
                        Vec2::from(uvs[indices[i + 2] as usize]),
                    ],
                }
            } else {
                let i = hit.triangle_index.unwrap() * 3;
                [
                    Vec2::from(uvs[i]),
                    Vec2::from(uvs[i + 1]),
                    Vec2::from(uvs[i + 2]),
                ]
            };

            let bc = hit.barycentric_coords.zxy();
            let uv = bc.x * uvs[0] + bc.y * uvs[1] + bc.z * uvs[2];
            let position = size * uv;

            if position != *cursor_last {
                pointer_input.write(PointerInput::new(
                    CUBE_POINTER_ID,
                    Location {
                        target: target.clone(),
                        position,
                    },
                    PointerAction::Move {
                        delta: position - *cursor_last,
                    },
                ));
                *cursor_last = position;
            }
        }
    }

    for window_event in window_events.read() {
        if let WindowEvent::MouseButtonInput(input) = window_event {
            let button = match input.button {
                MouseButton::Left => PointerButton::Primary,
                MouseButton::Right => PointerButton::Secondary,
                MouseButton::Middle => PointerButton::Middle,
                _ => continue,
            };
            let action = match input.state {
                ButtonState::Pressed => PointerAction::Press(button),
                ButtonState::Released => PointerAction::Release(button),
            };
            pointer_input.write(PointerInput::new(
                CUBE_POINTER_ID,
                Location {
                    target: target.clone(),
                    position: *cursor_last,
                },
                action,
            ));
        }
    }

    Result::Ok(())
}
