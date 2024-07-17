use std::io::{Read, Write};
use std::net::ToSocketAddrs;

#[cfg(feature = "tls")]
pub use quinn::rustls;

#[cfg(feature = "quic")]
pub mod quic;

