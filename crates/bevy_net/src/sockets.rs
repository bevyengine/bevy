use std::collections::HashMap;

use super::common::SocketId;
use super::socket::Socket;

#[derive(Default)]
pub struct Sockets {
    sockets: HashMap<SocketId, Socket>,
}

impl Sockets {
    pub fn add(&mut self, socket: Socket) {
        self.sockets.insert(socket.id, socket);
    }

    pub fn get(&self, id: SocketId) -> Option<&Socket> {
        self.sockets.get(&id)
    }

    pub fn get_mut(&mut self, id: SocketId) -> Option<&mut Socket> {
        self.sockets.get_mut(&id)
    }

    pub fn iter(&self) -> impl Iterator<Item=&Socket> {
        self.sockets.values()
    }

    pub fn iter_mut(&mut self) -> impl Iterator<Item=&mut Socket> {
        self.sockets.values_mut()
    }
}
