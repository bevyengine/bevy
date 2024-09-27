use anyhow::Result as AnyhowResult;
use bevy_derive::{Deref, DerefMut};
use bevy_ecs::{
    system::{In, Resource, SystemId},
    world::{Mut, World},
};
use bevy_tasks::IoTaskPool;
use bevy_utils::HashMap;
use futures_util::{
    stream::{SplitSink, SplitStream},
    SinkExt, StreamExt,
};
use http_body_util::Full;
use hyper::{
    body::{Bytes, Incoming},
    Request, Response,
};
use hyper_tungstenite::{HyperWebsocket, HyperWebsocketStream};
use serde_json::Value;
use smol::channel::{self, Receiver, Sender};
use tungstenite::Message;

use crate::{
    error_codes, BrpBatch, BrpClientId, BrpError, BrpMessage, BrpRequest, BrpResponse, BrpResult,
    RemoteMethod, RemoteMethods,
};

pub struct BrpStreamMessage {
    client_id: BrpClientId,
    kind: BrpStreamMessageKind,
}

enum BrpStreamMessageKind {
    Connect(BrpMessage),
    Disconnect,
}

#[derive(Resource, Deref, DerefMut)]
pub struct BrpStreamMailBox(pub Receiver<BrpStreamMessage>);

#[derive(Resource, Deref, DerefMut, Default)]
pub struct ActiveStreams(
    HashMap<BrpClientId, (BrpMessage, SystemId<In<Option<Value>>, Option<BrpResult>>)>,
);

pub fn process_remote_stream_messages(world: &mut World) {
    if !world.contains_resource::<BrpStreamMailBox>() {
        return;
    }

    while let Ok(stream_message) = world.resource_mut::<BrpStreamMailBox>().try_recv() {
        world.resource_scope(
            |world, methods: Mut<RemoteMethods>| match stream_message.kind {
                BrpStreamMessageKind::Connect(message) => {
                    let Some(handler) = methods.0.get(&message.method) else {
                        let _ = message.sender.send_blocking(Err(BrpError {
                            code: error_codes::METHOD_NOT_FOUND,
                            message: format!("Method `{}` not found", message.method),
                            data: None,
                        }));
                        return;
                    };

                    match handler {
                        RemoteMethod::Stream(system_id) => {
                            world
                                .resource_mut::<ActiveStreams>()
                                .insert(stream_message.client_id, (message, *system_id));
                        }
                        _ => {}
                    };
                }
                BrpStreamMessageKind::Disconnect => {
                    world
                        .resource_mut::<ActiveStreams>()
                        .remove(&stream_message.client_id);
                }
            },
        );
    }

    world.resource_scope(|world, mut streams: Mut<ActiveStreams>| {
        let to_remove = streams
            .iter()
            .filter_map(|(client_id, stream)| {
                let message = stream.0.clone();
                let system_id = stream.1;
                let result = world.run_system_with_input(system_id, message.params);

                let should_remove = match result {
                    Ok(handler_result) => {
                        if let Some(handler_result) = handler_result {
                            let handler_err = handler_result.is_err();
                            let channel_result = message.sender.send_blocking(handler_result);

                            // Remove when the handler return error or channel closed
                            handler_err || channel_result.is_err()
                        } else {
                            false
                        }
                    }
                    Err(error) => {
                        let _ = message.sender.send_blocking(Err(BrpError {
                            code: error_codes::INTERNAL_ERROR,
                            message: format!("Failed to run method handler: {error}"),
                            data: None,
                        }));

                        true
                    }
                };

                should_remove.then_some(*client_id)
            })
            .collect::<Vec<_>>();

        for client_id in to_remove {
            streams.remove(&client_id);
        }
    });
}

pub async fn process_brp_websocket(
    mut request: Request<Incoming>,
    request_sender: &Sender<BrpStreamMessage>,
    client_id: BrpClientId,
) -> AnyhowResult<Response<Full<Bytes>>> {
    let (response, websocket) = hyper_tungstenite::upgrade(&mut request, None)?;
    let body = match validate_websocket_request(&request) {
        Ok(body) => body,
        Err(err) => {
            let response = serde_json::to_string(&BrpError {
                code: error_codes::INVALID_REQUEST,
                message: format!("{err}"),
                data: None,
            })?;

            return Ok(Response::new(Full::new(response.into_bytes().into())));
        }
    };

    IoTaskPool::get()
        .spawn(process_websocket_stream(
            websocket,
            request_sender.clone(),
            body,
            client_id,
        ))
        .detach();

    Ok(response)
}

async fn process_websocket_stream(
    ws: HyperWebsocket,
    request_sender: Sender<BrpStreamMessage>,
    request: BrpRequest,
    client_id: BrpClientId,
) -> AnyhowResult<()> {
    let ws = ws.await?;

    let (write_stream, read_stream) = ws.split();

    let (result_sender, result_receiver) = channel::bounded(1);

    let id = request.id.clone();

    IoTaskPool::get()
        .spawn(send_stream_response(write_stream, result_receiver, id))
        .detach();

    send_stream_message(
        read_stream,
        request_sender.clone(),
        request,
        result_sender,
        client_id,
    )
    .await;

    Ok(())
}

fn validate_websocket_request(request: &Request<Incoming>) -> AnyhowResult<BrpRequest> {
    let body = request
        .uri()
        .query()
        .map(|query| {
            // Simple query string parsing
            let mut map = HashMap::new();
            for pair in query.split('&') {
                let mut it = pair.split('=').take(2);
                let (Some(k), Some(v)) = (it.next(), it.next()) else {
                    continue;
                };
                map.insert(k, v);
            }
            map
        })
        .and_then(|query| query.get("body").cloned())
        .ok_or_else(|| anyhow::anyhow!("Missing body"))?;

    let body = urlencoding::decode(body)?.into_owned();
    let batch = serde_json::from_str(&body).map_err(|err| anyhow::anyhow!(err))?;

    let body = match batch {
        BrpBatch::Batch(_vec) => {
            anyhow::bail!("Batch requests are not supported for streaming")
        }
        BrpBatch::Single(value) => value,
    };

    match serde_json::from_value::<BrpRequest>(body) {
        Ok(req) => {
            if req.jsonrpc != "2.0" {
                anyhow::bail!("JSON-RPC request requires `\"jsonrpc\": \"2.0\"`")
            }

            Ok(req)
        }
        Err(err) => anyhow::bail!(err),
    }
}

async fn send_stream_message(
    mut stream: SplitStream<HyperWebsocketStream>,
    sender: Sender<BrpStreamMessage>,
    request: BrpRequest,
    result_sender: Sender<BrpResult>,
    client_id: BrpClientId,
) {
    let _ = sender
        .send(BrpStreamMessage {
            client_id,
            kind: BrpStreamMessageKind::Connect(BrpMessage {
                method: request.method,
                params: request.params,
                sender: result_sender,
            }),
        })
        .await;
    while let Some(message) = stream.next().await {
        match message {
            Ok(Message::Close(_)) | Err(_) => break,
            _ => {}
        }
    }
    let _ = sender
        .send(BrpStreamMessage {
            client_id,
            kind: BrpStreamMessageKind::Disconnect,
        })
        .await;
}

async fn send_stream_response(
    mut stream: SplitSink<HyperWebsocketStream, Message>,
    result_receiver: Receiver<BrpResult>,
    id: Option<Value>,
) -> AnyhowResult<()> {
    while let Ok(result) = result_receiver.recv().await {
        let response = serde_json::to_string(&BrpResponse::new(id.clone(), result))?;
        stream.send(Message::text(response)).await?;
    }

    Ok(())
}
