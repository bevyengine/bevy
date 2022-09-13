use bevy_utils::get_short_name;
use lazy_static::lazy_static;
use regex::Regex;
use termcolor::{Color, ColorSpec, StandardStream, WriteColor};

use crate::SpanStats;

pub fn print_spanstats(
    stdout: &mut StandardStream,
    reference: Option<&SpanStats>,
    comparison: Option<&SpanStats>,
    reference_only: bool,
) {
    match (reference, comparison) {
        (Some(reference), Some(comparison)) if !reference_only => {
            let relative = comparison / reference;

            print!("[count: {:8} | {:8} | ", reference.count, comparison.count);
            print_relative(stdout, relative.count);
            print!("]  [min: ");
            print_delta_time_us(reference.min);
            print!(" | ");
            print_delta_time_us(comparison.min);
            print!(" | ");
            print_relative(stdout, relative.min);
            print!("]  [avg: ");
            print_delta_time_us(reference.avg);
            print!(" | ");
            print_delta_time_us(comparison.avg);
            print!(" | ");
            print_relative(stdout, relative.avg);
            print!("]  [max: ");
            print_delta_time_us(reference.max);
            print!(" | ");
            print_delta_time_us(comparison.max);
            print!(" | ");
            print_relative(stdout, relative.max);
            println!("]");
        }
        (Some(reference), None) if !reference_only => {
            print!(
                "[count: {:8} |          |        ]  [min:  ",
                reference.count
            );
            print_delta_time_us(reference.min);
            print!(" |         |         ]  [avg: ");
            print_delta_time_us(reference.avg);
            print!(" |         |         ]  [max: ");
            print_delta_time_us(reference.max);
            println!(" |         |         ]");
        }
        (None, Some(comparison)) => {
            print!("[count:          | {:8} |         ", comparison.count);
            print!("]  [min:         | ");
            print_delta_time_us(comparison.min);
            print!(" |         ]  [avg:         | ");
            print_delta_time_us(comparison.avg);
            print!(" |         ]  [max:         | ");
            print_delta_time_us(comparison.max);
            println!(" |         ]");
        }
        (Some(reference), _) if reference_only => {
            print!("[count: {:8}]  [min: ", reference.count);
            print_delta_time_us(reference.min);
            print!("]  [avg: ");
            print_delta_time_us(reference.avg);
            print!("]  [max: ");
            print_delta_time_us(reference.max);
            println!("]");
        }
        _ => {}
    }
}

const MARGIN_PERCENT: f32 = 2.0;
fn print_relative(stdout: &mut StandardStream, v: f32) {
    let v_delta_percent = if v.is_nan() { 0.0 } else { (v - 1.0) * 100.0 };
    set_fg(
        stdout,
        if v_delta_percent > MARGIN_PERCENT {
            Color::Red
        } else if v_delta_percent < -MARGIN_PERCENT {
            Color::Green
        } else {
            Color::White
        },
    );
    if v_delta_percent > MARGIN_PERCENT {
        print!("+");
    } else if v_delta_percent >= -MARGIN_PERCENT {
        print!(" ");
    } else {
        print!("-");
    }
    print_base10f32_fixed_width(v_delta_percent.abs(), 1.0);
    print!("%");
    set_fg(stdout, Color::White);
}

// Try to print time values using 4 numeric digits, a decimal point, and the unit
const ONE_US_IN_SECONDS: f32 = 1e-6;

fn print_delta_time_us(dt_us: f32) {
    print_base10f32_fixed_width(dt_us, ONE_US_IN_SECONDS);
    print!("s");
}

fn print_base10f32_fixed_width(v: f32, v_scale: f32) {
    Scale::from_value_and_scale(v, v_scale).print_with_scale(v, v_scale);
}

#[derive(Debug)]
pub struct Scale {
    name: &'static str,
    scale_factor: f32,
}

impl Scale {
    pub const TERA: f32 = 1e12;
    pub const GIGA: f32 = 1e9;
    pub const MEGA: f32 = 1e6;
    pub const KILO: f32 = 1e3;
    pub const UNIT: f32 = 1e0;
    pub const MILLI: f32 = 1e-3;
    pub const MICRO: f32 = 1e-6;
    pub const NANO: f32 = 1e-9;
    pub const PICO: f32 = 1e-12;

    pub fn from_value_and_scale(v: f32, v_scale: f32) -> Self {
        assert!(v >= 0.0);
        if v == 0.0 {
            Self {
                name: " ",
                scale_factor: Self::UNIT,
            }
        } else if v * v_scale >= Self::TERA {
            Self {
                name: "T",
                scale_factor: Self::TERA,
            }
        } else if v * v_scale >= Self::GIGA {
            Self {
                name: "G",
                scale_factor: Self::GIGA,
            }
        } else if v * v_scale >= Self::MEGA {
            Self {
                name: "M",
                scale_factor: Self::MEGA,
            }
        } else if v * v_scale >= Self::KILO {
            Self {
                name: "k",
                scale_factor: Self::KILO,
            }
        } else if v * v_scale >= Self::UNIT {
            Self {
                name: " ",
                scale_factor: Self::UNIT,
            }
        } else if v * v_scale >= Self::MILLI {
            Self {
                name: "m",
                scale_factor: Self::MILLI,
            }
        } else if v * v_scale >= Self::MICRO {
            Self {
                name: "µ",
                scale_factor: Self::MICRO,
            }
        } else if v * v_scale >= Self::NANO {
            Self {
                name: "n",
                scale_factor: Self::NANO,
            }
        } else {
            Self {
                name: "p",
                scale_factor: Self::PICO,
            }
        }
    }

    pub fn print(&self, v: f32) {
        // NOTE: Hacks for rounding to decimal places to ensure precision is correct
        let precision = if ((v * 10.0).round() / 10.0) >= 100.0 {
            1
        } else if ((v * 100.0).round() / 100.0) >= 10.0 {
            2
        } else {
            3
        };
        print!("{:5.precision$}{}", v, self.name, precision = precision);
    }

    pub fn print_with_scale(&self, v: f32, v_scale: f32) {
        self.print(v * v_scale / self.scale_factor);
    }
}

lazy_static! {
    static ref SYSTEM_NAME: Regex = Regex::new(r#"system: name="([^"]+)""#).unwrap();
    static ref SYSTEM_OVERHEAD: Regex = Regex::new(r#"system overhead: name="([^"]+)""#).unwrap();
    static ref SYSTEM_COMMANDS: Regex = Regex::new(r#"system_commands: name="([^"]+)""#).unwrap();
}

pub fn simplify_name(name: &str) -> String {
    if let Some(captures) = SYSTEM_NAME.captures(name) {
        return format!(r#"system: name="{}""#, get_short_name(&captures[1]));
    }
    if let Some(captures) = SYSTEM_OVERHEAD.captures(name) {
        return format!(
            r#"system overhead: name="{}""#,
            get_short_name(&captures[1])
        );
    }
    if let Some(captures) = SYSTEM_COMMANDS.captures(name) {
        return format!(
            r#"system_commands: name="{}""#,
            get_short_name(&captures[1])
        );
    }
    name.to_string()
}

fn set_fg(stdout: &mut StandardStream, color: Color) {
    stdout
        .set_color(ColorSpec::new().set_fg(Some(color)))
        .unwrap();
}

pub fn set_bold(stdout: &mut StandardStream, bold: bool) {
    stdout.set_color(ColorSpec::new().set_bold(bold)).unwrap();
}
