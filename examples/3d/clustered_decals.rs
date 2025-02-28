//! Demonstrates clustered decals, which affix decals to surfaces.

use std::f32::consts::{FRAC_PI_2, FRAC_PI_3, FRAC_PI_4, PI};
use std::fmt::{self, Formatter};
use std::process;

use bevy::{
    color::palettes::css::{LIME, ORANGE_RED, SILVER, YELLOW},
    input::mouse::AccumulatedMouseMotion,
    pbr::{
        decal::{
            self,
            clustered::{
                ClusteredDecal, DirectionalLightCookie, PointLightCookie, SpotLightCookie,
            },
        },
        ExtendedMaterial, MaterialExtension, NotShadowCaster,
    },
    prelude::*,
    render::{
        render_resource::{AsBindGroup, ShaderRef},
        renderer::{RenderAdapter, RenderDevice},
    },
    window::SystemCursorIcon,
    winit::cursor::CursorIcon,
};
use light_consts::lux::{AMBIENT_DAYLIGHT, CLEAR_SUNRISE};
use ops::{acos, cos, sin};
use widgets::{
    WidgetClickEvent, WidgetClickSender, BUTTON_BORDER, BUTTON_BORDER_COLOR,
    BUTTON_BORDER_RADIUS_SIZE, BUTTON_PADDING,
};

#[path = "../helpers/widgets.rs"]
mod widgets;

/// The custom material shader that we use to demonstrate how to use the decal
/// `tag` field.
const SHADER_ASSET_PATH: &str = "shaders/custom_clustered_decal.wgsl";

/// The speed at which the cube rotates, in radians per frame.
const CUBE_ROTATION_SPEED: f32 = 0.02;

/// The speed at which the selection can be moved, in spherical coordinate
/// radians per mouse unit.
const MOVE_SPEED: f32 = 0.008;
/// The speed at which the selection can be scaled, in reciprocal mouse units.
const SCALE_SPEED: f32 = 0.05;
/// The speed at which the selection can be scaled, in radians per mouse unit.
const ROLL_SPEED: f32 = 0.01;

/// Various settings for the demo.
#[derive(Resource, Default)]
struct AppStatus {
    /// The object that will be moved, scaled, or rotated when the mouse is
    /// dragged.
    selection: Selection,
    /// What happens when the mouse is dragged: one of a move, rotate, or scale
    /// operation.
    drag_mode: DragMode,
}

/// The object that will be moved, scaled, or rotated when the mouse is dragged.
#[derive(Clone, Copy, Component, Default, PartialEq)]
enum Selection {
    /// The camera.
    ///
    /// The camera can only be moved, not scaled or rotated.
    #[default]
    Camera,
    /// The first decal, which an orange bounding box surrounds.
    DecalA,
    /// The second decal, which a lime green bounding box surrounds.
    DecalB,
    /// The spotlight, which uses a torch-like light cookie
    SpotLight,
    /// The point light, which uses a light cookie cubemap constructed from the faces mesh
    PointLight,
    /// The directional light, which uses a caustic-like cookie
    DirectionalLight,
}

impl fmt::Display for Selection {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match *self {
            Selection::Camera => f.write_str("camera"),
            Selection::DecalA => f.write_str("decal A"),
            Selection::DecalB => f.write_str("decal B"),
            Selection::SpotLight => f.write_str("spotlight"),
            Selection::PointLight => f.write_str("point light"),
            Selection::DirectionalLight => f.write_str("directional light"),
        }
    }
}

/// What happens when the mouse is dragged: one of a move, rotate, or scale
/// operation.
#[derive(Clone, Copy, Component, Default, PartialEq, Debug)]
enum DragMode {
    /// The mouse moves the current selection.
    #[default]
    Move,
    /// The mouse scales the current selection.
    ///
    /// This only applies to decals, not cameras.
    Scale,
    /// The mouse rotates the current selection around its local Z axis.
    ///
    /// This only applies to decals, not cameras.
    Roll,
}

