//! helper to extract span stats from a chrome trace file
//! spec: <https://docs.google.com/document/d/1CvAClvFfyA5R-PhYUmn5OOQtYMH4h6I0nSsKchNAySU/preview#heading=h.puwqg050lyuy>

use std::{
    collections::HashMap,
    fs::File,
    io::{BufReader, Read},
    ops::Div,
};

use clap::Parser;
use lazy_static::lazy_static;
use regex::Regex;
use serde::Deserialize;
use termcolor::{Color, ColorChoice, ColorSpec, StandardStream, WriteColor};

use bevy_reflect::TypeRegistration;

/// A span from the trace
#[derive(Deserialize, Debug)]
struct Span {
    /// name
    name: String,
    /// phase
    ph: String,
    /// timestamp
    ts: f32,
}

/// Ignore entries in the trace that are not a span
#[derive(Deserialize, Debug)]
struct Ignore {}

/// deserialize helper
#[derive(Deserialize, Debug)]
#[serde(untagged)]
enum SpanOrIgnore {
    /// deserialize as a span
    Span(Span),
    /// catchall that didn't match a span
    Ignore(Ignore),
}

/// Trace files are unfinished and not properly formed json
/// this wrapper finishes the json so that its valid
struct UnfinishedWrapper {
    reader: BufReader<File>,
    buf: [u8; 1],
    finish: String,
}

impl UnfinishedWrapper {
    fn from(reader: BufReader<File>) -> UnfinishedWrapper {
        Self {
            reader,
            buf: [0; 1],
            finish: "{}]".to_string(),
        }
    }
}

impl Read for UnfinishedWrapper {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize, std::io::Error> {
        let last = self.reader.read(self.buf.as_mut());
        if matches!(last, Ok(0)) && self.buf[0] == 10 && !self.finish.is_empty() {
            let (next, remaining) = self.finish.as_bytes().split_at(1);
            buf[0] = next[0];
            self.finish = std::str::from_utf8(remaining).unwrap().to_string();
            return Ok(1);
        }
        buf[0] = self.buf[0];
        last
    }
}

#[derive(Parser, Debug)]
struct Args {
    #[clap(short, long, default_value_t = 0.0)]
    /// Filter spans that have an average shorther than the threshold
    threshold: f32,

    #[clap(short, long)]
    /// Filter spans by name matching the pattern
    pattern: Option<Regex>,

    #[clap(short, long)]
    /// Simplify system names
    short: bool,

    trace: String,
    /// Optional, second trace to compare
    second_trace: Option<String>,
}

fn main() {
    let cli = Args::parse();

    // Setup stdout to support colors
    let mut stdout = StandardStream::stdout(ColorChoice::Auto);

    // Read the first trace file
    let reference = read_trace(cli.trace);
    if let Some(comparison) = cli.second_trace {
        // Read the second trace file
        let mut comparison = read_trace(comparison);

        reference
            .iter()
            .filter(|(_, stats)| filter_by_threshold(stats, cli.threshold))
            .filter(|(name, _)| filter_by_pattern(name, cli.pattern.as_ref()))
            .for_each(|(span, reference)| {
                // for each span in the first trace
                set_bold(&mut stdout, true);
                if cli.short {
                    println!("{}", simplify_name(span));
                } else {
                    println!("{}", span);
                }
                set_bold(&mut stdout, false);
                print!("  ");
                let comparison = comparison.remove(span);
                print_spanstats(&mut stdout, Some(reference), comparison.as_ref(), false);
            });
        comparison
            .iter()
            .filter(|(_, stats)| filter_by_threshold(stats, cli.threshold))
            .filter(|(name, _)| filter_by_pattern(name, cli.pattern.as_ref()))
            .for_each(|(span, comparison)| {
                // print the spans only present in the second trace
                set_bold(&mut stdout, true);
                if cli.short {
                    println!("{}", simplify_name(span));
                } else {
                    println!("{}", span);
                }
                set_bold(&mut stdout, false);
                print!("  ");
                print_spanstats(&mut stdout, None, Some(comparison), false);
            });
    } else {
        // just print stats from the first trace
        reference
            .iter()
            .filter(|(_, stats)| filter_by_threshold(stats, cli.threshold))
            .filter(|(name, _)| filter_by_pattern(name, cli.pattern.as_ref()))
            .for_each(|(span, reference)| {
                set_bold(&mut stdout, true);
                if cli.short {
                    println!("{}", simplify_name(span));
                } else {
                    println!("{}", span);
                }
                set_bold(&mut stdout, false);
                print!("  ");
                print_spanstats(&mut stdout, Some(reference), None, true);
            });
    }
}

