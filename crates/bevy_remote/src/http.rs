//! The BRP transport using JSON-RPC over HTTP.
//!
//! Adding the [`RemoteHttpPlugin`] to your [`App`] causes Bevy to accept
//! connections over HTTP (by default, on port 15702) while your app is running.
//!
//! Clients are expected to `POST` JSON requests to the root URL; see the `client`
//! example for a trivial example of use.

#![cfg(not(target_family = "wasm"))]
use crate::schemas::open_rpc::ServerObject;
use crate::{
    error_codes, BrpBatch, BrpError, BrpMessage, BrpRequest, BrpResponse, BrpResult, BrpSender,
    RemoteMethods,
};
use anyhow::Result as AnyhowResult;
use async_channel::{Receiver, Sender};
use async_io::Async;
use bevy_app::{App, Plugin, Update};
use bevy_derive::{Deref, DerefMut};
use bevy_ecs::change_detection::DetectChanges;
use bevy_ecs::prelude::ReflectResource;
use bevy_ecs::resource::Resource;
use bevy_ecs::schedule::common_conditions::resource_changed_or_removed;
use bevy_ecs::schedule::IntoScheduleConfigs;
use bevy_ecs::system::{Res, ResMut};
use bevy_platform::collections::HashMap;
use bevy_reflect::{Reflect, TypePath};
use bevy_reflect::{ReflectDeserialize, ReflectSerialize};
use bevy_tasks::Task;
use bevy_tasks::{futures_lite::StreamExt, IoTaskPool};
use core::{
    convert::Infallible,
    net::{IpAddr, Ipv4Addr},
    pin::Pin,
    task::{Context, Poll},
};
use http_body_util::{BodyExt as _, Full};
use hyper::{
    body::{Body, Bytes, Frame, Incoming},
    header::{HeaderName, HeaderValue},
    server::conn::http1,
    service, Request, Response,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use smol_hyper::rt::{FuturesIo, SmolTimer};
use std::any::TypeId;
use std::net::Ipv6Addr;
use std::net::{TcpListener, TcpStream};

/// The default port that Bevy will listen on.
///
/// This value was chosen randomly.
pub const DEFAULT_PORT: u16 = 15702;

/// The default host address that Bevy will use for its server.
pub const DEFAULT_ADDR: IpAddr = IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1));

/// A struct that holds a collection of HTTP headers.
///
/// This struct is used to store a set of HTTP headers as key-value pairs, where the keys are
/// of type [`HeaderName`] and the values are of type [`HeaderValue`].
///
#[derive(Debug, Clone, Deref, DerefMut, Default)]
pub struct Headers(HashMap<HeaderName, HeaderValue>);

/// Add this plugin to your [`App`] to allow remote connections over HTTP to inspect and modify entities.
/// It requires the [`RemotePlugin`](super::RemotePlugin).
///
/// This BRP transport cannot be used when targeting WASM.
///
/// The defaults are:
/// - [`DEFAULT_ADDR`] : 127.0.0.1.
/// - [`DEFAULT_PORT`] : 15702.
///
pub struct RemoteHttpPlugin {
    /// The address that Bevy will bind to.
    address: IpAddr,
    /// The port that Bevy will listen on.
    port: u16,
    /// The headers that Bevy will include in its HTTP responses
    headers: Headers,
}

impl Default for RemoteHttpPlugin {
    fn default() -> Self {
        Self {
            address: DEFAULT_ADDR,
            port: DEFAULT_PORT,
            headers: Headers::default(),
        }
    }
}

impl Plugin for RemoteHttpPlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<HttpServerConfig>();
        app.insert_resource(HttpServerConfig {
            address: self.address.into(),
            port: self.port,
            headers: self.headers.clone(),
            task: None,
        })
        .add_systems(
            Update,
            update_server.run_if(resource_changed_or_removed::<HttpServerConfig>),
        );
    }
}

