//! Demonstrates how to create a mirror with a second camera.

use std::f32::consts::FRAC_PI_2;

use crate::widgets::{RadioButton, WidgetClickEvent, WidgetClickSender};
use bevy::camera::RenderTarget;
use bevy::{
    asset::RenderAssetUsages,
    color::palettes::css::GREEN,
    input::mouse::AccumulatedMouseMotion,
    math::{reflection_matrix, uvec2, vec3},
    pbr::{ExtendedMaterial, MaterialExtension},
    prelude::*,
    render::render_resource::{
        AsBindGroup, Extent3d, TextureDimension, TextureFormat, TextureUsages,
    },
    shader::ShaderRef,
    window::{PrimaryWindow, WindowResized},
};
use bevy_asset::RenderAssetTransferPriority;

#[path = "../helpers/widgets.rs"]
mod widgets;

/// A resource that stores a handle to the image that contains the rendered
/// mirror world.
#[derive(Resource)]
struct MirrorImage(Handle<Image>);

/// A marker component for the camera that renders the mirror world.
#[derive(Component)]
struct MirrorCamera;

/// A marker component for the mirror mesh itself.
#[derive(Component)]
struct Mirror;

/// The dummy material extension that we use for the mirror surface.
///
/// This shader samples its emissive texture at the screen space position of
/// each fragment rather than at the UVs. Effectively, this uses a PBR shader as
/// a mask that copies a portion of the emissive texture to the screen, all in
/// screen space.
///
/// We use [`ExtendedMaterial`], as that's the easiest way to implement custom
/// shaders that modify the built-in [`StandardMaterial`]. We don't require any
/// extra data to be passed to the shader beyond the [`StandardMaterial`] PBR
/// fields, but currently Bevy requires at least one field to be present in the
/// extended material, so we simply have an unused field.
#[derive(Clone, AsBindGroup, Asset, Reflect)]
struct ScreenSpaceTextureExtension {
    /// An unused value that we have just to satisfy [`ExtendedMaterial`]
    /// requirements.
    #[uniform(100)]
    dummy: f32,
}

impl MaterialExtension for ScreenSpaceTextureExtension {
    fn fragment_shader() -> ShaderRef {
        "shaders/screen_space_texture_material.wgsl".into()
    }
}

/// The action that will be performed when the user drags the mouse: either
/// moving the camera or moving the rigged model.
#[derive(Clone, Copy, PartialEq, Default)]
enum DragAction {
    /// Dragging will move the camera.
    #[default]
    MoveCamera,
    /// Dragging will move the animated fox.
    MoveFox,
}

/// The settings that the user has currently chosen.
///
/// Currently, this just consists of the [`DragAction`].
#[derive(Resource, Default)]
struct AppStatus {
    /// The action that will be performed when the user drags the mouse: either
    /// moving the camera or moving the rigged model.
    drag_action: DragAction,
}

/// A marker component for the help text at the top of the screen.
#[derive(Clone, Copy, Component)]
struct HelpText;

/// The coordinates that the camera looks at.
const CAMERA_TARGET: Vec3 = vec3(-25.0, 20.0, 0.0);
/// The camera stays this distance in meters from the camera target.
const CAMERA_ORBIT_DISTANCE: f32 = 500.0;
/// The speed at which the user can move the camera vertically, in radians per
/// mouse input unit.
const CAMERA_PITCH_SPEED: f32 = 0.003;
/// The speed at which the user can move the camera horizontally, in radians per
/// mouse input unit.
const CAMERA_YAW_SPEED: f32 = 0.004;
// Limiting pitch stops some unexpected rotation past 90° up or down.
const CAMERA_PITCH_LIMIT: f32 = FRAC_PI_2 - 0.01;

/// The angle that the mirror faces.
///
/// The mirror is rotated across the X axis in this many radians.
const MIRROR_ROTATION_ANGLE: f32 = -FRAC_PI_2;
const MIRROR_POSITION: Vec3 = vec3(-25.0, 75.0, 0.0);

/// The path to the animated fox model.
static FOX_ASSET_PATH: &str = "models/animated/Fox.glb";