impl fmt::Display for DragMode {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match *self {
            DragMode::Move => f.write_str("move"),
            DragMode::Scale => f.write_str("scale"),
            DragMode::Roll => f.write_str("roll"),
        }
    }
}

/// A marker component for the help text in the top left corner of the window.
#[derive(Clone, Copy, Component)]
struct HelpText;

/// A shader extension that demonstrates how to use the `tag` field to customize
/// the appearance of your decals.
#[derive(Asset, AsBindGroup, Reflect, Debug, Clone)]
struct CustomDecalExtension {}

impl MaterialExtension for CustomDecalExtension {
    fn fragment_shader() -> ShaderRef {
        SHADER_ASSET_PATH.into()
    }
}

/// Entry point.
fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Bevy Clustered Decals Example".into(),
                ..default()
            }),
            ..default()
        }))
        .add_plugins(MaterialPlugin::<
            ExtendedMaterial<StandardMaterial, CustomDecalExtension>,
        >::default())
        .init_resource::<AppStatus>()
        .add_event::<WidgetClickEvent<Selection>>()
        .add_event::<WidgetClickEvent<Visibility>>()
        .add_systems(Startup, setup)
        .add_systems(Update, draw_gizmos)
        .add_systems(Update, rotate_cube)
        .add_systems(Update, hide_shadows)
        .add_systems(Update, widgets::handle_ui_interactions::<Selection>)
        .add_systems(Update, widgets::handle_ui_interactions::<Visibility>)
        .add_systems(
            Update,
            (handle_selection_change, update_radio_buttons)
                .after(widgets::handle_ui_interactions::<Selection>)
                .after(widgets::handle_ui_interactions::<Visibility>),
        )
        .add_systems(Update, toggle_visibility)
        .add_systems(Update, update_directional_light)
        .add_systems(Update, process_move_input)
        .add_systems(Update, process_scale_input)
        .add_systems(Update, process_roll_input)
        .add_systems(Update, switch_drag_mode)
        .add_systems(Update, update_help_text)
        .add_systems(Update, update_button_visibility)
        .run();
}

/// Creates the scene.
fn setup(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    app_status: Res<AppStatus>,
    render_device: Res<RenderDevice>,
    render_adapter: Res<RenderAdapter>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ExtendedMaterial<StandardMaterial, CustomDecalExtension>>>,
) {
    // Error out if the clustered decals feature isn't enabled
    if !cfg!(feature = "pbr_clustered_decals") {
        eprintln!("Bevy was compiled without clustered decal support. Run with `--features=pbr_clustered_decals` to enable.");
        process::exit(1);
    }

    // Error out if clustered decals aren't supported on the current platform.
    if !decal::clustered::clustered_decals_are_usable(&render_device, &render_adapter) {
        eprintln!("Clustered decals aren't usable on this platform.");
        process::exit(1);
    }

    spawn_cubes(&mut commands, &mut meshes, &mut materials);
    spawn_camera(&mut commands);
    spawn_light(&mut commands, &asset_server);
    spawn_decals(&mut commands, &asset_server);
    spawn_buttons(&mut commands);
    spawn_help_text(&mut commands, &app_status);
    spawn_light_cookies(&mut commands, &asset_server, &mut meshes, &mut materials);
}

#[derive(Component)]
struct Rotate;

