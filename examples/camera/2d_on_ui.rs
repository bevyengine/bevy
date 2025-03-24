//! This example shows how to draw 2d objects on top of bevy ui, using two cameras and their order.

use bevy::{color::palettes::tailwind, prelude::*, render::view::RenderLayers};

fn main() {
    let mut app = App::new();
    app.add_plugins(DefaultPlugins).add_systems(Startup, setup);

    // example using gizmos in a specific render layer.
    app.insert_gizmo_config(
        DefaultGizmoConfigGroup,
        GizmoConfig {
            render_layers: RenderLayers::layer(1),
            ..default()
        },
    )
    .add_systems(Update, draw_gizmo);

    app.run();
}

fn setup(mut commands: Commands) {
    // the default camera. we explicitly set that this is the Ui render camera. you can also use `UiTargetCamera` on each entity.
    commands.spawn((Camera2d, IsDefaultUiCamera));

    // the second camera, with a higher order, will be drawn after the first camera. we will render to this camera to draw on top of the UI.
    commands.spawn((
        Camera2d,
        Camera {
            order: 1,
            // dont draw anything in the background, to see the previous cameras.
            clear_color: ClearColorConfig::None,
            ..default()
        },
        // this camera will only render entity which are on the same render layer.
        RenderLayers::layer(1),
    ));

    commands.spawn((
        // here we could also use a `UiTargetCamera` component instead of the general `IsDefaultUiCamera`
        Node {
            width: Val::Percent(100.),
            height: Val::Percent(100.),
            ..default()
        },
        ImageNode::solid_color(tailwind::ROSE_400.into()),
    ));

    // this 2d object, will be rendered on the second camera, on top of the default camera where the ui is rendered.
    commands.spawn((
        Text2d("This text a 2d object, in front of a UI Node background.".to_string()),
        RenderLayers::layer(1),
    ));
}

fn draw_gizmo(mut gizmos: Gizmos) {
    gizmos.rect_2d(Isometry2d::IDENTITY, Vec2::new(700., 100.), Color::WHITE);
}
