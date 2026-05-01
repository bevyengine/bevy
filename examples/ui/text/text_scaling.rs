//! This example shows off how to rasterize text at a higher resolution, so that
//! it stays sharp even when zoomed in.

use bevy::{
    feathers::{
        dark_theme::create_dark_theme,
        theme::{ThemeBackgroundColor, UiTheme},
        tokens, FeathersPlugins,
    },
    input::{keyboard::KeyboardInput, mouse::MouseWheel},
    prelude::*,
    ui::UiRasterScale,
    window::WindowResolution,
};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                resolution: WindowResolution::default(),
                ..default()
            }),
            ..default()
        }))
        .add_plugins(FeathersPlugins)
        .insert_resource(UiTheme(create_dark_theme()))
        .add_systems(Startup, scene.spawn())
        .add_systems(Update, zoom)
        .run();
}

fn scene() -> impl SceneList {
    bsn_list![Camera2d, root(), controls()]
}

#[derive(Component, Clone, Default)]
struct Root;

fn root() -> impl Scene {
    bsn! {
        Root
        Node {
            width: percent(100),
            height: percent(100),
            display: Display::Flex,
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            flex_direction: FlexDirection::Column
        }
        // Goal: Render text of logical pixel size 16 that stays sharp up to 4x zoom.
        Children [
            // For comparison non-sharp text.
            Node
            Text::new("Some Text")
            TextFont {
                font_size: FontSize::Px(16.),
            }
            ThemeBackgroundColor(tokens::PANE_BODY_BG),

            // Simple solution:
            // Scaling by UiRasterScale, which scales up the glyph rasterization resolution
            // and could be dynamically changed based on zoom level. When zoomed out very far
            // less raster resolution can look better.
            Node
            Text::new("Some Text")
            TextFont {
                font_size: FontSize::Px(16.),
            }
            UiRasterScale(4.)
            ThemeBackgroundColor(tokens::PANE_BODY_BG),

            // More complex, less flexible solution:
            // Increase font size by 4x and scale the node down to 1/4 size.
            // This works, but you need to multiply your font sizes everywhere by 4x
            // and there is extra complexity, because the 1/4 scale needs to happen somewhere.
            TextFont {
                font_size: {FontSize::Px(16. * 4.)},
            }
            template_value(UiTransform::from_scale(Vec2::splat(1./4.)))
            Text::new("Some Text")
            Node {
                // The node size is calculated before the scale is applied,
                // so we move it down to be next to the other text.
                top: px(-30.)
            }
            ThemeBackgroundColor(tokens::PANE_BODY_BG),
        ]
    }
}

fn zoom(
    mut root_transform: Single<&mut UiTransform, With<Root>>,
    mut scroll: MessageReader<MouseWheel>,
    mut keys: MessageReader<KeyboardInput>,
) {
    // Use screen-space cursor (top-left origin) directly for UI calculations.
    for event in scroll.read() {
        root_transform.scale *= 1.0 + event.y * 0.05;
    }
    for key in keys.read() {
        root_transform.scale = Vec2::splat(match key.key_code {
            KeyCode::Digit1 => 1.0,
            KeyCode::Digit2 => 2.0,
            KeyCode::Digit3 => 4.0,
            KeyCode::Digit4 => 1. / 1.5,
            KeyCode::Digit5 => 1. / 2.0,
            KeyCode::Digit6 => 1. / 2.5,
            _ => continue,
        });
    }
}

fn controls() -> impl Scene {
    bsn! {
        Node {
            position_type: PositionType::Absolute,
            top: px(16.),
            left: px(16.),
        }
        Text::new("Scroll to zoom or use 1, 2, 3, 4, 5 for fixed zoom levels")
    }
}