/// The app entry point.
fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Bevy Mirror Example".into(),
                ..default()
            }),
            ..default()
        }))
        .add_plugins(MaterialPlugin::<
            ExtendedMaterial<StandardMaterial, ScreenSpaceTextureExtension>,
        >::default())
        .init_resource::<AppStatus>()
        .add_message::<WidgetClickEvent<DragAction>>()
        .add_systems(Startup, setup)
        .add_systems(Update, handle_window_resize_messages)
        .add_systems(Update, (move_camera_on_mouse_down, move_fox_on_mouse_down))
        .add_systems(Update, widgets::handle_ui_interactions::<DragAction>)
        .add_systems(
            Update,
            (handle_mouse_action_change, update_radio_buttons)
                .after(widgets::handle_ui_interactions::<DragAction>),
        )
        .add_systems(
            Update,
            update_mirror_camera_on_main_camera_transform_change.after(move_camera_on_mouse_down),
        )
        .add_systems(Update, play_fox_animation)
        .add_systems(Update, update_help_text)
        .run();
}

/// A startup system that spawns the scene and sets up the mirror render target.
fn setup(
    mut commands: Commands,
    windows_query: Query<&Window>,
    asset_server: Res<AssetServer>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut standard_materials: ResMut<Assets<StandardMaterial>>,
    mut screen_space_texture_materials: ResMut<
        Assets<ExtendedMaterial<StandardMaterial, ScreenSpaceTextureExtension>>,
    >,
    mut images: ResMut<Assets<Image>>,
    app_status: Res<AppStatus>,
) {
    // Spawn the main camera.
    let camera_projection = PerspectiveProjection::default();
    let camera_transform = spawn_main_camera(&mut commands, &camera_projection);

    // Spawn the light.
    spawn_light(&mut commands);

    // Spawn the objects reflected in the mirror.
    spawn_ground_plane(&mut commands, &mut meshes, &mut standard_materials);
    spawn_fox(&mut commands, &asset_server);

    // Spawn the mirror and associated camera.
    let mirror_render_target_image =
        create_mirror_texture_resource(&mut commands, &windows_query, &mut images);
    let mirror_transform = spawn_mirror(
        &mut commands,
        &mut meshes,
        &mut screen_space_texture_materials,
        mirror_render_target_image.clone(),
    );
    spawn_mirror_camera(
        &mut commands,
        &camera_transform,
        &camera_projection,
        &mirror_transform,
        mirror_render_target_image,
    );

    // Spawn the UI.
    spawn_buttons(&mut commands);
    spawn_help_text(&mut commands, &app_status);
}

/// Spawns the main camera (not the mirror camera).
fn spawn_main_camera(
    commands: &mut Commands,
    camera_projection: &PerspectiveProjection,
) -> Transform {
    let camera_transform = Transform::from_translation(
        vec3(-2.0, 1.0, -2.0).normalize_or_zero() * CAMERA_ORBIT_DISTANCE,
    )
    .looking_at(CAMERA_TARGET, Vec3::Y);

    commands.spawn((
        Camera3d::default(),
        camera_transform,
        Projection::Perspective(camera_projection.clone()),
    ));

    camera_transform
}

/// Spawns a directional light to illuminate the scene.
fn spawn_light(commands: &mut Commands) {
    commands.spawn((
        DirectionalLight {
            illuminance: 5000.0,
            ..default()
        },
        Transform::from_xyz(-85.0, 16.0, -200.0).looking_at(vec3(-50.0, 0.0, 100.0), Vec3::Y),
    ));
}

/// Spawns the circular ground plane object.
fn spawn_ground_plane(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    standard_materials: &mut Assets<StandardMaterial>,
) {
    commands.spawn((
        Mesh3d(meshes.add(Circle::new(200.0))),
        MeshMaterial3d(standard_materials.add(Color::from(GREEN))),
        Transform::from_rotation(Quat::from_rotation_x(-FRAC_PI_2))
            .with_translation(vec3(-25.0, 0.0, 0.0)),
    ));
}

