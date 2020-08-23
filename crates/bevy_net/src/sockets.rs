use std::collections::HashMap;

use super::common::SocketId;
use super::socket::Socket;

#[derive(Default)]
pub struct Sockets {
    sockets: HashMap<SocketId, Box<dyn Socket>>,
}

impl Sockets {
    pub fn add(&mut self, socket: Box<dyn Socket>) {
        self.sockets.insert(socket.get_id(), socket);
    }

    pub fn get(&self, id: SocketId) -> Option<&Box<dyn Socket>> {
        self.sockets.get(&id)
    }

    pub fn get_mut(&mut self, id: SocketId) -> Option<&mut Box<dyn Socket>> {
        self.sockets.get_mut(&id)
    }

    pub fn iter(&self) -> impl Iterator<Item=&Box<dyn Socket>> {
        self.sockets.values()
    }

    pub fn iter_mut(&mut self) -> impl Iterator<Item=&mut Box<dyn Socket>> {
        self.sockets.values_mut()
    }
}
