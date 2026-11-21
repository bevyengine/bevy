//! A bar that chooses the source the inspector reads from, and shows the state of the connection to
//! a remote app.
//!
//! Assumes either connecting to Local, or Remote (both main and render), or Remote (main only).

use alloc::{
    format,
    string::{String, ToString},
};

use bevy_ecs::{
    change_detection::DetectChangesMut,
    component::Component,
    hierarchy::Children,
    observer::On,
    query::{Added, With, Without},
    reflect::ReflectComponent,
    system::{ParamSet, Query, Res, ResMut, Single},
};
use bevy_feathers::{
    constants::{fonts, size},
    controls::{FeathersButton, FeathersTextInput, FeathersTextInputContainer},
    display::caption,
    font_styles::InheritableFont,
};
use bevy_log::warn;
use bevy_reflect::{prelude::ReflectDefault, Reflect};
use bevy_remote::http::{DEFAULT_PORT, DEFAULT_RENDER_PORT};
use bevy_scene::{bsn, on, Scene};
use bevy_text::{EditableText, FontSourceTemplate, FontWeight, TextEdit};
use bevy_ui::{px, widget::Text, AlignItems, FlexDirection, Node};
use bevy_ui_widgets::Activate;

use crate::{
    entity_tree::InspectorUi,
    remote::{RemoteConnection, RemoteConnectionState, RemoteConnections, RemoteSource},
    InspectorSource, InspectorSources,
};

/// The number of characters of a connection error shown in the status caption.
const ERROR_CHARS: usize = 80;

/// Marker for the caption showing the state of the connection.
#[derive(Component, Debug, Default, Clone, Copy, Reflect)]
#[reflect(Component, Debug, Default, Clone)]
pub struct ConnectionStatusMainLabel;

/// Marker for the caption showing the state of the connection.
#[derive(Component, Debug, Default, Clone, Copy, Reflect)]
#[reflect(Component, Debug, Default, Clone)]
pub struct ConnectionStatusRenderLabel;

/// Marker for the text input holding the remote host to connect to.
#[derive(Component, Debug, Default, Clone, Copy, Reflect)]
#[reflect(Component, Debug, Default, Clone)]
pub struct ConnectionAddressHostInput;

/// Marker for the text input holding the remote main app port to connect to.
#[derive(Component, Debug, Default, Clone, Copy, Reflect)]
#[reflect(Component, Debug, Default, Clone)]
pub struct ConnectionAddressMainPort;

/// Marker for the text input holding the remote render app port to connect to.
#[derive(Component, Debug, Default, Clone, Copy, Reflect)]
#[reflect(Component, Debug, Default, Clone)]
pub struct ConnectionAddressRenderPort;

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
                ConnectionAddressHostInput
                @FeathersTextInput
                --
                ConnectionAddressMainPort
                @FeathersTextInput
                --
                ConnectionAddressRenderPort
                @FeathersTextInput
            ]
            --
            @FeathersButton {
                @caption: bsn! { @caption("Connect") },
            }
            on(connect_both_clicked)
            --
            @FeathersButton {
                @caption: bsn! { @caption("Connect (main only)") },
            }
            on(connect_main_clicked)
            --
            @FeathersButton {
                @caption: bsn! { @caption("Local") },
            }
            on(local_clicked)
            --
            ConnectionStatusMainLabel
            @caption("Local")
            --
            ConnectionStatusRenderLabel
            @caption("")
        ]
    }
}

/// Fills a new address input with the current remote address, or the default one.
pub fn init_address_input(
    maybe_sources: Option<Res<InspectorSources>>,
    mut inputs: ParamSet<(
        Query<&mut EditableText, Added<ConnectionAddressHostInput>>,
        Query<&mut EditableText, Added<ConnectionAddressMainPort>>,
        Query<&mut EditableText, Added<ConnectionAddressRenderPort>>,
    )>,
) {
    let mut address_host = format_host(&RemoteSource::default());
    let mut address_main = DEFAULT_PORT;
    let mut address_render = DEFAULT_RENDER_PORT;

    if let Some(sources) = maybe_sources {
        for source in &sources.sources {
            match source {
                InspectorSource::Local => {}
                #[cfg(feature = "remote")]
                InspectorSource::Remote(remote) => {
                    use crate::WorldKind;

                    address_host = format_host(remote);
                    if remote.subapp == WorldKind::Main {
                        address_main = remote.port;
                    } else {
                        address_render = remote.port;
                    }
                }
            }
        }
    };

    for mut input_host in inputs.p0().iter_mut() {
        input_host.queue_edit(TextEdit::SelectAll);
        input_host.queue_edit(TextEdit::Insert(address_host.clone().into()));
    }

    let main_port = format!("{}", address_main);
    for mut input_main in inputs.p1().iter_mut() {
        input_main.queue_edit(TextEdit::SelectAll);
        input_main.queue_edit(TextEdit::Insert(main_port.clone().into()));
    }

    let render_port = format!("{}", address_render);
    for mut input_render in inputs.p2().iter_mut() {
        input_render.queue_edit(TextEdit::SelectAll);
        input_render.queue_edit(TextEdit::Insert(render_port.clone().into()));
    }
}

