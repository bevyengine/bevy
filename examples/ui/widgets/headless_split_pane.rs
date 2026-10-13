//! Demonstrates the behavior-only split pane widget in `bevy_ui_widgets`.

use bevy::{
    picking::cursor::{CursorIconPlugin, EntityCursor},
    prelude::*,
    ui_widgets::{split_pane_self_update, ControlOrientation, Pane, SplitPane, SplitPaneHandle},
    window::SystemCursorIcon,
};

fn main() {
    App::new()
        .add_plugins((DefaultPlugins, CursorIconPlugin))
        .add_systems(Startup, showcase.spawn())
        .run();
}

fn showcase() -> impl SceneList {
    bsn_list! {
        Camera2d
        --
        Node {
            width: percent(100),
            height: percent(100),
        }
        SplitPane
        on(split_pane_self_update)
        Children [
            @pane(1.0, Color::srgb(0.16, 0.20, 0.28))
            --
            @handle(ControlOrientation::Horizontal)
            --
            Pane {
                size: 2.0,
                min_size: 120.0,
            }
            Node
            SplitPane {
                orientation: ControlOrientation::Vertical,
            }
            on(split_pane_self_update)
            Children [
                @pane(3.0, Color::srgb(0.10, 0.11, 0.14))
                --
                @handle(ControlOrientation::Vertical)
                --
                @pane(1.0, Color::srgb(0.22, 0.17, 0.24))
            ]
            --
            @handle(ControlOrientation::Horizontal)
            --
            @pane(1.0, Color::srgb(0.15, 0.24, 0.19))
        ]
    }
}

fn pane(size: f32, color: Color) -> impl Scene {
    bsn! {
        Pane {
            size,
            min_size: 60.0,
        }
        Node
        BackgroundColor(color)
    }
}

fn handle(orientation: ControlOrientation) -> impl Scene {
    let (width, height, icon) = match orientation {
        ControlOrientation::Horizontal => (px(6), Val::Auto, SystemCursorIcon::ColResize),
        ControlOrientation::Vertical => (Val::Auto, px(6), SystemCursorIcon::RowResize),
    };
    bsn! {
        SplitPaneHandle
        Node {
            width,
            height,
        }
        BackgroundColor(Color::srgb(0.32, 0.34, 0.40))
        EntityCursor::System(icon)
    }
}
