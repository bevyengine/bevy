//! A bar that chooses the source the inspector reads from, and shows the state of the connection to
//! a remote app.

use alloc::{
    format,
    string::{String, ToString},
};

use bevy_ecs::{
    change_detection::{DetectChanges, DetectChangesMut},
    component::Component,
    hierarchy::Children,
    observer::On,
    query::{Added, With},
    reflect::ReflectComponent,
    system::{Query, Res, ResMut},
};
use bevy_feathers::{
    constants::{fonts, size},
    controls::{FeathersButton, FeathersTextInput, FeathersTextInputContainer},
    display::caption,
    font_styles::InheritableFont,
};
use bevy_log::warn;
use bevy_reflect::{prelude::ReflectDefault, Reflect};
use bevy_scene::{bsn, on, Scene};
use bevy_text::{EditableText, FontSourceTemplate, FontWeight, TextEdit};
use bevy_ui::{px, widget::Text, AlignItems, FlexDirection, Node};
use bevy_ui_widgets::Activate;

use crate::{
    entity_tree::InspectorUi,
    remote::{RemoteConnection, RemoteConnectionState, RemoteSource},
    InspectorSource,
};

/// The number of characters of a connection error shown in the status caption.
const ERROR_CHARS: usize = 80;

/// Marker for the caption showing the state of the connection.
#[derive(Component, Debug, Default, Clone, Copy, Reflect)]
#[reflect(Component, Debug, Default, Clone)]
pub struct ConnectionStatusLabel;

/// Marker for the text input holding the `host:port` address to connect to.
#[derive(Component, Debug, Default, Clone, Copy, Reflect)]
#[reflect(Component, Debug, Default, Clone)]
pub struct ConnectionAddressInput;

/// A bar with an address input, buttons switching between the local world and a remote app, and
/// the state of the connection.
pub fn connection_panel() -> impl Scene {
    bsn! {
        InspectorUi
        Node {
            flex_direction: FlexDirection::Row,
            align_items: AlignItems::Center,
            column_gap: px(8),
        }
        InheritableFont {
            font: FontSourceTemplate::Families(fonts::SANS),
            font_size: size::MEDIUM_FONT,
            weight: FontWeight::NORMAL,
        }
        Children [
            @FeathersTextInputContainer
            Node {
                width: px(180),
            }
            Children [
                ConnectionAddressInput
                @FeathersTextInput
            ]
            --
            @FeathersButton {
                @caption: bsn! { @caption("Connect") },
            }
            on(connect_clicked)
            --
            @FeathersButton {
                @caption: bsn! { @caption("Local") },
            }
            on(local_clicked)
            --
            ConnectionStatusLabel
            @caption("Local")
        ]
    }
}

/// Fills a new address input with the current remote address, or the default one.
pub fn init_address_input(
    source: Option<Res<InspectorSource>>,
    mut inputs: Query<&mut EditableText, Added<ConnectionAddressInput>>,
) {
    let address = match source.as_deref() {
        Some(InspectorSource::Remote(remote)) => format_address(remote),
        _ => format_address(&RemoteSource::default()),
    };
    for mut input in &mut inputs {
        input.queue_edit(TextEdit::SelectAll);
        input.queue_edit(TextEdit::Insert(address.clone().into()));
    }
}

/// Observer switching the inspector to the remote app at the address in the address input.
pub fn connect_clicked(
    _activate: On<Activate>,
    inputs: Query<&EditableText, With<ConnectionAddressInput>>,
    mut source: ResMut<InspectorSource>,
) {
    let Some(address) = inputs.iter().next().map(|input| input.value().to_string()) else {
        return;
    };
    match parse_address(&address) {
        Some(remote) => {
            source.set_if_neq(InspectorSource::Remote(remote));
        }
        None => warn!("`{address}` is not a `host:port` address"),
    }
}

/// Observer switching the inspector back to the local world.
pub fn local_clicked(_activate: On<Activate>, mut source: ResMut<InspectorSource>) {
    source.set_if_neq(InspectorSource::Local);
}

