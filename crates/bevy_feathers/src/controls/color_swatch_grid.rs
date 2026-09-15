use bevy_app::{Plugin, PostUpdate};
use bevy_color::Color;
use bevy_ecs::{
    change_detection::DetectChangesMut,
    entity::Entity,
    hierarchy::Children,
    observer::On,
    query::{Has, With},
    reflect::ReflectComponent,
    system::{Commands, Query, SystemParam},
};
use bevy_input_focus::tab_navigation::TabIndex;
use bevy_math::UVec2;
use bevy_picking::{cursor::EntityCursor, Pickable};
use bevy_reflect::{prelude::ReflectDefault, Reflect};
use bevy_scene::{prelude::*, Ready};
use bevy_ui::{
    percent, px, AlignItems, BorderColor, BorderRadius, Checked, Display, InteractionDisabled,
    Node, Outline, PositionType, RepeatedGridTrack, UiRect, ZIndex,
};
use bevy_ui_widgets::{RadioButton, RadioGroup, ValueChange};
use bevy_window::SystemCursorIcon;

use crate::{
    controls::{ColorSwatchValue, FeathersColorSwatch},
    focus::FocusIndicator,
    palette,
    theme::ThemeBorderColor,
    tokens,
};

/// A rectangular grid of color swatches, which allows one swatch to be selected.
///
/// This is spawnable by inheriting it as a "scene component". The cells are populated by
/// calling [`ColorSwatchGridUpdate::update`]; cells beyond the end of the color list display an
/// empty placeholder.
///
/// # Emitted events
///
/// * [`ValueChange<Color>`] when a swatch is selected. The widget doesn't record the selection
///   itself: pass the new color back in through [`ColorSwatchGridUpdate::update`] to move the
///   selection ring.
#[derive(SceneComponent, Default, Clone, Reflect)]
#[reflect(Component, Clone, Default)]
pub struct FeathersColorSwatchGrid {
    /// Set a percentage of the swatch to display the opaque version of the
    /// current color.
    pub opaque_color_percentage: f32,

    /// Size of the grid in cells.
    pub size: UVec2,
}

impl FeathersColorSwatchGrid {
    fn scene() -> impl Scene {
        bsn! {
            Node {
                display: Display::Grid,
                column_gap: px(2),
                row_gap: px(2),
                grid_template_columns: vec![
                    RepeatedGridTrack::flex(1, 1.),
                ],
                grid_template_rows: vec![
                    RepeatedGridTrack::px(1, 20.),
                ],
                align_items: AlignItems::Stretch,
            }
            RadioGroup
            TabIndex
            FocusIndicator
            on(swatch_grid_ready)
            on(swatch_grid_on_value_change)
        }
    }
}

/// A single cell within a [`FeathersColorSwatchGrid`].
///
/// The cell is a stable entity: it owns the grid position and the selection state, while its
/// children hold the visual, which is either a color swatch or an empty placeholder, plus a
/// [`ColorSwatchGridCellRing`] overlay. Replacing a cell's contents therefore never disturbs the
/// grid's [`Children`], and the checked state survives the replacement.
///
/// Cells which don't hold a color are marked [`InteractionDisabled`], which excludes them from
/// both keyboard navigation and clicks.
#[derive(SceneComponent, Default, Clone, Reflect)]
#[reflect(Component, Clone, Default)]
pub struct FeathersColorSwatchGridCell;

impl FeathersColorSwatchGridCell {
    fn scene() -> impl Scene {
        bsn! {
            Node {
                display: Display::Flex,
            }
            RadioButton
            Children [
                @ColorSwatchGridCellRing
            ]
        }
    }
}

/// Draws the selection ring for a [`FeathersColorSwatchGridCell`].
///
/// This is a separate entity, positioned absolutely.
#[derive(SceneComponent, Default, Clone, Reflect)]
#[reflect(Component, Clone, Default)]
pub struct ColorSwatchGridCellRing;

impl ColorSwatchGridCellRing {
    fn scene() -> impl Scene {
        bsn! {
            Node {
                position_type: PositionType::Absolute,
                left: px(0),
                top: px(0),
                right: px(0),
                bottom: px(0),
                border: UiRect::all(px(2)),
                border_radius: BorderRadius::all(px(2)),
            }
            // Dark against the swatch, light against the popup background, so that one edge of
            // the ring stays visible whatever color the swatch happens to be.
            BorderColor::all(Color::NONE)
            Outline {
                width: px(1),
                offset: px(0),
                color: Color::NONE,
            }
            ZIndex(1)
            Pickable::IGNORE
        }
    }
}

