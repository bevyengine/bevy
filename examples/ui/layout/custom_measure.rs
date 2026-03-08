//! Demonstrates how to provide a custom [`Measure`](bevy::ui::Measure) implementation for UI layout.
//!
//! Controls:
//! - Up/Down: increase or decrease measured glyph count.
//! - Space: toggle between short and long measured content.

use bevy::{
    color::palettes::css::*,
    prelude::*,
    ui::{AvailableSpace, ContentSize, Measure, MeasureArgs, NodeMeasure},
};

fn main() {
    App::new()
        .init_resource::<MeasureSettings>()
        .add_plugins(DefaultPlugins)
        .add_systems(Startup, setup)
        .add_systems(
            Update,
            (
                update_settings_from_input,
                apply_measure_settings,
                update_size_readouts,
                update_settings_readout,
            ),
        )
        .run();
}

#[derive(Resource, Clone, Copy, PartialEq, Eq)]
struct MeasureSettings {
    glyph_count: u16,
}

impl Default for MeasureSettings {
    fn default() -> Self {
        Self { glyph_count: 48 }
    }
}

#[derive(Component)]
struct MeasuredLeaf;

#[derive(Component)]
struct SizeReadout(Entity);

#[derive(Component)]
struct SettingsReadout;

/// A simple custom measure that pretends to wrap monospaced glyphs.
struct ParagraphMeasure {
    glyph_count: f32,
    glyph_width: f32,
    line_height: f32,
    horizontal_padding: f32,
    vertical_padding: f32,
}

impl ParagraphMeasure {
    fn new(glyph_count: u16) -> Self {
        Self {
            glyph_count: f32::from(glyph_count),
            glyph_width: 8.0,
            line_height: 18.0,
            horizontal_padding: 16.0,
            vertical_padding: 10.0,
        }
    }
}

impl Measure for ParagraphMeasure {
    fn measure(&mut self, args: MeasureArgs<'_>, _style: &taffy::Style) -> Vec2 {
        let min_width = self.horizontal_padding * 2.0 + self.glyph_width;
        let preferred_width = self.horizontal_padding * 2.0 + self.glyph_count * self.glyph_width;

        let measured_width = if let Some(width) = args.width {
            width.max(min_width)
        } else {
            match args.available_width {
                AvailableSpace::Definite(max_width) => {
                    preferred_width.clamp(min_width, max_width.max(min_width))
                }
                AvailableSpace::MinContent => min_width,
                AvailableSpace::MaxContent => preferred_width,
            }
        };

        let content_width = (measured_width - self.horizontal_padding * 2.0).max(self.glyph_width);
        let glyphs_per_line = (content_width / self.glyph_width).floor().max(1.0);
        let line_count = (self.glyph_count / glyphs_per_line).ceil().max(1.0);
        let measured_height = args
            .height
            .unwrap_or(self.vertical_padding * 2.0 + line_count * self.line_height);

        Vec2::new(measured_width, measured_height)
    }
}

fn setup(mut commands: Commands, asset_server: Res<AssetServer>, settings: Res<MeasureSettings>) {
    commands.spawn(Camera2d);

    let title_font = TextFont {
        font: asset_server.load("fonts/FiraSans-Bold.ttf").into(),
        font_size: FontSize::Px(28.0),
        ..default()
    };
    let body_font = TextFont {
        font: asset_server.load("fonts/FiraSans-Bold.ttf").into(),
        font_size: FontSize::Px(15.0),
        ..default()
    };

    commands
        .spawn((
            Node {
                width: percent(100),
                height: percent(100),
                flex_direction: FlexDirection::Column,
                row_gap: px(12),
                padding: UiRect::all(px(16)),
                ..default()
            },
            BackgroundColor(Color::Srgba(MIDNIGHT_BLUE)),
        ))
        .with_children(|parent| {
            parent.spawn((
                Text::new("Custom Measure Demo"),
                title_font.clone(),
                TextColor(Color::Srgba(ALICE_BLUE)),
            ));

            parent.spawn((
                Text::new(""),
                body_font.clone(),
                TextColor(Color::Srgba(ANTIQUE_WHITE)),
                SettingsReadout,
            ));

            parent
                .spawn(Node {
                    width: percent(100),
                    flex_direction: FlexDirection::Row,
                    flex_wrap: FlexWrap::Wrap,
                    column_gap: px(12),
                    row_gap: px(12),
                    align_items: AlignItems::FlexStart,
                    ..default()
                })
                .with_children(|parent| {
                    for width in [190.0, 280.0, 420.0] {
                        spawn_measure_card(parent, width, settings.glyph_count, body_font.clone());
                    }
                });
        });
}

