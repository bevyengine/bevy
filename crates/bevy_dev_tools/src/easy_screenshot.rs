use std::time::{SystemTime, UNIX_EPOCH};
#[cfg(feature = "screenrecording")]
use std::{fs::File, io::Write, sync::mpsc::channel};

use bevy_app::{App, Plugin, Update};
use bevy_ecs::prelude::*;
#[cfg(feature = "screenrecording")]
use bevy_image::Image;
use bevy_input::{common_conditions::input_just_pressed, keyboard::KeyCode};
#[cfg(feature = "screenrecording")]
use bevy_render::view::screenshot::ScreenshotCaptured;
use bevy_render::view::screenshot::{save_to_disk, Screenshot};
#[cfg(feature = "screenrecording")]
use bevy_time::Time;
use bevy_window::{PrimaryWindow, Window};
#[cfg(feature = "screenrecording")]
use tracing::info;
#[cfg(feature = "screenrecording")]
use x264::{Colorspace, Encoder, Setup};
#[cfg(feature = "screenrecording")]
pub use x264::{Preset, Tune};

/// File format the screenshot will be saved in
#[derive(Clone, Copy)]
pub enum ScreenshotFormat {
    /// JPEG format
    Jpeg,
    /// PNG format
    Png,
    /// BMP format
    Bmp,
}

/// Add this plugin to your app to enable easy screenshotting.
///
/// Add this plugin, press the key, and you have a screenshot 🎉
pub struct EasyScreenshotPlugin {
    /// Key that will trigger a screenshot
    pub trigger: KeyCode,
    /// Format of the screenshot
    ///
    /// The corresponding image format must be supported by bevy renderer
    pub format: ScreenshotFormat,
}

impl Default for EasyScreenshotPlugin {
    fn default() -> Self {
        EasyScreenshotPlugin {
            trigger: KeyCode::PrintScreen,
            format: ScreenshotFormat::Png,
        }
    }
}

impl Plugin for EasyScreenshotPlugin {
    fn build(&self, app: &mut App) {
        let format = self.format;
        app.add_systems(
            Update,
            (move |mut commands: Commands, window: Single<&Window, With<PrimaryWindow>>| {
                let since_the_epoch = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .expect("time should go forward");

                commands
                    .spawn(Screenshot::primary_window())
                    .observe(save_to_disk(format!(
                        "{}-{}.{}",
                        window.title,
                        since_the_epoch.as_millis(),
                        match format {
                            ScreenshotFormat::Jpeg => "jpg",
                            ScreenshotFormat::Png => "png",
                            ScreenshotFormat::Bmp => "bmp",
                        }
                    )));
            })
            .run_if(input_just_pressed(self.trigger)),
        );
    }
}

#[cfg(feature = "screenrecording")]
/// Add this plugin to your app to enable easy screen recording.
pub struct EasyScreenRecordPlugin {
    /// The key to toggle recording.
    pub toggle: KeyCode,
    /// h264 encoder preset
    pub preset: Preset,
    /// h264 encoder tune
    pub tune: Tune,
}

#[cfg(feature = "screenrecording")]
impl Default for EasyScreenRecordPlugin {
    fn default() -> Self {
        EasyScreenRecordPlugin {
            toggle: KeyCode::Space,
            preset: Preset::Medium,
            tune: Tune::Animation,
        }
    }
}

#[cfg(feature = "screenrecording")]
#[expect(
    clippy::large_enum_variant,
    reason = "Large variant happens a lot more often than the others"
)]
enum RecordCommand {
    Start(String, Preset, Tune),
    Stop,
    Frame(Image, f64),
}

#[cfg(feature = "screenrecording")]
/// Controls screen recording
#[derive(Message)]
pub enum RecordScreen {
    /// Starts screen recording
    Start,
    /// Stops screen recording
    Stop,
}

