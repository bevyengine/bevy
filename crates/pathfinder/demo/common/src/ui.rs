// pathfinder/demo/src/ui.rs
//
// Copyright © 2019 The Pathfinder Project Developers.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

use crate::camera::Mode;
use crate::window::Window;
use crate::{BackgroundColor, Options};
use pathfinder_color::ColorU;
use pathfinder_geometry::rect::RectI;
use pathfinder_geometry::vector::{Vector2I, vec2i};
use pathfinder_gpu::Device;
use pathfinder_renderer::gpu::debug::DebugUIPresenter;
use pathfinder_resources::ResourceLoader;
use pathfinder_ui::{BUTTON_HEIGHT, BUTTON_TEXT_OFFSET, BUTTON_WIDTH, FONT_ASCENT, PADDING};
use pathfinder_ui::{TEXT_COLOR, TOOLTIP_HEIGHT, WINDOW_COLOR};
use std::f32::consts::PI;
use std::path::PathBuf;

const SLIDER_WIDTH: i32 = 360;
const SLIDER_HEIGHT: i32 = 48;
const SLIDER_TRACK_HEIGHT: i32 = 24;
const SLIDER_KNOB_WIDTH: i32 = 12;
const SLIDER_KNOB_HEIGHT: i32 = 48;

const EFFECTS_PANEL_WIDTH: i32 = 550;
const EFFECTS_PANEL_HEIGHT: i32 = BUTTON_HEIGHT * 3 + PADDING * 4;

const BACKGROUND_PANEL_WIDTH: i32 = 250;
const BACKGROUND_PANEL_HEIGHT: i32 = BUTTON_HEIGHT * 3;

const SCREENSHOT_PANEL_WIDTH: i32 = 275;
const SCREENSHOT_PANEL_HEIGHT: i32 = BUTTON_HEIGHT * 2;

const ROTATE_PANEL_WIDTH: i32 = SLIDER_WIDTH + PADDING * 2;
const ROTATE_PANEL_HEIGHT: i32 = PADDING * 2 + SLIDER_HEIGHT;

const LIGHT_BG_COLOR:       ColorU = ColorU { r: 248, g: 248, b: 248, a: 255, };
const DARK_BG_COLOR:        ColorU = ColorU { r: 32,  g: 32,  b: 32,  a: 255, };
const TRANSPARENT_BG_COLOR: ColorU = ColorU { r: 0,   g: 0,   b: 0,   a: 0,   };

static EFFECTS_PNG_NAME: &'static str = "demo-effects";
static OPEN_PNG_NAME: &'static str = "demo-open";
static ROTATE_PNG_NAME: &'static str = "demo-rotate";
static ZOOM_IN_PNG_NAME: &'static str = "demo-zoom-in";
static ZOOM_ACTUAL_SIZE_PNG_NAME: &'static str = "demo-zoom-actual-size";
static ZOOM_OUT_PNG_NAME: &'static str = "demo-zoom-out";
static BACKGROUND_PNG_NAME: &'static str = "demo-background";
static SCREENSHOT_PNG_NAME: &'static str = "demo-screenshot";

pub struct DemoUIModel {
    pub mode: Mode,
    pub background_color: BackgroundColor,
    pub gamma_correction_effect_enabled: bool,
    pub stem_darkening_effect_enabled: bool,
    pub subpixel_aa_effect_enabled: bool,
    pub rotation: i32,
    pub message: String,
}

impl DemoUIModel {
    pub fn new(options: &Options) -> DemoUIModel {
        DemoUIModel {
            mode: options.mode,
            background_color: options.background_color,
            gamma_correction_effect_enabled: false,
            stem_darkening_effect_enabled: false,
            subpixel_aa_effect_enabled: false,
            rotation: SLIDER_WIDTH / 2,
            message: String::new(),
        }
    }

    fn rotation(&self) -> f32 {
        (self.rotation as f32 / SLIDER_WIDTH as f32 * 2.0 - 1.0) * PI
    }

