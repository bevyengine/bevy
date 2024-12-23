//! Example to get all the fully-qualified paths that the `./client.rs` example is talking about.

use anyhow::Result as AnyhowResult;
use bevy::remote::{
    builtin_methods::BRP_LIST_METHOD, http::DEFAULT_ADDR, http::DEFAULT_PORT, BrpRequest,
};

/// The application entry point.
fn main() -> AnyhowResult<()> {
    // Create the URL. We're going to need it to issue the HTTP request.
    let host_part = format!("{}:{}", DEFAULT_ADDR, DEFAULT_PORT);
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