fn filter_by_threshold(span_stats: &SpanStats, threshold: f32) -> bool {
    span_stats.avg > threshold
}

fn filter_by_pattern(name: &str, pattern: Option<&Regex>) -> bool {
    if let Some(pattern) = pattern {
        pattern.is_match(name)
    } else {
        true
    }
}

fn read_trace(file: String) -> HashMap<String, SpanStats> {
    let file = File::open(file).unwrap();
    let reader = BufReader::new(file);
    let reader_wrapper = UnfinishedWrapper::from(reader);

    let spans: Vec<SpanOrIgnore> = serde_json::from_reader(reader_wrapper).unwrap();

    let mut open_spans: HashMap<String, f32> = HashMap::new();
    let mut all_spans: HashMap<String, Vec<f32>> = HashMap::new();

    spans
        .iter()
        .flat_map(|s| {
            if let SpanOrIgnore::Span(span) = s {
                Some(span)
            } else {
                None
            }
        })
        .for_each(|s| {
            if s.ph == "B" {
                open_spans.insert(s.name.clone(), s.ts);
            } else if s.ph == "E" {
                let begin = open_spans.get(&s.name).unwrap();
                all_spans
                    .entry(s.name.clone())
                    .or_default()
                    .push(s.ts - begin);
            }
        });
    let all_spans_stats: HashMap<_, _> = all_spans
        .into_iter()
        .map(|(name, durations)| {
            (
                name,
                SpanStats {
                    count: durations.len(),
                    min: *durations
                        .iter()
                        .min_by(|x, y| x.partial_cmp(y).unwrap())
                        .unwrap(),
                    max: *durations
                        .iter()
                        .max_by(|x, y| x.partial_cmp(y).unwrap())
                        .unwrap(),
                    avg: durations
                        .iter()
                        .fold((0.0, 0), |(current, count), new| {
                            (
                                (current * count as f32 + new) / (count as f32 + 1.0),
                                count + 1,
                            )
                        })
                        .0,
                },
            )
        })
        .collect();
    all_spans_stats
}

#[derive(Debug)]
struct SpanStats {
    count: usize,
    avg: f32,
    min: f32,
    max: f32,
}

fn print_spanstats(
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

struct SpanRelative {
    count: f32,
    avg: f32,
    min: f32,
    max: f32,
}

impl Div for &SpanStats {
    type Output = SpanRelative;

    fn div(self, rhs: Self) -> Self::Output {
        Self::Output {
            count: self.count as f32 / rhs.count as f32,
            avg: self.avg / rhs.avg,
            min: self.min / rhs.min,
            max: self.max / rhs.max,
        }
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
fn simplify_name(name: &str) -> String {
    if let Some(captures) = SYSTEM_NAME.captures(name) {
        return format!(
            r#"system: name="{}""#,
            TypeRegistration::get_short_name(&captures[1])
        );
    }
    if let Some(captures) = SYSTEM_OVERHEAD.captures(name) {
        return format!(
            r#"system overhead: name="{}""#,
            TypeRegistration::get_short_name(&captures[1])
        );
    }
    if let Some(captures) = SYSTEM_COMMANDS.captures(name) {
        return format!(
            r#"system_commands: name="{}""#,
            TypeRegistration::get_short_name(&captures[1])
        );
    }
    name.to_string()
}

fn set_fg(stdout: &mut StandardStream, color: Color) {
    stdout
        .set_color(ColorSpec::new().set_fg(Some(color)))
        .unwrap();
}
fn set_bold(stdout: &mut StandardStream, bold: bool) {
    stdout.set_color(ColorSpec::new().set_bold(bold)).unwrap();
}