/// Spawns the cube onto which the decals are projected.
fn spawn_cubes(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<ExtendedMaterial<StandardMaterial, CustomDecalExtension>>,
) {
    // Rotate the cube a bit just to make it more interesting.
    let mut transform = Transform::IDENTITY;
    transform.rotate_y(FRAC_PI_3);

    commands.spawn((
        Mesh3d(meshes.add(Cuboid::new(3.0, 3.0, 3.0))),
        MeshMaterial3d(materials.add(ExtendedMaterial {
            base: StandardMaterial {
                base_color: SILVER.into(),
                ..default()
            },
            extension: CustomDecalExtension {},
        })),
        transform,
        Rotate,
    ));

    commands.spawn((
        Mesh3d(meshes.add(Cuboid::new(-13.0, -13.0, -13.0))),
        MeshMaterial3d(materials.add(ExtendedMaterial {
            base: StandardMaterial {
                base_color: SILVER.into(),
                ..default()
            },
            extension: CustomDecalExtension {},
        })),
        transform,
    ));
}

/// Spawns the directional light.
fn spawn_light(commands: &mut Commands, asset_server: &AssetServer) {
    commands
        .spawn((
            Visibility::Hidden,
            Transform::from_xyz(8.0, 8.0, 4.0).looking_at(Vec3::ZERO, Vec3::Y),
            Selection::DirectionalLight,
        ))
        .with_child((
            DirectionalLight {
                illuminance: AMBIENT_DAYLIGHT,
                ..default()
            },
            DirectionalLightCookie {
                image: asset_server.load("lightmaps/caustic_directional_cookie.png"),
                tiled: true,
            },
            Visibility::Visible,
        ));
}

/// Spawns the camera.
fn spawn_camera(commands: &mut Commands) {
    commands
        .spawn(Camera3d::default())
        .insert(Transform::from_xyz(0.0, 2.5, 9.0).looking_at(Vec3::ZERO, Vec3::Y))
        // Tag the camera with `Selection::Camera`.
        .insert(Selection::Camera);
}

/// Spawns the actual clustered decals.
fn spawn_decals(commands: &mut Commands, asset_server: &AssetServer) {
    let image = asset_server.load("branding/icon.png");

    commands.spawn((
        ClusteredDecal {
            image: image.clone(),
            // Tint with red.
            tag: 1,
        },
        calculate_initial_decal_transform(vec3(1.0, 3.0, 5.0), Vec3::ZERO, Vec2::splat(1.1)),
        Selection::DecalA,
    ));

    commands.spawn((
        ClusteredDecal {
            image: image.clone(),
            // Tint with blue.
            tag: 2,
        },
        calculate_initial_decal_transform(vec3(-2.0, -1.0, 4.0), Vec3::ZERO, Vec2::splat(2.0)),
        Selection::DecalB,
    ));
}

fn spawn_light_cookies(
    commands: &mut Commands,
    asset_server: &AssetServer,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<ExtendedMaterial<StandardMaterial, CustomDecalExtension>>,
) {
    commands.spawn((
        SpotLight {
            color: Color::srgb(1.0, 1.0, 0.8),
            intensity: 10e6,
            outer_angle: 0.25,
            inner_angle: 0.25,
            shadows_enabled: true,
            ..default()
        },
        Transform::from_translation(Vec3::new(-6.0, 1.0, 2.0)).looking_at(Vec3::ZERO, Vec3::Y),
        SpotLightCookie {
            image: asset_server.load("lightmaps/torch_spotlight_cookie.png"),
        },
        Visibility::Hidden,
        Selection::SpotLight,
    ));

    commands
        .spawn((
            Visibility::Hidden,
            Transform::from_translation(Vec3::new(0.0, 1.8, 0.01)).with_scale(Vec3::splat(0.1)),
            Selection::PointLight,
        ))
        .with_children(|parent| {
            parent.spawn(SceneRoot(
                asset_server.load(GltfAssetLabel::Scene(0).from_asset("models/Faces/faces.glb")),
            ));

            parent.spawn((
                Mesh3d(meshes.add(Sphere::new(1.0))),
                MeshMaterial3d(materials.add(ExtendedMaterial {
                    base: StandardMaterial {
                        emissive: Color::srgb(0.0, 0.0, 300.0).to_linear(),
                        ..default()
                    },
                    extension: CustomDecalExtension {},
                })),
            ));

            parent.spawn((
                PointLight {
                    color: Color::srgb(0.0, 0.0, 1.0),
                    intensity: 1e6,
                    shadows_enabled: true,
                    ..default()
                },
                PointLightCookie {
                    image: asset_server.load("lightmaps/faces_pointlight_cookie_blurred.png"),
                    cubemap_layout: decal::clustered::CubemapLayout::CrossVertical,
                },
            ));
        });
}

