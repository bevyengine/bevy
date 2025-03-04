//! A simple command line client that allows issuing queries to a remote Bevy
//! app via the BRP.

use anyhow::Result as AnyhowResult;
use argh::FromArgs;
use bevy::remote::{
    builtin_methods::{BrpQuery, BrpQueryFilter, BrpQueryParams, BRP_QUERY_METHOD},
    http::DEFAULT_ADDR,
    http::DEFAULT_PORT,
    BrpRequest,
};

/// Struct containing the command-line arguments that can be passed to this example.
///
/// The components are passed by their full type names positionally, while `host`
/// and `port` are optional arguments which should correspond to those used on
/// the server.
///
/// When running this example in conjunction with the `server` example, the `host`
/// and `port` can be left as their defaults.
///
/// For example, to connect to port 1337 on the default IP address and query for entities
/// with `Transform` components:
/// ```text
/// cargo run --example client -- --port 1337 bevy_transform::components::transform::Transform
/// ```
#[derive(FromArgs)]
struct Args {
    /// the host IP address to connect to
    #[argh(option, default = "DEFAULT_ADDR.to_string()")]
    host: String,
    /// the port to connect to
    #[argh(option, default = "DEFAULT_PORT")]
    port: u16,
    /// the full type names of the components to query for
    #[argh(positional, greedy)]
    components: Vec<String>,
}

/// The application entry point.
fn main() -> AnyhowResult<()> {
    // Parse the arguments.
    let args: Args = argh::from_env();

    // Create the URL. We're going to need it to issue the HTTP request.
    let host_part = format!("{}:{}", args.host, args.port);
    let url = format!("http://{}/", host_part);

    let req = BrpRequest {
        jsonrpc: String::from("2.0"),
        method: String::from(BRP_QUERY_METHOD),
        id: Some(serde_json::to_value(1)?),
        params: Some(
            serde_json::to_value(BrpQueryParams {
                data: BrpQuery {
                    components: args.components,
                    option: Vec::default(),
                    has: Vec::default(),
                },
                strict: false,
                filter: BrpQueryFilter::default(),
            })
            .expect("Unable to convert query parameters to a valid JSON value"),
        ),
    };

    let res = ureq::post(&url)
        .send_json(req)?
        .body_mut()
        .read_json::<serde_json::Value>()?;

    println!("{:#}", res);

    Ok(())
}
