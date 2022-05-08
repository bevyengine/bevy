use std::{
    cell::RefCell,
    collections::HashMap,
    fs::File,
    io::{BufReader, Read},
    rc::Rc,
};

use serde::Deserialize;
use serde_json::Deserializer;

use crate::SpanStats;

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

#[derive(Clone)]
struct SkipperWrapper {
    reader: Rc<RefCell<BufReader<File>>>,
}

impl SkipperWrapper {
    fn from(mut reader: BufReader<File>) -> SkipperWrapper {
        let _ = reader.seek_relative(1);

        Self {
            reader: Rc::new(RefCell::new(reader)),
        }
    }

    fn skip(&self) {
        let _ = self.reader.borrow_mut().seek_relative(1);
    }
}

impl Read for SkipperWrapper {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize, std::io::Error> {
        self.reader.borrow_mut().read(buf)
    }
}

pub fn read_trace(file: String) -> HashMap<String, SpanStats> {
    let file = File::open(file).unwrap();
    let reader = BufReader::new(file);
    let reader_wrapper = SkipperWrapper::from(reader);

    let spans = Deserializer::from_reader(reader_wrapper.clone()).into_iter::<SpanOrIgnore>();

    let mut open_spans: HashMap<String, f32> = HashMap::new();
    let mut all_spans_stats: HashMap<String, SpanStats> = HashMap::new();
    spans
        .flat_map(move |s| {
            reader_wrapper.skip();

            if let Ok(SpanOrIgnore::Span(span)) = s {
                Some(span)
            } else {
                None
            }
        })
        .for_each(|s| {
            if s.ph == "B" {
                open_spans.insert(s.name.clone(), s.ts);
            } else if s.ph == "E" {
                let begin = open_spans.remove(&s.name).unwrap();
                all_spans_stats
                    .entry(s.name)
                    .or_default()
                    .add_span(s.ts - begin);
            }
        });

    all_spans_stats
}
