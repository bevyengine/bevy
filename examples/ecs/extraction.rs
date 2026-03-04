//! Demonstrates different ways to extract components to another world.
//!
//! Multiple entities are spawned, each with a different marker component: A, B, C.
//! Each component contains the current elapsed time, updated each frame on the Main World.

use bevy::prelude::*;
use bevy::render::{
    extract_component::{ExtractComponent, ExtractComponentPlugin},
    sync_world::{RenderEntity, SyncToRenderWorld},
    Extract, Render, RenderApp,
};

// The A component is extracted automatically through `ExtractComponentPlugin`. For this,
// it is required to implement `ExtractComponent`. You can do a custom implementation if you wish to
// do a custom extraction instead of just cloning the entire component.
//
// To be noted that the `SyncToRenderWorld` component, which spawns the corresponding entity on the Render World,
// is automatically added as a requirement through the `ExtractComponentPlugin`.
#[derive(Component, Clone, ExtractComponent)]
struct A(pub f32);

// The B component is extracted manually inside the `extract_components` system.
// `SyncToRenderWorld` ensures that an equivalent entity will be spawned in the Render World
// and the two will be associated in the Extract schedule through the `RenderEntity` component.
#[derive(Component, Clone)]
#[require(SyncToRenderWorld)]
struct B(pub f32);

// The C component is the same B, but it only extracts when the `Space` key is pressed.
#[derive(Component, Clone)]
#[require(SyncToRenderWorld)]
struct C(pub f32);

// Message sent when the `Space` key is pressed, causing the extraction of C.
#[derive(Message)]
struct ExtractMessage;

// Resource inserted in each World, used to display its name.
#[derive(Resource)]
struct WorldName(pub String);

fn main() {
    let mut app = App::new();

    // Main World
    app.insert_resource(WorldName("Main World".into()));

    app.add_systems(Startup, setup)
        .add_systems(Update, (set_time, trigger_extraction, display_state));

    app.add_plugins((
        DefaultPlugins,
        // Plugin for automatically extracting A.
        ExtractComponentPlugin::<A>::default(),
    ));

    app.add_message::<ExtractMessage>();

    let Some(render_app) = app.get_sub_app_mut(RenderApp) else {
        return;
    };

    // Render World
    render_app.insert_resource(WorldName("Render World".into()));

    render_app
        .add_systems(ExtractSchedule, extract_components)
        .add_systems(Render, display_state);

    app.run();
}

// Spawns the components on the Main World. Runs on Startup.
fn setup(mut commands: Commands, time: Res<Time>) {
    commands.spawn(A(time.elapsed_secs()));
    commands.spawn(B(time.elapsed_secs()));
    commands.spawn(C(time.elapsed_secs()));
}

// Sets the elapsed time on each of the components on the Main World. Runs each frame.
fn set_time(mut a: Single<&mut A>, mut b: Single<&mut B>, mut c: Single<&mut C>, time: Res<Time>) {
    a.0 = time.elapsed_secs();
    b.0 = time.elapsed_secs();
    c.0 = time.elapsed_secs();
}

// Displays the values from each of the components. The same system is used for both Worlds.
fn display_state(
    a: Option<Single<&A>>,
    b: Option<Single<&B>>,
    c: Option<Single<&C>>,

    // Resource used to debug the name of the World.
    world_name: Res<WorldName>,
) {
    let (a, b, c) = (a.map(|a| a.0), b.map(|b| b.0), c.map(|c| c.0));
    info!(?a, ?b, ?c, "{}", world_name.0);
}

// Writes a message when the `Space` key is pressed, which is later read by the `extract_components` system.
fn trigger_extraction(mut writer: MessageWriter<ExtractMessage>, keys: Res<ButtonInput<KeyCode>>) {
    if keys.pressed(KeyCode::Space) {
        writer.write(ExtractMessage);
    }
}

// System that Extracts B each frame, and C only when the `Space` key was just pressed (see the `trigger_extraction` system).
// Extraction is done by inserting a clone of the component on the corresponding Render World entity.
fn extract_components(
    b: Extract<Query<(RenderEntity, &B)>>,
    c: Extract<Query<(RenderEntity, &C)>>,
    mut reader: Extract<MessageReader<ExtractMessage>>,
    mut commands: Commands,
) {
    for (entity, b) in &b {
        commands.entity(entity).insert(b.clone());
    }

    if !reader.is_empty() {
        for (entity, c) in &c {
            commands.entity(entity).insert(c.clone());
        }
        reader.clear();
    }
}
