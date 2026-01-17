//! Overlay showing diagnostics
//!
//! The window can be created using the [`DiagnosticsOverlay`] component

use alloc::borrow::Cow;
use core::time::Duration;

use bevy_app::prelude::*;
use bevy_color::{palettes, prelude::*};
use bevy_diagnostic::{Diagnostic, DiagnosticPath, DiagnosticsStore, FrameTimeDiagnosticsPlugin};
use bevy_ecs::{prelude::*, relationship::Relationship};
use bevy_pbr::{diagnostic::MaterialAllocatorDiagnosticPlugin, StandardMaterial};
use bevy_picking::prelude::*;
use bevy_render::diagnostic::MeshAllocatorDiagnosticPlugin;
use bevy_text::prelude::*;
use bevy_time::common_conditions::on_timer;
use bevy_ui::prelude::*;

/// Initial offset from the top left corner of the window
/// for the diagnostics overlay
const INITIAL_OFFSET: Val = Val::Px(32.);
/// Alpha value for [`BackgroundColor`] of the overlay
const BACKGROUND_COLOR_ALPHA: f32 = 0.75;
/// Row and column gap for the diagnostics overlay
const ROW_COLUMN_GAP: Val = Val::Px(4.);
/// Padding for cels of the diagnostics overlay
const DEFAULT_PADDING: UiRect = UiRect::all(Val::Px(4.));
/// Initial Z-index for the [`DiagnosticsOverlayPlane`]
pub const INITIAL_DIAGNOSTICS_OVERLAY_PLANE_Z_INDEX: GlobalZIndex = GlobalZIndex(1_000_000);

/// Diagnostics overlay displays on a draggable and collapsable window
/// statistics stored on the [`DiagnosticStore`]. Spawining an entity
/// with this component will create the window for you. Some presets
/// are also provided.
///
/// ```
/// # use bevy_dev_tools::diagnostics_overlay::{DiagnosticsOverlay, DiagnosticOverlayItem, DiagnosticOverlayStatistic};
/// # use bevy_ecs::prelude::{Commands, World};
/// # use bevy_diagnostic::DiagnosticPath;
/// # let mut world = World::new();
/// # let mut commands = world.commands();
/// // Spawning an overlay window from the struct
/// commands.spawn(DiagnosticsOverlay {
///     title: "Fps".into(),
///     diagnostic_overlay_items: vec![DiagnosticPath::new("fps").into()]
/// });
/// // Spawning an overlay window from the `new` method
/// commands.spawn(DiagnosticsOverlay::new(
///     "Fps",
///     vec![DiagnosticPath::new("fps").into()]
/// ));
/// // Spawning an overlay window from the `new` method using a different statistic
/// commands.spawn(DiagnosticsOverlay::new(
///     "Fps",
///     vec![DiagnosticOverlayItem {
///         path: DiagnosticPath::new("fps"),
///         statistic: DiagnosticOverlayStatistic::Value
///     }]
/// ));
/// // Spawning an overlay window from the `fps` preset
/// commands.spawn(DiagnosticsOverlay::fps());
/// ```
///
/// A [`DiagnosticsOverlay`] entity will be managed by [`DiagnosticsOverlayPlugin`],
/// and be added as a child of the [`DiagnosticsOverlayPlane`].
///
/// If any value is showing as `Missing`, means that the [`DiagnosticPath`] is not registered,
/// so make sure that the plugin that writes to it is properly set up.
#[derive(Component)]
pub struct DiagnosticsOverlay {
    /// Title that will appear on the overlay window
    pub title: Cow<'static, str>,
    /// Items that will appear on this overlay window
    pub diagnostic_overlay_items: Vec<DiagnosticOverlayItem>,
}

impl DiagnosticsOverlay {
    /// Creates a new instance of a [`DiagnosticsOverlay`]
    pub fn new(
        title: impl Into<Cow<'static, str>>,
        diagnostic_paths: Vec<DiagnosticOverlayItem>,
    ) -> Self {
        Self {
            title: title.into(),
            diagnostic_overlay_items: diagnostic_paths,
        }
    }

