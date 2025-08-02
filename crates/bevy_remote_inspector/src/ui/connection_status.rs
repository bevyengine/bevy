//! Connection status indicator UI

use bevy::prelude::*;
use crate::http_client::HttpRemoteConfig;

/// Component for connection status indicator
#[derive(Component)]
pub struct ConnectionStatus {
    pub is_connected: bool,
    pub last_error: Option<String>,
}

/// System to update connection status display
pub fn update_connection_status(
    mut status_query: Query<(&mut Text, &mut TextColor, &ConnectionStatus)>,
    config: Res<HttpRemoteConfig>,
) {
    for (mut text, mut color, status) in status_query.iter_mut() {
        if status.is_connected {
            text.0 = format!("Connected to {}:{}", config.host, config.port);
            color.0 = Color::srgb(0.2, 0.8, 0.2); // Green
        } else {
            let error_msg = status.last_error.as_deref().unwrap_or("Disconnected");
            text.0 = format!("Disconnected: {}", error_msg);
            color.0 = Color::srgb(0.8, 0.2, 0.2); // Red
        }
    }
}

/// Spawn connection status indicator
pub fn spawn_connection_status(commands: &mut Commands, parent: Entity) -> Entity {
    let status = commands.spawn((
        ConnectionStatus {
            is_connected: false,
            last_error: None,
        },
        Text::new("Connecting..."),
        TextFont {
            font_size: 12.0,
            ..default()
        },
        TextColor(Color::srgb(0.8, 0.8, 0.2)), // Yellow
        Node {
            position_type: PositionType::Absolute,
            bottom: Val::Px(8.0),
            right: Val::Px(8.0),
            padding: UiRect::all(Val::Px(4.0)),
            ..default()
        },
        BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.8)),
    )).id();
    
    commands.entity(parent).add_child(status);
    status
}