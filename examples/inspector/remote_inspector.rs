//! Shows the `bevy_inspector` entity tree and details panels inspecting a separate running app
//! over the Bevy Remote Protocol.
//!
//! Run the app to inspect in one terminal:
//! ```bash
//! cargo run --example server --features="bevy_remote"
//! ```
//!
//! Add the `bevy_debug_stepping` feature to the server to enable stepping through systems using controls on the inspector.
//!
//! Then run this example in another terminal:
//! ```bash
//! cargo run --example remote_inspector --features="bevy_inspector,bevy_remote_client"
//! ```
//!
//! The address defaults to `127.0.0.1:15702` and can be overridden with the `BRP_HOST` and
//! `BRP_PORT` environment variables, or changed in the connection bar at the top.

use bevy::{
    feathers::{
        controls::{FeathersPane, FeathersSplitPane, FeathersSplitPaneHandle},
        dark_theme::create_dark_theme,
        theme::UiTheme,
        FeathersPlugins,
    },
    inspector::{
        connection_panel::connection_panel,
        details_panel::details_panel,
        entity_tree::{entity_tree_panel, InspectorUi},
        remote::{stepping::stepping_controls, RemoteSource},
        InspectorPlugin, InspectorSource,
    },
    prelude::*,
    remote::http::DEFAULT_PORT,
    ui_widgets::split_pane_self_update,
};

fn main() {
    App::new()
        .add_plugins((DefaultPlugins, FeathersPlugins, InspectorPlugin))
        .insert_resource(UiTheme(create_dark_theme()))
        .insert_resource(InspectorSource::Remote(remote_source()))
        .add_systems(Startup, (camera.spawn(), inspector_ui.spawn()))
        .run();
}

fn remote_source() -> RemoteSource {
    let host = std::env::var("BRP_HOST").unwrap_or_else(|_| "127.0.0.1".to_string());
    let port = std::env::var("BRP_PORT")
        .ok()
        .and_then(|port| port.parse().ok())
        .unwrap_or(DEFAULT_PORT);
    RemoteSource::new(host, port)
}

fn camera() -> impl Scene {
    bsn! {
        Name("Camera")
        Camera2d
    }
}

fn inspector_ui() -> impl Scene {
    bsn! {
        InspectorUi
        Name::new("Inspector")
        Node {
            position_type: PositionType::Absolute,
            width: percent(100),
            height: percent(100),
            padding: px(12),
            flex_direction: FlexDirection::Column,
            align_items: AlignItems::FlexStart,
            row_gap: px(8),
        }
        Children [
            Node {
                flex_direction: FlexDirection::Row,
                align_items: AlignItems::Center,
                flex_wrap: FlexWrap::Wrap,
                column_gap: px(16),
                row_gap: px(8),
            }
            Children [
                @connection_panel()
                --
                @stepping_controls()
            ]
            --
            @FeathersSplitPane
            Node {
                width: px(720),
                max_height: percent(100),
            }
            on(split_pane_self_update)
            Children [
                @FeathersPane { @size: 1.0, @min_size: 220.0 }
                Children [
                    @entity_tree_panel()
                    Node {
                        width: Val::Auto,
                    }
                ]
                --
                @FeathersSplitPaneHandle
                --
                @FeathersPane { @size: 2.0, @min_size: 240.0 }
                Children [
                    @details_panel()
                    Node {
                        width: Val::Auto,
                    }
                ]
            ]
        ]
    }
}