fn spawn_measure_card(
    parent: &mut ChildSpawnerCommands,
    width: f32,
    glyph_count: u16,
    body_font: TextFont,
) {
    parent
        .spawn((
            Node {
                width: px(width),
                flex_direction: FlexDirection::Column,
                row_gap: px(8),
                padding: UiRect::all(px(12)),
                border: UiRect::all(px(1)),
                border_radius: BorderRadius::all(px(8)),
                ..default()
            },
            BackgroundColor(Color::srgb(0.11, 0.13, 0.24)),
            BorderColor::all(Color::Srgba(LIGHT_GRAY)),
        ))
        .with_children(|parent| {
            parent.spawn((
                Text::new(format!("Parent width: {:.0}px", width)),
                body_font.clone(),
                TextColor(Color::Srgba(WHITE)),
            ));

            let mut content_size = ContentSize::default();
            content_size.set(NodeMeasure::Custom(Box::new(ParagraphMeasure::new(
                glyph_count,
            ))));

            let measured_leaf = parent
                .spawn((
                    Node {
                        max_width: percent(100),
                        border: UiRect::all(px(1)),
                        border_radius: BorderRadius::all(px(6)),
                        ..default()
                    },
                    content_size,
                    MeasuredLeaf,
                    BackgroundColor(Color::srgb(0.24, 0.59, 0.74)),
                    BorderColor::all(Color::Srgba(BLACK)),
                ))
                .id();

            parent.spawn((
                Text::new(""),
                body_font,
                TextColor(Color::Srgba(WHITE)),
                SizeReadout(measured_leaf),
            ));
        });
}

fn update_settings_from_input(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut settings: ResMut<MeasureSettings>,
) {
    let mut next = *settings;

    if keyboard.just_pressed(KeyCode::ArrowUp) {
        next.glyph_count = (next.glyph_count + 8).min(240);
    }
    if keyboard.just_pressed(KeyCode::ArrowDown) {
        next.glyph_count = next.glyph_count.saturating_sub(8).max(8);
    }
    if keyboard.just_pressed(KeyCode::Space) {
        next.glyph_count = if next.glyph_count > 60 { 24 } else { 120 };
    }

    settings.set_if_neq(next);
}

fn apply_measure_settings(
    settings: Res<MeasureSettings>,
    mut query: Query<&mut ContentSize, With<MeasuredLeaf>>,
) {
    if !settings.is_changed() {
        return;
    }

    for mut content_size in &mut query {
        content_size.set(NodeMeasure::Custom(Box::new(ParagraphMeasure::new(
            settings.glyph_count,
        ))));
    }
}

fn update_size_readouts(
    measured_nodes: Query<&ComputedNode, With<MeasuredLeaf>>,
    mut readouts: Query<(&SizeReadout, &mut Text)>,
) {
    for (size_readout, mut text) in &mut readouts {
        if let Ok(computed_node) = measured_nodes.get(size_readout.0) {
            *text = Text::new(format!(
                "Measured node: {:.0} x {:.0}",
                computed_node.size.x, computed_node.size.y
            ));
        }
    }
}

fn update_settings_readout(
    settings: Res<MeasureSettings>,
    mut readouts: Query<&mut Text, With<SettingsReadout>>,
) {
    if !settings.is_changed() {
        return;
    }

    for mut text in &mut readouts {
        *text = Text::new(format!(
            "Use Up/Down to change measured glyph count, Space to toggle short/long. \
             Glyph count: {}",
            settings.glyph_count
        ));
    }
}
