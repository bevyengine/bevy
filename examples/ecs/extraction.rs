//! Demonstrates different ways to extract components to another world.
//!
//! Multiple entities are spawned, each with a different marker component: A, B, C.
//! Each component contains the current elapsed time, updated each frame on the Main World.
//!
//! A is extracted via the automatic `ComponentExtractionPlguin`.
//! B is extracted manually each frame.
//! C is extracted manually when the 'Space' key is pressed.
//!
//! Press 'Enter' to display the state of each component from both worlds.

use bevy::prelude::*;
use bevy_render::{
    extract_component::{ExtractComponent, ExtractComponentPlugin},
    sync_world::{RenderEntity, SyncToRenderWorld},
    Extract, Render, RenderApp,
};

#[derive(Component, Clone, ExtractComponent)]
struct A(pub f32);

// Note that entities need the `SyncToRenderWorld`` component in order to exist in both worlds.
// This requirement is normally added by `ExtractComponentPlugin`, but needs to be added manually if not using that.

#[derive(Component, Clone, ExtractComponent)]
#[require(SyncToRenderWorld)]
struct B(pub f32);

#[derive(Component, Clone, ExtractComponent)]
#[require(SyncToRenderWorld)]
struct C(pub f32);

// Component synced between Worlds, used to control when the states are displayed.
#[derive(Component, Clone, ExtractComponent)]
struct ShouldDisplay(pub bool);

// Message sent to trigger the extraction of C.
#[derive(Message)]
struct ExtractMessage;

fn main() {
    let mut app = App::new();

    app.add_systems(Startup, setup).add_systems(
        Update,
        (
            set_time,
            read_inputs,
            display_state::<0>.run_if(should_display),
        ),
    );

    app.add_plugins((
        DefaultPlugins,
        ExtractComponentPlugin::<ShouldDisplay>::default(),
        ExtractComponentPlugin::<A>::default(),
    ));

    app.add_message::<ExtractMessage>();

    let Some(render_app) = app.get_sub_app_mut(RenderApp) else {
        return;
    };

    render_app.add_systems(ExtractSchedule, extract_components);

    render_app.add_systems(Render, display_state::<1>.run_if(should_display));

    app.run();
}

fn setup(mut commands: Commands) {
    commands.spawn(A(0.0));
    commands.spawn(B(0.0));
    commands.spawn(C(0.0));

    commands.spawn(ShouldDisplay(false));
}

fn set_time(mut a: Single<&mut A>, mut b: Single<&mut B>, mut c: Single<&mut C>, time: Res<Time>) {
    a.0 = time.elapsed_secs();
    b.0 = time.elapsed_secs();
    c.0 = time.elapsed_secs();
}

fn should_display(should_display: Single<&ShouldDisplay>) -> bool {
    return should_display.0;
}

// The WORLD generic is used to avoid duplicating the system for each world.
fn display_state<const WORLD: usize>(
    a: Option<Single<&A>>,
    b: Option<Single<&B>>,
    c: Option<Single<&C>>,
) {
    info!("State of Components in World {WORLD}:");

    info!("A: {:?}", a.map(|a| a.0));
    info!("B: {:?}", b.map(|b| b.0));
    info!("C: {:?}", c.map(|c| c.0));
    info!("");
}

fn read_inputs(
    mut should_display: Single<&mut ShouldDisplay>,
    mut writer: MessageWriter<ExtractMessage>,
    keys: Res<ButtonInput<KeyCode>>,
) {
    should_display.0 = keys.just_pressed(KeyCode::Enter);

    if keys.pressed(KeyCode::Space) {
        writer.write(ExtractMessage);
    }
}

fn extract_components(
    b: Extract<Query<(RenderEntity, &B)>>,
    c: Extract<Query<(RenderEntity, &C)>>,
    reader: Extract<MessageReader<ExtractMessage>>,
    mut commands: Commands,
) {
    for (entity, b) in &b {
        commands.entity(entity).insert(b.clone());
    }

    if !reader.is_empty() {
        for (entity, c) in &c {
            commands.entity(entity).insert(c.clone());
        }
    }
}