/// A placeholder for an unoccupied grid cell.
///
/// Deliberately square-cornered: rounded corners are reserved for cells holding a color.
#[derive(SceneComponent, Default, Clone, Reflect)]
#[reflect(Component, Clone, Default)]
pub struct FeathersColorSwatchEmptyCell;

impl FeathersColorSwatchEmptyCell {
    fn scene() -> impl Scene {
        bsn! {
            Node {
                display: Display::Flex,
                border: UiRect::all(px(1)),
            }
            ThemeBorderColor(tokens::BUTTON_BG)
        }
    }
}

/// Translate the [`RadioGroup`]'s entity-based selection event into a color.
fn swatch_grid_on_value_change(
    change: On<ValueChange<Entity>>,
    q_cells: Query<&Children, With<FeathersColorSwatchGridCell>>,
    q_contents: Query<&ColorSwatchValue>,
    mut commands: Commands,
) {
    let Some(color) = q_cells
        .get(change.value)
        .ok()
        .and_then(|children| {
            children
                .iter()
                .find_map(|child| q_contents.get(*child).ok())
        })
        .map(|value| value.0)
    else {
        return;
    };

    commands.trigger(ValueChange {
        source: change.source,
        value: color,
        is_final: true,
    });
}

fn swatch_grid_ready(ready: On<Ready>, mut q_grid: Query<(&FeathersColorSwatchGrid, &mut Node)>) {
    let Ok((grid, mut node)) = q_grid.get_mut(ready.entity) else {
        return;
    };

    node.grid_template_columns = RepeatedGridTrack::flex(grid.size.x as u16, 1.);
    node.grid_template_rows = RepeatedGridTrack::px(grid.size.y as u16, 20.);
}

/// What a grid cell is currently displaying.
enum CellContent {
    /// The cell holds a color swatch showing the given color.
    Swatch(Entity, Color),
    /// The cell holds an empty placeholder.
    Empty,
    /// The cell is empty or holds something we don't recognize, and must be rebuilt.
    Unknown,
}

/// [`SystemParam`] which contains the machinery for updating the color swatch grid from a slice
/// of colors.
#[derive(SystemParam)]
pub struct ColorSwatchGridUpdate<'w, 's> {
    grids: Query<'w, 's, (&'static FeathersColorSwatchGrid, Option<&'static Children>)>,
    cells: Query<
        'w,
        's,
        (
            Option<&'static Children>,
            Has<Checked>,
            Has<InteractionDisabled>,
        ),
        With<FeathersColorSwatchGridCell>,
    >,
    contents: Query<
        'w,
        's,
        (
            Option<&'static ColorSwatchValue>,
            Has<FeathersColorSwatchEmptyCell>,
        ),
    >,
    rings: Query<'w, 's, (), With<ColorSwatchGridCellRing>>,
    commands: Commands<'w, 's>,
}

impl ColorSwatchGridUpdate<'_, '_> {
    /// Update the grid from `colors`, checking whichever cell holds `selected`.
    ///
    /// Cells are stable entities: only their contents are replaced.
    pub fn update(&mut self, grid: Entity, colors: &[Color], selected: Option<Color>) {
        let Ok((cfg, children)) = self.grids.get(grid) else {
            return;
        };
        let cell_count = (cfg.size.x * cfg.size.y) as usize;
        let opaque_pct = cfg.opaque_color_percentage;
        // Copy avoids borrowing issues.
        let children: Vec<Entity> = children
            .map(|children| children.iter().copied().collect())
            .unwrap_or_default();

        // Patch the cells we already have.
        for (i, cell_id) in children.iter().take(cell_count).enumerate() {
            self.update_cell(*cell_id, colors.get(i).copied(), selected, opaque_pct);
        }

        // Spawn any cells we're missing.
        for i in children.len()..cell_count {
            let color = colors.get(i).copied();
            let cell_id = self
                .commands
                .queue_spawn_scene(bsn! { @FeathersColorSwatchGridCell })
                .id();
            self.commands.entity(grid).add_child(cell_id);
            self.set_content(cell_id, color, opaque_pct);
            self.set_checked(cell_id, color.is_some() && color == selected);
            self.set_disabled(cell_id, color.is_none());
        }

        // Despawn excess cells. This only happens if the grid shrank.
        for cell_id in children.iter().skip(cell_count) {
            self.commands.entity(*cell_id).despawn();
        }
    }

