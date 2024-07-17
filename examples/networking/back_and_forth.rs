//! back and forth
use std::collections::VecDeque;
use std::fs::File;
use std::io::{Read};
use std::path::Path;
use std::time::{Duration, Instant};
use rustls_pemfile::{Item, read_all};
use bevy::net::quic::{ConnectionError, EndPoint, ServerConfig};
use bevy::net::rustls::pki_types::{CertificateDer, PrivatePkcs8KeyDer};

use bevy::prelude::*;
use bevy::tasks::IoTaskPool;

fn load_cert() -> (CertificateDer<'static>, PrivatePkcs8KeyDer<'static>) {
    #[cfg(not(target_os = "windows"))]
    let mut f= File::open(Path::new(r"../../assets/crypto/bevy_emaple_cert.pem")).unwrap();
    #[cfg(target_os = "windows")]
    let mut f= File::open(Path::new(r"./././assets/crypto/bevy_emaple_cert.pem")).unwrap();
    
    let mut bytes = vec![];
    
    f.read_to_end(&mut bytes).unwrap();
    
    for item in read_all(&mut VecDeque::from(bytes))  {

        match item.unwrap() {
            Item::X509Certificate(_) => {todo!()}
            Item::Pkcs1Key(_) => {todo!()}
            Item::Pkcs8Key(_) => {todo!()}
            Item::Sec1Key(_) => {todo!()}
            Item::Crl(_) => {todo!()}
            Item::Csr(_) => {todo!()}
            _ => {unimplemented!()}
        }
        
        todo!()
    }

    unimplemented!("no certificate or and/or private key was found");
}

fn end_points() -> (EndPoint, EndPoint) {
    let (cert, key) = load_cert();
    
    let client_endpoint = EndPoint::client("[::]:0".parse().unwrap()).unwrap();
    
    let server_endpoint = EndPoint::server(
        ServerConfig::with_single_cert(
            vec![cert], key.into()).unwrap(), 
        "[::]:0".parse().unwrap()
    ).unwrap();

    (client_endpoint, server_endpoint)
}

fn start_ping_pong() {
    let (client, server) = end_points();
    
    let server_addr = server.local_addr().unwrap();
    let client_addr = client.local_addr().unwrap();
    
    IoTaskPool::get().spawn(async move {
        let connecting = IoTaskPool::get().spawn(async move {
            (client.connect(server_addr, "").unwrap().await.unwrap(), client)
        });

        let listening = IoTaskPool::get().spawn(async move {
            let start = Instant::now();

            while Instant::now() - start < Duration::from_millis(1000) {
                if let Some(incoming) = server.accept().await {
                    assert!(incoming.remote_address() == client_addr);
                    return (server, incoming.accept().unwrap().await.unwrap())
                }
            }

            panic!("no connection request was received");
        });

        let (client_connection, client) = connecting.await;
        
        let (server, server_connection) = listening.await;
        
        IoTaskPool::get().spawn(async move {
            loop {
                client_connection.send_datagram(vec![0].into()).unwrap();
                println!("client sent a ping");
                let data = client_connection.read_datagram().await.unwrap();
                assert!(data.len() == 1 && data[0] == 1);
                println!("client servers pong")
            }
        }).detach();

        IoTaskPool::get().spawn(async move {
            loop {
                let data = server_connection.read_datagram().await.unwrap();
                assert!(data.len() == 1 && data[0] == 0);
                println!("server received clients ping");
                server_connection.send_datagram(vec![1].into()).unwrap();
                println!("server send client a pong")
            }
        }).detach();
    }).detach();
}

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_systems(Startup, start_ping_pong)
        .run();
}
