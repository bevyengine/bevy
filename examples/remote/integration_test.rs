//! Connects to a running Bevy app via the BRP, takes a screenshot, finds a button's position, and
//! clicks it. It then asks the app to exit. Run the `app_under_test` example on the same machine,
//! then:
//!
//! ```bash
//! cargo run --example integration_test --features="bevy_remote_client bevy_feathers"
//! ```
//!
//! It exits with an error if the button wasn't clicked after three tries.

use std::{
    any::type_name,
    io::BufRead,
    sync::mpsc::{self, Receiver, RecvTimeoutError},
    thread,
    time::{Duration, Instant},
};

use anyhow::{bail, Result as AnyhowResult};
use bevy::{
    app::AppExit,
    feathers::controls::FeathersButton,
    platform::collections::HashMap,
    remote::{
        builtin_methods::{
            BrpObserveParams, BrpQuery, BrpQueryFilter, BrpQueryParams, BrpSpawnEntityParams,
            BrpWriteMessageParams, ComponentSelector, BRP_OBSERVE_METHOD, BRP_QUERY_METHOD,
            BRP_SPAWN_ENTITY_METHOD, BRP_WRITE_MESSAGE_METHOD,
        },
        client::BrpClient,
        http::{DEFAULT_ADDR, DEFAULT_PORT},
        BrpRequest,
    },
    render::view::screenshot::{Screenshot, ScreenshotCaptured},
    tasks::block_on,
    ui::UiGlobalTransform,
    ui_widgets::Activate,
    window::{Window, WindowEvent},
};

/// How long to wait for the app to answer its first request.
const STARTUP_TIMEOUT: Duration = Duration::from_secs(60);
/// How long to wait for the screenshot.
const SCREENSHOT_TIMEOUT: Duration = Duration::from_secs(30);
/// How long to wait for the button to be activated after each click.
const CLICK_TIMEOUT: Duration = Duration::from_secs(3);
/// How many times to try to click the button.
const CLICK_TRIES: u32 = 3;

fn main() -> AnyhowResult<()> {
    let url = format!("http://{DEFAULT_ADDR}:{DEFAULT_PORT}/");
    let client = BrpClient::new(DEFAULT_ADDR.to_string(), DEFAULT_PORT);

    // Step 1: Wait for the app to answer, and find the window entity
    println!("Waiting for the app, and querying for window entity...");
    let window_entity = wait_for_window(&client)?;
    println!("Found window entity: {window_entity}");

    // Step 2: Take a screenshot via BRP
    // The window must be visible (not fully occluded) for the GPU to render content
    // If the window is hidden, the screenshot will be black
    println!("Spawning Screenshot entity...");
    let spawn_response = brp_request(
        &client,
        BRP_SPAWN_ENTITY_METHOD,
        &BrpSpawnEntityParams {
            components: HashMap::from([(
                type_name::<Screenshot>().to_string(),
                serde_json::json!({"Window": "Primary"}),
            )]),
        },
    )?;
    let screenshot_entity = &spawn_response["entity"];

    println!("Observing ScreenshotCaptured on entity {screenshot_entity}...");
    let screenshots = watch(
        &url,
        BrpObserveParams {
            event: type_name::<ScreenshotCaptured>().to_string(),
            entity: Some(serde_json::from_value(screenshot_entity.clone())?),
        },
    )?;

    println!("Waiting for screenshot capture...");
    let events = match screenshots.recv_timeout(SCREENSHOT_TIMEOUT) {
        Ok(events) => events,
        Err(RecvTimeoutError::Timeout) => bail!("No screenshot after {SCREENSHOT_TIMEOUT:?}"),
        Err(RecvTimeoutError::Disconnected) => bail!("Screenshot stream closed"),
    };
    let image_data = &events[0]["image"];
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

    // Step 3: Find the button entity, and watch for it being activated
    println!("Querying for button entity...");
    let (button_entity, _) = find_button(&client)?;
    println!("Observing Activate on entity {button_entity}...");
    let activations = watch(
        &url,
        BrpObserveParams {
            event: type_name::<Activate>().to_string(),
            entity: Some(serde_json::from_value(button_entity)?),
        },
    )?;

    // Step 4: Click the button, until it's activated
    let mut clicked = false;
    for attempt in 1..=CLICK_TRIES {
        // The button moves, so find it again before each click
        let (_, (phys_x, phys_y)) = find_button(&client)?;
        println!("Click {attempt}/{CLICK_TRIES} at physical ({phys_x}, {phys_y})");
        click(&client, &window_entity, phys_x, phys_y)?;

        match activations.recv_timeout(CLICK_TIMEOUT) {
            Ok(_) => {
                println!("Button activated!");
                clicked = true;
                break;
            }
            Err(RecvTimeoutError::Timeout) => {
                println!("Button not activated after {CLICK_TIMEOUT:?}");
            }
            Err(RecvTimeoutError::Disconnected) => bail!("Activate stream closed"),
        }
    }
    if !clicked {
        bail!("Button not activated after {CLICK_TRIES} clicks");
    }

    // Step 5: Ask the app to exit
    println!("Asking the app to exit...");
    brp_request(
        &client,
        BRP_WRITE_MESSAGE_METHOD,
        &BrpWriteMessageParams {
            message: type_name::<AppExit>().to_string(),
            value: Some(serde_json::json!("Success")),
        },
    )?;

    Ok(())
}

