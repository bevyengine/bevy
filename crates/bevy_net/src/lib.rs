use std::collections::VecDeque;
use std::io;
use std::io::{Error, Read, Write};
use std::net::{SocketAddr, TcpStream, ToSocketAddrs, UdpSocket};
use std::time::Duration;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use bevy_internal::tasks::IoTaskPool;

pub mod easy_sockets;