/// Keeps the status caption in sync with the source and the connection.
pub fn sync_connection_status(
    source: Option<Res<InspectorSource>>,
    connection: Res<RemoteConnection>,
    mut labels: Query<&mut Text, With<ConnectionStatusLabel>>,
) {
    let Some(source) = source else {
        return;
    };
    if !source.is_changed() && !connection.is_changed() {
        return;
    }
    let status = status_text(&source, &connection);
    for mut text in &mut labels {
        if text.0 != status {
            text.0 = status.clone();
        }
    }
}

fn status_text(source: &InspectorSource, connection: &RemoteConnection) -> String {
    let InspectorSource::Remote(remote) = source else {
        return "Local".to_string();
    };
    match &connection.state {
        RemoteConnectionState::Disconnected | RemoteConnectionState::Connecting => {
            format!("Connecting to {}", format_address(remote))
        }
        RemoteConnectionState::Connected {
            app_name,
            bevy_version,
            sub_app,
        } if sub_app.is_empty() => format!("Connected to {app_name} ({bevy_version})"),
        RemoteConnectionState::Connected {
            app_name,
            bevy_version,
            sub_app,
        } => format!("Connected to {app_name} ({bevy_version}), {sub_app}"),
        RemoteConnectionState::Failed(error) => {
            let mut shown: String = error.chars().take(ERROR_CHARS).collect();
            if shown.len() < error.len() {
                shown.push_str("...");
            }
            format!("Retrying {}: {shown}", format_address(remote))
        }
    }
}

fn format_address(source: &RemoteSource) -> String {
    if source.host.contains(':') {
        format!("[{}]:{}", source.host, source.port)
    } else {
        format!("{}:{}", source.host, source.port)
    }
}

/// Parses a `host:port` address, where an IPv6 host is written in brackets.
fn parse_address(address: &str) -> Option<RemoteSource> {
    let (host, port) = address.trim().rsplit_once(':')?;
    let host = host
        .strip_prefix('[')
        .and_then(|host| host.strip_suffix(']'))
        .unwrap_or(host);
    if host.is_empty() {
        return None;
    }
    Some(RemoteSource::new(host, port.parse().ok()?))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_addresses() {
        assert_eq!(
            parse_address(" 127.0.0.1:15702 "),
            Some(RemoteSource::localhost(15702))
        );
        assert_eq!(
            parse_address("[::1]:80"),
            Some(RemoteSource::new("::1", 80))
        );
        assert_eq!(parse_address("localhost"), None);
        assert_eq!(parse_address(":80"), None);
        assert_eq!(parse_address("host:99999"), None);
        assert_eq!(
            format_address(&RemoteSource::new("::1", 80)),
            "[::1]:80".to_string()
        );
    }

    #[test]
    fn describes_the_connection() {
        let remote = InspectorSource::Remote(RemoteSource::localhost(15702));
        let mut connection = RemoteConnection::default();
        assert_eq!(status_text(&InspectorSource::Local, &connection), "Local");
        assert_eq!(
            status_text(&remote, &connection),
            "Connecting to 127.0.0.1:15702"
        );
        connection.state = RemoteConnectionState::Connected {
            app_name: "demo".to_string(),
            bevy_version: "0.20.0-dev".to_string(),
            sub_app: String::new(),
        };
        assert_eq!(
            status_text(&remote, &connection),
            "Connected to demo (0.20.0-dev)"
        );
        connection.state = RemoteConnectionState::Connected {
            app_name: "demo".to_string(),
            bevy_version: "0.20.0-dev".to_string(),
            sub_app: "render".to_string(),
        };
        assert_eq!(
            status_text(&remote, &connection),
            "Connected to demo (0.20.0-dev), render"
        );
        connection.state = RemoteConnectionState::Failed("refused".to_string());
        assert_eq!(
            status_text(&remote, &connection),
            "Retrying 127.0.0.1:15702: refused"
        );
    }
}
