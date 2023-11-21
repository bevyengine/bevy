use crate::idle_color::IdleColor;
use bevy::prelude::{
    default, AlignItems, AssetServer, BackgroundColor, BorderColor, BuildChildren, ButtonBundle,
    ChildBuilder, Color, Handle, Image, JustifyContent, Res, Style, TextBundle, TextStyle, UiImage,
    UiRect, Val,
};

use crate::args::Args;

const FONT_SIZE: f32 = 7.0;

#[derive(Debug, Clone, Copy)]
struct ButtonBundleSettings {
    background_color: BackgroundColor,
    buttons: f32,
    border: UiRect,
    border_color: BorderColor,
}

impl ButtonBundleSettings {
    fn new(background_color: BackgroundColor, buttons: f32, no_borders: bool) -> Self {
        Self {
            background_color,
            border: if no_borders {
                UiRect::ZERO
            } else {
                UiRect::all(Val::VMin(0.05 * 90. / buttons))
            },
            border_color: Color::WHITE.with_a(0.5).into(),
            buttons,
        }
    }
}

pub(crate) fn create_button_row(
    args: &Res<Args>,
    asset_server: &Res<AssetServer>,
    commands: &mut ChildBuilder,
    column: usize,
) {
    let buttons_f = args.buttons as f32;
    let as_rainbow = |i: usize| Color::hsl((i as f32 / buttons_f) * 360.0, 0.9, 0.8);
    let image = if 0 < args.image_freq {
        Some(asset_server.load("branding/icon.png"))
    } else {
        None
    };

    for row in 0..args.buttons {
        let background_color = as_rainbow(row % column.max(1)).into();
        let image = image
            .as_ref()
            .filter(|_| (column + row) % args.image_freq == 0)
            .cloned();
        let settings = ButtonBundleSettings::new(background_color, buttons_f, args.no_borders);
        if !args.no_text {
            spawn_button_with_text(commands, settings, row, column, image);
        } else {
            spawn_button(commands, settings, image);
        }
    }
}

fn spawn_button_with_text(
    commands: &mut ChildBuilder,
    button_bundle_settings: ButtonBundleSettings,
    column: usize,
    row: usize,
    image: Option<Handle<Image>>,
) {
    let mut builder = commands.spawn((
        create_button_bundle(button_bundle_settings),
        IdleColor(button_bundle_settings.background_color),
    ));

    if let Some(image) = image {
        builder.insert(UiImage::new(image));
    }

    builder.with_children(|parent| {
        parent.spawn(TextBundle::from_section(
            format!("{column}, {row}"),
            TextStyle {
                font_size: FONT_SIZE,
                color: Color::rgb(0.2, 0.2, 0.2),
                ..default()
            },
        ));
    });
}

fn spawn_button(
    commands: &mut ChildBuilder,
    button_bundle_settings: ButtonBundleSettings,
    image: Option<Handle<Image>>,
) {
    let mut builder = commands.spawn((
        create_button_bundle(button_bundle_settings),
        IdleColor(button_bundle_settings.background_color),
    ));

    if let Some(image) = image {
        builder.insert(UiImage::new(image));
    }
}

fn create_button_bundle(button_bundle_settings: ButtonBundleSettings) -> ButtonBundle {
    let width = Val::Vw(90.0 / button_bundle_settings.buttons);
    let height = Val::Vh(90.0 / button_bundle_settings.buttons);

    ButtonBundle {
        style: Style {
            width,
            height,
            margin: UiRect::axes(width * 0.05, height * 0.05),
            align_items: AlignItems::Center,
            justify_content: JustifyContent::Center,
            border: button_bundle_settings.border,
            ..default()
        },
        background_color: button_bundle_settings.background_color,
        border_color: button_bundle_settings.border_color,
        ..default()
    }
}
