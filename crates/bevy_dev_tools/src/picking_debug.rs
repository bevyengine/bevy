//! Text and on-screen debugging tools

use bevy_app::prelude::*;
use bevy_camera::visibility::Visibility;
use bevy_camera::{Camera, RenderTarget};
use bevy_color::prelude::*;
use bevy_ecs::prelude::*;
use bevy_picking::backend::HitData;
use bevy_picking::hover::HoverMap;
use bevy_picking::pointer::{
    Location, PointerId, PointerInput, PointerLocation, PointerPressState,
};
use bevy_picking::prelude::*;
use bevy_picking::PickingSystems;
use bevy_reflect::prelude::*;
use bevy_text::prelude::*;
use bevy_ui::prelude::*;
use core::cmp::Ordering;
use core::fmt::{Debug, Display, Formatter, Result};
use tracing::{debug, trace};

/// This resource determines the runtime behavior of the debug plugin.
#[derive(Debug, Clone, Copy, Default, Eq, PartialEq, Hash, Resource)]
pub enum DebugPickingMode {
    /// Only log non-noisy events, show the debug overlay.
    Normal,
    /// Log all events, including noisy events like `Move` and `Drag`, show the debug overlay.
    Noisy,
    /// Do not show the debug overlay or log any messages.
    #[default]
    Disabled,
}

impl DebugPickingMode {
    /// A condition indicating the plugin is enabled
    pub fn is_enabled(this: Res<Self>) -> bool {
        matches!(*this, Self::Normal | Self::Noisy)
    }
    /// A condition indicating the plugin is disabled
    pub fn is_disabled(this: Res<Self>) -> bool {
        matches!(*this, Self::Disabled)
    }
    /// A condition indicating the plugin is enabled and in noisy mode
    pub fn is_noisy(this: Res<Self>) -> bool {
        matches!(*this, Self::Noisy)
    }
}

/// Logs events for debugging
///
/// "Normal" events are logged at the `debug` level. "Noisy" events are logged at the `trace` level.
/// See [Bevy's LogPlugin](https://docs.rs/bevy/latest/bevy/log/struct.LogPlugin.html) and [Bevy
/// Cheatbook: Logging, Console Messages](https://bevy-cheatbook.github.io/features/log.html) for
/// details.
///
/// Usually, the default level printed is `info`, so debug and trace messages will not be displayed
/// even when this plugin is active. You can set `RUST_LOG` to change this.
///
/// You can also change the log filter at runtime in your code. The [LogPlugin
/// docs](https://docs.rs/bevy/latest/bevy/log/struct.LogPlugin.html) give an example.
///
/// Use the [`DebugPickingMode`] state resource to control this plugin. Example:
///
/// ```ignore
/// use DebugPickingMode::{Normal, Disabled};
/// app.insert_resource(DebugPickingMode::Normal)
///     .add_systems(
///         PreUpdate,
///         (|mut mode: ResMut<DebugPickingMode>| {
///             *mode = match *mode {
///                 DebugPickingMode::Disabled => DebugPickingMode::Normal,
///                 _ => DebugPickingMode::Disabled,
///             };
///         })
///         .distributive_run_if(bevy::input::common_conditions::input_just_pressed(
///             KeyCode::F3,
///         )),
///     )
/// ```
/// This sets the starting mode of the plugin to [`DebugPickingMode::Disabled`] and binds the F3 key
/// to toggle it.
#[derive(Debug, Default, Clone)]
pub struct DebugPickingPlugin;

impl Plugin for DebugPickingPlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<PointerDebugOverlayOf>()
            .register_type::<PointerDebugOverlay>()
            .init_resource::<DebugPickingMode>()
            .add_systems(
                PreUpdate,
                pointer_debug_visibility.in_set(PickingSystems::PostHover),
            )
            .add_systems(
                PreUpdate,
                (
                    // This leaves room to easily change the log-level associated
                    // with different events, should that be desired.
                    log_message_debug::<PointerInput>.run_if(DebugPickingMode::is_noisy),
                    log_message_debug::<PointerOver>,
                    log_message_debug::<PointerOut>,
                    log_message_debug::<PointerPress>,
                    log_message_debug::<PointerRelease>,
                    log_message_debug::<PointerClick>,
                    log_message_trace::<PointerMove>.run_if(DebugPickingMode::is_noisy),
                    log_message_debug::<PointerDragStart>,
                    log_message_trace::<PointerDrag>.run_if(DebugPickingMode::is_noisy),
                    log_message_debug::<PointerDragEnd>,
                    log_message_debug::<PointerDragEnter>,
                    log_message_trace::<PointerDragOver>.run_if(DebugPickingMode::is_noisy),
                    log_message_debug::<PointerDragLeave>,
                    log_message_debug::<PointerDragDrop>,
                )
                    .distributive_run_if(DebugPickingMode::is_enabled)
                    .in_set(PickingSystems::Last),
            );

        app.add_systems(
            PreUpdate,
            (add_pointer_debug, update_debug_data, debug_draw)
                .chain()
                .distributive_run_if(DebugPickingMode::is_enabled)
                .in_set(PickingSystems::Last),
        );
    }
}

/// Listen for any message and logs it at the debug level
pub fn log_message_debug<M: Message + Debug>(mut events: MessageReader<M>) {
    for event in events.read() {
        debug!("{event:?}");
    }
}

/// Listen for any message and logs it at the trace level
pub fn log_message_trace<M: Message + Debug>(mut events: MessageReader<M>) {
    for event in events.read() {
        trace!("{event:?}");
    }
}