/// Spawns the buttons at the bottom of the screen.
fn spawn_buttons(commands: &mut Commands) {
    // Spawn the radio buttons that allow the user to select an object to
    // control.
    commands
        .spawn(widgets::main_ui_node())
        .with_children(|parent| {
            widgets::spawn_option_buttons(
                parent,
                "Drag to Move",
                &[
                    (Selection::Camera, "Camera"),
                    (Selection::DecalA, "Decal A"),
                    (Selection::DecalB, "Decal B"),
                    (Selection::SpotLight, "Spotlight"),
                    (Selection::PointLight, "Point Light"),
                    (Selection::DirectionalLight, "Directional Light"),
                ],
            );
        });

    // Spawn the drag buttons that allow the user to control the scale and roll
    // of the selected object.
    commands
        .spawn(Node {
            flex_direction: FlexDirection::Row,
            position_type: PositionType::Absolute,
            right: Val::Px(10.0),
            bottom: Val::Px(10.0),
            column_gap: Val::Px(6.0),
            ..default()
        })
        .with_children(|parent| {
            widgets::spawn_option_buttons(
                parent,
                "",
                &[
                    (Visibility::Inherited, "Show"),
                    (Visibility::Hidden, "Hide"),
                ],
            );
            spawn_drag_button(parent, "Scale").insert(DragMode::Scale);
            spawn_drag_button(parent, "Roll").insert(DragMode::Roll);
        });
}

/// Spawns a button that the user can drag to change a parameter.
fn spawn_drag_button<'a>(
    commands: &'a mut ChildSpawnerCommands,
    label: &str,
) -> EntityCommands<'a> {
    let mut kid = commands.spawn(Node {
        border: BUTTON_BORDER,
        justify_content: JustifyContent::Center,
        align_items: AlignItems::Center,
        padding: BUTTON_PADDING,
        ..default()
    });
    kid.insert((
        Button,
        BackgroundColor(Color::BLACK),
        BorderRadius::all(BUTTON_BORDER_RADIUS_SIZE),
        BUTTON_BORDER_COLOR,
    ))
    .with_children(|parent| {
        widgets::spawn_ui_text(parent, label, Color::WHITE);
    });
    kid
}

/// Spawns the help text at the top of the screen.
fn spawn_help_text(commands: &mut Commands, app_status: &AppStatus) {
    commands.spawn((
        Text::new(create_help_string(app_status)),
        Node {
            position_type: PositionType::Absolute,
            top: Val::Px(12.0),
            left: Val::Px(12.0),
            ..default()
        },
        HelpText,
    ));
}

/// Draws the outlines that show the bounds of the clustered decals.
fn draw_gizmos(
    mut gizmos: Gizmos,
    decals: Query<(&GlobalTransform, &Selection, &Visibility), With<ClusteredDecal>>,
    spotlight: Query<(&GlobalTransform, &SpotLight, &Visibility)>,
) {
    for (global_transform, selection, visibility) in &decals {
        if visibility == Visibility::Hidden {
            continue;
        }

        let color = match *selection {
            Selection::Camera => continue,
            Selection::DecalA => ORANGE_RED,
            Selection::DecalB => LIME,
            _ => unreachable!(),
        };

        gizmos.primitive_3d(
            &Cuboid {
                // Since the clustered decal is a 1×1×1 cube in model space, its
                // half-size is half of the scaling part of its transform.
                half_size: global_transform.scale() * 0.5,
            },
            Isometry3d {
                rotation: global_transform.rotation(),
                translation: global_transform.translation_vec3a(),
            },
            color,
        );
    }

    if let Ok((global_transform, spotlight, visibility)) = spotlight.get_single() {
        if visibility != Visibility::Hidden {
            gizmos.primitive_3d(
                &Cone::new(7.0 * spotlight.outer_angle, 7.0),
                Isometry3d {
                    rotation: global_transform.rotation() * Quat::from_rotation_x(FRAC_PI_2),
                    translation: global_transform.translation_vec3a() * 0.5,
                },
                YELLOW,
            );
        }
    }
}

