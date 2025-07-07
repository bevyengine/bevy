//! A simple command line client that allows issuing queries to a remote Bevy
//! app via the BRP.
//! This example requires the `bevy_remote` feature to be enabled.
//! You can run it with the following command:
//! ```text
//! cargo run --example client --features="bevy_remote"
//! ```
//! This example assumes that the `server` example is running on the same machine.

use std::any::type_name;

use anyhow::Result as AnyhowResult;
use bevy::{
    ecs::hierarchy::ChildOf,
    prelude::info,
    remote::{
        builtin_methods::{
            BrpQuery, BrpQueryFilter, BrpQueryParams, ComponentSelector, BRP_QUERY_METHOD,
        },
        http::{DEFAULT_ADDR, DEFAULT_PORT},
        BrpRequest,
    },
    transform::components::Transform,
};

/// The application entry point.
fn main() -> AnyhowResult<()> {
    // Create the URL. We're going to need it to issue the HTTP request.
    let host_part = format!("{DEFAULT_ADDR}:{DEFAULT_PORT}");
    let url = format!("http://{host_part}/");
    // Creates a request to get all Transform components from the remote Bevy app.
    // This request will return all entities that have a Transform component.
    run_transform_only_query(&url)?;

    // Create a query that only returns root entities - ie, entities that do not
    // have a parent.
    run_query_root_entities(&url)?;

    // Create a query all request to send to the remote Bevy app.
    // This request will return all entities in the app, their components, and their
    // component values.
    run_query_all_components_and_entities(&url)?;

    Ok(())
}

fn run_query_all_components_and_entities(url: &str) -> Result<(), anyhow::Error> {
    let query_all_req = BrpRequest {
        jsonrpc: String::from("2.0"),
        method: String::from(BRP_QUERY_METHOD),
        id: Some(serde_json::to_value(1)?),
        params: Some(
            serde_json::to_value(BrpQueryParams {
                data: BrpQuery {
                    components: Vec::default(),
                    option: ComponentSelector::All,
                    has: Vec::default(),
                },
                strict: false,
                filter: BrpQueryFilter::default(),
            })
            .expect("Unable to convert query parameters to a valid JSON value"),
        ),
    };
    info!("query_all req: {query_all_req:#?}");
    let query_all_res = ureq::post(url)
        .send_json(query_all_req)?
        .body_mut()
        .read_json::<serde_json::Value>()?;
    info!("{query_all_res:#}");
    Ok(())
}

fn run_transform_only_query(url: &str) -> Result<(), anyhow::Error> {
    let get_transform_request = BrpRequest {
        jsonrpc: String::from("2.0"),
        method: String::from(BRP_QUERY_METHOD),
        id: Some(serde_json::to_value(1)?),
        params: Some(
            serde_json::to_value(BrpQueryParams {
                data: BrpQuery {
                    components: vec![type_name::<Transform>().to_string()],
                    ..Default::default()
                },
                strict: false,
                filter: BrpQueryFilter::default(),
            })
            .expect("Unable to convert query parameters to a valid JSON value"),
        ),
    };
    info!("transform request: {get_transform_request:#?}");
    let res = ureq::post(url)
        .send_json(get_transform_request)?
        .body_mut()
        .read_json::<serde_json::Value>()?;
    info!("{res:#}");
    Ok(())
}

fn run_query_root_entities(url: &str) -> Result<(), anyhow::Error> {
    let get_transform_request = BrpRequest {
        jsonrpc: String::from("2.0"),
        method: String::from(BRP_QUERY_METHOD),
        id: Some(serde_json::to_value(1)?),
        params: Some(
            serde_json::to_value(BrpQueryParams {
                data: BrpQuery {
                    components: Vec::default(),
                    option: ComponentSelector::All,
                    has: Vec::default(),
                },
                strict: false,
                filter: BrpQueryFilter {
                    without: vec![type_name::<ChildOf>().to_string()],
                    with: Vec::default(),
                },
            })
            .expect("Unable to convert query parameters to a valid JSON value"),
        ),
    };
    info!("transform request: {get_transform_request:#?}");
    let res = ureq::post(url)
        .send_json(get_transform_request)?
        .body_mut()
        .read_json::<serde_json::Value>()?;
    info!("{res:#}");
    Ok(())
}
