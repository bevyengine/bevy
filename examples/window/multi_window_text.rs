//! Renders text to multiple windows with different scale factors using both Text and Text2d.
use bevy::{
    camera::{visibility::RenderLayers, RenderTarget},
    color::palettes::css::{LIGHT_CYAN, YELLOW},
    prelude::*,
    sprite::Text2dShadow,
    window::{WindowRef, WindowResolution},
};

fn main() {
    App::new()
        // By default, a primary window is spawned by `WindowPlugin`, contained in `DefaultPlugins`.
        // The primary window is given the `PrimaryWindow` marker component.
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Primary window".to_owned(),
                // Override the primary window's scale factor and use `1.` (no scaling).
                resolution: WindowResolution::default().with_scale_factor_override(1.),
                ..default()
            }),
            ..Default::default()
        }))
        .add_systems(Startup, setup_scene)
        .run();
}

fn setup_scene(mut commands: Commands) {
    // The first camera; no render target is specified, its render target will be set to the primary window automatically.
    // This camera has no `RenderLayers` component, so it only renders entities belonging to render layer `0`.
    commands.spawn(Camera2d);

    // Spawn a second window
    let secondary_window = commands
        .spawn(Window {
            title: "Secondary Window".to_owned(),
            // Override the secondary window's scale factor and set it to double that of the primary window.
            // This means the second window's text will use glyphs drawn at twice the resolution of the primary window's text,
            // and they will be twice as big on screen.
            resolution: WindowResolution::default().with_scale_factor_override(2.),
            ..default()
        })
        .id();

    // Spawn a second camera
    let secondary_window_camera = commands
        .spawn((
            Camera2d,
            // This camera will only render entities belonging to render layer `1`.
            RenderLayers::layer(1),
            Camera {
                // Without an explicit render target, this camera would also target the primary window.
                target: RenderTarget::Window(WindowRef::Entity(secondary_window)),
                ..default()
            },
        ))
        .id();

    let node = Node {
        position_type: PositionType::Absolute,
        top: Val::Px(12.0),
        left: Val::Px(12.0),
        ..default()
    };

    let text_font = TextFont::from_font_size(30.);

    // UI nodes can only be rendered by one camera at a time and ignore `RenderLayers`.
    // This root UI node has no `UiTargetCamera` so `bevy_ui` will try to find a
    // camera with the `IsDefaultUiCamera` marker component. When that fails (neither
    // camera spawned here has an `IsDefaultUiCamera`), it queries for the
    // first camera targeting the primary window and uses that.
    commands.spawn(node.clone()).with_child((
        Text::new("UI Text Primary Window"),
        text_font.clone(),
        TextShadow::default(),
    ));

    commands
        .spawn((node, UiTargetCamera(secondary_window_camera)))
        .with_child((
            Text::new("UI Text Secondary Window"),
            text_font.clone(),
            TextShadow::default(),
        ));

    // `Text2d` belonging to render layer `0`.
    commands.spawn((
        Text2d::new("Text2d Primary Window"),
        TextColor(YELLOW.into()),
        text_font.clone(),
        Text2dShadow::default(),
    ));

    // `Text2d` belonging to render layer `1`.
    commands.spawn((
        Text2d::new("Text2d Secondary Window"),
        TextColor(YELLOW.into()),
        text_font.clone(),
        Text2dShadow::default(),
        RenderLayers::layer(1),
    ));

    // This `Text2d` entity belongs to both render layers `0` and `1`, so it will be rendered by both
    // cameras. A single text layout is generated per `Text2d` entity, targeting a specific scale
    // factor. Since the two camera's render targets have different scale factors, the text layout
    // will be generated using the higher scale factor (the secondary window's), and then downscaled when it is
    // drawn by the camera targeting the primary window.
    commands.spawn((
        Text2d::new("Text2d Both Windows"),
        TextColor(LIGHT_CYAN.into()),
        text_font,
        Text2dShadow::default(),
        RenderLayers::from_layers(&[0, 1]),
        Transform::from_xyz(0., -50., 0.),
    ));
}