    /// Bring a single existing cell in line with the color it ought to be showing.
    fn update_cell(
        &mut self,
        cell_id: Entity,
        color: Option<Color>,
        selected: Option<Color>,
        opaque_pct: f32,
    ) {
        let Ok((children, is_checked, is_disabled)) = self.cells.get(cell_id) else {
            return;
        };

        // Copy the inner entity id out so that the query borrows end here.
        let inner = self.content_child(children);

        match (color, self.cell_content(inner)) {
            // Already showing the right color.
            (Some(color), CellContent::Swatch(_, current)) if current == color => {}
            // Already a swatch, so patch the color in place.
            (Some(color), CellContent::Swatch(inner, _)) => {
                self.commands.entity(inner).insert(ColorSwatchValue(color));
            }
            // Already a placeholder.
            (None, CellContent::Empty) => {}
            // Showing the wrong kind of thing, so rebuild the cell's contents.
            (color, _) => self.set_content(cell_id, color, opaque_pct),
        }

        let should_be_checked = color.is_some() && color == selected;
        if is_checked != should_be_checked {
            self.set_checked(cell_id, should_be_checked);
        }
        if is_disabled != color.is_none() {
            self.set_disabled(cell_id, color.is_none());
        }
    }

    /// The cell's content child, as distinct from its permanent ring overlay.
    fn content_child(&self, children: Option<&Children>) -> Option<Entity> {
        children?
            .iter()
            .copied()
            .find(|child| !self.rings.contains(*child))
    }

    /// Determine what a cell is currently displaying.
    fn cell_content(&self, inner: Option<Entity>) -> CellContent {
        let Some(inner) = inner else {
            return CellContent::Unknown;
        };
        match self.contents.get(inner) {
            Ok((Some(value), _)) => CellContent::Swatch(inner, value.0),
            Ok((None, true)) => CellContent::Empty,
            _ => CellContent::Unknown,
        }
    }

    /// Replace a cell's contents with a swatch showing `color`, or with an empty placeholder if
    /// `color` is `None`.
    fn set_content(&mut self, cell_id: Entity, color: Option<Color>, opaque_pct: f32) {
        let existing = self
            .cells
            .get(cell_id)
            .ok()
            .and_then(|(children, _, _)| self.content_child(children));
        if let Some(existing) = existing {
            self.commands.entity(existing).despawn();
        }
        match color {
            Some(color) => {
                self.commands
                    .entity(cell_id)
                    .queue_spawn_related_scenes::<Children>(bsn! {
                        @FeathersColorSwatch {
                            @border_radius: 2.0,
                            @opaque_color_percentage: {opaque_pct},
                        }
                        ColorSwatchValue({color})
                        // Override hard-coded swatch height.
                        Node {
                            height: percent(100),
                            min_width: px(0),
                            flex_grow: 1.0,
                        }
                    });
            }
            None => {
                self.commands
                    .entity(cell_id)
                    .queue_spawn_related_scenes::<Children>(bsn! {
                        @FeathersColorSwatchEmptyCell
                        Node {
                            flex_grow: 1.0,
                        }
                    });
            }
        }
    }

    /// Set or clear a cell's [`Checked`] state. The ring follows via [`update_cell_ring`].
    fn set_checked(&mut self, cell_id: Entity, checked: bool) {
        if checked {
            self.commands.entity(cell_id).insert(Checked);
        } else {
            self.commands.entity(cell_id).remove::<Checked>();
        }
    }

    /// Empty cells are disabled, which excludes them from keyboard navigation and clicks.
    fn set_disabled(&mut self, cell_id: Entity, disabled: bool) {
        let cursor = if disabled {
            SystemCursorIcon::NotAllowed
        } else {
            SystemCursorIcon::Pointer
        };
        let mut cell = self.commands.entity(cell_id);
        cell.insert(EntityCursor::System(cursor));
        if disabled {
            cell.insert(InteractionDisabled);
        } else {
            cell.remove::<InteractionDisabled>();
        }
    }
}

/// Sync each cell's ring overlay to its [`Checked`] state.
///
/// Reading the state every frame avoids having to detect [`Checked`] being removed, and a
/// grid only has cells while its popup is open.
fn update_cell_ring(
    q_cells: Query<(&Children, Has<Checked>), With<FeathersColorSwatchGridCell>>,
    mut q_rings: Query<(&mut BorderColor, &mut Outline), With<ColorSwatchGridCellRing>>,
) {
    for (children, checked) in q_cells.iter() {
        let (border, outline) = if checked {
            (palette::BLACK, palette::WHITE)
        } else {
            (Color::NONE, Color::NONE)
        };

        for child in children.iter() {
            if let Ok((mut border_color, mut ring_outline)) = q_rings.get_mut(*child) {
                border_color.set_if_neq(BorderColor::all(border));
                if ring_outline.color != outline {
                    ring_outline.color = outline;
                }
            }
        }
    }
}

/// Plugin which registers the systems for the color swatch grid.
pub struct ColorSwatchGridPlugin;

impl Plugin for ColorSwatchGridPlugin {
    fn build(&self, app: &mut bevy_app::App) {
        app.add_systems(PostUpdate, update_cell_ring);
    }
}
