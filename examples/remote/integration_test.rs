//! Connects to a running Bevy app via the BRP, takes a screenshot, finds a button, and clicks it
//! with `input.click`. It then focuses the button with Tab and activates it with Space, using
//! `input.key`, and asks the app to exit. Run the `app_under_test` example on the same machine,
//! then:
//!
//! ```bash
//! cargo run --example integration_test --features="bevy_remote_client bevy_feathers"
//! ```
//!
//! It exits with an error if the button wasn't clicked after three tries, or wasn't activated from
//! the keyboard.

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
    ecs::entity::Entity,
    feathers::controls::FeathersButton,
    input::{keyboard::KeyCode, mouse::MouseButton},
    platform::collections::HashMap,
    remote::{
        builtin_methods::{
            BrpObserveParams, BrpQuery, BrpQueryFilter, BrpQueryParams, BrpSpawnEntityParams,
            BrpWriteMessageParams, ComponentSelector, BRP_OBSERVE_METHOD, BRP_QUERY_METHOD,
            BRP_SPAWN_ENTITY_METHOD, BRP_WRITE_MESSAGE_METHOD,
        },
        client::BrpClient,
        http::{DEFAULT_ADDR, DEFAULT_PORT},
        input_methods::{
            BrpClickParams, BrpClickResponse, BrpKeyParams, BrpKeyResponse, BRP_CLICK_METHOD,
            BRP_KEY_METHOD,
        },
        BrpRequest,
    },
    render::view::screenshot::{Screenshot, ScreenshotCaptured},
    tasks::block_on,
    ui_widgets::Activate,
    window::Window,
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
    let button_entity = find_button(&client)?;
    println!("Observing Activate on entity {button_entity}...");
    let activations = watch(
        &url,
        BrpObserveParams {
            event: type_name::<Activate>().to_string(),
            entity: Some(button_entity),
        },
    )?;

    // Step 4: Click the button, until it's activated
    let mut clicked = false;
    for attempt in 1..=CLICK_TRIES {
        // The button moves, but the app finds its center when the cursor moves, so the click
        // follows it.
        let frame = click(&client, button_entity)?;
        println!("Click {attempt}/{CLICK_TRIES}, released on frame {frame}");

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

    // Step 5: Focus the button with Tab, and activate it with Space
    // A click that was slow to land could still be on its way; only a new activation counts.
    while activations.try_recv().is_ok() {}
    println!("Pressing Tab, then Space...");
    key(&client, KeyCode::Tab)?;
    let frame = key(&client, KeyCode::Space)?;
    println!("Space released on frame {frame}");
    match activations.recv_timeout(CLICK_TIMEOUT) {
        Ok(_) => println!("Button activated from the keyboard!"),
        Err(RecvTimeoutError::Timeout) => {
            bail!("Button not activated from the keyboard after {CLICK_TIMEOUT:?}")
        }
        Err(RecvTimeoutError::Disconnected) => bail!("Activate stream closed"),
    }

    // Step 6: Ask the app to exit
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

/// Finds the button entity.
fn find_button(client: &BrpClient) -> AnyhowResult<Entity> {
    let button_query = brp_request(
        client,
        BRP_QUERY_METHOD,
        &BrpQueryParams {
            data: BrpQuery {
                components: Vec::default(),
                option: ComponentSelector::default(),
                has: Vec::default(),
            },
            strict: false,
            filter: BrpQueryFilter {
                with: vec![type_name::<FeathersButton>().to_string()],
                ..Default::default()
            },
        },
    )?;

    let button_result = button_query.as_array().expect("Expected result array");
    Ok(serde_json::from_value(button_result[0]["entity"].clone())?)
}

/// Clicks on a UI node with the left mouse button, and returns the frame that reads the release.
///
/// The app moves the cursor to the node's center, then presses and releases the button, each on
/// its own frame, as a window backend would report a real click.
fn click(client: &BrpClient, node: Entity) -> AnyhowResult<u32> {
    let response = brp_request(
        client,
        BRP_CLICK_METHOD,
        &BrpClickParams {
            entity: Some(node),
            window: None,
            position: None,
            button: MouseButton::Left,
            hold: 1,
        },
    )?;
    let response: BrpClickResponse = serde_json::from_value(response)?;
    Ok(response.frame)
}

/// Presses and releases a key in the primary window, and returns the frame that receives the
/// release.
///
/// The app fills in the key's logical key and text from a US keyboard layout.
fn key(client: &BrpClient, key_code: KeyCode) -> AnyhowResult<u32> {
    let response = brp_request(
        client,
        BRP_KEY_METHOD,
        &BrpKeyParams {
            key_code,
            logical_key: None,
            text: None,
            state: None,
            hold: None,
            window: None,
        },
    )?;
    let response: BrpKeyResponse = serde_json::from_value(response)?;
    Ok(response.frame)
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