/// Queries for the window entity until the app answers, for up to [`STARTUP_TIMEOUT`].
fn wait_for_window(client: &BrpClient) -> AnyhowResult<serde_json::Value> {
    let start = Instant::now();
    loop {
        let window_query = brp_request(
            client,
            BRP_QUERY_METHOD,
            &BrpQueryParams {
                data: BrpQuery {
                    components: vec![type_name::<Window>().to_string()],
                    option: ComponentSelector::default(),
                    has: Vec::default(),
                },
                strict: false,
                filter: BrpQueryFilter::default(),
            },
        );
        match window_query {
            Ok(window_query) => {
                let window_result = window_query.as_array().expect("Expected result array");
                if let Some(window) = window_result.first() {
                    return Ok(window["entity"].clone());
                }
            }
            Err(err) if start.elapsed() >= STARTUP_TIMEOUT => {
                bail!("App didn't answer after {STARTUP_TIMEOUT:?}: {err}");
            }
            Err(_) => {}
        }
        if start.elapsed() >= STARTUP_TIMEOUT {
            bail!("No window after {STARTUP_TIMEOUT:?}");
        }
        thread::sleep(Duration::from_millis(500));
    }
}

/// Finds the button entity, and the physical position of its center.
fn find_button(client: &BrpClient) -> AnyhowResult<(serde_json::Value, (f64, f64))> {
    let button_query = brp_request(
        client,
        BRP_QUERY_METHOD,
        &BrpQueryParams {
            data: BrpQuery {
                components: vec![type_name::<UiGlobalTransform>().to_string()],
                option: ComponentSelector::default(),
                has: Vec::default(),
            },
            strict: false,
            filter: BrpQueryFilter {
                with: vec![type_name::<FeathersButton>().to_string()],
                without: Vec::default(),
            },
        },
    )?;

    let button_result = button_query.as_array().expect("Expected result array");
    let button = &button_result[0];

    // UiGlobalTransform wraps an Affine2, serialized as a flat array:
    // [_, _, _, _, translation_x, translation_y]
    // The translation gives the node's center in physical pixels.
    let transform = &button["components"][type_name::<UiGlobalTransform>()];
    let transform_arr = transform.as_array().expect("Expected transform array");
    let phys_x = transform_arr[4].as_f64().unwrap();
    let phys_y = transform_arr[5].as_f64().unwrap();
    Ok((button["entity"].clone(), (phys_x, phys_y)))
}

/// Clicks at a physical position in the window.
fn click(
    client: &BrpClient,
    window_entity: &serde_json::Value,
    phys_x: f64,
    phys_y: f64,
) -> AnyhowResult<()> {
    // Send CursorMoved via WindowEvent message, in physical pixels like the button position.
    // This lets the picking system know where the pointer is.
    brp_request(
        client,
        BRP_WRITE_MESSAGE_METHOD,
        &BrpWriteMessageParams {
            message: type_name::<WindowEvent>().to_string(),
            value: Some(serde_json::json!({
                "CursorMoved": {
                    "window": window_entity,
                    "physical_position": [phys_x, phys_y]
                }
            })),
        },
    )?;

    // Send MouseButtonInput Pressed + Released via WindowEvent messages.
    // The picking system needs both press and release to generate a PointerClick.
    for state in ["Pressed", "Released"] {
        brp_request(
            client,
            BRP_WRITE_MESSAGE_METHOD,
            &BrpWriteMessageParams {
                message: type_name::<WindowEvent>().to_string(),
                value: Some(serde_json::json!({
                    "MouseButtonInput": {
                        "button": "Left",
                        "state": state,
                        "window": window_entity,
                    }
                })),
            },
        )?;
    }

    Ok(())
}

/// Starts watching for an event, and returns a channel receiving the events for each frame they
/// were observed in. The channel is closed if the stream fails or ends.
fn watch(url: &str, params: BrpObserveParams) -> AnyhowResult<Receiver<Vec<serde_json::Value>>> {
    let request = BrpRequest {
        method: BRP_OBSERVE_METHOD.to_string(),
        id: Some(serde_json::to_value(0)?),
        params: Some(serde_json::to_value(params)?),
    };
    let url = url.to_string();
    let (sender, receiver) = mpsc::channel();
    thread::spawn(move || {
        if let Err(err) = read_watch_stream(&url, request, sender) {
            eprintln!("Watching failed: {err}");
        }
    });
    Ok(receiver)
}

/// Sends a watching request, and forwards the events of each server-sent event to `sender`.
fn read_watch_stream(
    url: &str,
    request: BrpRequest,
    sender: mpsc::Sender<Vec<serde_json::Value>>,
) -> AnyhowResult<()> {
    let response = ureq::post(url).send_json(request)?;
    let reader = std::io::BufReader::new(response.into_body().into_reader());
    for line in reader.lines() {
        let line = line?;
        let Some(json_str) = line.strip_prefix("data: ") else {
            continue;
        };
        let response: serde_json::Value = serde_json::from_str(json_str)?;
        if let Some(error) = response.get("error") {
            bail!("Observe error: {error}");
        }
        if let Some(result) = response.get("result") {
            let events = result.as_array().expect("Expected events array").clone();
            if sender.send(events).is_err() {
                break;
            }
        }
    }
    Ok(())
}

fn brp_request(
    client: &BrpClient,
    method: &str,
    params: &impl serde::Serialize,
) -> AnyhowResult<serde_json::Value> {
    let params = serde_json::to_value(params)?;
    Ok(block_on(client.call(method, Some(params)))?)
}