/// Relates a debug overlay entity to the pointer it visualizes.
#[derive(Component, Reflect)]
#[reflect(Component)]
#[relationship(relationship_target = PointerDebugOverlay)]
pub struct PointerDebugOverlayOf {
    /// The pointer visualized by this debug overlay.
    #[relationship]
    pub pointer: Entity,
}

/// Stores the debug overlay entity associated with a pointer.
#[derive(Component, Reflect)]
#[reflect(Component)]
#[relationship_target(relationship = PointerDebugOverlayOf, linked_spawn)]
pub struct PointerDebugOverlay {
    overlay: Entity,
}

impl PointerDebugOverlay {
    /// Returns the entity containing the pointer's debug overlay.
    pub fn overlay(&self) -> Entity {
        self.overlay
    }
}

/// Adds [`PointerDebug`] to pointers automatically.
pub fn add_pointer_debug(
    mut commands: Commands,
    pointers: Query<Entity, (With<PointerId>, Without<PointerDebugOverlay>)>,
) {
    for pointer in &pointers {
        commands.spawn(PointerDebugOverlayOf { pointer });
        commands.entity(pointer).insert(PointerDebug::default());
    }
}

/// Hide pointer debug overlays.
pub fn pointer_debug_visibility(
    debug: Res<DebugPickingMode>,
    mut overlays: Query<&mut Visibility, With<PointerDebugOverlayOf>>,
) {
    let visible = match *debug {
        DebugPickingMode::Disabled => Visibility::Hidden,
        _ => Visibility::Visible,
    };
    for mut vis in &mut overlays {
        *vis = visible;
    }
}

/// Storage for per-pointer debug information.
#[derive(Debug, Component, Clone, Default)]
pub struct PointerDebug {
    /// The pointer location.
    pub location: Option<Location>,

    /// Representation of the different pointer button states.
    pub press: PointerPressState,

    /// List of hit elements to be displayed.
    pub hits: Vec<(String, HitData)>,
}

fn bool_to_icon(f: &mut Formatter, prefix: &str, input: bool) -> Result {
    write!(f, "{prefix}{}", if input { "[X]" } else { "[ ]" })
}

impl Display for PointerDebug {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        if let Some(location) = &self.location {
            writeln!(f, "Location: {:.2?}", location.position)?;
        }
        bool_to_icon(f, "Pressed: ", self.press.is_primary_pressed())?;
        bool_to_icon(f, " ", self.press.is_middle_pressed())?;
        bool_to_icon(f, " ", self.press.is_secondary_pressed())?;
        let mut sorted_hits = self.hits.clone();
        sorted_hits.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(Ordering::Equal));
        for (entity, hit) in sorted_hits.iter() {
            write!(f, "\nEntity: {entity:?}")?;
            if let Some((position, normal)) = hit.position.zip(hit.normal) {
                write!(f, ", Position: {position:.2?}, Normal: {normal:.2?}")?;
            }
            write!(f, ", Depth: {:.2?}", hit.depth)?;
        }

        Ok(())
    }
}

/// Update typed debug data used to draw overlays
pub fn update_debug_data(
    hover_map: Res<HoverMap>,
    entity_names: Query<NameOrEntity>,
    mut pointers: Query<(
        &PointerId,
        &PointerLocation,
        &PointerPressState,
        &mut PointerDebug,
    )>,
) {
    for (id, location, press, mut debug) in &mut pointers {
        *debug = PointerDebug {
            location: location.location().cloned(),
            press: press.to_owned(),
            hits: hover_map
                .get(id)
                .iter()
                .flat_map(|h| h.iter())
                .filter_map(|(e, h)| {
                    if let Ok(entity_name) = entity_names.get(*e) {
                        Some((entity_name.to_string(), h.to_owned()))
                    } else {
                        None
                    }
                })
                .collect(),
        };
    }
}

/// Draw text on each cursor with debug info
pub fn debug_draw(
    mut commands: Commands,
    camera_query: Query<(Entity, &Camera, &RenderTarget)>,
    primary_window: Query<Entity, With<bevy_window::PrimaryWindow>>,
    pointers: Query<(&PointerDebugOverlay, &PointerId, &PointerDebug)>,
    scale: Res<UiScale>,
) {
    for (overlay, id, debug) in &pointers {
        let Some(pointer_location) = &debug.location else {
            continue;
        };
        let text = format!("{id:?}\n{debug}");

        for (camera, _, _) in camera_query.iter().filter(|(_, _, render_target)| {
            render_target
                .normalize(primary_window.single().ok())
                .is_some_and(|target| target == pointer_location.target)
        }) {
            let mut pointer_pos = pointer_location.position;
            if let Some(viewport) = camera_query
                .get(camera)
                .ok()
                .and_then(|(_, camera, _)| camera.logical_viewport_rect())
            {
                pointer_pos -= viewport.min;
            }

            commands
                .entity(overlay.overlay())
                .despawn_related::<Children>()
                .insert((
                    Node {
                        position_type: PositionType::Absolute,
                        left: Val::Px(pointer_pos.x + 5.0) / scale.0,
                        top: Val::Px(pointer_pos.y + 5.0) / scale.0,
                        padding: UiRect::px(10.0, 10.0, 8.0, 6.0),
                        ..Default::default()
                    },
                    BackgroundColor(Color::BLACK.with_alpha(0.75)),
                    GlobalZIndex(i32::MAX),
                    Pickable::IGNORE,
                    UiTargetCamera(camera),
                    children![(Text::new(text.clone()), TextFont::from_font_size(12.0))],
                ));
        }
    }
}
