//! helper to extract span stats from a chrome trace file
//! spec: <https://docs.google.com/document/d/1CvAClvFfyA5R-PhYUmn5OOQtYMH4h6I0nSsKchNAySU/preview#heading=h.puwqg050lyuy>

use std::ops::Div;

use clap::Parser;
use parse::read_trace;
use regex::Regex;
use termcolor::{ColorChoice, StandardStream};

use crate::pretty::{print_spanstats, set_bold, simplify_name};

mod parse;
mod pretty;

#[derive(Parser, Debug)]
struct Args {
    #[arg(short, long, default_value_t = 0.0)]
    /// Filter spans that have an average shorther than the threshold
    threshold: f32,

    #[arg(short, long)]
    /// Filter spans by name matching the pattern
    pattern: Option<Regex>,

    #[arg(short, long)]
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
                    println!("{span}");
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
                    println!("{span}");
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
                    println!("{span}");
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

#[derive(Debug)]
pub struct SpanStats {
    pub count: usize,
    pub avg: f32,
    pub min: f32,
    pub max: f32,
}

impl Default for SpanStats {
    fn default() -> Self {
        Self {
            count: 0,
            avg: 0.0,
            min: f32::MAX,
            max: 0.0,
        }
    }
}

impl SpanStats {
    fn add_span(&mut self, duration: f32) {
        if duration < self.min {
            self.min = duration;
        }
        if duration > self.max {
            self.max = duration;
        }
        self.avg = (self.avg * self.count as f32 + duration) / (self.count as f32 + 1.0);
        self.count += 1;
    }
}

pub struct SpanRelative {
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
