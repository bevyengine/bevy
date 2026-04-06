//! An integration test that connects to a running Bevy app via the BRP,
//! finds a button's position, and sends a mouse click to press it.
//!
//! Run with the `bevy_remote` feature enabled:
//! ```bash
//! cargo run --example integration_test --features="bevy_remote"
//! ```
//! This example assumes that the `app_under_test` example is running on the same machine.

use std::any::type_name;

use anyhow::Result as AnyhowResult;
use bevy::{
    remote::{
        builtin_methods::{
            BrpQuery, BrpQueryFilter, BrpQueryParams, BrpWriteMessageParams, ComponentSelector,
            BRP_QUERY_METHOD, BRP_WRITE_MESSAGE_METHOD,
        },
        http::{DEFAULT_ADDR, DEFAULT_PORT},
        BrpRequest,
    },
    ui::{widget::Button, UiGlobalTransform},
    window::{Window, WindowEvent},
};

fn main() -> AnyhowResult<()> {
    let url = format!("http://{DEFAULT_ADDR}:{DEFAULT_PORT}/");

    // Step 1: Find the button entity, and its global transform
    println!("Querying for button entity...");
    let button_query = brp_request(
        &url,
        BRP_QUERY_METHOD,
        1,
        &BrpQueryParams {
            data: BrpQuery {
                components: vec![type_name::<UiGlobalTransform>().to_string()],
                option: ComponentSelector::default(),
                has: Vec::default(),
            },
            strict: false,
            filter: BrpQueryFilter {
                with: vec![type_name::<Button>().to_string()],
                without: Vec::default(),
            },
        },
    )?;

    let button_result = button_query["result"]
        .as_array()
        .expect("Expected result array");
    let button = &button_result[0];

    // UiGlobalTransform wraps an Affine2, serialized as a flat array:
    // [_, _, _, _, translation_x, translation_y]
    // The translation gives the node's center in physical pixels.
    let transform = &button["components"][type_name::<UiGlobalTransform>()];
    let transform_arr = transform.as_array().expect("Expected transform array");
    let phys_x = transform_arr[4].as_f64().unwrap();
    let phys_y = transform_arr[5].as_f64().unwrap();
    println!("Found button at physical ({phys_x}, {phys_y})");

    // Step 2: Find the window entity and scale factor
    println!("Querying for window entity...");
    let window_query = brp_request(
        &url,
        BRP_QUERY_METHOD,
        2,
        &BrpQueryParams {
            data: BrpQuery {
                components: vec![type_name::<Window>().to_string()],
                option: ComponentSelector::default(),
                has: Vec::default(),
            },
            strict: false,
            filter: BrpQueryFilter::default(),
        },
    )?;

    let window_result = window_query["result"]
        .as_array()
        .expect("Expected result array");
    let window = &window_result[0];
    let window_entity = &window["entity"];
    let window_data = &window["components"][type_name::<Window>()];
    let scale_factor = window_data["resolution"]["scale_factor"].as_f64().unwrap();
    println!("Found window entity: {window_entity}, scale_factor: {scale_factor}");

    // Step 3: Convert button center from physical to logical pixels
    let logical_x = phys_x / scale_factor;
    let logical_y = phys_y / scale_factor;
    println!("Clicking at logical position: ({logical_x}, {logical_y})");

    // Step 4: Send CursorMoved via WindowEvent message
    // This lets the picking system know where the pointer is.
    println!("Sending CursorMoved message...");
    brp_request(
        &url,
        BRP_WRITE_MESSAGE_METHOD,
        3,
        &BrpWriteMessageParams {
            message: type_name::<WindowEvent>().to_string(),
            value: Some(serde_json::json!({
                "CursorMoved": {
                    "window": window_entity,
                    "position": [logical_x, logical_y],
                    "delta": null
                }
            })),
        },
    )?;

    // Step 5: Send MouseButtonInput Pressed + Released via WindowEvent messages.
    // The picking system needs both press and release to generate a Pointer<Click>.
    println!("Sending mouse press...");
    brp_request(
        &url,
        BRP_WRITE_MESSAGE_METHOD,
        4,
        &BrpWriteMessageParams {
            message: type_name::<WindowEvent>().to_string(),
            value: Some(serde_json::json!({
                "MouseButtonInput": {
                    "button": "Left",
                    "state": "Pressed",
                    "window": window_entity,
                }
            })),
        },
    )?;

    println!("Sending mouse release...");
    brp_request(
        &url,
        BRP_WRITE_MESSAGE_METHOD,
        5,
        &BrpWriteMessageParams {
            message: type_name::<WindowEvent>().to_string(),
            value: Some(serde_json::json!({
                "MouseButtonInput": {
                    "button": "Left",
                    "state": "Released",
                    "window": window_entity,
                }
            })),
        },
    )?;

    Ok(())
}

fn brp_request(
    url: &str,
    method: &str,
    id: u32,
    params: &impl serde::Serialize,
) -> AnyhowResult<serde_json::Value> {
    let req = BrpRequest {
        method: method.to_string(),
        id: Some(serde_json::to_value(id)?),
        params: Some(serde_json::to_value(params)?),
    };
    Ok(ureq::post(url).send_json(req)?.body_mut().read_json()?)
}
