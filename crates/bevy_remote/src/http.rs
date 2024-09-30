//! The BRP transport using JSON-RPC over HTTP.
//!
//! Adding the [`RemoteHttpPlugin`] to your [`App`] causes Bevy to accept
//! connections over HTTP (by default, on port 15702) while your app is running.
//!
//! Clients are expected to `POST` JSON requests to the root URL; see the `client`
//! example for a trivial example of use.

#![cfg(not(target_family = "wasm"))]

use crate::{error_codes, BrpBatch, BrpError, BrpMessage, BrpRequest, BrpResponse, BrpSender};
use anyhow::Result as AnyhowResult;
use async_channel::Sender;
use async_io::Async;
use bevy_app::{App, Plugin, Startup};
use bevy_ecs::system::{Res, Resource};
use bevy_tasks::IoTaskPool;
use core::net::{IpAddr, Ipv4Addr};
use http_body_util::{BodyExt as _, Full};
use hyper::{
    body::{Bytes, Incoming},
    server::conn::http1,
    service, Request, Response,
};
use serde_json::Value;
use smol_hyper::rt::{FuturesIo, SmolTimer};
use std::net::TcpListener;
use std::net::TcpStream;

/// The default port that Bevy will listen on.
///
/// This value was chosen randomly.
pub const DEFAULT_PORT: u16 = 15702;

/// The default host address that Bevy will use for its server.
pub const DEFAULT_ADDR: IpAddr = IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1));

/// Add this plugin to your [`App`] to allow remote connections over HTTP to inspect and modify entities.
/// It requires the [`RemotePlugin`](super::RemotePlugin).
///
/// This BRP transport cannot be used when targeting WASM.
///
/// The defaults are:
/// - [`DEFAULT_ADDR`] : 127.0.0.1.
/// - [`DEFAULT_PORT`] : 15702.
pub struct RemoteHttpPlugin {
    /// The address that Bevy will bind to.
    address: IpAddr,
    /// The port that Bevy will listen on.
    port: u16,
}

impl Default for RemoteHttpPlugin {
    fn default() -> Self {
        Self {
            address: DEFAULT_ADDR,
            port: DEFAULT_PORT,
        }
    }
}

impl Plugin for RemoteHttpPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(HostAddress(self.address))
            .insert_resource(HostPort(self.port))
            .add_systems(Startup, start_http_server);
    }
}

impl RemoteHttpPlugin {
    /// Set the IP address that the server will use.
    #[must_use]
    pub fn with_address(mut self, address: impl Into<IpAddr>) -> Self {
        self.address = address.into();
        self
    }

    /// Set the remote port that the server will listen on.
    #[must_use]
    pub fn with_port(mut self, port: u16) -> Self {
        self.port = port;
        self
    }
}

/// A resource containing the IP address that Bevy will host on.
///
/// Currently, changing this while the application is running has no effect; this merely
/// reflects the IP address that is set during the setup of the [`RemoteHttpPlugin`].
#[derive(Debug, Resource)]
pub struct HostAddress(pub IpAddr);

/// A resource containing the port number that Bevy will listen on.
///
/// Currently, changing this while the application is running has no effect; this merely
/// reflects the host that is set during the setup of the [`RemoteHttpPlugin`].
#[derive(Debug, Resource)]
pub struct HostPort(pub u16);

/// A system that starts up the Bevy Remote Protocol HTTP server.
fn start_http_server(
    request_sender: Res<BrpSender>,
    address: Res<HostAddress>,
    remote_port: Res<HostPort>,
) {
    IoTaskPool::get()
        .spawn(server_main(
            address.0,
            remote_port.0,
            request_sender.clone(),
        ))
        .detach();
}

/// The Bevy Remote Protocol server main loop.
async fn server_main(
    address: IpAddr,
    port: u16,
    request_sender: Sender<BrpMessage>,
) -> AnyhowResult<()> {
    listen(
        Async::<TcpListener>::bind((address, port))?,
        &request_sender,
    )
    .await
}

async fn listen(
    listener: Async<TcpListener>,
    request_sender: &Sender<BrpMessage>,
) -> AnyhowResult<()> {
    loop {
        let (client, _) = listener.accept().await?;

        let request_sender = request_sender.clone();
        IoTaskPool::get()
            .spawn(async move {
                let _ = handle_client(client, request_sender).await;
            })
            .detach();
    }
}

async fn handle_client(
    client: Async<TcpStream>,
    request_sender: Sender<BrpMessage>,
) -> AnyhowResult<()> {
    http1::Builder::new()
        .timer(SmolTimer::new())
        .serve_connection(
            FuturesIo::new(client),
            service::service_fn(|request| process_request_batch(request, &request_sender)),
        )
        .await?;

    Ok(())
}

/// A helper function for the Bevy Remote Protocol server that handles a batch
/// of requests coming from a client.
async fn process_request_batch(
    request: Request<Incoming>,
    request_sender: &Sender<BrpMessage>,
) -> AnyhowResult<Response<Full<Bytes>>> {
    let batch_bytes = request.into_body().collect().await?.to_bytes();
    let batch: Result<BrpBatch, _> = serde_json::from_slice(&batch_bytes);

    let serialized = match batch {
        Ok(BrpBatch::Single(request)) => {
            serde_json::to_string(&process_single_request(request, request_sender).await?)?
        }
        Ok(BrpBatch::Batch(requests)) => {
            let mut responses = Vec::new();

            for request in requests {
                responses.push(process_single_request(request, request_sender).await?);
            }

            serde_json::to_string(&responses)?
        }
        Err(err) => {
            let err = BrpResponse::new(
                None,
                Err(BrpError {
                    code: error_codes::INVALID_REQUEST,
                    message: err.to_string(),
                    data: None,
                }),
            );

            serde_json::to_string(&err)?
        }
    };

    Ok(Response::new(Full::new(Bytes::from(
        serialized.as_bytes().to_owned(),
    ))))
}

/// A helper function for the Bevy Remote Protocol server that processes a single
/// request coming from a client.
async fn process_single_request(
    request: Value,
    request_sender: &Sender<BrpMessage>,
) -> AnyhowResult<BrpResponse> {
    // Reach in and get the request ID early so that we can report it even when parsing fails.
    let id = request.as_object().and_then(|map| map.get("id")).cloned();

    let request: BrpRequest = match serde_json::from_value(request) {
        Ok(v) => v,
        Err(err) => {
            return Ok(BrpResponse::new(
                id,
                Err(BrpError {
                    code: error_codes::INVALID_REQUEST,
                    message: err.to_string(),
                    data: None,
                }),
            ));
        }
    };

    if request.jsonrpc != "2.0" {
        return Ok(BrpResponse::new(
            id,
            Err(BrpError {
                code: error_codes::INVALID_REQUEST,
                message: String::from("JSON-RPC request requires `\"jsonrpc\": \"2.0\"`"),
                data: None,
            }),
        ));
    }

    let (result_sender, result_receiver) = async_channel::bounded(1);

    let _ = request_sender
        .send(BrpMessage {
            method: request.method,
            params: request.params,
            sender: result_sender,
        })
        .await;

    let result = result_receiver.recv().await?;
    Ok(BrpResponse::new(request.id, result))
}
