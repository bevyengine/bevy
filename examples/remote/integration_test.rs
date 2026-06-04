//! An integration test that connects to a running Bevy app via the BRP,
//! finds a button's position, and sends a mouse click to press it.
//!
//! Run with the `bevy_remote` feature enabled:
//! ```bash
//! cargo run --example integration_test --features="bevy_remote"
//! ```
//! This example assumes that the `app_under_test` example is running on the same machine.

use std::{any::type_name, io::BufRead};

use anyhow::Result as AnyhowResult;
use bevy::{
    platform::collections::HashMap,
    remote::{
        builtin_methods::{
            BrpObserveParams, BrpQuery, BrpQueryFilter, BrpQueryParams, BrpSpawnEntityParams,
            BrpWriteMessageParams, ComponentSelector, BRP_OBSERVE_METHOD, BRP_QUERY_METHOD,
            BRP_SPAWN_ENTITY_METHOD, BRP_WRITE_MESSAGE_METHOD,
        },
        http::{DEFAULT_ADDR, DEFAULT_PORT},
        BrpRequest,
    },
    render::view::screenshot::{Screenshot, ScreenshotCaptured},
    ui::{widget::Button, UiGlobalTransform},
    window::{Window, WindowEvent},
};

fn main() -> AnyhowResult<()> {
    let url = format!("http://{DEFAULT_ADDR}:{DEFAULT_PORT}/");

    // Step 1: Take a screenshot via BRP
    // The window must be visible (not fully occluded) for the GPU to render content
    // If the window is hidden, the screenshot will be black
    println!("Spawning Screenshot entity...");
    let spawn_response = brp_request(
        &url,
        BRP_SPAWN_ENTITY_METHOD,
        1,
        &BrpSpawnEntityParams {
            components: HashMap::from([(
                type_name::<Screenshot>().to_string(),
                serde_json::json!({"Window": "Primary"}),
            )]),
        },
    )?;
    let screenshot_entity = &spawn_response["result"]["entity"];

    println!("Observing ScreenshotCaptured on entity {screenshot_entity}...");
    let observe_response = ureq::post(&url).send_json(BrpRequest {
        method: BRP_OBSERVE_METHOD.to_string(),
        id: Some(serde_json::to_value(2)?),
        params: Some(serde_json::to_value(BrpObserveParams {
            event: type_name::<ScreenshotCaptured>().to_string(),
            entity: Some(serde_json::from_value(screenshot_entity.clone())?),
        })?),
    })?;

    println!("Waiting for screenshot capture...");
    let reader = std::io::BufReader::new(observe_response.into_body().into_reader());
    for line in reader.lines() {
        let line = line?;
        if let Some(json_str) = line.strip_prefix("data: ") {
            let response: serde_json::Value = serde_json::from_str(json_str)?;
            if let Some(error) = response.get("error") {
                anyhow::bail!("Observe error: {error}");
            }
            if let Some(result) = response.get("result") {
                let events = result.as_array().expect("Expected events array");
                let event = &events[0];

                let image_data = &event["image"];
                let width = image_data["texture_descriptor"]["size"]["width"]
                    .as_u64()
                    .unwrap();
                let height = image_data["texture_descriptor"]["size"]["height"]
                    .as_u64()
                    .unwrap();
                println!("Screenshot captured! Image size: {width}x{height}");

                let image: bevy::image::Image = serde_json::from_value(image_data.clone())?;
                let dyn_img = image
                    .try_into_dynamic()
                    .expect("Failed to convert screenshot to dynamic image");
                let path = "screenshot.png";
                dyn_img.to_rgb8().save(path)?;
                println!("Screenshot saved to {path}");
                break;
            }
        }
    }

    // Step 2: Find the button entity, and its global transform
    println!("Querying for button entity...");
    let button_query = brp_request(
        &url,
        BRP_QUERY_METHOD,
        3,
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

    // Step 3: Find the window entity and scale factor
    println!("Querying for window entity...");
    let window_query = brp_request(
        &url,
        BRP_QUERY_METHOD,
        4,
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

    // Step 4: Convert button center from physical to logical pixels
    let logical_x = phys_x / scale_factor;
    let logical_y = phys_y / scale_factor;
    println!("Clicking at logical position: ({logical_x}, {logical_y})");

    // Step 5: Send CursorMoved via WindowEvent message
    // This lets the picking system know where the pointer is.
    println!("Sending CursorMoved message...");
    brp_request(
        &url,
        BRP_WRITE_MESSAGE_METHOD,
        5,
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

    // Step 6: Send MouseButtonInput Pressed + Released via WindowEvent messages.
    // The picking system needs both press and release to generate a Pointer<Click>.
    println!("Sending mouse press...");
    brp_request(
        &url,
        BRP_WRITE_MESSAGE_METHOD,
        6,
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
        7,
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
