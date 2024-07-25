//! This example shows how you can use 2 async tasks (one representing the client, and the other the server)
//! to ping each other back and forth. The client, establishes a connection (with reduced security for this example) with the
//! server, then the client sends a single datagram to the server, a 'ping'.
//! The server then sends a 'pong' back to the client, and they repeat.
//! Both the client and serer have been rate limited to keep the console outputs readable in real time.
use async_std::task::sleep;
use bevy::net::crypto_utils::SkipServerVerification;
use bevy::net::quic::crypto::rustls::QuicClientConfig;
use bevy::net::quic::{ClientConfig, EndPoint, ServerConfig};
use bevy::net::rustls;
use bevy::net::rustls::pki_types::{CertificateDer, PrivatePkcs1KeyDer};
use bevy::net::rustls::RootCertStore;
use rustls_pemfile::{read_all, Item};
use std::collections::VecDeque;
use std::fs::File;
use std::io::Read;
use std::net::{Ipv4Addr, SocketAddr};
use std::path::Path;
use std::sync::Arc;
use std::time::{Duration, Instant};

use bevy::prelude::*;
use bevy::tasks::IoTaskPool;

/// Load self-sighed certificate and private key using the [`rustls_pemfile`]
fn load_cert() -> (CertificateDer<'static>, PrivatePkcs1KeyDer<'static>) {
    #[cfg(not(target_os = "windows"))]
    let mut file = File::open(Path::new(
        r"../../../assets/cypto/bevy_ping_pong_example.pem",
    ))
    .unwrap();

    #[cfg(target_os = "windows")]
    let mut file = File::open(Path::new(r"./././assets/cypto/bevy_ping_pong_example.pem")).unwrap();

    let mut bytes = Vec::new();

    file.read_to_end(&mut bytes).unwrap();

    let mut bytes = VecDeque::from(bytes);

    let mut cert = None;
    let mut key = None;

    for item in read_all(&mut bytes) {
        match item.unwrap() {
            Item::X509Certificate(c) => {
                cert = Some(c);
            }
            Item::Pkcs1Key(k) => {
                key = Some(k);
            }
            _ => {
                unimplemented!()
            }
        }
    }

    (cert.unwrap(), key.unwrap())
}

// Set up endpoints for the client and server
fn end_points() -> (EndPoint, EndPoint) {
    let (cert, key) = load_cert();

    // Set up the client to use the loopback address, this means any data sent through this endpoint will
    // never leave the local machine, and will only be available to other endpoints assigned
    // to the local host ip. This makes it very useful for testing and development.
    let mut client_endpoint =
        EndPoint::client(SocketAddr::new(Ipv4Addr::LOCALHOST.into(), 0)).unwrap();

    let mut root_store = RootCertStore::empty();
    root_store.add(cert.clone()).unwrap();

    // Here we are setting up the client to use a given default config
    // for all outgoing connections. If you don't do this and use the [`Endpoint::connect`] method
    // it will throw an error.

    client_endpoint.set_default_client_config(ClientConfig::new(Arc::new(
        // Create a new tls client config for QUIC
        QuicClientConfig::try_from(
            // Create a new tls config
            rustls::ClientConfig::builder()
                // Use the dangerous (possible insecure) settings to bypass server authentication
                .dangerous()
                // Use a dummy certificate verifier.
                // This is quite dangerous and should not be used in production
                // unless your application doesn't need security (most will).
                .with_custom_certificate_verifier(SkipServerVerification::new())
                // Indicate that we are only going to verify the public key sent by the server.
                // Because the verifier we use for that purpose is a dummy
                // and doesn't actually do that there is in fact no verification
                // going here at all.
                .with_no_client_auth(),
        )
        .unwrap(),
    )));

    // Set up the server endpoint will the certificate we
    // loaded earlier, also using the loopback address.

    let server_endpoint = EndPoint::server(
        ServerConfig::with_single_cert(vec![cert], key.into()).unwrap(),
        SocketAddr::new(Ipv4Addr::LOCALHOST.into(), 0),
    )
    .unwrap();

    (client_endpoint, server_endpoint)
}

fn start_ping_pong() {
    let (client, server) = end_points();

    let server_addr = server.local_addr().unwrap();
    let client_addr = client.local_addr().unwrap();

    IoTaskPool::get()
        .spawn(async move {
            // Start attempting to connect to the server from the client
            let connecting = IoTaskPool::get().spawn(async move {
                (
                    client.connect(server_addr, "dummy").unwrap().await.unwrap(),
                    client,
                )
            });

            // Start listening for the client from the server
            let listening = IoTaskPool::get().spawn(async move {
                let start = Instant::now();

                while Instant::now() - start < Duration::from_millis(1000) {
                    if let Some(incoming) = server.accept().await {
                        assert_eq!(incoming.remote_address(), client_addr);
                        return (server, incoming.accept().unwrap().await.unwrap());
                    }
                }

                panic!("no connection request was received");
            });

            // Wait for the client and server to connect to each other
            let (client_connection, _client) = connecting.await;

            let (_server, server_connection) = listening.await;

            // Create and spawn a task that sends pings from the client,
            // then awaits a response from the server,
            // verifies it is in the correct format, then sends a ping.
            IoTaskPool::get()
                .spawn(async move {
                    loop {
                        client_connection.send_datagram(vec![0].into()).unwrap();
                        println!("client sent a ping");
                        sleep(Duration::from_millis(1000)).await;
                        let data = client_connection.read_datagram().await.unwrap();
                        assert!(data.len() == 1 && data[0] == 1);
                        println!("client receives servers pong");
                        sleep(Duration::from_millis(1000)).await;
                    }
                })
                .detach();

            // Create and spawn a task that awaits pings from the client,
            // verifies that it is in the correct format, then sends a pong.
            IoTaskPool::get()
                .spawn(async move {
                    loop {
                        let data = server_connection.read_datagram().await.unwrap();
                        assert!(data.len() == 1 && data[0] == 0);
                        println!("server received clients ping");
                        sleep(Duration::from_millis(1000)).await;
                        server_connection.send_datagram(vec![1].into()).unwrap();
                        println!("server send client a pong");
                        sleep(Duration::from_millis(1000)).await;
                    }
                })
                .detach();
        })
        .detach();
}

fn main() {
    App::new()
        .add_plugins(MinimalPlugins)
        .add_systems(Startup, start_ping_pong)
        .run();
}