/// Creates the initial image that the mirror camera will render the mirror
/// world to.
fn create_mirror_texture_resource(
    commands: &mut Commands,
    windows_query: &Query<&Window>,
    images: &mut Assets<Image>,
) -> Handle<Image> {
    let window = windows_query.iter().next().expect("No window found");
    let window_size = uvec2(window.physical_width(), window.physical_height());
    let image = create_mirror_texture_image(images, window_size);
    commands.insert_resource(MirrorImage(image.clone()));
    image
}

/// Spawns the camera that renders the mirror world.
fn spawn_mirror_camera(
    commands: &mut Commands,
    camera_transform: &Transform,
    camera_projection: &PerspectiveProjection,
    mirror_transform: &Transform,
    mirror_render_target: Handle<Image>,
) {
    let (mirror_camera_transform, mirror_camera_projection) =
        calculate_mirror_camera_transform_and_projection(
            camera_transform,
            camera_projection,
            mirror_transform,
        );

    commands.spawn((
        Camera3d::default(),
        Camera {
            order: -1,
            // Reflecting the model across the mirror will flip the winding of
            // all the polygons. Therefore, in order to properly backface cull,
            // we need to turn on `invert_culling`.
            invert_culling: true,
            ..default()
        },
        RenderTarget::Image(mirror_render_target.clone().into()),
        mirror_camera_transform,
        Projection::Perspective(mirror_camera_projection),
        MirrorCamera,
    ));
}

/// Spawns the animated fox.
///
/// Note that this doesn't play the animation; that's handled in
/// [`play_fox_animation`].
fn spawn_fox(commands: &mut Commands, asset_server: &AssetServer) {
    commands.spawn((
        SceneRoot(asset_server.load(GltfAssetLabel::Scene(0).from_asset(FOX_ASSET_PATH))),
        Transform::from_xyz(-50.0, 0.0, -100.0),
    ));
}

/// Spawns the mirror plane mesh and returns its transform.
fn spawn_mirror(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    screen_space_texture_materials: &mut Assets<
        ExtendedMaterial<StandardMaterial, ScreenSpaceTextureExtension>,
    >,
    mirror_render_target: Handle<Image>,
) -> Transform {
    let mirror_transform = Transform::from_scale(vec3(300.0, 1.0, 150.0))
        .with_rotation(Quat::from_rotation_x(MIRROR_ROTATION_ANGLE))
        .with_translation(MIRROR_POSITION);

    commands.spawn((
        Mesh3d(meshes.add(Plane3d::default().mesh().size(1.0, 1.0))),
        MeshMaterial3d(screen_space_texture_materials.add(ExtendedMaterial {
            base: StandardMaterial {
                base_color: Color::BLACK,
                emissive: Color::WHITE.into(),
                emissive_texture: Some(mirror_render_target),
                perceptual_roughness: 0.0,
                metallic: 1.0,
                ..default()
            },
            extension: ScreenSpaceTextureExtension { dummy: 0.0 },
        })),
        mirror_transform,
        Mirror,
    ));

    mirror_transform
}

/// Spawns the buttons at the bottom of the screen.
fn spawn_buttons(commands: &mut Commands) {
    // Spawn the radio buttons that allow the user to select an object to
    // control.
    commands.spawn((
        widgets::main_ui_node(),
        children![widgets::option_buttons(
            "Drag Action",
            &[
                (DragAction::MoveCamera, "Move Camera"),
                (DragAction::MoveFox, "Move Fox"),
            ],
        )],
    ));
}

