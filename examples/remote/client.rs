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
    remote::{
        builtin_methods::{
            BrpQuery, BrpQueryFilter, BrpQueryParams, BrpWriteMessageParams, ComponentSelector,
            BRP_QUERY_METHOD, BRP_WRITE_MESSAGE_METHOD,
        },
        http::{DEFAULT_ADDR, DEFAULT_PORT, DEFAULT_RENDER_PORT},
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

    // Run again against the render port
    let host_part2 = format!("{DEFAULT_ADDR}:{DEFAULT_RENDER_PORT}");
    let url2 = format!("http://{host_part2}/");

    run_transform_only_query(&url2)?;
    run_query_root_entities(&url2)?;
    run_query_all_components_and_entities(&url2)?;

    // Send an `AppExit::Success` message to the app to the remote Bevy app.
    // This will make it quit.
    send_app_exit(&url)?;

    Ok(())
}

fn run_query_all_components_and_entities(url: &str) -> Result<(), anyhow::Error> {
    let query_all_req = BrpRequest {
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
    println!("query_all req: {query_all_req:#?}");
    let query_all_res = ureq::post(url)
        .send_json(query_all_req)?
        .body_mut()
        .read_json::<serde_json::Value>()?;
    println!("{query_all_res:#}");
    Ok(())
}

fn run_transform_only_query(url: &str) -> Result<(), anyhow::Error> {
    let get_transform_request = BrpRequest {
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
    println!("transform request: {get_transform_request:#?}");
    let res = ureq::post(url)
        .send_json(get_transform_request)?
        .body_mut()
        .read_json::<serde_json::Value>()?;
    println!("{res:#}");
    Ok(())
}

fn run_query_root_entities(url: &str) -> Result<(), anyhow::Error> {
    let get_transform_request = BrpRequest {
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
    println!("transform request: {get_transform_request:#?}");
    let res = ureq::post(url)
        .send_json(get_transform_request)?
        .body_mut()
        .read_json::<serde_json::Value>()?;
    println!("{res:#}");
    Ok(())
}

fn send_app_exit(url: &str) -> Result<(), anyhow::Error> {
    let write_message_request = BrpRequest {
        method: String::from(BRP_WRITE_MESSAGE_METHOD),
        id: Some(serde_json::to_value(1)?),
        params: Some(
            serde_json::to_value(BrpWriteMessageParams {
                message: "bevy_app::app::AppExit".to_string(),
                value: Some("Success".into()),
            })
            .expect("Unable to convert write message parameters to a valid JSON value"),
        ),
    };
    println!("write message request: {write_message_request:#?}");
    let res = ureq::post(url)
        .send_json(write_message_request)?
        .body_mut()
        .read_json::<serde_json::Value>()?;
    println!("{res:#}");
    Ok(())
}