/// Observer switching the inspector to the remote app at the address in the address input.
pub fn connect_both_clicked(
    _activate: On<Activate>,
    inputs_host: Single<&EditableText, With<ConnectionAddressHostInput>>,
    inputs_main: Single<&EditableText, With<ConnectionAddressMainPort>>,
    inputs_render: Single<&EditableText, With<ConnectionAddressRenderPort>>,
    mut sources: ResMut<InspectorSources>,
) {
    let address = inputs_host.value().to_string();
    let Some(address) = parse_address(&address) else {
        warn!("`{address}` is not a valid address");
        return;
    };

    let main_port = inputs_main.value().to_string();
    let Some(main_port) = main_port.parse::<u16>().ok() else {
        warn!("`{main_port}` is not a valid port");
        return;
    };

    let render_port = inputs_render.value().to_string();
    let Some(render_port) = render_port.parse::<u16>().ok() else {
        warn!("`{main_port}` is not a valid port");
        return;
    };

    sources.set_if_neq(InspectorSources::remote(address, main_port, render_port));
}

/// Observer switching the inspector to the remote app at the address in the address input.
pub fn connect_main_clicked(
    _activate: On<Activate>,
    inputs_host: Single<&EditableText, With<ConnectionAddressHostInput>>,
    inputs_main: Single<&EditableText, With<ConnectionAddressMainPort>>,
    mut sources: ResMut<InspectorSources>,
) {
    let address = inputs_host.value().to_string();
    let Some(address) = parse_address(&address) else {
        warn!("`{address}` is not a valid address");
        return;
    };

    let main_port = inputs_main.value().to_string();
    let Some(main_port) = main_port.parse::<u16>().ok() else {
        warn!("`{main_port}` is not a valid port");
        return;
    };

    sources.set_if_neq(InspectorSources::remote_main(address, main_port));
}

/// Observer switching the inspector back to the local world.
pub fn local_clicked(_activate: On<Activate>, mut sources: ResMut<InspectorSources>) {
    sources.set_if_neq(InspectorSources::local());
}

/// Keeps the status caption in sync with the source and the connection.
pub fn sync_connection_status(
    sources: Option<Res<InspectorSources>>,
    connections: Res<RemoteConnections>,
    mut main_label: Single<&mut Text, With<ConnectionStatusMainLabel>>,
    mut render_label: Single<
        &mut Text,
        (
            With<ConnectionStatusRenderLabel>,
            Without<ConnectionStatusMainLabel>,
        ),
    >,
) {
    let Some(sources) = sources else {
        main_label.0 = "No sources".to_string();
        return;
    };

    let mut seen_render = false;

    for source in &sources.sources {
        match source {
            InspectorSource::Local => {
                let main_status = "Local".to_string();
                if main_label.0 != main_status {
                    main_label.0 = main_status.clone();
                }
            }
            #[cfg(feature = "remote")]
            InspectorSource::Remote(remote) => {
                if remote.subapp == crate::WorldKind::Main {
                    let status = status_text(remote, &connections.main);

                    if main_label.0 != status {
                        main_label.0 = status.clone();
                    }
                } else {
                    seen_render = true;

                    let status = status_text(remote, &connections.render);

                    if render_label.0 != status {
                        render_label.0 = status.clone();
                    }
                }
            }
        }
    }

    if !seen_render {
        render_label.0 = "".to_string();
    }
}

#[cfg(feature = "remote")]
fn status_text(remote: &RemoteSource, connection: &RemoteConnection) -> String {
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

fn format_host(source: &RemoteSource) -> String {
    if source.host.contains(':') {
        format!("[{}]", source.host.0)
    } else {
        source.host.0.to_string()
    }
}

fn format_address(source: &RemoteSource) -> String {
    if source.host.contains(':') {
        format!("[{}]:{}", source.host.0, source.port)
    } else {
        format!("{}:{}", source.host.0, source.port)
    }
}

/// Parses a `host` address, where an IPv6 host is written in brackets.
fn parse_address(address: &str) -> Option<&str> {
    let host = address.trim();
    let host = host
        .strip_prefix('[')
        .and_then(|host| host.strip_suffix(']'))
        .unwrap_or(host);
    if host.is_empty() {
        return None;
    }
    Some(host)
}

#[cfg(test)]
mod tests {
    use crate::WorldKind::Main;

    use super::*;

    #[test]
    fn parses_addresses() {
        assert_eq!(parse_address(" 127.0.0.1 "), Some("127.0.0.1"));
        assert_eq!(parse_address("[::1]"), Some("::1"));
        assert_eq!(parse_address("localhost"), Some("localhost"));
        assert_eq!(parse_address(""), None);
        assert_eq!(
            format_address(&RemoteSource::new("::1", 80, Main)),
            "[::1]:80".to_string()
        );
    }

    #[test]
    fn describes_the_connection() {
        let remote = RemoteSource::localhost(15702);
        let mut connection = RemoteConnection::default();
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