    /// Create a [`DiagnosticsOverlay`] with the diagnostcs from [`FrameTimeDiagnosticsPlugin`]
    pub fn fps() -> Self {
        Self {
            title: Cow::Owned("Fps".to_owned()),
            diagnostic_overlay_items: vec![
                FrameTimeDiagnosticsPlugin::FPS.into(),
                FrameTimeDiagnosticsPlugin::FRAME_TIME.into(),
                FrameTimeDiagnosticsPlugin::FRAME_COUNT.into(),
            ],
        }
    }

    /// Create a [`DiagnosticsOverlay`] with the diagnostics from
    /// [`MaterialAllocatorDiagnosticPlugin`] of [`StandardMaterial`] and
    /// [`MeshAllocatorDiagnosticPlugin`]
    pub fn mesh_and_standard_material() -> Self {
        Self {
            title: Cow::Owned("Mesh and standard materials".to_owned()),
            diagnostic_overlay_items: vec![
                MaterialAllocatorDiagnosticPlugin::<StandardMaterial>::slabs_diagnostic_path()
                    .into(),
                MaterialAllocatorDiagnosticPlugin::<StandardMaterial>::slabs_size_diagnostic_path()
                    .into(),
                MaterialAllocatorDiagnosticPlugin::<StandardMaterial>::allocations_diagnostic_path(
                )
                .into(),
                MeshAllocatorDiagnosticPlugin::slabs_diagnostic_path()
                    .clone()
                    .into(),
                MeshAllocatorDiagnosticPlugin::slabs_size_diagnostic_path()
                    .clone()
                    .into(),
                MeshAllocatorDiagnosticPlugin::allocations_diagnostic_path()
                    .clone()
                    .into(),
            ],
        }
    }
}

/// Marker for the UI root that will hold all of the [`DiagnosticsOverlay`]
/// entities.
///
/// Initially the [`DiagnosticsOverlayPlane`] will be positioned at the
/// [`GlobalZIndex`] of [`INITIAL_DIAGNOSTICS_OVERLAY_PLANE_Z_INDEX`].
/// You are free to edit the z index of the plane or have your ui hierarchies
/// be relative to it.
#[derive(Component)]
pub struct DiagnosticsOverlayPlane;

/// An item to be displayed on the overlay.
///
/// Items built using `From<DiagnosticPath>` will use
/// [`DiagnosticOverlayStatistic::Smoothed`].
pub struct DiagnosticOverlayItem {
    /// The statistic of the diagnostic to display
    pub statistic: DiagnosticOverlayStatistic,
    /// The diagnostic to display
    pub path: DiagnosticPath,
}

impl From<DiagnosticPath> for DiagnosticOverlayItem {
    /// Creates an instance of [`DiagnosticOverlayItem`]
    /// from a [`DiagnosticPath`] using [`DiagnosticOverlayStatistic::Smoothed`].
    fn from(value: DiagnosticPath) -> Self {
        Self {
            path: value,
            statistic: Default::default(),
        }
    }
}

/// The statistic to use when displaying a diagnostic
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum DiagnosticOverlayStatistic {
    /// The most recent value of on the diagnostic store
    Value,
    /// The average of a window of values in the diagnostic store.
    Average,
    /// The smoothed average of a window of values in the diagnostic store
    /// using the [EMA](https://en.wikipedia.org/wiki/Exponential_smoothing).
    #[default]
    Smoothed,
}

impl DiagnosticOverlayStatistic {
    /// Fetch the appropriate statistic from a [`Diagnostic`]
    pub fn fetch(&self, diagnostic: &Diagnostic) -> Option<f64> {
        match self {
            Self::Value => diagnostic.value(),
            Self::Average => diagnostic.average(),
            Self::Smoothed => diagnostic.smoothed(),
        }
    }
}

/// System set for the systems of the [`DiagnosticsOverlayPlugin`]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, SystemSet)]
pub enum DiagnosticsOverlaySystems {
    /// Rebuild the contents of the [`DiagnosticsOverlay`] entities
    Rebuild,
}

/// Plugin that builds a visual overlay to present diagnostics.
///
/// The contents of each [`DiagnosticsOverlay`] are rebuilt ever second.
pub struct DiagnosticsOverlayPlugin;

