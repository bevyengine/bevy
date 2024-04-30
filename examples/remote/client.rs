//! A simple command line client that allows issuing queries to a remote Bevy
//! app via the BRP.

use std::process;

use argh::FromArgs;
use bevy::prelude::default;
use bevy::remote::{BrpQuery, BrpRequest, DEFAULT_PORT};
use http_body_util::BodyExt as _;
use hyper::client::conn::http1;
use hyper::header::HOST;
use hyper::Request;
use hyper_util::rt::TokioIo;
use serde_json::Value;
use tokio::net::TcpStream;
use tokio::task;

/// TODO
#[derive(FromArgs)]
struct Args {
    /// the host to connect to
    #[argh(option, default = "\"127.0.0.1\".to_owned()")]
    host: String,
    /// the port to connect to
    #[argh(option, default = "DEFAULT_PORT")]
    port: u16,
    /// the full type names of the components to query for
    #[argh(positional, greedy)]
    components: Vec<String>,
}

/// The application entry point.
#[tokio::main]
async fn main() {
    // Parse the arguments.
    let args: Args = argh::from_env();

    // Create the URL. We're going to need it to issue the HTTP request.
    let host_part = format!("{}:{}", args.host, args.port);
    let url = format!("https://{}/", host_part)
        .parse::<hyper::Uri>()
        .unwrap();

    // Create our Tokio stream.
    let stream = TcpStream::connect(host_part).await.unwrap();
    let io = TokioIo::new(stream);

    // Create a HTTP 1.x connection.
    let (mut sender, connection) = http1::handshake::<_, String>(io).await.unwrap();

    // Build our BRP request. Include the full type names of all the components,
    // as specified on the command line.
    let brp_request = BrpRequest::Query {
        data: BrpQuery {
            components: args.components,
            ..default()
        },
        filter: default(),
    };

    let mut brp_request = match serde_json::to_value(&brp_request) {
        Ok(request) => request,
        Err(error) => die(&format!("Failed to serialize request: {}", error)),
    };

    // We need to set the `id` field so that it can be echoed back to us. Just
    // set it to 0.
    brp_request
        .as_object_mut()
        .expect("Request must be an object")
        .insert("id".to_owned(), (0).into());

    // Connect.
    task::spawn(async move {
        if let Err(error) = connection.await {
            die(&format!("Failed to connect: {}", error));
        }
    });

    // We're connected, so build the HTTP request.
    let authority = url.authority().unwrap();
    let http_request = Request::builder()
        .uri(&url)
        .header(HOST, authority.as_str())
        .body(brp_request.to_string())
        .unwrap();

    let response = match sender.send_request(http_request).await {
        Ok(response) => response,
        Err(error) => die(&format!("Failed to send request: {}", error)),
    };

    let body = match response.collect().await {
        Ok(body) => body.to_bytes(),
        Err(error) => die(&format!("Failed to receive data: {}", error)),
    };

    let response: Value = match serde_json::from_slice(&body) {
        Ok(response) => response,
        Err(error) => die(&format!("Failed to parse response as JSON: {}", error)),
    };

    // Just print the JSON to stdout.
    println!("{}", serde_json::to_string_pretty(&response).unwrap());
}

/// Exits with an error message.
fn die(message: &str) -> ! {
    eprintln!("{}", message);
    process::exit(1);
}