#[cfg(feature = "screenrecording")]
impl Plugin for EasyScreenRecordPlugin {
    fn build(&self, app: &mut App) {
        let (tx, rx) = channel::<RecordCommand>();

        std::thread::spawn(move || {
            let mut encoder: Option<Encoder> = None;
            let mut setup = None;
            let mut file: Option<File> = None;
            let mut first_frame_time = None;
            let mut previous_pts = 0;
            loop {
                let Ok(next) = rx.recv() else {
                    break;
                };
                match next {
                    RecordCommand::Start(name, preset, tune) => {
                        info!("starting recording at {}", name);
                        file = Some(File::create(name).unwrap());
                        first_frame_time = None;
                        setup = Some(Setup::preset(preset, tune, false, true).high());
                    }
                    RecordCommand::Stop => {
                        info!("stopping recording");
                        if let Some(encoder) = encoder.take() {
                            let mut flush = encoder.flush();
                            let mut file = file.take().unwrap();
                            while let Some(result) = flush.next() {
                                let (data, _) = result.unwrap();
                                file.write_all(data.entirety()).unwrap();
                            }
                        }
                    }
                    RecordCommand::Frame(image, frame_time) => {
                        if first_frame_time.is_none() {
                            first_frame_time = Some(frame_time);
                            continue;
                        }
                        if let Some(setup) = setup.take() {
                            let mut new_encoder = setup
                                .fps((1.0 / (frame_time - first_frame_time.unwrap())) as u32, 1)
                                .build(Colorspace::RGB, image.width() as i32, image.height() as i32)
                                .unwrap();
                            let headers = new_encoder.headers().unwrap();
                            file.as_mut()
                                .unwrap()
                                .write_all(headers.entirety())
                                .unwrap();
                            encoder = Some(new_encoder);
                        }
                        if let Some(encoder) = encoder.as_mut() {
                            let pts = ((frame_time - first_frame_time.unwrap()) * 1000.0) as i64;
                            if pts == previous_pts {
                                continue;
                            }
                            previous_pts = pts;

                            let (data, _) = encoder
                                .encode(
                                    pts,
                                    x264::Image::rgb(
                                        image.width() as i32,
                                        image.height() as i32,
                                        &image.try_into_dynamic().unwrap().to_rgb8(),
                                    ),
                                )
                                .unwrap();
                            file.as_mut().unwrap().write_all(data.entirety()).unwrap();
                        }
                    }
                }
            }
        });

        app.add_message::<RecordScreen>().add_systems(
            Update,
            (
                (move |mut messages: MessageWriter<RecordScreen>, mut recording: Local<bool>| {
                    *recording = !*recording;
                    if *recording {
                        messages.write(RecordScreen::Start);
                    } else {
                        messages.write(RecordScreen::Stop);
                    }
                })
                .run_if(input_just_pressed(self.toggle)),
                {
                    let tx = tx.clone();
                    let preset = self.preset;
                    let tune = self.tune;
                    move |mut commands: Commands,
                          mut recording: Local<bool>,
                          mut messages: MessageReader<RecordScreen>,
                          window: Single<&Window, With<PrimaryWindow>>| {
                        match messages.read().last() {
                            Some(RecordScreen::Start) => {
                                let since_the_epoch = SystemTime::now()
                                    .duration_since(UNIX_EPOCH)
                                    .expect("time should go forward");
                                let filename = format!(
                                    "{}-{}.h264",
                                    window.title,
                                    since_the_epoch.as_millis(),
                                );
                                tx.send(RecordCommand::Start(filename, preset, tune))
                                    .unwrap();
                                *recording = true;
                            }
                            Some(RecordScreen::Stop) => {
                                tx.send(RecordCommand::Stop).unwrap();
                                *recording = false;
                            }
                            _ => {}
                        }
                        if *recording {
                            let tx = tx.clone();
                            commands.spawn(Screenshot::primary_window()).observe(
                                move |screenshot_captured: On<ScreenshotCaptured>,
                                      time: Res<Time>| {
                                    let img = screenshot_captured.image.clone();
                                    tx.send(RecordCommand::Frame(
                                        img,
                                        time.elapsed().as_secs_f64(),
                                    ))
                                    .unwrap();
                                },
                            );
                        }
                    }
                },
            )
                .chain(),
        );
    }
}