fn update_server(
    server_config: Option<ResMut<HttpServerConfig>>,
    request_sender: Res<BrpSender>,
    mut remote_methods: ResMut<RemoteMethods>,
) {
    if server_config.is_none() {
        remote_methods.remove_server(TypeId::of::<HttpServerConfig>());
    }
    bevy_log::info!("exist: {}", server_config.is_some());
    if let Some(mut config) = server_config {
        let should_start_server = (config.is_added() && config.task.is_none())
            || (config.is_changed() && config.task.is_some());
        bevy_log::info!(
            "added: {}, changed: {}, should_start_server: {}, config: {:?}",
            config.is_added(),
            config.is_changed(),
            should_start_server,
            &config
        );

        if should_start_server && config.start_server(request_sender).is_ok() {
            remote_methods.register_server(TypeId::of::<HttpServerConfig>(), (&*config).into());
        };
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
    /// Set the extra headers that the response will include.
    ///
    /// ////// /// # Example
    ///
    /// ```ignore
    ///
    /// // Create CORS headers
    /// let cors_headers = Headers::new()
    ///        .insert("Access-Control-Allow-Origin", "*")
    ///        .insert("Access-Control-Allow-Headers", "Content-Type");
    ///
    /// // Create the Bevy app and add the RemoteHttpPlugin with CORS headers
    /// fn main() {
    ///     App::new()
    ///     .add_plugins(DefaultPlugins)
    ///     .add_plugins(RemotePlugin)
    ///     .add_plugins(RemoteHttpPlugin::default()
    ///         .with_headers(cors_headers))
    ///     .run();
    /// }
    /// ```
    #[must_use]
    pub fn with_headers(mut self, headers: Headers) -> Self {
        self.headers = headers;
        self
    }
    /// Add a single header to the response headers.
    #[must_use]
    pub fn with_header(
        mut self,
        name: impl TryInto<HeaderName>,
        value: impl TryInto<HeaderValue>,
    ) -> Self {
        match (name.try_into(), value.try_into()) {
            (Ok(name), Ok(value)) => _ = self.headers.insert(name, value),
            _ => {}
        }
        self
    }
}
/// A reflectable representation of an IP address.
///
/// This enum provides a serializable and reflectable alternative to [`std::net::IpAddr`]
/// for use in Bevy's reflection system. It can represent both IPv4 and IPv6 addresses
/// as byte arrays.
#[derive(Debug, Resource, Reflect, Clone, Serialize, Deserialize)]
#[reflect(Serialize, Deserialize)]
pub enum IpAddressReflect {
    /// An IPv4 address represented as a 4-byte array.
    V4([u8; 4]),
    /// An IPv6 address represented as a 16-byte array.
    V6([u8; 16]),
}

impl From<IpAddr> for IpAddressReflect {
    fn from(value: IpAddr) -> Self {
        match value {
            IpAddr::V4(addr) => IpAddressReflect::V4(addr.octets()),
            IpAddr::V6(addr) => IpAddressReflect::V6(addr.octets()),
        }
    }
}

impl From<IpAddressReflect> for IpAddr {
    fn from(value: IpAddressReflect) -> Self {
        match value {
            IpAddressReflect::V4(addr) => IpAddr::V4(Ipv4Addr::from(addr)),
            IpAddressReflect::V6(addr) => IpAddr::V6(Ipv6Addr::from(addr)),
        }
    }
}

#[derive(Debug, Resource, Reflect, Serialize, Deserialize)]
#[reflect(Resource, Serialize, Deserialize)]
/// A resource containing the data for the HTTP server.
pub struct HttpServerConfig {
    /// The address to bind the server to.
    pub address: IpAddressReflect,
    /// The port to bind the server to.
    pub port: u16,
    #[reflect(ignore)]
    #[serde(skip)]
    /// The headers to send with each response.
    pub headers: Headers,
    #[reflect(ignore)]
    #[serde(skip)]
    /// The task that is running the server.
    pub task: Option<Task<AnyhowResult<()>>>,
}

impl From<&HttpServerConfig> for ServerObject {
    fn from(value: &HttpServerConfig) -> Self {
        let ip: IpAddr = value.address.clone().into();
        ServerObject {
            name: HttpServerConfig::short_type_path().into(),
            url: format!("{}:{}", ip, value.port),
            ..Default::default()
        }
    }
}

impl HttpServerConfig {
    fn build_listener(&self) -> AnyhowResult<Async<TcpListener>> {
        let ip: IpAddr = self.address.clone().into();
        let listener = Async::<TcpListener>::bind((ip, self.port))?;
        Ok(listener)
    }

    fn start_server(&mut self, request_sender: Res<BrpSender>) -> AnyhowResult<()> {
        let listener = self.build_listener()?;
        let headers = self.headers.clone();
        self.task =
            Some(IoTaskPool::get().spawn(server_main(listener, request_sender.clone(), headers)));
        Ok(())
    }
}
/// The Bevy Remote Protocol server main loop.
async fn server_main(
    listener: Async<TcpListener>,
    request_sender: Sender<BrpMessage>,
    headers: Headers,
) -> AnyhowResult<()> {
    listen(listener, &request_sender, headers).await
}

async fn listen(
    listener: Async<TcpListener>,
    request_sender: &Sender<BrpMessage>,
    headers: Headers,
) -> AnyhowResult<()> {
    loop {
        let (client, _) = listener.accept().await?;

        let request_sender = request_sender.clone();
        let headers = headers.clone();
        IoTaskPool::get()
            .spawn(async move {
                let _ = handle_client(client, request_sender, &headers).await;
            })
            .detach();
    }
}

async fn handle_client(
    client: Async<TcpStream>,
    request_sender: Sender<BrpMessage>,
    headers: &Headers,
) -> AnyhowResult<()> {
    http1::Builder::new()
        .timer(SmolTimer::new())
        .serve_connection(
            FuturesIo::new(client),
            service::service_fn(|request| process_request_batch(request, &request_sender, headers)),
        )
        .await?;

    Ok(())
}

/// A helper function for the Bevy Remote Protocol server that handles a batch
/// of requests coming from a client.
async fn process_request_batch(
    request: Request<Incoming>,
    request_sender: &Sender<BrpMessage>,
    headers: &Headers,
) -> AnyhowResult<Response<BrpHttpBody>> {
    let batch_bytes = request.into_body().collect().await?.to_bytes();
    let batch: Result<BrpBatch, _> = serde_json::from_slice(&batch_bytes);

    let result = match batch {
        Ok(BrpBatch::Single(request)) => {
            let response = process_single_request(request, request_sender).await?;
            match response {
                BrpHttpResponse::Complete(res) => {
                    BrpHttpResponse::Complete(serde_json::to_string(&res)?)
                }
                BrpHttpResponse::Stream(stream) => BrpHttpResponse::Stream(stream),
            }
        }
        Ok(BrpBatch::Batch(requests)) => {
            let mut responses = Vec::new();

            for request in requests {
                let response = process_single_request(request, request_sender).await?;
                match response {
                    BrpHttpResponse::Complete(res) => responses.push(res),
                    BrpHttpResponse::Stream(BrpStream { id, .. }) => {
                        responses.push(BrpResponse::new(
                            id,
                            Err(BrpError {
                                code: error_codes::INVALID_REQUEST,
                                message: "Streaming can not be used in batch requests".to_string(),
                                data: None,
                            }),
                        ));
                    }
                }
            }

            BrpHttpResponse::Complete(serde_json::to_string(&responses)?)
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

            BrpHttpResponse::Complete(serde_json::to_string(&err)?)
        }
    };

    let mut response = match result {
        BrpHttpResponse::Complete(serialized) => {
            let mut response = Response::new(BrpHttpBody::Complete(Full::new(Bytes::from(
                serialized.as_bytes().to_owned(),
            ))));
            response.headers_mut().insert(
                hyper::header::CONTENT_TYPE,
                HeaderValue::from_static("application/json"),
            );
            response
        }
        BrpHttpResponse::Stream(stream) => {
            let mut response = Response::new(BrpHttpBody::Stream(stream));
            response.headers_mut().insert(
                hyper::header::CONTENT_TYPE,
                HeaderValue::from_static("text/event-stream"),
            );
            response
        }
    };
    for (key, value) in headers.iter() {
        response.headers_mut().insert(key, value.clone());
    }
    Ok(response)
}

/// A helper function for the Bevy Remote Protocol server that processes a single
/// request coming from a client.
async fn process_single_request(
    request: Value,
    request_sender: &Sender<BrpMessage>,
) -> AnyhowResult<BrpHttpResponse<BrpResponse, BrpStream>> {
    // Reach in and get the request ID early so that we can report it even when parsing fails.
    let id = request.as_object().and_then(|map| map.get("id")).cloned();

    let request: BrpRequest = match serde_json::from_value(request) {
        Ok(v) => v,
        Err(err) => {
            return Ok(BrpHttpResponse::Complete(BrpResponse::new(
                id,
                Err(BrpError {
                    code: error_codes::INVALID_REQUEST,
                    message: err.to_string(),
                    data: None,
                }),
            )));
        }
    };

    if request.jsonrpc != "2.0" {
        return Ok(BrpHttpResponse::Complete(BrpResponse::new(
            id,
            Err(BrpError {
                code: error_codes::INVALID_REQUEST,
                message: String::from("JSON-RPC request requires `\"jsonrpc\": \"2.0\"`"),
                data: None,
            }),
        )));
    }

    let watch = request.method.contains("+watch");
    let size = if watch { 8 } else { 1 };
    let (result_sender, result_receiver) = async_channel::bounded(size);

    let _ = request_sender
        .send(BrpMessage {
            method: request.method,
            params: request.params,
            sender: result_sender,
        })
        .await;

    if watch {
        Ok(BrpHttpResponse::Stream(BrpStream {
            id: request.id,
            rx: Box::pin(result_receiver),
        }))
    } else {
        let result = result_receiver.recv().await?;
        Ok(BrpHttpResponse::Complete(BrpResponse::new(
            request.id, result,
        )))
    }
}

struct BrpStream {
    id: Option<Value>,
    rx: Pin<Box<Receiver<BrpResult>>>,
}

impl Body for BrpStream {
    type Data = Bytes;
    type Error = Infallible;

    fn poll_frame(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<Option<Result<Frame<Self::Data>, Self::Error>>> {
        match self.as_mut().rx.poll_next(cx) {
            Poll::Ready(result) => match result {
                Some(result) => {
                    let response = BrpResponse::new(self.id.clone(), result);
                    let serialized = serde_json::to_string(&response).unwrap();
                    let bytes =
                        Bytes::from(format!("data: {serialized}\n\n").as_bytes().to_owned());
                    let frame = Frame::data(bytes);
                    Poll::Ready(Some(Ok(frame)))
                }
                None => Poll::Ready(None),
            },
            Poll::Pending => Poll::Pending,
        }
    }

    fn is_end_stream(&self) -> bool {
        self.rx.is_closed()
    }
}

enum BrpHttpResponse<C, S> {
    Complete(C),
    Stream(S),
}

enum BrpHttpBody {
    Complete(Full<Bytes>),
    Stream(BrpStream),
}

impl Body for BrpHttpBody {
    type Data = Bytes;
    type Error = Infallible;

    fn poll_frame(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<Option<Result<Frame<Self::Data>, Self::Error>>> {
        match &mut *self.get_mut() {
            BrpHttpBody::Complete(body) => Body::poll_frame(Pin::new(body), cx),
            BrpHttpBody::Stream(body) => Body::poll_frame(Pin::new(body), cx),
        }
    }
}