/// Calculates the initial transform of the clustered decal.
fn calculate_initial_decal_transform(start: Vec3, looking_at: Vec3, size: Vec2) -> Transform {
    let direction = looking_at - start;
    let center = start + direction * 0.5;
    Transform::from_translation(center)
        .with_scale((size * 0.5).extend(direction.length()))
        .looking_to(direction, Vec3::Y)
}

/// Rotates the cube a bit every frame.
fn rotate_cube(mut meshes: Query<&mut Transform, With<Rotate>>) {
    for mut transform in &mut meshes {
        transform.rotate_y(CUBE_ROTATION_SPEED);
    }
}

/// Hide shadows on all meshes except the main cube
fn hide_shadows(
    mut commands: Commands,
    meshes: Query<Entity, (With<Mesh3d>, Without<NotShadowCaster>, Without<Rotate>)>,
) {
    for ent in &meshes {
        commands.entity(ent).insert(NotShadowCaster);
    }
}

/// Updates the state of the radio buttons when the user clicks on one.
fn update_radio_buttons(
    mut widgets: Query<(
        Entity,
        Option<&mut BackgroundColor>,
        Has<Text>,
        &WidgetClickSender<Selection>,
    )>,
    app_status: Res<AppStatus>,
    mut writer: TextUiWriter,
    visible: Query<(&Visibility, &Selection)>,
    mut visibility_widgets: Query<
        (
            Entity,
            Option<&mut BackgroundColor>,
            Has<Text>,
            &WidgetClickSender<Visibility>,
        ),
        Without<WidgetClickSender<Selection>>,
    >,
) {
    for (entity, maybe_bg_color, has_text, sender) in &mut widgets {
        let selected = app_status.selection == **sender;
        if let Some(mut bg_color) = maybe_bg_color {
            widgets::update_ui_radio_button(&mut bg_color, selected);
        }
        if has_text {
            widgets::update_ui_radio_button_text(entity, &mut writer, selected);
        }
    }

    let visibility = visible
        .iter()
        .filter(|(_, selection)| **selection == app_status.selection)
        .map(|(visibility, _)| *visibility)
        .next()
        .unwrap_or_default();
    for (entity, maybe_bg_color, has_text, sender) in &mut visibility_widgets {
        if let Some(mut bg_color) = maybe_bg_color {
            widgets::update_ui_radio_button(&mut bg_color, **sender == visibility);
        }
        if has_text {
            widgets::update_ui_radio_button_text(entity, &mut writer, **sender == visibility);
        }
    }
}

/// Changes the selection when the user clicks a radio button.
fn handle_selection_change(
    mut events: EventReader<WidgetClickEvent<Selection>>,
    mut app_status: ResMut<AppStatus>,
) {
    for event in events.read() {
        app_status.selection = **event;
    }
}

fn toggle_visibility(
    mut events: EventReader<WidgetClickEvent<Visibility>>,
    app_status: Res<AppStatus>,
    mut visibility: Query<(&mut Visibility, &Selection)>,
) {
    if let Some(vis) = events.read().last() {
        for (mut visibility, selection) in visibility.iter_mut() {
            if selection == &app_status.selection {
                *visibility = **vis;
            }
        }
    }
}