/// Given the transform and projection of the main camera, returns an
/// appropriate transform and projection for the mirror camera.
fn calculate_mirror_camera_transform_and_projection(
    main_camera_transform: &Transform,
    main_camera_projection: &PerspectiveProjection,
    mirror_transform: &Transform,
) -> (Transform, PerspectiveProjection) {
    // Calculate the reflection matrix (a.k.a. Householder matrix) that will
    // reflect the scene across the mirror plane.
    //
    // Note that you must calculate this in *matrix* form and only *afterward*
    // convert to a `Transform` instead of composing `Transform`s. This is
    // because the reflection matrix has non-uniform scale, and composing
    // transforms can't always handle composition of matrices with non-uniform
    // scales.
    let mirror_camera_transform = Transform::from_matrix(
        Mat4::from_mat3a(reflection_matrix(Vec3::NEG_Z)) * main_camera_transform.to_matrix(),
    );

    // Compute the distance from the camera to the mirror plane. This will be
    // used to calculate the distance to the near clip plane for the mirror
    // world.
    let distance_from_camera_to_mirror = InfinitePlane3d::new(mirror_transform.rotation * Vec3::Y)
        .signed_distance(
            Isometry3d::IDENTITY,
            mirror_transform.translation - main_camera_transform.translation,
        );

    // Compute the normal of the mirror plane in view space.
    let view_from_world = main_camera_transform.compute_affine().matrix3.inverse();
    let mirror_projection_plane_normal =
        (view_from_world * (mirror_transform.rotation * Vec3::NEG_Y)).normalize();

    // Compute the final projection. It should match the main camera projection,
    // except that `near` and `near_normal` should be set to the updated near
    // plane and near normal plane as above.
    let mirror_camera_projection = PerspectiveProjection {
        near_clip_plane: mirror_projection_plane_normal.extend(distance_from_camera_to_mirror),
        ..*main_camera_projection
    };

    (mirror_camera_transform, mirror_camera_projection)
}

/// A system that resizes the render target image when the user resizes the window.
///
/// Since the image that stores the rendered mirror world has the same physical
/// size as the window, we need to reallocate it and reattach it to the mirror
/// material whenever the window size changes.
fn handle_window_resize_messages(
    windows_query: Query<&Window>,
    mut mirror_cameras_query: Query<&mut RenderTarget, With<MirrorCamera>>,
    mut images: ResMut<Assets<Image>>,
    mut mirror_image: ResMut<MirrorImage>,
    mut screen_space_texture_materials: ResMut<
        Assets<ExtendedMaterial<StandardMaterial, ScreenSpaceTextureExtension>>,
    >,
    mut resize_messages: MessageReader<WindowResized>,
) {
    // We run at most once, regardless of the number of window resize messages
    // there were this frame.
    let Some(resize_message) = resize_messages.read().next() else {
        return;
    };
    let Ok(window) = windows_query.get(resize_message.window) else {
        return;
    };

    let window_size = uvec2(window.physical_width(), window.physical_height());
    let image = create_mirror_texture_image(&mut images, window_size);
    images.remove(mirror_image.0.id());

    mirror_image.0 = image.clone();

    for mut target in mirror_cameras_query.iter_mut() {
        *target = image.clone().into();
    }

    for (_, material) in screen_space_texture_materials.iter_mut() {
        material.base.emissive_texture = Some(image.clone());
    }
}

/// Creates the image that will be used to store the reflected scene.
fn create_mirror_texture_image(images: &mut Assets<Image>, window_size: UVec2) -> Handle<Image> {
    let mirror_image_extent = Extent3d {
        width: window_size.x,
        height: window_size.y,
        depth_or_array_layers: 1,
    };

    let mut image = Image::new_uninit(
        mirror_image_extent,
        TextureDimension::D2,
        TextureFormat::Bgra8UnormSrgb,
        RenderAssetUsages::MAIN_WORLD | RenderAssetUsages::RENDER_WORLD,
        RenderAssetTransferPriority::default(),
    );
    image.texture_descriptor.usage |=
        TextureUsages::TEXTURE_BINDING | TextureUsages::COPY_DST | TextureUsages::RENDER_ATTACHMENT;

    images.add(image)
}

