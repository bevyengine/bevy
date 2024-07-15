use bevy_app::{App, Plugin, Update};
use bevy_asset::AssetPlugin;
use crate::easy_sockets::quic::{Connection, RecvStream, SendStream};
use crate::easy_sockets::socket_manager::{handle_socket_events_system, Sockets, update_ports_system};

#[derive(Default)]
pub struct SocketManagerPlugin;

impl Plugin for SocketManagerPlugin {
    fn build(&self, app: &mut App) {
        assert!(!app.is_plugin_added::<AssetPlugin>());
        
        #[cfg(feature = "QUIC")]
        app.init_resource::<Sockets<Connection>>()
            .init_resource::<Sockets<RecvStream>>()
            .init_resource::<Sockets<SendStream>>()
            .add_systems(Update, (
                update_ports_system::<RecvStream>, 
                update_ports_system::<SendStream>,
                update_ports_system::<Connection>,
                handle_socket_events_system::<RecvStream>,
                handle_socket_events_system::<SendStream>,
                handle_socket_events_system::<Connection>
            ));
        
        
        todo!()
    }
}