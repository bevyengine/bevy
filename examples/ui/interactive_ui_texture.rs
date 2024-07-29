//! Shows how to interact with a texture-based UI.

use bevy::{
    color::palettes::basic::*,
    input::InputSystem,
    prelude::*,
    render::{
        camera::RenderTarget,
        render_resource::{
            Extent3d, TextureDescriptor, TextureDimension, TextureFormat, TextureUsages,
        },
    },
    ui::{ManualCursorPosition, UiSystem},
    window::PrimaryWindow,
};

#[path = "../helpers/camera_controller.rs"]
mod camera_controller;
use camera_controller::{CameraController, CameraControllerPlugin};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(CameraControllerPlugin)
        .add_systems(Startup, setup)
        .add_systems(Update, button_system)
        .add_systems(
            PreUpdate,
            update_manual_cursor
                .after(InputSystem)
                .before(UiSystem::Focus),
        )
        .run();
}

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut images: ResMut<Assets<Image>>,
    asset_server: Res<AssetServer>,
) {
    let size = Extent3d {
        width: 512,
        height: 512,
        ..default()
    };

    // This is the texture that will be rendered to.
    let mut image = Image {
        texture_descriptor: TextureDescriptor {
            label: None,
            size,
            dimension: TextureDimension::D2,
            format: TextureFormat::Bgra8UnormSrgb,
            mip_level_count: 1,
            sample_count: 1,
            usage: TextureUsages::TEXTURE_BINDING
                | TextureUsages::COPY_DST
                | TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        },
        ..default()
    };

    // fill image.data with zeroes
    image.resize(size);

    let image_handle = images.add(image);

    // Light
    commands.spawn(DirectionalLightBundle::default());

    // UI texture camera
    let texture_camera = commands
        .spawn((
            Camera2dBundle {
                camera: Camera {
                    // render before the "main pass" camera
                    order: -1,
                    target: RenderTarget::Image(image_handle.clone()),
                    ..default()
                },
                ..default()
            },
            // add `ManualCursorPosition` which we will update in `update_manual_cursor`
            ManualCursorPosition::default(),
        ))
        .id();

    // make the button ui
    commands
        .spawn((
            NodeBundle {
                style: Style {
                    // Cover the whole image
                    width: Val::Percent(100.),
                    height: Val::Percent(100.),
                    flex_direction: FlexDirection::Column,
                    justify_content: JustifyContent::SpaceAround,
                    align_items: AlignItems::Center,
                    ..default()
                },
                background_color: NORMAL_BUTTON.into(),
                ..default()
            },
            TargetCamera(texture_camera),
        ))
        .with_children(|parent| {
            for _ in 0..=1 {
                parent
                    .spawn(ButtonBundle {
                        style: Style {
                            width: Val::Px(150.0),
                            height: Val::Px(65.0),
                            border: UiRect::all(Val::Px(5.0)),
                            // horizontally center child text
                            justify_content: JustifyContent::Center,
                            // vertically center child text
                            align_items: AlignItems::Center,
                            ..default()
                        },
                        border_color: BorderColor(Color::BLACK),
                        background_color: NORMAL_BUTTON.into(),
                        ..default()
                    })
                    .with_children(|parent| {
                        parent.spawn(TextBundle::from_section(
                            "Button",
                            TextStyle {
                                font: asset_server.load("fonts/FiraSans-Bold.ttf"),
                                font_size: 40.0,
                                color: Color::srgb(0.9, 0.9, 0.9),
                            },
                        ));
                    });
            }
        });

    // This material has the texture that has been rendered.
    let material_handle = materials.add(StandardMaterial {
        base_color_texture: Some(image_handle),
        reflectance: 0.02,
        unlit: false,

        ..default()
    });

    // quad with material containing the rendered UI texture.
    commands.spawn((PbrBundle {
        mesh: meshes.add(Rectangle::new(2.0, 2.0)),
        material: material_handle,
        transform: Transform::from_xyz(0.0, 0.0, 0.0),
        ..default()
    },));

    // The main pass camera.
    commands
        .spawn(Camera3dBundle {
            transform: Transform::from_xyz(-1.0, 1.0, 5.0).looking_at(Vec3::ZERO, Vec3::Y),
            ..default()
        })
        .insert(CameraController {
            mouse_key_cursor_grab: MouseButton::Right,
            ..Default::default()
        });
}

fn update_manual_cursor(
    main_pass_camera: Query<(&GlobalTransform, &Camera), Without<ManualCursorPosition>>,
    main_window: Query<&Window, With<PrimaryWindow>>,
    touches_input: Res<Touches>,
    mut position: Query<&mut ManualCursorPosition>,
) {
    // clear the manual position
    position.single_mut().0 = None;

    let (camera_position, camera) = main_pass_camera.single();

    // get cursor position in the window
    let Some(cursor_position) = main_window
        .get_single()
        .unwrap()
        .cursor_position()
        .or_else(|| touches_input.first_pressed_position())
    else {
        return;
    };

    // here we convert the cursor position into a position inside the in-world ui texture.
    // because our texture is a flat quad on the x/y plane this is easy, but more complex
    // scenarios are also possible, e.g. using a collision library raycast to get contact
    // faces on non-flat meshes and extracting uvs from the mesh vertices

    // get a ray from the cursor position on the main pass camera into the 3d world
    let ray = camera
        .viewport_to_world(camera_position, cursor_position)
        .expect("viewport_to_world failed");

    // check if we hit the plane containing the ui texture
    let Some(intersect) = ray.intersect_plane(Vec3::ZERO, InfinitePlane3d { normal: Dir3::Z })
    else {
        return;
    };

    // limit the length of the ray so we can't interact from too far away
    if intersect * ray.direction.length() > 10.0 {
        return;
    }

    // get the point on the plane
    let hit_xy = ray.get_point(intersect).xy();

    // check if it's within our rectangle
    if hit_xy.max(Vec2::NEG_ONE).min(Vec2::ONE) != hit_xy {
        return;
    }

    // transform it into texture coords for the in-world ui rect
    position.single_mut().0 = Some((hit_xy * Vec2::new(0.5, -0.5) + 0.5) * 512.0);
}

// remainder is copied from button.rs example
const NORMAL_BUTTON: Color = Color::srgb(0.15, 0.15, 0.15);
const HOVERED_BUTTON: Color = Color::srgb(0.25, 0.25, 0.25);
const PRESSED_BUTTON: Color = Color::srgb(0.35, 0.75, 0.35);

fn button_system(
    mut interaction_query: Query<
        (
            &Interaction,
            &mut BackgroundColor,
            &mut BorderColor,
            &Children,
        ),
        (Changed<Interaction>, With<Button>),
    >,
    mut text_query: Query<&mut Text>,
) {
    for (interaction, mut color, mut border_color, children) in &mut interaction_query {
        let mut text = text_query.get_mut(children[0]).unwrap();
        match *interaction {
            Interaction::Pressed => {
                text.sections[0].value = "Press".to_string();
                *color = PRESSED_BUTTON.into();
                border_color.0 = RED.into();
            }
            Interaction::Hovered => {
                text.sections[0].value = "Hover".to_string();
                *color = HOVERED_BUTTON.into();
                border_color.0 = WHITE.into();
            }
            Interaction::None => {
                text.sections[0].value = "Button".to_string();
                *color = NORMAL_BUTTON.into();
                border_color.0 = BLACK.into();
            }
        }
    }
}