impl Plugin for DiagnosticsOverlayPlugin {
    fn build(&self, app: &mut App) {
        app.configure_sets(Update, DiagnosticsOverlaySystems::Rebuild);
        app.add_systems(Startup, build_plane);
        app.add_systems(
            Update,
            rebuild_diagnostics_list
                .run_if(on_timer(Duration::from_secs(1)))
                .in_set(DiagnosticsOverlaySystems::Rebuild),
        );

        app.add_observer(build_overlay);
        app.add_observer(drag_by_header);
        app.add_observer(collapse_on_click_to_header);
        app.add_observer(bring_to_front);
    }
}

/// Builds the Ui plane where the [`DiagnosticsOverlay`] entities
/// will reside.
fn build_plane(mut commands: Commands) {
    commands.spawn((
        DiagnosticsOverlayPlane,
        Node {
            width: Val::Percent(100.),
            height: Val::Percent(100.),
            ..Default::default()
        },
        INITIAL_DIAGNOSTICS_OVERLAY_PLANE_Z_INDEX,
    ));
}

/// Header of the overlay
#[derive(Component)]
struct DiagnosticOverlayHeader;

/// Section of the overlay that will have the diagnostics
#[derive(Component)]
struct DiagnosticsOverlayContents;

fn rebuild_diagnostics_list(
    mut commands: Commands,
    diagnostics_overlays: Query<&DiagnosticsOverlay>,
    diagnostics_overlay_contents: Query<(Entity, &ChildOf), With<DiagnosticsOverlayContents>>,
    diagnostics_store: Res<DiagnosticsStore>,
) {
    for (entity, child_of) in diagnostics_overlay_contents {
        commands.entity(entity).despawn_children();

        let Ok(diagnostics_overlay) = diagnostics_overlays.get(child_of.get()) else {
            panic!("DiagnosticsOverlayContents has been tempered with. Parent was not a DiagnosticsOverlay.");
        };

        for (i, diagnostic_overlay_item) in diagnostics_overlay
            .diagnostic_overlay_items
            .iter()
            .enumerate()
        {
            let maybe_diagnostic = diagnostics_store.get(&diagnostic_overlay_item.path);
            let diagnostic = maybe_diagnostic
                .map(|diagnostic| {
                    format!(
                        "{}{}",
                        diagnostic
                            .value()
                            .as_ref()
                            .map(ToString::to_string)
                            .unwrap_or("No sample".to_owned()),
                        diagnostic.suffix
                    )
                })
                .unwrap_or("Missing".to_owned());

            commands.spawn((
                ChildOf(entity),
                Node {
                    grid_row: GridPlacement::start(i as i16 + 1),
                    grid_column: GridPlacement::start(1),
                    ..Default::default()
                },
                Pickable::IGNORE,
                children![(
                    Text::new(diagnostic_overlay_item.path.to_string()),
                    TextFont {
                        font_size: 10.,
                        ..Default::default()
                    },
                    Pickable::IGNORE,
                )],
            ));
            commands.spawn((
                ChildOf(entity),
                Node {
                    grid_row: GridPlacement::start(i as i16 + 1),
                    grid_column: GridPlacement::start(2),
                    ..Default::default()
                },
                Pickable::IGNORE,
                children![(
                    Text::new(diagnostic),
                    TextFont {
                        font_size: 10.,
                        ..Default::default()
                    },
                    Pickable::IGNORE,
                )],
            ));
        }
    }
}

