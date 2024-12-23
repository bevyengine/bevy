//! Example to get all the fully-qualified paths that the `./client.rs` example is talking about.

use anyhow::Result as AnyhowResult;
use argh::FromArgs;
use bevy::remote::{
    builtin_methods::{
        BrpQuery, BrpQueryFilter, BrpQueryParams, BRP_LIST_METHOD, BRP_QUERY_METHOD,
    },
    http::DEFAULT_ADDR,
    http::DEFAULT_PORT,
    BrpRequest,
};

/// Struct containing the command-line arguments that can be passed to this example.
///
/// When running this example in conjunction with the `server` example, the `host`
/// and `port` can be left as their defaults.
///
/// For example, to connect to port 1337 on the default IP address and query for entities
/// with `Transform` components:
/// ```text
/// cargo run --example client -- --port 1337 | jq
/// ```
/// NOTE: the `jq` is optional, but if you're on linux/mac and have it installed, it'll improve your terminal experience.
#[derive(FromArgs)]
struct Args {
    /// the host IP address to connect to
    #[argh(option, default = "DEFAULT_ADDR.to_string()")]
    host: String,
    /// the port to connect to
    #[argh(option, default = "DEFAULT_PORT")]
    port: u16,
}

/// The application entry point.
fn main() -> AnyhowResult<()> {
    // Parse the arguments.
    let args: Args = argh::from_env();

    // Create the URL. We're going to need it to issue the HTTP request.
    let host_part = format!("{}:{}", args.host, args.port);
    let url = format!("http://{}/", host_part);

    // If you pass no params, you'll be able to see all the optinos for arguments you can pass to the `./client.rs` remote example.
    let req = BrpRequest {
        jsonrpc: "2.0".to_string(),
        method: BRP_LIST_METHOD.to_string(),
        id: Some(ureq::json!(1)),
        params: None,
    };

    let res = ureq::post(&url)
        .send_json(req)?
        .into_json::<serde_json::Value>()?;

    println!("{:#}", res);

    Ok(())
}