    // Only relevant if in monochrome mode.
    pub fn foreground_color(&self) -> ColorU {
        match self.background_color {
            BackgroundColor::Light | BackgroundColor::Transparent => ColorU::black(),
            BackgroundColor::Dark => ColorU::white(),
        }
    }

    pub fn background_color(&self) -> ColorU {
        match self.background_color {
            BackgroundColor::Light => LIGHT_BG_COLOR,
            BackgroundColor::Dark => DARK_BG_COLOR,
            BackgroundColor::Transparent => TRANSPARENT_BG_COLOR,
        }
    }
}

pub struct DemoUIPresenter<D>
where
    D: Device,
{
    effects_texture: D::Texture,
    open_texture: D::Texture,
    rotate_texture: D::Texture,
    zoom_in_texture: D::Texture,
    zoom_actual_size_texture: D::Texture,
    zoom_out_texture: D::Texture,
    background_texture: D::Texture,
    screenshot_texture: D::Texture,

    effects_panel_visible: bool,
    background_panel_visible: bool,
    screenshot_panel_visible: bool,
    rotate_panel_visible: bool,
}

impl<D> DemoUIPresenter<D>
where
    D: Device,
{
    pub fn new(device: &D, resources: &dyn ResourceLoader) -> DemoUIPresenter<D> {
        let effects_texture = device.create_texture_from_png(resources, EFFECTS_PNG_NAME);
        let open_texture = device.create_texture_from_png(resources, OPEN_PNG_NAME);
        let rotate_texture = device.create_texture_from_png(resources, ROTATE_PNG_NAME);
        let zoom_in_texture = device.create_texture_from_png(resources, ZOOM_IN_PNG_NAME);
        let zoom_actual_size_texture = device.create_texture_from_png(resources,
                                                                      ZOOM_ACTUAL_SIZE_PNG_NAME);
        let zoom_out_texture = device.create_texture_from_png(resources, ZOOM_OUT_PNG_NAME);
        let background_texture = device.create_texture_from_png(resources, BACKGROUND_PNG_NAME);
        let screenshot_texture = device.create_texture_from_png(resources, SCREENSHOT_PNG_NAME);

        DemoUIPresenter {
            effects_texture,
            open_texture,
            rotate_texture,
            zoom_in_texture,
            zoom_actual_size_texture,
            zoom_out_texture,
            background_texture,
            screenshot_texture,

            effects_panel_visible: false,
            background_panel_visible: false,
            screenshot_panel_visible: false,
            rotate_panel_visible: false,
        }
    }

    pub fn update<W>(
        &mut self,
        device: &D,
        window: &mut W,
        debug_ui_presenter: &mut DebugUIPresenter<D>,
        action: &mut UIAction,
        model: &mut DemoUIModel
    ) where
        W: Window,
    {
        // Draw message text.

        self.draw_message_text(device, debug_ui_presenter, model);

        // Draw button strip.

        let bottom = debug_ui_presenter.ui_presenter.framebuffer_size().y() - PADDING;
        let mut position = vec2i(PADDING, bottom - BUTTON_HEIGHT);

        let button_size = vec2i(BUTTON_WIDTH, BUTTON_HEIGHT);

        // Draw effects button.
        if debug_ui_presenter.ui_presenter.draw_button(device, position, &self.effects_texture) {
            self.effects_panel_visible = !self.effects_panel_visible;
        }
        if !self.effects_panel_visible {
            debug_ui_presenter.ui_presenter.draw_tooltip(
                device,
                "Effects",
                RectI::new(position, button_size),
            );
        }
        position += vec2i(button_size.x() + PADDING, 0);

        // Draw open button.
        if debug_ui_presenter.ui_presenter.draw_button(device, position, &self.open_texture) {
            // FIXME(pcwalton): This is not sufficient for Android, where we will need to take in
            // the contents of the file.
            window.present_open_svg_dialog();
        }
        debug_ui_presenter.ui_presenter.draw_tooltip(device,
                                                     "Open SVG",
                                                     RectI::new(position, button_size));
        position += vec2i(BUTTON_WIDTH + PADDING, 0);

        // Draw screenshot button.
        if debug_ui_presenter.ui_presenter.draw_button(device,
                                                       position,
                                                       &self.screenshot_texture) {
            self.screenshot_panel_visible = !self.screenshot_panel_visible;
        }
        if !self.screenshot_panel_visible {
            debug_ui_presenter.ui_presenter.draw_tooltip(
                device,
                "Take Screenshot",
                RectI::new(position, button_size),
            );
        }

        // Draw screenshot panel, if necessary.
        self.draw_screenshot_panel(device, window, debug_ui_presenter, position.x(), action);
        position += vec2i(button_size.x() + PADDING, 0);

        // Draw mode switch.
        let new_mode = debug_ui_presenter.ui_presenter.draw_text_switch(
            device,
            position,
            &["2D", "3D", "VR"],
            model.mode as u8);
        if new_mode != model.mode as u8 {
            model.mode = match new_mode {
                0 => Mode::TwoD,
                1 => Mode::ThreeD,
                _ => Mode::VR,
            };
            *action = UIAction::ModelChanged;
        }

        let mode_switch_width = debug_ui_presenter.ui_presenter.measure_segmented_control(3);
        let mode_switch_size = vec2i(mode_switch_width, BUTTON_HEIGHT);
        debug_ui_presenter.ui_presenter.draw_tooltip(
            device,
            "2D/3D/VR Mode",
            RectI::new(position, mode_switch_size),
        );
        position += vec2i(mode_switch_width + PADDING, 0);

        // Draw background switch.
        if debug_ui_presenter.ui_presenter.draw_button(device,
                                                       position,
                                                       &self.background_texture) {
            self.background_panel_visible = !self.background_panel_visible;
        }
        if !self.background_panel_visible {
            debug_ui_presenter.ui_presenter.draw_tooltip(
                device,
                "Background Color",
                RectI::new(position, button_size),
            );
        }

        // Draw background panel, if necessary.
        self.draw_background_panel(device, debug_ui_presenter, position.x(), action, model);
        position += vec2i(button_size.x() + PADDING, 0);

        // Draw effects panel, if necessary.
        self.draw_effects_panel(device, debug_ui_presenter, model, action);

        // Draw rotate and zoom buttons, if applicable.
        if model.mode != Mode::TwoD {
            return;
        }

        if debug_ui_presenter.ui_presenter.draw_button(device, position, &self.rotate_texture) {
            self.rotate_panel_visible = !self.rotate_panel_visible;
        }
        if !self.rotate_panel_visible {
            debug_ui_presenter.ui_presenter.draw_tooltip(device,
                                                         "Rotate",
                                                         RectI::new(position, button_size));
        }
        self.draw_rotate_panel(device, debug_ui_presenter, position.x(), action, model);
        position += vec2i(BUTTON_WIDTH + PADDING, 0);

        // Draw zoom control.
        self.draw_zoom_control(device, debug_ui_presenter, position, action);
    }

    fn draw_zoom_control(
        &mut self,
        device: &D,
        debug_ui_presenter: &mut DebugUIPresenter<D>,
        position: Vector2I,
        action: &mut UIAction,
    ) {
        let zoom_segmented_control_width =
            debug_ui_presenter.ui_presenter.measure_segmented_control(3);
        let zoom_segmented_control_rect =
            RectI::new(position, vec2i(zoom_segmented_control_width, BUTTON_HEIGHT));
        debug_ui_presenter.ui_presenter.draw_tooltip(device, "Zoom", zoom_segmented_control_rect);

        let zoom_textures = &[
            &self.zoom_in_texture,
            &self.zoom_actual_size_texture,
            &self.zoom_out_texture
        ];

        match debug_ui_presenter.ui_presenter.draw_image_segmented_control(device,
                                                                           position,
                                                                           zoom_textures,
                                                                           None) {
            Some(0) => *action = UIAction::ZoomIn,
            Some(1) => *action = UIAction::ZoomActualSize,
            Some(2) => *action = UIAction::ZoomOut,
            _ => {}
        }
    }

    fn draw_message_text(&mut self,
                         device: &D,
                         debug_ui_presenter: &mut DebugUIPresenter<D>,
                         model: &mut DemoUIModel) {
        if model.message.is_empty() {
            return;
        }

        let message_size = debug_ui_presenter.ui_presenter.measure_text(&model.message);
        let window_origin = vec2i(PADDING, PADDING);
        let window_size = vec2i(PADDING * 2 + message_size, TOOLTIP_HEIGHT);
        debug_ui_presenter.ui_presenter.draw_solid_rounded_rect(
            device,
            RectI::new(window_origin, window_size),
            WINDOW_COLOR,
        );
        debug_ui_presenter.ui_presenter.draw_text(
            device,
            &model.message,
            window_origin + vec2i(PADDING, PADDING + FONT_ASCENT),
            false,
        );
    }

    fn draw_effects_panel(&mut self,
                          device: &D,
                          debug_ui_presenter: &mut DebugUIPresenter<D>,
                          model: &mut DemoUIModel,
                          action: &mut UIAction) {
        if !self.effects_panel_visible {
            return;
        }

        let bottom = debug_ui_presenter.ui_presenter.framebuffer_size().y() - PADDING;
        let effects_panel_y = bottom - (BUTTON_HEIGHT + PADDING + EFFECTS_PANEL_HEIGHT);
        debug_ui_presenter.ui_presenter.draw_solid_rounded_rect(
            device,
            RectI::new(vec2i(PADDING, effects_panel_y),
                       vec2i(EFFECTS_PANEL_WIDTH, EFFECTS_PANEL_HEIGHT)),
            WINDOW_COLOR,
        );

        self.draw_effects_switch(
            device,
            action,
            debug_ui_presenter,
            "Gamma Correction",
            0,
            effects_panel_y,
            &mut model.gamma_correction_effect_enabled);
        self.draw_effects_switch(
            device,
            action,
            debug_ui_presenter,
            "Stem Darkening",
            1,
            effects_panel_y,
            &mut model.stem_darkening_effect_enabled);
        self.draw_effects_switch(
            device,
            action,
            debug_ui_presenter,
            "Subpixel AA",
            2,
            effects_panel_y,
            &mut model.subpixel_aa_effect_enabled);
    }

    fn draw_screenshot_panel<W>(
        &mut self,
        device: &D,
        window: &mut W,
        debug_ui_presenter: &mut DebugUIPresenter<D>,
        panel_x: i32,
        action: &mut UIAction,
    ) where W: Window {
        if !self.screenshot_panel_visible {
            return;
        }

        let bottom = debug_ui_presenter.ui_presenter.framebuffer_size().y() - PADDING;
        let panel_y = bottom - (BUTTON_HEIGHT + PADDING + SCREENSHOT_PANEL_HEIGHT);
        let panel_position = vec2i(panel_x, panel_y);
        debug_ui_presenter.ui_presenter.draw_solid_rounded_rect(
            device,
            RectI::new(panel_position, vec2i(SCREENSHOT_PANEL_WIDTH, SCREENSHOT_PANEL_HEIGHT)),
            WINDOW_COLOR,
        );

        self.draw_screenshot_menu_item(
            device,
            window,
            debug_ui_presenter,
            ScreenshotType::PNG,
            panel_position,
            action,
        );
        self.draw_screenshot_menu_item(
            device,
            window,
            debug_ui_presenter,
            ScreenshotType::SVG,
            panel_position,
            action,
        );
    }

    fn draw_background_panel(
        &mut self,
        device: &D,
        debug_ui_presenter: &mut DebugUIPresenter<D>,
        panel_x: i32,
        action: &mut UIAction,
        model: &mut DemoUIModel,
    ) {
        if !self.background_panel_visible {
            return;
        }

        let bottom = debug_ui_presenter.ui_presenter.framebuffer_size().y() - PADDING;
        let panel_y = bottom - (BUTTON_HEIGHT + PADDING + BACKGROUND_PANEL_HEIGHT);
        let panel_position = vec2i(panel_x, panel_y);
        debug_ui_presenter.ui_presenter.draw_solid_rounded_rect(
            device,
            RectI::new(panel_position, vec2i(BACKGROUND_PANEL_WIDTH, BACKGROUND_PANEL_HEIGHT)),
            WINDOW_COLOR,
        );

        self.draw_background_menu_item(
            device,
            debug_ui_presenter,
            BackgroundColor::Light,
            panel_position,
            action,
            model,
        );
        self.draw_background_menu_item(
            device,
            debug_ui_presenter,
            BackgroundColor::Dark,
            panel_position,
            action,
            model,
        );
        self.draw_background_menu_item(
            device,
            debug_ui_presenter,
            BackgroundColor::Transparent,
            panel_position,
            action,
            model,
        );
    }

    fn draw_rotate_panel(
        &mut self,
        device: &D,
        debug_ui_presenter: &mut DebugUIPresenter<D>,
        rotate_panel_x: i32,
        action: &mut UIAction,
        model: &mut DemoUIModel
    ) {
        if !self.rotate_panel_visible {
            return;
        }

        let bottom = debug_ui_presenter.ui_presenter.framebuffer_size().y() - PADDING;
        let rotate_panel_y = bottom - (BUTTON_HEIGHT + PADDING + ROTATE_PANEL_HEIGHT);
        let rotate_panel_origin = vec2i(rotate_panel_x, rotate_panel_y);
        let rotate_panel_size = vec2i(ROTATE_PANEL_WIDTH, ROTATE_PANEL_HEIGHT);
        debug_ui_presenter.ui_presenter.draw_solid_rounded_rect(
            device,
            RectI::new(rotate_panel_origin, rotate_panel_size),
            WINDOW_COLOR,
        );

        let (widget_x, widget_y) = (rotate_panel_x + PADDING, rotate_panel_y + PADDING);
        let widget_rect = RectI::new(vec2i(widget_x, widget_y),
                                     vec2i(SLIDER_WIDTH, SLIDER_KNOB_HEIGHT));
        if let Some(position) = debug_ui_presenter
            .ui_presenter
            .event_queue
            .handle_mouse_down_or_dragged_in_rect(widget_rect)
        {
            model.rotation = position.x();
            *action = UIAction::Rotate(model.rotation());
        }

        let slider_track_y =
            rotate_panel_y + PADDING + SLIDER_KNOB_HEIGHT / 2 - SLIDER_TRACK_HEIGHT / 2;
        let slider_track_rect = RectI::new(vec2i(widget_x, slider_track_y),
                                           vec2i(SLIDER_WIDTH, SLIDER_TRACK_HEIGHT));
        debug_ui_presenter.ui_presenter.draw_rect_outline(device, slider_track_rect, TEXT_COLOR);

        let slider_knob_x = widget_x + model.rotation - SLIDER_KNOB_WIDTH / 2;
        let slider_knob_rect = RectI::new(vec2i(slider_knob_x, widget_y),
                                          vec2i(SLIDER_KNOB_WIDTH, SLIDER_KNOB_HEIGHT));
        debug_ui_presenter.ui_presenter.draw_solid_rect(device, slider_knob_rect, TEXT_COLOR);
    }

    fn draw_screenshot_menu_item<W>(
        &mut self,
        device: &D,
        window: &mut W,
        debug_ui_presenter: &mut DebugUIPresenter<D>,
        screenshot_type: ScreenshotType,
        panel_position: Vector2I,
        action: &mut UIAction,
    ) where W: Window {
        let index = screenshot_type as i32;
        let text = format!("Save as {}...", screenshot_type.as_str());

        let widget_size = vec2i(BACKGROUND_PANEL_WIDTH, BUTTON_HEIGHT);
        let widget_origin = panel_position + vec2i(0, widget_size.y() * index);
        let widget_rect = RectI::new(widget_origin, widget_size);

        if self.draw_menu_item(device,
                               debug_ui_presenter,
                               &text,
                               widget_rect,
                               false) {
            // FIXME(pcwalton): This is not sufficient for Android, where we will need to take in
            // the contents of the file.
            if let Ok(path) = window.run_save_dialog(screenshot_type.extension()) {
                self.screenshot_panel_visible = false;
                *action = UIAction::TakeScreenshot(ScreenshotInfo { kind: screenshot_type, path });
            }
        }
    }

    fn draw_background_menu_item(
        &mut self,
        device: &D,
        debug_ui_presenter: &mut DebugUIPresenter<D>,
        color: BackgroundColor,
        panel_position: Vector2I,
        action: &mut UIAction,
        model: &mut DemoUIModel,
    ) {
        let (text, index) = (color.as_str(), color as i32);

        let widget_size = vec2i(BACKGROUND_PANEL_WIDTH, BUTTON_HEIGHT);
        let widget_origin = panel_position + vec2i(0, widget_size.y() * index);
        let widget_rect = RectI::new(widget_origin, widget_size);

        let selected = color == model.background_color;
        if self.draw_menu_item(device,
                               debug_ui_presenter,
                               text,
                               widget_rect,
                               selected) {
            model.background_color = color;
            *action = UIAction::ModelChanged;
        }
    }

    fn draw_menu_item(&self,
                      device: &D,
                      debug_ui_presenter: &mut DebugUIPresenter<D>,
                      text: &str,
                      widget_rect: RectI,
                      selected: bool)
                      -> bool {
        if selected {
            debug_ui_presenter.ui_presenter.draw_solid_rounded_rect(device,
                                                                    widget_rect,
                                                                    TEXT_COLOR);
        }

        let (text_x, text_y) = (PADDING * 2, BUTTON_TEXT_OFFSET);
        let text_position = widget_rect.origin() + vec2i(text_x, text_y);
        debug_ui_presenter.ui_presenter.draw_text(device, text, text_position, selected);

        debug_ui_presenter.ui_presenter
                          .event_queue
                          .handle_mouse_down_in_rect(widget_rect)
                          .is_some()
    }

    fn draw_effects_switch(
        &self,
        device: &D,
        action: &mut UIAction,
        debug_ui_presenter: &mut DebugUIPresenter<D>,
        text: &str,
        index: i32,
        window_y: i32,
        value: &mut bool) {
        let text_x = PADDING * 2;
        let text_y = window_y + PADDING + BUTTON_TEXT_OFFSET + (BUTTON_HEIGHT + PADDING) * index;
        debug_ui_presenter.ui_presenter.draw_text(device, text, vec2i(text_x, text_y), false);

        let switch_width = debug_ui_presenter.ui_presenter.measure_segmented_control(2);
        let switch_x = PADDING + EFFECTS_PANEL_WIDTH - (switch_width + PADDING);
        let switch_y = window_y + PADDING + (BUTTON_HEIGHT + PADDING) * index;
        let switch_position = vec2i(switch_x, switch_y);

        let new_value =
            debug_ui_presenter
                .ui_presenter
                .draw_text_switch(device, switch_position, &["Off", "On"], *value as u8) != 0;

        if new_value != *value {
            *action = UIAction::EffectsChanged;
            *value = new_value;
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum UIAction {
    None,
    ModelChanged,
    EffectsChanged,
    TakeScreenshot(ScreenshotInfo),
    ZoomIn,
    ZoomActualSize,
    ZoomOut,
    Rotate(f32),
}

#[derive(Clone, Debug, PartialEq)]
pub struct ScreenshotInfo {
    pub kind: ScreenshotType,
    pub path: PathBuf,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ScreenshotType {
    PNG = 0,
    SVG = 1,
}

impl ScreenshotType {
    fn extension(&self) -> &'static str {
        match *self {
            ScreenshotType::PNG => "png",
            ScreenshotType::SVG => "svg",
        }
    }

    fn as_str(&self) -> &'static str {
        match *self {
            ScreenshotType::PNG => "PNG",
            ScreenshotType::SVG => "SVG",
        }
    }
}
