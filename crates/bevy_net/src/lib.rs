
use std::io::{Error, Read, Write};
use std::net::{SocketAddr, TcpStream, ToSocketAddrs, UdpSocket};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use serde::{Deserialize, Deserializer, Serialize, Serializer};

pub mod easy_sockets;

/// The WebTransport protocol this crate uses
/// is currently a draft, meaning it's exact specifications are unstable.
/// New drafts of the standard are released periodicity which
/// means our implementation (https://crates.io/crates/wtransport)
/// might not meet the standard at some point in the future.
/// This test will fail if the draft has expired 
/// since we last updated our implementation.
///
/// The most recent draft can be found here: 
/// https://datatracker.ietf.org/doc/html/draft-ietf-webtrans-overview
///
/// The current draft as of the time of writing is version 7.
/// This draft will expire on the 5th of september 2024.
#[test]
fn transport_api_outdated_test() {
    //equivalent to the 5th of september 2024 at 00:00
    let expiry_time_stamp = 1725458400;

    let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();
    
    assert!(now < expiry_time_stamp, 
            "The WebTransport protocol has been updated, our implementation is out of data! \
            Check the documentation on this test for details.")
}




