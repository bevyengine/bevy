//! helper to extract span stats from a chrome trace file
//! spec: <https://docs.google.com/document/d/1CvAClvFfyA5R-PhYUmn5OOQtYMH4h6I0nSsKchNAySU/preview#heading=h.puwqg050lyuy>

use std::{
    collections::HashMap,
    fs::File,
    io::{BufReader, Read},
};

use clap::Parser;
use regex::Regex;
use serde::Deserialize;
use termcolor::{Color, ColorChoice, ColorSpec, StandardStream, WriteColor};

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
    buf: Box<[u8; 1]>,
    finish: String,
}

impl UnfinishedWrapper {
    fn from(reader: BufReader<File>) -> UnfinishedWrapper {
        Self {
            reader,
            // last: Ok(0),
            buf: Box::new([0; 1]),
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

    trace: String,
    second_trace: Option<String>,
}

fn main() {
    let cli = Args::parse();

    // Read the first trace file
    let first = read_trace(cli.trace);
    if let Some(second) = cli.second_trace {
        // Read the second trace file
        let mut second = read_trace(second);

        // Setup stdout to support colors
        let mut stdout = StandardStream::stdout(ColorChoice::Auto);

        first
            .iter()
            .filter(|(_, stats)| filter_by_threshold(stats, cli.threshold))
            .filter(|(name, _)| filter_by_pattern(name, cli.pattern.as_ref()))
            .for_each(|(span, stats)| {
                // for each span in the first trace
                println!("{}", span);
                if let Some(other) = second.remove(span) {
                    // if there is a matching span in the second trace, compare the two
                    print!("  ");
                    if stats.avg > other.avg {
                        stdout
                            .set_color(ColorSpec::new().set_fg(Some(Color::Green)))
                            .unwrap();
                        print!("{:1.2}", 1.0);
                    } else {
                        print!("{:1.2}", other.avg / stats.avg);
                    }
                    print!(
                        "    {:10} {:15} {:15} {:15}",
                        stats.count, stats.min, stats.avg, stats.max,
                    );
                    stdout
                        .set_color(ColorSpec::new().set_fg(Some(Color::White)))
                        .unwrap();
                    print!("       |      ");
                    if stats.avg < other.avg {
                        stdout
                            .set_color(ColorSpec::new().set_fg(Some(Color::Green)))
                            .unwrap();
                        print!("{:1.2}", 1.0);
                    } else {
                        print!("{:1.2}", stats.avg / other.avg);
                    }

                    println!(
                        "    {:10} {:15} {:15} {:15}",
                        other.count, other.min, other.avg, other.max,
                    );
                    stdout
                        .set_color(ColorSpec::new().set_fg(Some(Color::White)))
                        .unwrap();
                } else {
                    // print the spans only present in the first trace
                    println!(
                        "{:10} {:15} {:15} {:15}",
                        stats.count, stats.min, stats.avg, stats.max
                    );
                }
            });
        second
            .iter()
            .filter(|(_, stats)| filter_by_threshold(stats, cli.threshold))
            .filter(|(name, _)| filter_by_pattern(name, cli.pattern.as_ref()))
            .for_each(|(span, stats)| {
                // print the spans only present in the second trace
                println!("{}", span);
                print!("  ");
                print!("    ");
                print!("    {:10} {:15} {:15} {:15}", "", "", "", "",);
                print!("       |      ");
                print!("    ");
                println!(
                    "    {:10} {:15} {:15} {:15}",
                    stats.count, stats.min, stats.avg, stats.max
                );
            });
    } else {
        // just print stats from the first trace
        first
            .iter()
            .filter(|(_, stats)| filter_by_threshold(stats, cli.threshold))
            .filter(|(name, _)| filter_by_pattern(name, cli.pattern.as_ref()))
            .for_each(|(span, stats)| {
                println!("{}", span);
                println!(
                    "    {:10} {:15} {:15} {:15}",
                    stats.count, stats.min, stats.avg, stats.max
                );
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