fn build_overlay(
    event: On<Add, DiagnosticsOverlay>,
    mut commands: Commands,
    diagnostics_overlays: Query<&DiagnosticsOverlay>,
    diagnostics_overlay_plane: Single<Entity, With<DiagnosticsOverlayPlane>>,
) {
    let entity = event.entity;
    let Ok(diagnostics_overlay) = diagnostics_overlays.get(entity) else {
        unreachable!("DiagnosticsOverlay must be available.");
    };

    commands.entity(entity).insert((
        Node {
            position_type: PositionType::Absolute,
            top: INITIAL_OFFSET,
            left: INITIAL_OFFSET,
            flex_direction: FlexDirection::Column,
            ..Default::default()
        },
        ChildOf(*diagnostics_overlay_plane),
        children![
            (
                Node {
                    padding: DEFAULT_PADDING,
                    ..Default::default()
                },
                DiagnosticOverlayHeader,
                BackgroundColor(
                    palettes::tailwind::GRAY_900
                        .with_alpha(BACKGROUND_COLOR_ALPHA)
                        .into()
                ),
                children![(
                    Text::new(diagnostics_overlay.title.as_ref()),
                    TextFont {
                        font_size: 12.,
                        ..Default::default()
                    },
                    Pickable::IGNORE
                )],
            ),
            (
                Node {
                    display: Display::Grid,
                    row_gap: ROW_COLUMN_GAP,
                    column_gap: ROW_COLUMN_GAP,
                    padding: DEFAULT_PADDING,
                    ..Default::default()
                },
                DiagnosticsOverlayContents,
                BackgroundColor(
                    palettes::tailwind::GRAY_600
                        .with_alpha(BACKGROUND_COLOR_ALPHA)
                        .into()
                ),
            )
        ],
    ));
}

fn drag_by_header(
    mut event: On<Pointer<Drag>>,
    mut diagnostics_overlays: Query<&mut Node, With<DiagnosticsOverlay>>,
    diagnostics_overlay_headers: Query<&ChildOf, With<DiagnosticOverlayHeader>>,
) {
    let entity = event.entity;
    if let Ok(child_of) = diagnostics_overlay_headers.get(entity) {
        event.propagate(false);
        let Ok(mut node) = diagnostics_overlays.get_mut(child_of.get()) else {
            panic!("DiagnosticsOverlayHeader has been tempered with. Parent was not a DiagnosticsOverlay.");
        };
        let delta = event.delta;
        let Val::Px(top) = &mut node.top else {
            panic!(
                "DiagnosticsOverlay has been tempered with. Node must have `top` using `Val::Px`."
            );
        };
        *top += delta.y;
        let Val::Px(left) = &mut node.left else {
            panic!(
                "DiagnosticsOverlay has been tempered with. Node must have `left` using `Val::Px`."
            );
        };
        *left += delta.x;
    }
}

fn collapse_on_click_to_header(
    mut event: On<Pointer<Click>>,
    mut diagnostics_overlays: Query<&Children, With<DiagnosticsOverlay>>,
    mut diagnostics_overlay_contents: Query<&mut Node, With<DiagnosticsOverlayContents>>,
    diagnostics_overlay_header: Query<&ChildOf, With<DiagnosticOverlayHeader>>,
) {
    if event.duration > Duration::from_millis(250) {
        return;
    }

    let entity = event.entity;
    if let Ok(child_of) = diagnostics_overlay_header.get(entity) {
        event.propagate(false);

        let Ok(children) = diagnostics_overlays.get_mut(child_of.get()) else {
            unreachable!("DiagnosticsOverlay has been tempered with. Do not despawn its children.");
        };
        let mut lists_iter = diagnostics_overlay_contents.iter_many_mut(children.collection());

        let Some(mut node) = lists_iter.fetch_next() else {
            panic!(
                "DiagnosticsOverlay has been tempered with. DiagnosticsOverlay must\
            have a child with DiagnosticsList."
            );
        };

        let next_display_mode = match node.display {
            Display::Grid => Display::None,
            Display::None => Display::Grid,
            _ => panic!(
                "The DiagnosticsList has be tempered with. Valid Displays for a\
            DiagnosticsList are Grid or None."
            ),
        };
        node.display = next_display_mode;

        if lists_iter.fetch_next().is_some() {
            panic!(
                "DiagnosticsOverlay has been tempered with. DiagnosticsOverlay must\
            only ever have one single child with DiagnosticsList."
            );
        }
    }
}

fn bring_to_front(
    event: On<Pointer<Press>>,
    mut commands: Commands,
    diagnostics_overlays: Query<(), With<DiagnosticsOverlay>>,
    diagnostics_overlay_plane: Single<Entity, With<DiagnosticsOverlayPlane>>,
) {
    let entity = event.entity;
    if diagnostics_overlays.contains(entity) {
        commands
            .entity(entity)
            .remove::<ChildOf>()
            .insert(ChildOf(*diagnostics_overlay_plane));
    }
}
