use alloc::ffi::CString;
use core::fmt::{Debug, Write};
use tracing::{
    field::Field,
    span::{Attributes, Record},
    Event, Id, Level, Subscriber,
};
use tracing_subscriber::{field::Visit, layer::Context, registry::LookupSpan, Layer};

#[derive(Default)]
pub(crate) struct AndroidLayer;

/// Accumulates the fields of a span or event into a single string for the
/// Android log output.
///
/// The first element holds the recorded message and fields. The second element
/// holds the value of the synthesized `log.target` field, if present (see
/// `record_str`).
struct StringRecorder(String, Option<String>);
impl StringRecorder {
    fn new() -> Self {
        StringRecorder(String::new(), None)
    }
}

impl Visit for StringRecorder {
    fn record_str(&mut self, field: &Field, value: &str) {
        // `tracing-log` emits events forwarded from the `log` crate with a fixed
        // metadata target of "log"; the record's real target is only available in
        // the synthesized `log.target` field. Capture it so it can be used as the
        // logcat tag. `str` values are dispatched to `record_str`, so this captures
        // the target exactly as-is.
        if field.name() == "log.target" {
            self.1 = Some(value.to_owned());
        }
        self.record_debug(field, &value);
    }

    fn record_debug(&mut self, field: &Field, value: &dyn Debug) {
        if field.name() == "message" {
            if !self.0.is_empty() {
                self.0 = format!("{:?}\n{}", value, self.0)
            } else {
                self.0 = format!("{:?}", value)
            }
        } else {
            // Separate fields from anything already recorded (e.g. the message),
            // so that the first field does not get glued to it.
            if !self.0.is_empty() {
                write!(self.0, " ").unwrap();
            }
            write!(self.0, "{} = {:?};", field.name(), value).unwrap();
        }
    }
}

impl Default for StringRecorder {
    fn default() -> Self {
        StringRecorder::new()
    }
}

impl<S: Subscriber + for<'a> LookupSpan<'a>> Layer<S> for AndroidLayer {
    fn on_new_span(&self, attrs: &Attributes<'_>, id: &Id, ctx: Context<'_, S>) {
        let mut new_debug_record = StringRecorder::new();
        attrs.record(&mut new_debug_record);

        if let Some(span_ref) = ctx.span(id) {
            span_ref
                .extensions_mut()
                .insert::<StringRecorder>(new_debug_record);
        }
    }

    fn on_record(&self, id: &Id, values: &Record<'_>, ctx: Context<'_, S>) {
        if let Some(span_ref) = ctx.span(id) {
            if let Some(debug_record) = span_ref.extensions_mut().get_mut::<StringRecorder>() {
                values.record(debug_record);
            }
        }
    }

    #[allow(unsafe_code)]
    fn on_event(&self, event: &Event<'_>, _ctx: Context<'_, S>) {
        fn sanitize(string: &str) -> CString {
            let bytes: Vec<u8> = string
                .as_bytes()
                .into_iter()
                .copied()
                .filter(|byte| *byte != 0)
                .collect();
            CString::new(bytes).unwrap()
        }

        let mut recorder = StringRecorder::new();
        event.record(&mut recorder);
        let meta = event.metadata();
        let priority = match *meta.level() {
            Level::TRACE => android_log_sys::LogPriority::VERBOSE,
            Level::DEBUG => android_log_sys::LogPriority::DEBUG,
            Level::INFO => android_log_sys::LogPriority::INFO,
            Level::WARN => android_log_sys::LogPriority::WARN,
            Level::ERROR => android_log_sys::LogPriority::ERROR,
        };
        // Use the record's target as the logcat tag so that logs are grouped by the
        // crate or module that emitted them. Prefer the target captured from the
        // synthesized `log.target` field, because `tracing-log` emits events forwarded
        // from the `log` crate with a fixed metadata target of "log". The event name
        // would be useless as a tag too: it is "log event" for all bridged records.
        let tag = recorder.1.as_deref().unwrap_or(meta.target());
        // SAFETY: Called only on Android platforms. priority is guaranteed to be in range of c_int.
        // The provided tag and message are null terminated properly.
        unsafe {
            android_log_sys::__android_log_write(
                priority as android_log_sys::c_int,
                sanitize(tag).as_ptr(),
                sanitize(&recorder.0).as_ptr(),
            );
        }
    }
}