/// Process a drag event that moves the selected object.
fn process_move_input(
    mut selections: Query<(&mut Transform, &Selection)>,
    mouse_buttons: Res<ButtonInput<MouseButton>>,
    mouse_motion: Res<AccumulatedMouseMotion>,
    app_status: Res<AppStatus>,
) {
    // Only process drags when movement is selected.
    if !mouse_buttons.pressed(MouseButton::Left) || app_status.drag_mode != DragMode::Move {
        return;
    }

    for (mut transform, selection) in &mut selections {
        if app_status.selection != *selection {
            continue;
        }

        // use simple movement for the point light
        if *selection == Selection::PointLight {
            transform.translation +=
                (mouse_motion.delta * Vec2::new(1.0, -1.0) * MOVE_SPEED).extend(0.0);
            return;
        }

        let position = transform.translation;

        // Convert to spherical coordinates.
        let radius = position.length();
        let mut theta = acos(position.y / radius);
        let mut phi = position.z.signum() * acos(position.x * position.xz().length_recip());

        // Camera movement is the inverse of object movement.
        let (phi_factor, theta_factor) = match *selection {
            Selection::Camera => (1.0, -1.0),
            _ => (-1.0, 1.0),
        };

        // Adjust the spherical coordinates. Clamp the inclination to (0, π).
        phi += phi_factor * mouse_motion.delta.x * MOVE_SPEED;
        theta = f32::clamp(
            theta + theta_factor * mouse_motion.delta.y * MOVE_SPEED,
            0.001,
            PI - 0.001,
        );

        // Convert spherical coordinates back to Cartesian coordinates.
        transform.translation =
            radius * vec3(sin(theta) * cos(phi), cos(theta), sin(theta) * sin(phi));

        // Look at the center, but preserve the previous roll angle.
        let roll = transform.rotation.to_euler(EulerRot::YXZ).2;
        transform.look_at(Vec3::ZERO, Vec3::Y);
        let (yaw, pitch, _) = transform.rotation.to_euler(EulerRot::YXZ);
        transform.rotation = Quat::from_euler(EulerRot::YXZ, yaw, pitch, roll);
    }
}

/// Processes a drag event that scales the selected target.
fn process_scale_input(
    mut scale_selections: Query<(&mut Transform, &Selection)>,
    mut spotlight_selections: Query<(&mut SpotLight, &Selection)>,
    mouse_buttons: Res<ButtonInput<MouseButton>>,
    mouse_motion: Res<AccumulatedMouseMotion>,
    app_status: Res<AppStatus>,
) {
    // Only process drags when the scaling operation is selected.
    if !mouse_buttons.pressed(MouseButton::Left) || app_status.drag_mode != DragMode::Scale {
        return;
    }

    for (mut transform, selection) in &mut scale_selections {
        if app_status.selection == *selection {
            transform.scale *= 1.0 + mouse_motion.delta.x * SCALE_SPEED;
        }
    }

    for (mut spotlight, selection) in &mut spotlight_selections {
        if app_status.selection == *selection {
            spotlight.outer_angle =
                (spotlight.outer_angle * (1.0 + mouse_motion.delta.x * SCALE_SPEED)).min(FRAC_PI_4);
            spotlight.inner_angle = spotlight.outer_angle;
        }
    }
}

/// Processes a drag event that rotates the selected target along its local Z
/// axis.
fn process_roll_input(
    mut selections: Query<(&mut Transform, &Selection)>,
    mouse_buttons: Res<ButtonInput<MouseButton>>,
    mouse_motion: Res<AccumulatedMouseMotion>,
    app_status: Res<AppStatus>,
) {
    // Only process drags when the rolling operation is selected.
    if !mouse_buttons.pressed(MouseButton::Left) || app_status.drag_mode != DragMode::Roll {
        return;
    }

    for (mut transform, selection) in &mut selections {
        if app_status.selection != *selection {
            continue;
        }

        let (yaw, pitch, mut roll) = transform.rotation.to_euler(EulerRot::YXZ);
        roll += mouse_motion.delta.x * ROLL_SPEED;
        transform.rotation = Quat::from_euler(EulerRot::YXZ, yaw, pitch, roll);
    }
}