// Moves the fox when the user moves the mouse with the left button down.
fn move_fox_on_mouse_down(
    mut scene_roots_query: Query<&mut Transform, With<SceneRoot>>,
    windows_query: Query<&Window, With<PrimaryWindow>>,
    cameras_query: Query<(&Camera, &GlobalTransform)>,
    interactions_query: Query<&Interaction, With<RadioButton>>,
    buttons: Res<ButtonInput<MouseButton>>,
    app_status: Res<AppStatus>,
) {
    // Only process the mouse motion if the left mouse button is pressed, the
    // mouse action is set to move the fox, and the pointer isn't over a UI
    // widget.
    if app_status.drag_action != DragAction::MoveFox
        || !buttons.pressed(MouseButton::Left)
        || interactions_query
            .iter()
            .any(|interaction| *interaction != Interaction::None)
    {
        return;
    }

    // Find out where the user clicked the mouse.
    let Some(mouse_position) = windows_query
        .iter()
        .next()
        .and_then(Window::cursor_position)
    else {
        return;
    };

    // Grab the camera.
    let Some((camera, camera_transform)) = cameras_query.iter().next() else {
        return;
    };

    // Figure out where the user clicked on the plane.
    let Ok(ray) = camera.viewport_to_world(camera_transform, mouse_position) else {
        return;
    };
    let Some(ray_distance) = ray.intersect_plane(Vec3::ZERO, InfinitePlane3d::new(Vec3::Y)) else {
        return;
    };
    let plane_intersection = ray.origin + ray.direction.normalize() * ray_distance;

    // Move the fox.
    for mut transform in scene_roots_query.iter_mut() {
        transform.translation = transform.translation.with_xz(plane_intersection.xz());
    }
}

/// A system that changes the drag action when the user clicks on one of the
/// radio buttons.
fn handle_mouse_action_change(
    mut app_status: ResMut<AppStatus>,
    mut messages: MessageReader<WidgetClickEvent<DragAction>>,
) {
    for message in messages.read() {
        app_status.drag_action = **message;
    }
}

/// A system that updates the radio buttons at the bottom of the screen to
/// reflect the current drag action.
fn update_radio_buttons(
    mut widgets_query: Query<(
        Entity,
        Option<&mut BackgroundColor>,
        Has<Text>,
        &WidgetClickSender<DragAction>,
    )>,
    app_status: Res<AppStatus>,
    mut text_ui_writer: TextUiWriter,
) {
    for (entity, maybe_bg_color, has_text, sender) in &mut widgets_query {
        let selected = app_status.drag_action == **sender;
        if let Some(mut bg_color) = maybe_bg_color {
            widgets::update_ui_radio_button(&mut bg_color, selected);
        }
        if has_text {
            widgets::update_ui_radio_button_text(entity, &mut text_ui_writer, selected);
        }
    }
}

/// A system that processes user mouse actions that move the camera.
///
/// This is mostly copied from `examples/camera/camera_orbit.rs`.
fn move_camera_on_mouse_down(
    mut main_cameras_query: Query<&mut Transform, (With<Camera>, Without<MirrorCamera>)>,
    interactions_query: Query<&Interaction, With<RadioButton>>,
    mouse_buttons: Res<ButtonInput<MouseButton>>,
    mouse_motion: Res<AccumulatedMouseMotion>,
    app_status: Res<AppStatus>,
) {
    // Only process the mouse motion if the left mouse button is pressed, the
    // mouse action is set to move the fox, and the pointer isn't over a UI
    // widget.
    if app_status.drag_action != DragAction::MoveCamera
        || !mouse_buttons.pressed(MouseButton::Left)
        || interactions_query
            .iter()
            .any(|interaction| *interaction != Interaction::None)
    {
        return;
    }

    let delta = mouse_motion.delta;

    // Mouse motion is one of the few inputs that should not be multiplied by delta time,
    // as we are already receiving the full movement since the last frame was rendered. Multiplying
    // by delta time here would make the movement slower that it should be.
    let delta_pitch = delta.y * CAMERA_PITCH_SPEED;
    let delta_yaw = delta.x * CAMERA_YAW_SPEED;

    for mut main_camera_transform in &mut main_cameras_query {
        // Obtain the existing pitch and yaw values from the transform.
        let (yaw, pitch, _) = main_camera_transform.rotation.to_euler(EulerRot::YXZ);

        // Establish the new yaw and pitch, preventing the pitch value from exceeding our limits.
        let pitch = (pitch + delta_pitch).clamp(-CAMERA_PITCH_LIMIT, CAMERA_PITCH_LIMIT);
        let yaw = yaw + delta_yaw;
        main_camera_transform.rotation = Quat::from_euler(EulerRot::YXZ, yaw, pitch, 0.0);

        // Adjust the translation to maintain the correct orientation toward the orbit target.
        // In our example it's a static target, but this could easily be customized.
        let target = Vec3::ZERO;
        main_camera_transform.translation =
            target - main_camera_transform.forward() * CAMERA_ORBIT_DISTANCE;
    }
}

