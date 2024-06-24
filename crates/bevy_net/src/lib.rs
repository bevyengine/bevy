
use std::io::{Error, Read, Write};
use std::net::{SocketAddr, TcpStream, ToSocketAddrs, UdpSocket};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use serde::{Deserialize, Deserializer, Serialize, Serializer};

pub mod easy_sockets;
mod net_types;