/// Creates the help string at the top left of the screen.
fn create_help_string(app_status: &AppStatus) -> String {
    format!(
        "Click and drag to {} {}",
        app_status.drag_mode, app_status.selection
    )
}

/// Changes the drag mode when the user hovers over the "Scale" and "Roll"
/// buttons in the lower right.
///
/// If the user is hovering over no such button, this system changes the drag
/// mode back to its default value of [`DragMode::Move`].
fn switch_drag_mode(
    mut commands: Commands,
    mut interactions: Query<(&Interaction, &DragMode)>,
    mut windows: Query<Entity, With<Window>>,
    mouse_buttons: Res<ButtonInput<MouseButton>>,
    mut app_status: ResMut<AppStatus>,
) {
    if mouse_buttons.pressed(MouseButton::Left) {
        return;
    }

    for (interaction, drag_mode) in &mut interactions {
        if *interaction != Interaction::Hovered {
            continue;
        }

        app_status.drag_mode = *drag_mode;

        // Set the cursor to provide the user with a nice visual hint.
        for window in &mut windows {
            commands
                .entity(window)
                .insert(CursorIcon::from(SystemCursorIcon::EwResize));
        }
        return;
    }

    app_status.drag_mode = DragMode::Move;

    for window in &mut windows {
        commands.entity(window).remove::<CursorIcon>();
    }
}

/// Updates the help text in the top left of the screen to reflect the current
/// selection and drag mode.
fn update_help_text(mut help_text: Query<&mut Text, With<HelpText>>, app_status: Res<AppStatus>) {
    for mut text in &mut help_text {
        text.0 = create_help_string(&app_status);
    }
}

/// Updates the visibility of the drag mode buttons so that they aren't visible
/// if the camera is selected.
fn update_button_visibility(
    mut nodes: Query<&mut Visibility, Or<(With<DragMode>, With<WidgetClickSender<Visibility>>)>>,
    app_status: Res<AppStatus>,
) {
    for mut visibility in &mut nodes {
        *visibility = match app_status.selection {
            Selection::Camera => Visibility::Hidden,
            _ => Visibility::Visible,
        };
    }
}

fn update_directional_light(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    selections: Query<(&Selection, &Visibility)>,
    mut light: Query<(
        Entity,
        &mut DirectionalLight,
        Option<&DirectionalLightCookie>,
    )>,
) {
    let directional_visible = selections
        .iter()
        .filter(|(selection, _)| **selection == Selection::DirectionalLight)
        .any(|(_, visibility)| visibility != Visibility::Hidden);
    let any_cookie_light_visible = selections
        .iter()
        .filter(|(selection, _)| {
            **selection == Selection::PointLight || **selection == Selection::SpotLight
        })
        .any(|(_, visibility)| visibility != Visibility::Hidden);

    if directional_visible {
        let (entity, mut light, maybe_cookie) = light.single_mut();
        light.illuminance = AMBIENT_DAYLIGHT;
        if maybe_cookie.is_none() {
            commands.entity(entity).insert(DirectionalLightCookie {
                image: asset_server.load("lightmaps/caustic_directional_cookie.png"),
                tiled: true,
            });
        }
    } else if any_cookie_light_visible {
        let (entity, mut light, maybe_cookie) = light.single_mut();
        light.illuminance = CLEAR_SUNRISE;
        if maybe_cookie.is_some() {
            commands.entity(entity).remove::<DirectionalLightCookie>();
        }
    } else {
        let (entity, mut light, maybe_cookie) = light.single_mut();
        light.illuminance = AMBIENT_DAYLIGHT;
        if maybe_cookie.is_some() {
            commands.entity(entity).remove::<DirectionalLightCookie>();
        }
    }
}