/// Updates the position, rotation, and projection of the mirror camera when the
/// main camera is moved.
///
/// When the main camera is moved, the mirror camera must be moved to match it.
/// The *projection* on the mirror camera must also be altered, because the
/// projection takes the view-space rotation of and distance to the mirror into
/// account.
fn update_mirror_camera_on_main_camera_transform_change(
    main_cameras_query: Query<
        (&Transform, &Projection),
        (Changed<Transform>, With<Camera>, Without<MirrorCamera>),
    >,
    mut mirror_cameras_query: Query<
        (&mut Transform, &mut Projection),
        (With<Camera>, With<MirrorCamera>, Without<Mirror>),
    >,
    mirrors_query: Query<&Transform, (Without<MirrorCamera>, With<Mirror>)>,
) {
    let Some((main_camera_transform, Projection::Perspective(main_camera_projection))) =
        main_cameras_query.iter().next()
    else {
        return;
    };

    let Some(mirror_transform) = mirrors_query.iter().next() else {
        return;
    };

    // Here we need the transforms of both the camera and the mirror in order to
    // properly calculate the new projection.
    let (new_mirror_camera_transform, new_mirror_camera_projection) =
        calculate_mirror_camera_transform_and_projection(
            main_camera_transform,
            main_camera_projection,
            mirror_transform,
        );

    for (mut mirror_camera_transform, mut mirror_camera_projection) in &mut mirror_cameras_query {
        *mirror_camera_transform = new_mirror_camera_transform;
        *mirror_camera_projection = Projection::Perspective(new_mirror_camera_projection.clone());
    }
}

/// Plays the initial animation on the fox model.
fn play_fox_animation(
    mut commands: Commands,
    mut animation_players_query: Query<
        (Entity, &mut AnimationPlayer),
        Without<AnimationGraphHandle>,
    >,
    asset_server: Res<AssetServer>,
    mut animation_graphs: ResMut<Assets<AnimationGraph>>,
) {
    // Only pick up animation players that don't already have an animation graph
    // handle.
    // This ensures that we only start playing the animation once.
    if animation_players_query.is_empty() {
        return;
    }

    let fox_animation = asset_server.load(GltfAssetLabel::Animation(0).from_asset(FOX_ASSET_PATH));
    let (fox_animation_graph, fox_animation_node) =
        AnimationGraph::from_clip(fox_animation.clone());
    let fox_animation_graph = animation_graphs.add(fox_animation_graph);

    for (entity, mut animation_player) in animation_players_query.iter_mut() {
        commands
            .entity(entity)
            .insert(AnimationGraphHandle(fox_animation_graph.clone()));
        animation_player.play(fox_animation_node).repeat();
    }
}

/// Spawns the help text at the top of the screen.
fn spawn_help_text(commands: &mut Commands, app_status: &AppStatus) {
    commands.spawn((
        Text::new(create_help_string(app_status)),
        Node {
            position_type: PositionType::Absolute,
            top: px(12),
            left: px(12),
            ..default()
        },
        HelpText,
    ));
}

/// Creates the help string at the top left of the screen.
fn create_help_string(app_status: &AppStatus) -> String {
    format!(
        "Click and drag to move the {}",
        match app_status.drag_action {
            DragAction::MoveCamera => "camera",
            DragAction::MoveFox => "fox",
        }
    )
}

/// Updates the help text in the top left of the screen to reflect the current
/// drag mode.
fn update_help_text(mut help_text: Query<&mut Text, With<HelpText>>, app_status: Res<AppStatus>) {
    for mut text in &mut help_text {
        text.0 = create_help_string(&app_status);
    }
}
